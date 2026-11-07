//! Tiger spike (T2): TLS connect, a scratch hypertable, one COPY of 10,000 rows, count them back, drop the table.
//! Prints one tiger_ok or tiger_fail line on stdout. The connection string is never printed.

use std::io::Cursor;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use aether_engine::tiger;
use futures_util::SinkExt;

const TABLE: &str = "aether_spike_t2";
const ROWS: i64 = 10_000;
const STEP_US: i64 = 20_000;
const DAY_US: i64 = 86_400_000_000;

struct Outcome {
    tls: bool,
    hypertable: bool,
    copied: u64,
    counted: i64,
    rows_per_s: i64,
    dropped: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    dotenvy::dotenv().ok();
    let url = std::env::var("TIGER_DATABASE_URL")?;
    let client = tiger::connect(&url).await?;

    let o = match run(&client).await {
        Ok(o) => o,
        Err(e) => {
            println!("tiger_fail error=see_stderr");
            eprintln!("{e:#}");
            return Ok(ExitCode::from(1));
        }
    };
    let ok = o.tls && o.hypertable && o.copied == ROWS as u64 && o.counted == ROWS && o.rows_per_s >= 1 && o.dropped;
    let line = format!(
        "tls={} hypertable={} counted={} rows_per_s={} dropped={}",
        u8::from(o.tls),
        u8::from(o.hypertable),
        o.counted,
        o.rows_per_s,
        u8::from(o.dropped),
    );
    if ok {
        println!("tiger_ok {line}");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("tiger_fail copied={} {line}", o.copied);
        Ok(ExitCode::from(1))
    }
}

async fn run(client: &tokio_postgres::Client) -> anyhow::Result<Outcome> {
    client.batch_execute(&format!("DROP TABLE IF EXISTS {TABLE}")).await?;
    let tls: bool = client
        .query_one("SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()", &[])
        .await?
        .get(0);

    let version: String = client
        .query_one("SELECT extversion FROM pg_extension WHERE extname = 'timescaledb'", &[])
        .await?
        .get(0);
    eprintln!("timescaledb_version={version}");
    let mut parts = version.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let since_2_13 = (parts.next().unwrap_or(0), parts.next().unwrap_or(0)) >= (2, 13);
    let dimension = if since_2_13 { "by_range('ts')" } else { "'ts'" };

    client
        .batch_execute(&format!(
            "CREATE TABLE {TABLE} (ts timestamptz not null, drone smallint not null, value double precision not null)"
        ))
        .await?;
    client.execute(&format!("SELECT create_hypertable('{TABLE}', {dimension})"), &[]).await?;
    let hypertable: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM timescaledb_information.hypertables WHERE hypertable_name = $1)",
            &[&TABLE],
        )
        .await?
        .get(0);

    // Rows end at the current time, 20 ms apart, all in one COPY.
    let now_us = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as i64;
    let mut text = String::new();
    for i in 0..ROWS {
        let ts = now_us - (ROWS - 1 - i) * STEP_US;
        let value = (i as f64 * 0.01).sin() * 100.0;
        text.push_str(&format!("{}\t{}\t{}\n", utc_text(ts), i % 6 + 1, value));
    }
    let copy_sql = format!("COPY {TABLE} (ts, drone, value) FROM STDIN");
    let started = Instant::now();
    let mut sink = Box::pin(client.copy_in::<_, Cursor<Vec<u8>>>(copy_sql.as_str()).await?);
    sink.send(Cursor::new(text.into_bytes())).await?;
    let copied = sink.as_mut().finish().await?;
    let elapsed = started.elapsed();
    let rows_per_s = (ROWS as f64 / elapsed.as_secs_f64()) as i64;
    eprintln!("copy rows={copied} elapsed_ms={:.1}", elapsed.as_secs_f64() * 1e3);

    let counted: i64 = client
        .query_one(&format!("SELECT count(*) FROM {TABLE}"), &[])
        .await?
        .get(0);

    client.batch_execute(&format!("DROP TABLE {TABLE}")).await?;
    let dropped: bool = client
        .query_one(&format!("SELECT to_regclass('{TABLE}') IS NULL"), &[])
        .await?
        .get(0);

    Ok(Outcome { tls, hypertable, copied, counted, rows_per_s, dropped })
}

/// UTC text for Unix microseconds, the form COPY reads for timestamptz (proleptic Gregorian, Hinnant's civil_from_days).
fn utc_text(us: i64) -> String {
    let days = us.div_euclid(DAY_US) + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let rem = us.rem_euclid(DAY_US);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}.{:06}+00",
        rem / 3_600_000_000,
        rem / 60_000_000 % 60,
        rem / 1_000_000 % 60,
        rem % 1_000_000,
    )
}

#[cfg(test)]
mod tests {
    use super::utc_text;

    #[test]
    fn utc_text_known_answers() {
        assert_eq!(utc_text(0), "1970-01-01 00:00:00.000000+00");
        assert_eq!(utc_text(951_782_400_000_000), "2000-02-29 00:00:00.000000+00");
        assert_eq!(utc_text(951_782_400_000_000 + 3_723_000_001), "2000-02-29 01:02:03.000001+00");
    }
}
