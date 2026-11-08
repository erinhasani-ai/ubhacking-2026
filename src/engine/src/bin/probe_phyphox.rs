//! Streams the live phyphox experiment for 5 s from the iPhone and reports the sample rate and round-trip time.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use aether_engine::phyphox::{Client, Reader, Sample, SENSOR_BUFFERS};
use anyhow::{bail, Context, Result};

const RUN: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);
const MIN_SAMPLES: usize = 100;

#[derive(Default)]
struct Run {
    samples: Vec<Sample>,
    rtt_ms: Vec<f64>,
}

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    let mut run = Run::default();
    let result = probe(&mut run).await;
    let (samples, rate_hz, rtt_ms) = measured(&run);
    match result {
        Ok(()) if samples >= MIN_SAMPLES => {
            println!("phyphox_ok samples={samples} rate_hz={rate_hz:.0} rtt_ms={rtt_ms:.0}");
            ExitCode::SUCCESS
        }
        Ok(()) => {
            println!("phyphox_fail samples={samples} rate_hz={rate_hz:.0} rtt_ms={rtt_ms:.0} reason=fewer than {MIN_SAMPLES} samples");
            ExitCode::from(1)
        }
        Err(e) => {
            println!("phyphox_fail samples={samples} rate_hz={rate_hz:.0} rtt_ms={rtt_ms:.0} reason={e:#}");
            ExitCode::from(1)
        }
    }
}

async fn probe(run: &mut Run) -> Result<()> {
    let phone = Client::new(&std::env::var("PHYPHOX_URL").context("PHYPHOX_URL is not set")?)?;
    let names = phone.buffer_names().await?;
    let missing: Vec<&str> = SENSOR_BUFFERS.into_iter().filter(|b| !names.iter().any(|n| n == b)).collect();
    if !missing.is_empty() {
        bail!("phyphox experiment lacks buffers {missing:?}");
    }
    let streamed = stream(&phone, run).await;
    // Stop even after a failed stream so the phone is not left recording.
    let stopped = phone.stop().await;
    streamed?;
    stopped
}

async fn stream(phone: &Client, run: &mut Run) -> Result<()> {
    phone.clear().await?;
    phone.start().await?;
    let mut reader = Reader::default();
    let mut tick = tokio::time::interval(POLL);
    let began = Instant::now();
    while began.elapsed() < RUN {
        tick.tick().await;
        let asked = Instant::now();
        let fresh = phone.poll(&mut reader).await?;
        run.rtt_ms.push(asked.elapsed().as_secs_f64() * 1e3);
        run.samples.extend(fresh);
    }
    Ok(())
}

/// Sample count, rate from the sample timestamps (count / span), and median /get round trip.
fn measured(run: &Run) -> (usize, f64, f64) {
    let span = match (run.samples.first(), run.samples.last()) {
        (Some(first), Some(last)) => last.t - first.t,
        _ => 0.0,
    };
    let rate = if span > 0.0 { run.samples.len() as f64 / span } else { 0.0 };
    let mut rtt = run.rtt_ms.clone();
    rtt.sort_by(f64::total_cmp);
    let mid = rtt.len() / 2;
    let median = match rtt.len() {
        0 => 0.0,
        n if n % 2 == 0 => (rtt[mid - 1] + rtt[mid]) / 2.0,
        _ => rtt[mid],
    };
    (run.samples.len(), rate, median)
}
