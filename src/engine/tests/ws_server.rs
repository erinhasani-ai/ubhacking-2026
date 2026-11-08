//! T14.G1: the Stream protocol server with a stand-in Mission and the six simulated Drones of sim_swarm.
//! Simulated Drones only; nothing here is flown on hardware.

use std::process::Child;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aether_core::command::{Command, Source, Verdict};
use aether_engine::gcs::Gcs;
use aether_engine::server::{serve, Config};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;
const CLOUD_POINTS: usize = 1000;

/// Kills sim_swarm on every path out of the test, a panic included.
struct Sim(Child);
impl Drop for Sim {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn next(ws: &mut Ws) -> Message {
    tokio::time::timeout(Duration::from_secs(5), ws.next()).await.expect("a frame within 5 s").expect("socket open").expect("frame ok")
}

async fn next_json(ws: &mut Ws) -> Value {
    match next(ws).await {
        Message::Text(t) => serde_json::from_str(t.as_str()).expect("text frames are JSON"),
        other => panic!("expected a text frame, got {other:?}"),
    }
}

/// Skips state frames until a frame of `kind` arrives.
async fn next_of(ws: &mut Ws, kind: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "timed out waiting for {kind}");
        let v = next_json(ws).await;
        if v["type"] == kind {
            return v;
        }
        assert_eq!(v["type"], "state", "unexpected frame {v}");
    }
}

async fn send(ws: &mut Ws, v: Value) {
    ws.send(Message::text(v.to_string())).await.unwrap();
}

async fn http_get(port: u16, path: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).await.unwrap();
    out
}

#[tokio::test]
async fn stream_protocol_end_to_end() {
    let gcs = Arc::new(Gcs::bind("127.0.0.1:0").await.expect("bind the ground station"));
    let gcs_port = gcs.local_addr().unwrap().port().to_string();
    let _sim = Sim(
        std::process::Command::new(env!("CARGO_BIN_EXE_sim_swarm")).args(["--count", "6", "--gcs-port", &gcs_port]).spawn().expect("start sim_swarm"),
    );
    let t = Instant::now();
    while !(gcs.drones().len() == 6 && gcs.drones().values().all(|d| d.position.is_some() && d.mode.is_some())) {
        assert!(t.elapsed() < Duration::from_secs(30), "six Drones heard within 30 s");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Stand-in Mission: approves every well-formed Command and publishes it.
    let (inbox, mut rx) = mpsc::channel::<(Source, Command, oneshot::Sender<Verdict>)>(16);
    let (verdicts, _keep) = broadcast::channel(16);
    let (resets, mut reset_rx) = mpsc::channel::<oneshot::Sender<()>>(8);
    tokio::spawn(async move { while let Some(reply) = reset_rx.recv().await { let _ = reply.send(()); } });
    let published = verdicts.clone();
    tokio::spawn(async move {
        while let Some((source, command, reply)) = rx.recv().await {
            let verdict = Verdict { approved: true, reason: None };
            let _ = published.send((source, command, verdict.clone()));
            let _ = reply.send(verdict);
        }
    });

    let dir = std::env::temp_dir().join(format!("aether-ws-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<h1>aether</h1>").unwrap();
    let secret = std::env::temp_dir().join(format!("aether-ws-{}-secret.txt", std::process::id()));
    std::fs::write(&secret, "SECRET").unwrap();

    let cloud: Vec<f32> = (0..CLOUD_POINTS * 3).map(|i| i as f32 * 0.5).collect();
    let hello = json!({"type": "hello", "swarm_size": 6, "room": {"provenance": "LIVE", "min": [0, 0, -3], "max": [5, 5, 0]},
        "pad": [[0, 0, 0]], "places": [{"name": "door", "n": 1.0, "e": 2.0, "d": 0.0}]});
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(serve(listener, Config { hello: hello.to_string(), cloud: cloud.clone(), mesh: Vec::new(), gcs, inbox, verdicts, resets, static_dir: Some(dir.clone()) }));

    let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}/ws")).await.expect("connect");

    // hello, then ONE binary frame of the right length and content
    assert_eq!(next_json(&mut ws).await, hello);
    match next(&mut ws).await {
        Message::Binary(b) => {
            assert_eq!(b.len(), CLOUD_POINTS * 3 * 4);
            assert_eq!(f32::from_le_bytes(b[4..8].try_into().unwrap()), cloud[1]);
        }
        other => panic!("expected the binary cloud, got {other:?}"),
    }

    // state frames: shape and rate over 2 s
    let start = Instant::now();
    let mut frames = 0u32;
    let mut last_t = 0;
    while start.elapsed() < Duration::from_secs(2) {
        let v = next_json(&mut ws).await;
        assert_eq!(v["type"], "state");
        let t_ms = v["t_ms"].as_u64().unwrap();
        assert!(t_ms >= last_t);
        last_t = t_ms;
        let drones = v["drones"].as_array().unwrap();
        assert_eq!(drones.len(), 6);
        for (i, d) in drones.iter().enumerate() {
            assert_eq!(d["id"], i as u64 + 1);
            assert_eq!(d["provenance"], "SIM");
            for k in ["n", "e", "d", "roll", "pitch", "yaw", "vn", "ve", "vd"] {
                assert!(d[k].is_number(), "{k} in {d}");
            }
            assert!(d["mode"].is_string() && d["armed"].is_boolean() && d["link_ms"].is_u64());
        }
        frames += 1;
    }
    let hz = frames as f64 / start.elapsed().as_secs_f64();
    println!("ws_server state rate: {hz:.1} Hz ({frames} frames)");
    assert!(hz >= 25.0, "state rate {hz:.1} Hz");

    // a click dispatch gets its verdict
    let cmd = json!({"kind": "dispatch", "drones": {"ids": [1, 2]}, "target": {"point": {"n": 1.0, "e": 1.0, "d": -1.0}}});
    send(&mut ws, json!({"type": "command", "source": "click", "command": cmd})).await;
    let v = next_of(&mut ws, "verdict").await;
    assert_eq!((v["source"].as_str(), v["approved"].as_bool()), (Some("click"), Some(true)));
    assert_eq!(v["command"], cmd);
    assert!(v["reason"].is_null());

    // a malformed command is rejected with a reason and never reaches the Mission
    send(&mut ws, json!({"type": "command", "source": "click", "command": {"kind": "dispatch"}})).await;
    let v = next_of(&mut ws, "verdict").await;
    assert_eq!(v["approved"], false);
    assert!(v["reason"].as_str().is_some_and(|r| !r.is_empty()));

    // Value parsing must not erase duplicate keys before the validated Command parser sees them.
    ws.send(Message::text(r#"{"type":"command","source":"click","command":{"kind":"dispatch","drones":{"count":1,"count":2},"target":{"place":"door"}}}"#)).await.unwrap();
    let v = next_of(&mut ws, "verdict").await;
    assert_eq!(v["approved"], false);
    assert!(v["reason"].as_str().is_some_and(|r| !r.is_empty()));

    // Reset has its own acknowledgment and does not pretend a Recall was approved.
    send(&mut ws, json!({"type": "reset"})).await;
    let v = next_of(&mut ws, "reset").await;
    assert_eq!(v["ok"], true);

    // unknown types and malformed JSON are ignored; the stream keeps going
    send(&mut ws, json!({"type": "teleport"})).await;
    ws.send(Message::text("not json {")).await.unwrap();
    assert_eq!(next_of(&mut ws, "state").await["drones"].as_array().unwrap().len(), 6);

    // static files: index.html at /, traversal refused
    let r = http_get(port, "/").await;
    assert!(r.starts_with("HTTP/1.1 200") && r.to_lowercase().contains("content-type: text/html") && r.ends_with("<h1>aether</h1>"), "{r}");
    let name = secret.file_name().unwrap().to_str().unwrap();
    for bad in [format!("/../{name}"), format!("//{}", secret.display())] {
        let r = http_get(port, &bad).await;
        assert!(!r.starts_with("HTTP/1.1 200") && !r.contains("SECRET"), "{bad}: {r}");
    }

    // clean close
    ws.close(None).await.unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(&secret);
}
