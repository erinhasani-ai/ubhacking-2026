//! Plans a path across the real Room Twin at ROOM_SCAN_PATH and checks it keeps the clearance everywhere (T10).
//! Scan frame, metres, Y up.

use std::process::ExitCode;
use std::time::Instant;

use aether_core::planner::{plan, DistanceField};
use aether_core::room_twin::{load_obj, OccupancyGrid};

const VOXEL_M: f64 = 0.10;
const CLEARANCE_M: f64 = 0.35;
const END_CLEARANCE_M: f64 = 0.5;
const HOVER_M: f64 = 1.0;
const SAMPLE_M: f64 = 0.02;

fn main() -> anyhow::Result<ExitCode> {
    dotenvy::dotenv().ok();
    let path = std::env::var("ROOM_SCAN_PATH")?;
    let bytes = std::fs::read(&path)?;
    let mesh = load_obj(&bytes).map_err(|e| anyhow::anyhow!("OBJ load failed: {e}"))?;
    let grid = OccupancyGrid::from_mesh(&mesh, VOXEL_M).map_err(anyhow::Error::msg)?;

    let started = Instant::now();
    let field = DistanceField::from_grid(&grid);
    let field_time = started.elapsed();
    let Some((start, goal)) = endpoints(&grid, &field) else {
        println!("path_fail reason=no_free_cell_near_axis_ends");
        return Ok(ExitCode::from(1));
    };
    let searching = Instant::now();
    let planned = plan(&field, start, goal, CLEARANCE_M);
    let search_time = searching.elapsed();
    let ms = (field_time + search_time).as_millis();
    let straight = dist(start, goal);
    eprintln!(
        "dims={:?} start={start:.3?} goal={goal:.3?} field_ms={} search_ms={}",
        grid.dims(),
        field_time.as_millis(),
        search_time.as_millis()
    );

    let waypoints = match planned {
        Ok(waypoints) => waypoints,
        Err(e) => {
            println!("path_fail reason={e:?} straight_m={straight:.2} ms={ms}");
            return Ok(ExitCode::from(1));
        }
    };
    let length: f64 = waypoints.windows(2).map(|w| dist(w[0], w[1])).sum();
    let mut min_clearance = f64::INFINITY;
    for w in waypoints.windows(2) {
        let n = (dist(w[0], w[1]) / SAMPLE_M).ceil().max(1.0) as usize;
        for s in 0..=n {
            let t = s as f64 / n as f64;
            min_clearance = min_clearance.min(field.at_point([0, 1, 2].map(|k| w[0][k] + (w[1][k] - w[0][k]) * t)));
        }
    }
    eprintln!("waypoints={} {waypoints:.3?}", waypoints.len());

    let numbers = format!("length_m={length:.2} straight_m={straight:.2} min_clearance_m={min_clearance:.2} ms={ms}");
    if min_clearance < CLEARANCE_M {
        println!("path_fail reason=clearance_below_{CLEARANCE_M} {numbers}");
        return Ok(ExitCode::from(1));
    }
    println!("path_ok {numbers}");
    Ok(ExitCode::SUCCESS)
}

/// Free cells with at least END_CLEARANCE_M, about HOVER_M above the floor under them, nearest the two ends of the
/// room's longest horizontal axis. The floor is a band of heights, so each column uses its own lowest occupied cell.
fn endpoints(grid: &OccupancyGrid, field: &DistanceField) -> Option<([f64; 3], [f64; 3])> {
    let dims = grid.dims();
    let size = grid.size();
    let (long, short) = if size[0] >= size[2] { (0, 2) } else { (2, 0) };
    let mut ends = [[0usize; 3]; 2];
    ends[1][long] = dims[long] - 1;
    for end in &mut ends {
        end[short] = dims[short] / 2;
    }
    let hover = (HOVER_M / grid.voxel()).round() as usize;
    let mut best: [Option<(usize, [usize; 3])>; 2] = [None, None];
    for k in 0..dims[2] {
        for i in 0..dims[0] {
            let Some(floor) = (0..dims[1]).find(|&j| grid.is_occupied([i, j, k])) else { continue };
            let j = floor + hover;
            if j >= dims[1] || field.at([i, j, k]) < END_CLEARANCE_M {
                continue;
            }
            for (e, end) in ends.iter().enumerate() {
                let d = i.abs_diff(end[0]).pow(2) + k.abs_diff(end[2]).pow(2);
                if best[e].is_none_or(|(nearest, _)| d < nearest) {
                    best[e] = Some((d, [i, j, k]));
                }
            }
        }
    }
    Some((field.centre(best[0]?.1), field.centre(best[1]?.1)))
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}
