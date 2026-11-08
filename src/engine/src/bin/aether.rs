//! Local Aether demo: a real scanned room and six simulated MAVLink Drones. Not flown on hardware.

use std::{path::PathBuf, sync::Arc, time::{Duration, Instant}};

use aether_core::{frame, places, planner::DistanceField, room_twin::{load_obj, OccupancyGrid}};
use aether_engine::{gcs::Gcs, mavlink_link::{self, Ground}, mission, server};
use anyhow::{bail, Context, Result};
use serde_json::json;
use tokio::net::TcpListener;

/// Planning voxel. 0.2 m keeps A* on a 60 m floor scan to about a second; clearance stays 0.35 m.
const VOXEL_M: f64 = 0.2;

#[tokio::main]
async fn main() -> Result<()> {
    let (mut port, mut gcs_port) = (7700u16, 14550u16);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => port = args.next().context("--port needs a number")?.parse()?,
            "--gcs-port" => gcs_port = args.next().context("--gcs-port needs a number")?.parse()?,
            _ => bail!("usage: aether [--port P] [--gcs-port P]"),
        }
    }
    dotenvy::dotenv().ok();
    // Release layout: the scan as `room.obj` and the UI in `web/` next to the binary; env vars and dev paths win first.
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
    let scan = std::env::var("ROOM_SCAN_PATH")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| exe_dir.as_ref().map(|d| d.join("room.obj")).filter(|p| p.exists()).map(|p| p.display().to_string()))
        .context("ROOM_SCAN_PATH is not set and there is no room.obj next to aether")?;
    let mesh = load_obj(&std::fs::read(scan).context("reading the Room Twin")?)?;
    let grid = Arc::new(OccupancyGrid::from_mesh(&mesh, VOXEL_M).map_err(anyhow::Error::msg)?);
    eprintln!("aether: scan loaded; preparing distance field");
    let field = Arc::new(DistanceField::from_grid(&grid));
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in &mesh.vertices {
        let p = frame::scan_to_mission(*p);
        for k in 0..3 { min[k] = min[k].min(p[k]); max[k] = max[k].max(p[k]); }
    }
    let stride = mesh.vertices.len().div_ceil(200_000).max(1);
    let cloud = mesh.vertices.iter().step_by(stride)
        .flat_map(|&p| frame::scan_to_mission(p).map(|v| v as f32)).collect();
    // The holographic layer: the whole scanned surface in the Mission frame, served once at /api/mesh.
    let mut mesh_bytes = Vec::with_capacity(8 + mesh.vertices.len() * 12 + mesh.triangles.len() * 12);
    mesh_bytes.extend((mesh.vertices.len() as u32).to_le_bytes());
    mesh_bytes.extend((mesh.triangles.len() as u32).to_le_bytes());
    for &p in &mesh.vertices { for v in frame::scan_to_mission(p) { mesh_bytes.extend((v as f32).to_le_bytes()); } }
    for t in &mesh.triangles { for i in t { mesh_bytes.extend(i.to_le_bytes()); } }
    let pad: Vec<_> = (0..6).map(frame::pad_slot).collect();
    let named: Vec<_> = places::all().iter().map(|p| {
        let [n, e, d] = frame::scan_to_mission([p.x, p.y, p.z]);
        json!({"name": p.name, "n": n, "e": e, "d": d})
    }).collect();
    let hello = json!({"type": "hello", "swarm_size": 6,
        "room": {"provenance": "LIVE", "min": min, "max": max}, "pad": pad, "places": named}).to_string();

    // Bind both ports before starting background simulation threads; a port conflict fails cleanly.
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let address = listener.local_addr()?;
    let gcs = Arc::new(Gcs::bind(("127.0.0.1", gcs_port)).await?);
    let ground_station = gcs.local_addr()?.to_string();
    let mut simulated = Vec::new();
    for i in 0..6 {
        let ground = Ground::Room(Arc::clone(&grid));
        simulated.push(mavlink_link::spawn_on_ground(i + 1, frame::pad_slot(i as usize), ground, &ground_station)?);
    }
    let started = Instant::now();
    loop {
        let drones = gcs.drones();
        if drones.len() == 6 && drones.values().all(|d| d.position.is_some() && d.mode.is_some()) { break; }
        if started.elapsed() > Duration::from_secs(10) { bail!("six simulated Drones did not connect within 10 seconds"); }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let mission = mission::spawn_with_reset(Arc::clone(&gcs), field,
        |name| places::find(name).map(|p| [p.x, p.y, p.z]),
        move || { for (i, drone) in simulated.iter().enumerate() { drone.reset_to(frame::pad_slot(i)); } });
    let static_dir = std::env::var_os("AETHER_WEB_DIR")
        .map(PathBuf::from)
        .or_else(|| exe_dir.as_ref().map(|d| d.join("web")).filter(|d| d.join("index.html").exists()))
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web/dist"));
    println!("aether_ready url=http://{address} drones=6 provenance=SIM");
    tokio::select! {
        _ = server::serve(listener, server::Config {
            hello, cloud, mesh: mesh_bytes, gcs, inbox: mission.inbox, verdicts: mission.verdicts, resets: mission.resets, static_dir: Some(static_dir),
        }) => {},
        _ = tokio::signal::ctrl_c() => {},
    }
    Ok(())
}
