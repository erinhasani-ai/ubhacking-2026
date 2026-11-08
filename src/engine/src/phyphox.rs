//! phyphox remote-access client (T5). Parses /config and /get replies, keeps only samples newer than the
//! last one received, and errors if the experiment session changes.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// Buffers the probe reads: time, the x/y/z accelerometer axes and the absolute acceleration.
pub const SENSOR_BUFFERS: [&str; 5] = ["acc_time", "accX", "accY", "accZ", "acc"];

/// Long enough for a slow phone; a hung phone must fail the probe, not stall it.
const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub t: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub magnitude: f64,
}

#[derive(Deserialize)]
struct Config {
    buffers: Vec<NamedBuffer>,
}

#[derive(Deserialize)]
struct NamedBuffer {
    name: String,
}

#[derive(Deserialize)]
struct Get {
    status: Status,
    buffer: HashMap<String, Values>,
}

#[derive(Deserialize)]
struct Status {
    session: String,
}

#[derive(Deserialize)]
struct Values {
    buffer: Vec<f64>,
}

/// Buffer names listed by a /config reply.
pub fn buffer_names(config: &str) -> Result<Vec<String>> {
    let config: Config = serde_json::from_str(config).context("parse /config")?;
    Ok(config.buffers.into_iter().map(|b| b.name).collect())
}

/// Parses a /get reply into its session id and its samples, in reply order.
pub fn parse_get(reply: &str) -> Result<(String, Vec<Sample>)> {
    let get: Get = serde_json::from_str(reply).context("parse /get")?;
    let column = |name: &str| -> Result<&Vec<f64>> {
        get.buffer.get(name).map(|v| &v.buffer).with_context(|| format!("/get lacks buffer {name}"))
    };
    let (t, x, y, z, m) = (column("acc_time")?, column("accX")?, column("accY")?, column("accZ")?, column("acc")?);
    if [x.len(), y.len(), z.len(), m.len()].iter().any(|&n| n != t.len()) {
        bail!("/get buffers differ in length");
    }
    let samples = (0..t.len())
        .map(|i| Sample { t: t[i], x: x[i], y: y[i], z: z[i], magnitude: m[i] })
        .collect();
    Ok((get.status.session, samples))
}

/// Remembers the session and the last sample time across /get replies.
#[derive(Debug, Default)]
pub struct Reader {
    session: Option<String>,
    last_t: Option<f64>,
}

impl Reader {
    /// Time of the last sample received, if any.
    pub fn last_t(&self) -> Option<f64> {
        self.last_t
    }

    /// Returns the samples in `reply` newer than every sample received before. Errors if the session changed.
    pub fn feed(&mut self, reply: &str) -> Result<Vec<Sample>> {
        let (session, samples) = parse_get(reply)?;
        if self.session.as_ref().is_some_and(|prev| *prev != session) {
            bail!("phyphox session changed to {session}");
        }
        self.session = Some(session);
        let mut fresh = Vec::new();
        for s in samples {
            if self.last_t.is_none_or(|last| s.t > last) {
                self.last_t = Some(s.t);
                fresh.push(s);
            }
        }
        Ok(fresh)
    }
}

pub struct Client {
    http: reqwest::Client,
    base: String,
}

impl Client {
    pub fn new(base: &str) -> Result<Self> {
        let http = reqwest::Client::builder().timeout(TIMEOUT).build()?;
        Ok(Self { http, base: base.trim_end_matches('/').to_string() })
    }

    async fn text(&self, path: &str) -> Result<String> {
        let url = format!("{}/{path}", self.base);
        let reply = self.http.get(&url).send().await.with_context(|| format!("GET {url}"))?;
        Ok(reply.error_for_status()?.text().await?)
    }

    async fn control(&self, cmd: &str) -> Result<()> {
        let reply: serde_json::Value = serde_json::from_str(&self.text(&format!("control?cmd={cmd}")).await?)
            .with_context(|| format!("parse /control?cmd={cmd}"))?;
        if reply["result"] != true {
            bail!("phyphox refused {cmd}: {reply}");
        }
        Ok(())
    }

    pub async fn buffer_names(&self) -> Result<Vec<String>> {
        buffer_names(&self.text("config").await?)
    }

    pub async fn clear(&self) -> Result<()> {
        self.control("clear").await
    }

    pub async fn start(&self) -> Result<()> {
        self.control("start").await
    }

    pub async fn stop(&self) -> Result<()> {
        self.control("stop").await
    }

    /// One /get asking only for samples newer than the reader's last one (-1 before the first sample).
    /// The `|acc_time` threshold is sent as a literal pipe, the form verified on the live phone (a
    /// percent-encoded pipe gave conflicting results in two tests, so it is avoided; unverified).
    pub async fn poll(&self, reader: &mut Reader) -> Result<Vec<Sample>> {
        let t = reader.last_t().unwrap_or(-1.0);
        let reply = self
            .text(&format!("get?acc_time={t}&accX={t}|acc_time&accY={t}|acc_time&accZ={t}|acc_time&acc={t}|acc_time"))
            .await?;
        reader.feed(&reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Replies from the iPhone (phyphox "Acceleration with g"), trimmed to a few samples. CONFIG, FIRST and
    // EMPTY are real replies; SECOND is a window of real samples overlapping FIRST (the phone itself only
    // returns samples newer than the threshold); CHANGED has its session edited by hand; RAGGED is hand-written.
    const CONFIG: &str = r#"{"localCategory":"Raw Sensors","category":"Raw Sensors","buffers":[{"size":0,"name":"accZ"},{"size":0,"name":"accX"},{"size":0,"name":"accY"},{"size":0,"name":"acc"},{"size":0,"name":"acc_time"}],"localTitle":"Acceleration with g","crc32":"18545a17","title":"Acceleration with g"}"#;

    const FIRST: &str = r#"{"buffer":{"accX":{"updateMode":"partial","size":0,"buffer":[0.03861968994140625,0.02963836669921875,0.04296066284179688]},"accY":{"updateMode":"partial","size":0,"buffer":[0.18396743774414062,0.19294876098632813,0.17004638671875]},"accZ":{"updateMode":"partial","size":0,"buffer":[9.774224395751954,9.777068481445314,9.769284667968751]},"acc":{"updateMode":"partial","size":0,"buffer":[9.776031814448741,9.779017115641945,9.770858934402709]},"acc_time":{"updateMode":"partial","size":0,"buffer":[3.1467000832781196,3.1566700832918286,3.1666400833055377]}},"status":{"countDown":0,"session":"2443520","measuring":true,"timedRun":false}}"#;

    // Overlaps FIRST by two samples and adds one new sample at t=3.1766100833192468.
    const SECOND: &str = r#"{"buffer":{"accX":{"updateMode":"partial","size":0,"buffer":[0.02963836669921875,0.04296066284179688,0.0568817138671875]},"accY":{"updateMode":"partial","size":0,"buffer":[0.19294876098632813,0.17004638671875,0.1742376708984375]},"accZ":{"updateMode":"partial","size":0,"buffer":[9.777068481445314,9.769284667968751,9.77736785888672]},"acc":{"updateMode":"partial","size":0,"buffer":[9.779017115641945,9.770858934402709,9.779085670108616]},"acc_time":{"updateMode":"partial","size":0,"buffer":[3.1566700832918286,3.1666400833055377,3.1766100833192468]}},"status":{"countDown":0,"session":"2443520","measuring":true,"timedRun":false}}"#;

    // Same samples as SECOND, but from a new experiment session.
    const CHANGED: &str = r#"{"buffer":{"accX":{"updateMode":"partial","size":0,"buffer":[0.02963836669921875]},"accY":{"updateMode":"partial","size":0,"buffer":[0.19294876098632813]},"accZ":{"updateMode":"partial","size":0,"buffer":[9.777068481445314]},"acc":{"updateMode":"partial","size":0,"buffer":[9.779017115641945]},"acc_time":{"updateMode":"partial","size":0,"buffer":[3.1566700832918286]}},"status":{"countDown":0,"session":"9999999","measuring":true,"timedRun":false}}"#;

    // Real reply to a threshold after the newest sample: every buffer empty, status still present.
    const EMPTY: &str = r#"{"buffer":{"accX":{"size":0,"updateMode":"partial","buffer":[]},"accZ":{"updateMode":"partial","buffer":[],"size":0},"acc":{"buffer":[],"size":0,"updateMode":"partial"},"acc_time":{"updateMode":"partial","size":0,"buffer":[]},"accY":{"updateMode":"partial","size":0,"buffer":[]}},"status":{"countDown":0,"session":"2443520","measuring":true,"timedRun":false}}"#;

    // accY is one sample short.
    const RAGGED: &str = r#"{"buffer":{"accX":{"buffer":[0.1,0.2]},"accY":{"buffer":[0.1]},"accZ":{"buffer":[0.1,0.2]},"acc":{"buffer":[9.8,9.8]},"acc_time":{"buffer":[1.0,1.05]}},"status":{"session":"1"}}"#;

    #[test]
    fn phyphox_config_lists_the_buffers() {
        let names = buffer_names(CONFIG).unwrap();
        for b in SENSOR_BUFFERS {
            assert!(names.iter().any(|n| n == b), "missing {b}");
        }
    }

    #[test]
    fn phyphox_keeps_only_new_samples_across_overlapping_replies() {
        let mut reader = Reader::default();
        // serde_json's default float parser may differ by an ulp, so compare with a tolerance.
        let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{a} != {b}");
        let first = reader.feed(FIRST).unwrap();
        assert_eq!(first.len(), 3);
        close(first[0].t, 3.1467000832781196);
        close(first[0].magnitude, 9.776031814448741);

        let second = reader.feed(SECOND).unwrap();
        assert_eq!(second.len(), 1);
        close(second[0].t, 3.1766100833192468);
        close(reader.last_t().unwrap(), 3.1766100833192468);

        // The phone resends the same reply: nothing is new.
        assert!(reader.feed(SECOND).unwrap().is_empty());
    }

    #[test]
    fn phyphox_empty_reply_yields_nothing() {
        let mut reader = Reader::default();
        assert!(reader.feed(EMPTY).unwrap().is_empty());
        assert_eq!(reader.last_t(), None);
    }

    #[test]
    fn phyphox_changed_session_is_an_error() {
        let mut reader = Reader::default();
        reader.feed(SECOND).unwrap();
        let err = reader.feed(CHANGED).unwrap_err();
        assert!(err.to_string().contains("session changed to 9999999"), "{err}");
    }

    #[test]
    fn phyphox_malformed_reply_is_an_error_not_a_panic() {
        let mut reader = Reader::default();
        assert!(reader.feed(r#"{"buffer":{"accX":{"buffer":[0.1"#).is_err());
        assert!(reader.feed(r#"{"buffer":{}}"#).is_err());
        assert!(reader.feed(RAGGED).is_err());
        assert_eq!(reader.last_t(), None);
    }
}
