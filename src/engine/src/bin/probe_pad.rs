//! Checks the hardcoded Launch pad (a clear floor patch, T8) against the real scan.
//! `probe_pad --search` instead lists candidate floor patches on stderr (how the constants were chosen).
use std::collections::HashMap;
use std::process::ExitCode;

use aether_core::frame::{mission_to_scan, pad_slot, FLOOR_Y, PAD_XZ};
use aether_core::room_twin::{load_obj, OccupancyGrid, RoomMesh};

const CELL: f64 = 0.1;
const NEAR_M: f64 = 0.15;
const LEVEL_MM: i64 = 60;
const CLEAR_MIN_M: f64 = 0.6;
const SEARCH_RADIUS_M: f64 = 2.0;
/// Search grid step; 0.2 m keeps a 60 m floor scan to seconds.
const SEARCH_STEP_M: f64 = 0.2;

/// Floor heights sampled from near-horizontal mesh triangles (every 5 cm) within 0.4 m of FLOOR_Y, binned in 10 cm cells.
fn floor_cells(mesh: &RoomMesh) -> HashMap<(i64, i64), Vec<f64>> {
    let v = &mesh.vertices;
    let mut cells: HashMap<(i64, i64), Vec<f64>> = HashMap::new();
    for t in &mesh.triangles {
        let (a, b, c) = (v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]);
        let (u, w) = ([0, 1, 2].map(|k| b[k] - a[k]), [0, 1, 2].map(|k| c[k] - a[k]));
        let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len == 0.0 || (n[1] / len).abs() < 0.9 || (a[1] - FLOOR_Y).abs() > 0.4 {
            continue;
        }
        let longest = [u, w, [c[0] - b[0], c[1] - b[1], c[2] - b[2]]].iter().map(|e| (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt()).fold(0.0, f64::max);
        let steps = ((longest / 0.05).ceil() as usize).max(1);
        for i in 0..=steps {
            for j in 0..=(steps - i) {
                let (ti, tj) = (i as f64 / steps as f64, j as f64 / steps as f64);
                let p = [0, 1, 2].map(|k| a[k] + u[k] * ti + w[k] * tj);
                cells.entry(((p[0] / CELL).floor() as i64, (p[2] / CELL).floor() as i64)).or_default().push(p[1]);
            }
        }
    }
    cells
}

fn floor_near(cells: &HashMap<(i64, i64), Vec<f64>>, x: f64, z: f64) -> Vec<f64> {
    let (cx, cz) = ((x / CELL).floor() as i64, (z / CELL).floor() as i64);
    let r = (NEAR_M / CELL).ceil() as i64;
    let mut out = Vec::new();
    for dx in -r..=r {
        for dz in -r..=r {
            if let Some(ys) = cells.get(&(cx + dx, cz + dz)) {
                let (px, pz) = ((cx + dx) as f64 * CELL + CELL / 2.0 - x, (cz + dz) as f64 * CELL + CELL / 2.0 - z);
                if px.hypot(pz) <= NEAR_M {
                    out.extend(ys);
                }
            }
        }
    }
    out
}

/// Horizontal distance from (sx, sz) to the nearest occupied voxel 0.1-1.5 m above `floor_y` (capped at the search radius).
fn clear_distance(grid: &OccupancyGrid, floor_y: f64, sx: f64, sz: f64) -> f64 {
    let (o, d, vx) = (grid.origin(), grid.dims(), grid.voxel());
    let cell = |axis: usize, p: f64| ((p - o[axis]) / vx).floor() as i64;
    let r = (SEARCH_RADIUS_M / vx).ceil() as i64;
    let (ci, ck) = (cell(0, sx), cell(2, sz));
    let (j0, j1) = (cell(1, floor_y + 0.1).max(0), cell(1, floor_y + 1.5).min(d[1] as i64 - 1));
    let mut best = SEARCH_RADIUS_M;
    for i in (ci - r).max(0)..=(ci + r).min(d[0] as i64 - 1) {
        for k in (ck - r).max(0)..=(ck + r).min(d[2] as i64 - 1) {
            let dist = (o[0] + (i as f64 + 0.5) * vx - sx).hypot(o[2] + (k as f64 + 0.5) * vx - sz);
            if dist < best && (j0..=j1).any(|j| grid.is_occupied([i as usize, j as usize, k as usize])) {
                best = dist;
            }
        }
    }
    best
}

/// Pad evaluation at pad centre (px, pz) and floor height fy: (largest floor deviation mm or None if a slot has no floor sample,
/// slots with free space above, smallest clearance in metres).
fn evaluate(cells: &HashMap<(i64, i64), Vec<f64>>, grid: &OccupancyGrid, px: f64, pz: f64, fy: f64) -> (Option<i64>, usize, f64) {
    let (mut level, mut free, mut clear) = (Some(0i64), 0, SEARCH_RADIUS_M);
    for i in 0..6 {
        let m = pad_slot(i);
        let (sx, sz) = (px + m[1], pz - m[0]);
        let ys = floor_near(cells, sx, sz);
        // Floor under a slot = median of the samples within NEAR_M (the scan's noise is per sample, not the floor's shape).
        let mut sorted = ys.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        level = match (level, sorted.get(sorted.len() / 2)) {
            (Some(l), Some(m)) if sorted.len() >= 10 => Some(l.max(((m - fy).abs() * 1000.0).round() as i64)),
            _ => None,
        };
        free += (clear_distance(grid, fy, sx, sz) >= NEAR_M) as usize;
        clear = clear.min(clear_distance(grid, fy, sx, sz));
    }
    (level, free, clear)
}

fn main() -> anyhow::Result<ExitCode> {
    dotenvy::dotenv().ok();
    let bytes = std::fs::read(std::env::var("ROOM_SCAN_PATH")?)?;
    let mesh = load_obj(&bytes).map_err(|e| anyhow::anyhow!("OBJ load failed: {e}"))?;
    let grid = OccupancyGrid::from_mesh(&mesh, 0.10).map_err(anyhow::Error::msg)?;
    let cells = floor_cells(&mesh);

    if std::env::args().any(|a| a == "--search") {
        search(&cells, &grid);
        return Ok(ExitCode::SUCCESS);
    }

    let (level, free, clear) = evaluate(&cells, &grid, PAD_XZ[0], PAD_XZ[1], FLOOR_Y);
    // mission_to_scan keeps the formula in one place; the pad centre must land back on PAD_XZ at FLOOR_Y.
    let c = mission_to_scan([0.0, 0.0, 0.0]);
    let ok = level.is_some_and(|l| l <= LEVEL_MM) && free == 6 && clear >= CLEAR_MIN_M && (c[1] - FLOOR_Y).abs() < 1e-12;
    println!("{} floor_level_mm={} slots_free={free} clear_m={clear:.2}", if ok { "pad_ok" } else { "pad_fail" }, level.unwrap_or(-1));
    Ok(if ok { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

/// Candidate pad centres over the whole scan, 0.1 m apart, best clearance first; floor height is the median of the samples under the slots.
fn search(cells: &HashMap<(i64, i64), Vec<f64>>, grid: &OccupancyGrid) {
    let (o, s) = (grid.origin(), grid.size());
    let mut found: Vec<(f64, f64, f64, f64, i64, usize)> = Vec::new();
    let mut x = (o[0] / CELL).ceil() * CELL;
    while x < o[0] + s[0] {
        let mut z = (o[2] / CELL).ceil() * CELL;
        while z < o[2] + s[2] {
            let mut ys: Vec<f64> = (0..6).flat_map(|i| {
                let m = pad_slot(i);
                floor_near(cells, x + m[1], z - m[0])
            }).collect();
            if ys.len() > 30 {
                ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let fy = ys[ys.len() / 2];
                let (level, free, clear) = evaluate(cells, grid, x, z, fy);
                found.push((clear, x, z, fy, level.unwrap_or(9999), free));
            }
            z += SEARCH_STEP_M;
        }
        x += SEARCH_STEP_M;
    }
    found.retain(|c| c.4 <= LEVEL_MM && c.5 == 6 && c.0 >= CLEAR_MIN_M);
    found.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.4.cmp(&b.4)));
    eprintln!("{} candidate pad centres (clear >= {CLEAR_MIN_M} m, level <= {LEVEL_MM} mm, slots free 6); best first:", found.len());
    for (clear, x, z, fy, l, _) in found.iter().take(15) {
        eprintln!("cand x={x:.2} z={z:.2} floor_y={fy:.3} level_mm={l} clear_m={clear:.2}");
    }
}
