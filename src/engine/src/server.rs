//! Live stream to the browser: the Stream protocol (SPEC decisions log) over one WebSocket at `/ws`,
//! plus the built UI served by hand from an optional directory at `/`.
//!
//! On connect each client gets, in order: the hello text frame, the Room Twin point cloud as ONE binary
//! frame (little-endian f32 n,e,d triples), then state frames at 30 Hz and verdict frames. State frames are
//! built fresh at send time, so a slow client skips ticks (drops stale frames) and never delays another.
//! Every Drone here is simulated, so every state entry says `"provenance":"SIM"`.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use aether_core::command::{Command, Source, Verdict};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::gcs::Gcs;

/// 30 Hz.
const STATE_PERIOD: Duration = Duration::from_micros(33_333);

pub type Inbox = mpsc::Sender<(Source, Command, oneshot::Sender<Verdict>)>;

pub struct Config {
    /// The hello frame text, built by the caller from the frame, the pad slots and the Places.
    pub hello: String,
    /// Room Twin point cloud as n,e,d triples; sent as one binary frame.
    pub cloud: Vec<f32>,
    /// Room Twin surface for the holographic layer at `GET /api/mesh`: u32 vertex count, u32 triangle count,
    /// then n,e,d f32 per vertex and three u32 indices per triangle (little-endian). Empty answers 404.
    pub mesh: Vec<u8>,
    pub gcs: Arc<Gcs>,
    /// The Mission inbox. Answers come back on `verdicts`.
    pub inbox: Inbox,
    /// Every answered Command; each connection forwards it as a verdict frame.
    pub verdicts: broadcast::Sender<(Source, Command, Verdict)>,
    pub resets: crate::mission::Resets,
    /// Built UI directory served at `/`; None serves nothing there.
    pub static_dir: Option<PathBuf>,
}

struct Shared {
    hello: String,
    cloud: Vec<u8>,
    mesh: axum::body::Bytes,
    gcs: Arc<Gcs>,
    inbox: Inbox,
    verdicts: broadcast::Sender<(Source, Command, Verdict)>,
    resets: crate::mission::Resets,
    static_dir: Option<PathBuf>,
    /// The engine clock: `t_ms` in state frames counts from here.
    boot: Instant,
}

/// Serves until the task is dropped. The caller binds (127.0.0.1:7700; tests use port 0).
pub async fn serve(listener: TcpListener, cfg: Config) {
    let shared = Arc::new(Shared {
        hello: cfg.hello,
        cloud: cfg.cloud.iter().flat_map(|v| v.to_le_bytes()).collect(),
        mesh: cfg.mesh.into(),
        gcs: cfg.gcs,
        inbox: cfg.inbox,
        verdicts: cfg.verdicts,
        resets: cfg.resets,
        static_dir: cfg.static_dir,
        boot: Instant::now(),
    });
    let app = Router::new()
        .route("/ws", get(|ws: WebSocketUpgrade, State(s): State<Arc<Shared>>| async move { ws.on_upgrade(move |sock| connection(sock, s)) }))
        .route("/api/tts", post(tts))
        .route("/api/stt", post(stt))
        .route("/api/accel", get(accel))
        .route("/api/mesh", get(|State(s): State<Arc<Shared>>| async move {
            if s.mesh.is_empty() { return StatusCode::NOT_FOUND.into_response(); }
            ([(axum::http::header::CONTENT_TYPE, "application/octet-stream")], s.mesh.clone()).into_response()
        }))
        .fallback(static_file)
        .with_state(shared);
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("server: {e}");
    }
}

// shortcut: symlinks inside the static directory are followed; the directory is our own build output.
async fn static_file(State(s): State<Arc<Shared>>, uri: Uri) -> Response {
    let Some(dir) = &s.static_dir else { return StatusCode::NOT_FOUND.into_response() };
    let rel = uri.path().strip_prefix('/').unwrap_or(uri.path());
    let rel = if rel.is_empty() { "index.html" } else { rel };
    // Only plain names: refuses `..`, absolute paths and drive prefixes. Percent-escapes are not decoded.
    if !Path::new(rel).components().all(|c| matches!(c, Component::Normal(_))) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = dir.join(rel);
    let content_type = match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("wasm") => "application/wasm",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, content_type)], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// Campus voice (T32). JSON and audio/* bodies are not CORS-safelisted, so another site's page cannot call these
// without a preflight, which this server never answers.
#[derive(Deserialize)]
struct TtsRequest {
    text: String,
}

async fn tts(axum::Json(req): axum::Json<TtsRequest>) -> Response {
    let text = req.text.trim();
    if text.is_empty() || text.chars().count() > 600 {
        return (StatusCode::BAD_REQUEST, "text must be 1-600 characters").into_response();
    }
    match crate::voice::speak_mp3(text).await {
        Ok(mp3) => ([(header::CONTENT_TYPE, "audio/mpeg")], mp3).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

async fn stt(headers: axum::http::HeaderMap, body: axum::body::Bytes) -> Response {
    let mime = headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
    if !mime.starts_with("audio/") || body.is_empty() {
        return (StatusCode::BAD_REQUEST, "send one recorded utterance as audio/*").into_response();
    }
    match crate::voice::transcribe(body.to_vec(), mime).await {
        Ok(text) => axum::Json(json!({ "text": text })).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

// Phone accelerometer for the campus quake trigger (T33): the engine polls phyphox (PHYPHOX_URL) so the browser
// never talks to the phone. Each GET returns the [t, |a|] samples since the previous call; a failure drops the
// session so the next call reconnects.
static PHONE: std::sync::OnceLock<tokio::sync::Mutex<Option<(crate::phyphox::Client, crate::phyphox::Reader)>>> = std::sync::OnceLock::new();

async fn accel() -> Response {
    let mut phone = PHONE.get_or_init(|| tokio::sync::Mutex::new(None)).lock().await;
    if phone.is_none() {
        let opened = async {
            let url = std::env::var("PHYPHOX_URL").map_err(|_| anyhow::anyhow!("PHYPHOX_URL is not set"))?;
            let c = crate::phyphox::Client::new(&url)?;
            c.clear().await?;
            c.start().await?;
            anyhow::Ok(c)
        }
        .await;
        match opened {
            Ok(c) => *phone = Some((c, crate::phyphox::Reader::default())),
            Err(e) => return (StatusCode::BAD_GATEWAY, format!("phone not reachable: {e}")).into_response(),
        }
    }
    let (c, r) = phone.as_mut().expect("opened above");
    match c.poll(r).await {
        Ok(samples) => axum::Json(json!({ "samples": samples.iter().map(|s| [s.t, s.magnitude]).collect::<Vec<_>>() })).into_response(),
        Err(e) => {
            *phone = None;
            (StatusCode::BAD_GATEWAY, format!("phone read failed: {e}")).into_response()
        }
    }
}

fn state_frame(s: &Shared) -> String {
    let now = Instant::now();
    let drones: Vec<Value> = s
        .gcs
        .drones()
        .into_iter()
        .filter_map(|(id, d)| {
            let p = d.position.as_ref()?; // no position yet: not drawable
            let (roll, pitch, yaw) = d.attitude.as_ref().map_or((0.0, 0.0, 0.0), |a| (a.roll, a.pitch, a.yaw));
            Some(json!({
                "id": id, "n": p.x, "e": p.y, "d": p.z, "roll": roll, "pitch": pitch, "yaw": yaw,
                "vn": p.vx, "ve": p.vy, "vd": p.vz, "mode": d.mode.unwrap_or("UNKNOWN"), "armed": d.armed,
                "link_ms": d.link_age(now).as_millis() as u64, "provenance": "SIM",
            }))
        })
        .collect();
    json!({"type": "state", "t_ms": now.duration_since(s.boot).as_millis() as u64, "drones": drones}).to_string()
}

fn verdict_frame(source: Source, command: Value, verdict: &Verdict) -> String {
    json!({"type": "verdict", "source": source, "command": command,
           "approved": verdict.approved, "reason": verdict.reason})
    .to_string()
}

/// A Command from the browser: `source` must be click or key, the inner `command` goes through
/// `Command::from_json`. Err carries the offending command JSON (or null) and the reason.
fn parse_command(text: &str, msg: &Value) -> Result<(Source, Command), (Value, String)> {
    let raw = msg.get("command").cloned().unwrap_or(Value::Null);
    let source = match msg.get("source").and_then(Value::as_str) {
        Some("click") => Source::Click,
        Some("key") => Source::Key,
        _ => return Err((raw, "source must be \"click\" or \"key\"".into())),
    };
    // Validate wire shape and values first, then deserialize the original text as well:
    // Value keeps only the last duplicate key, while typed deserialization rejects duplicates.
    Command::from_json(&raw.to_string()).map_err(|e| (raw.clone(), e))?;
    #[derive(Deserialize)]
    struct Envelope {
        #[serde(rename = "type")]
        _kind: String,
        source: Source,
        command: Command,
    }
    let envelope: Envelope = serde_json::from_str(text).map_err(|e| (raw, e.to_string()))?;
    debug_assert_eq!(source, envelope.source);
    Ok((envelope.source, envelope.command))
}

/// Hands one Command to the Mission. The Verdict reaches every client on the broadcast; the oneshot
/// reply is awaited only to keep the Mission's answer channel open.
fn to_mission(s: &Shared, source: Source, command: Command) {
    let inbox = s.inbox.clone();
    tokio::spawn(async move {
        let (tx, rx) = oneshot::channel();
        if inbox.send((source, command, tx)).await.is_ok() {
            let _ = rx.await;
        }
    });
}

/// Handles one incoming text frame; returns a frame to send back to this client only, if any.
/// Unknown types and malformed JSON are ignored (`warned` logs the first one per connection).
fn on_text(s: &Shared, text: &str, warned: &mut bool) -> Option<String> {
    let msg: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    match msg.get("type").and_then(Value::as_str) {
        Some("command") => match parse_command(text, &msg) {
            Ok((source, command)) => {
                to_mission(s, source, command);
                None
            }
            // A malformed Command never reaches the Mission; only this client hears about it.
            Err((raw, reason)) => {
                let verdict = Verdict { approved: false, reason: Some(reason) };
                Some(verdict_frame(if msg["source"] == "key" { Source::Key } else { Source::Click }, raw, &verdict))
            }
        },
        _ => {
            if !*warned {
                *warned = true;
                eprintln!("server: ignoring a message that is not valid JSON or has an unknown type (logged once per connection)");
            }
            None
        }
    }
}

async fn connection(mut ws: WebSocket, s: Arc<Shared>) {
    let mut verdicts = s.verdicts.subscribe(); // before hello, so no verdict is missed
    if ws.send(Message::Text(s.hello.clone().into())).await.is_err()
        || ws.send(Message::Binary(s.cloud.clone().into())).await.is_err()
    {
        return;
    }
    let mut tick = tokio::time::interval(STATE_PERIOD);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut warned = false;
    loop {
        let out = tokio::select! {
            _ = tick.tick() => Some(state_frame(&s)),
            v = verdicts.recv() => match v {
                Ok((source, command, verdict)) => {
                    let command = serde_json::from_str(&command.to_json()).unwrap_or(Value::Null);
                    Some(verdict_frame(source, command, &verdict))
                }
                Err(broadcast::error::RecvError::Lagged(_)) => None,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            m = ws.recv() => match m {
                Some(Ok(Message::Text(t))) => {
                    if serde_json::from_str::<Value>(t.as_str()).ok().is_some_and(|v| v["type"] == "reset") {
                        let (reply, done) = oneshot::channel();
                        let ok = s.resets.send(reply).await.is_ok() && done.await.is_ok();
                        Some(json!({"type": "reset", "ok": ok}).to_string())
                    } else {
                        on_text(&s, t.as_str(), &mut warned)
                    }
                },
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => None, // ping/pong are answered by the library; binary from a client is ignored
            },
        };
        if let Some(text) = out {
            if ws.send(Message::Text(text.into())).await.is_err() {
                return;
            }
        }
    }
    let _ = ws.send(Message::Close(None)).await;
}
