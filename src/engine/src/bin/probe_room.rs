//! Loads the room scan at ROOM_SCAN_PATH into a Room Twin occupancy grid and checks it is plausible.

use std::process::ExitCode;
use std::time::Instant;

use aether_core::room_twin::{load_obj, OccupancyGrid, ScanHeader};

const VOXEL_M: f64 = 0.10;

fn main() -> anyhow::Result<ExitCode> {
    dotenvy::dotenv().ok();
    let path = std::env::var("ROOM_SCAN_PATH")?;
    let bytes = std::fs::read(&path)?;

    let started = Instant::now();
    let header = ScanHeader::parse(&bytes);
    let mesh = load_obj(&bytes).map_err(|e| anyhow::anyhow!("OBJ load failed: {e}"))?;
    let loaded = started.elapsed();
    let grid = OccupancyGrid::from_mesh(&mesh, VOXEL_M).map_err(anyhow::Error::msg)?;
    eprintln!("timing load_ms={:.1} grid_ms={:.1}", loaded.as_secs_f64() * 1e3, (started.elapsed() - loaded).as_secs_f64() * 1e3);

    let [sx, sy, sz] = grid.size();
    let occupied = grid.occupied_count();
    let free_share = grid.free_count() as f64 / grid.total_cells() as f64;

    let mut failures = Vec::new();
    if !(2.0..=6.0).contains(&sy) {
        failures.push(format!("height {sy:.2} m outside 2-6 m"));
    }
    for (axis, side) in [("x", sx), ("z", sz)] {
        if !(2.0..=60.0).contains(&side) {
            failures.push(format!("floor side {axis} {side:.2} m outside 2-60 m"));
        }
    }
    if occupied < 1000 {
        failures.push(format!("occupied voxels {occupied} below 1000"));
    }
    if !(0.50..=0.99).contains(&free_share) {
        failures.push(format!("free share {:.1}% outside 50-99%", free_share * 100.0));
    }
    let (Some(lat), Some(lon), Some(heading)) = (header.latitude, header.longitude, header.heading_deg) else {
        failures.push(format!("header lacks location or heading: {header:?}"));
        return Ok(report(failures));
    };
    if !failures.is_empty() {
        return Ok(report(failures));
    }

    println!(
        "room_ok points={} triangles={} voxels={} free_pct={} size_m={sx:.1}x{sy:.1}x{sz:.1} geo={lat:.7},{lon:.7} heading_deg={heading:.2}",
        mesh.vertices.len(),
        mesh.triangles.len(),
        occupied,
        (free_share * 100.0).round() as i64,
    );
    Ok(ExitCode::SUCCESS)
}

fn report(failures: Vec<String>) -> ExitCode {
    for f in failures {
        eprintln!("room_check_failed: {f}");
    }
    ExitCode::from(1)
}
