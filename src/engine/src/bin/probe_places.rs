//! T31: draws a top-down map of the real scan with every Place on it, then checks each Place is inside the scan,
//! in free space, and at least CLEARANCE_M from anything occupied. The map goes next to the scan, not in the repo.

use std::fmt::Write as _;
use std::process::ExitCode;

use aether_core::planner::DistanceField;
use aether_core::places;
use aether_core::room_twin::{load_obj, OccupancyGrid};

const VOXEL_M: f64 = 0.10;
const CLEARANCE_M: f64 = 0.35;
const PX_PER_M: f64 = 100.0;
const MARGIN_PX: f64 = 50.0;

fn main() -> anyhow::Result<ExitCode> {
    dotenvy::dotenv().ok();
    let path = std::env::var("ROOM_SCAN_PATH")?;
    let bytes = std::fs::read(&path)?;
    let mesh = load_obj(&bytes).map_err(|e| anyhow::anyhow!("OBJ load failed: {e}"))?;
    let grid = OccupancyGrid::from_mesh(&mesh, VOXEL_M).map_err(anyhow::Error::msg)?;
    let field = DistanceField::from_grid(&grid);

    let svg = map_svg(&grid);
    let map_path = std::path::Path::new(&path).with_file_name("aether-places-map.svg");
    std::fs::write(&map_path, svg)?;

    let mut failures = Vec::new();
    let mut min_clearance = f64::INFINITY;
    for p in places::all() {
        let at = [p.x, p.y, p.z];
        let Some(cell) = field.cell_of(at) else {
            failures.push(format!("{}: outside the scan bounds", p.name));
            continue;
        };
        if grid.is_occupied(cell) {
            failures.push(format!("{}: inside an occupied voxel", p.name));
        }
        let clearance = field.at(cell);
        min_clearance = min_clearance.min(clearance);
        if clearance < CLEARANCE_M {
            failures.push(format!("{}: clearance {clearance:.2} m below {CLEARANCE_M} m", p.name));
        }
    }
    if places::all().is_empty() {
        failures.push("no places".into());
    }
    if failures.is_empty() {
        println!("places_ok places={} min_clearance_m={min_clearance:.2} map=written", places::all().len());
        return Ok(ExitCode::SUCCESS);
    }
    for f in &failures {
        eprintln!("{f}");
    }
    println!("places_fail places={} failures={} map=written", places::all().len(), failures.len());
    Ok(ExitCode::from(1))
}

/// Top-down SVG: x across, z down the page (north = -z at the top). Later bands paint over earlier ones.
fn map_svg(grid: &OccupancyGrid) -> String {
    let [nx, ny, nz] = grid.dims();
    let (o, v) = (grid.origin(), grid.voxel());
    let floor_y = (0..ny).find(|&j| (0..nz).any(|k| (0..nx).any(|i| grid.is_occupied([i, j, k])))).map_or(o[1], |j| o[1] + (j as f64 + 0.5) * v);
    // (lowest height above floor, colour, label); painted in this order.
    let bands = [(-1.0, "#e6e1d6", "floor band (under 0.3 m)"), (2.2, "#c9c4b9", "above 2.2 m"), (1.2, "#9b968b", "1.2 to 2.2 m"), (0.3, "#3f3b36", "furniture 0.3 to 1.2 m"), (0.2, "#b4442c", "wall (60% filled, 0.2 to 2 m)")];
    let band_of = |h: f64| if h < 0.3 { 0 } else if h < 1.2 { 3 } else if h < 2.2 { 2 } else { 1 };
    let (w, h) = (nx as f64 * v * PX_PER_M + 2.0 * MARGIN_PX, nz as f64 * v * PX_PER_M + 2.0 * MARGIN_PX + 40.0);
    let side = w.max(h); // square canvas, so the PNG preview is not cropped
    let px = |x: f64| MARGIN_PX + (x - o[0]) * PX_PER_M;
    let pz = |z: f64| MARGIN_PX + (z - o[2]) * PX_PER_M;
    let mut s = String::new();
    let _ = writeln!(s, r##"<svg xmlns="http://www.w3.org/2000/svg" width="{side:.0}" height="{side:.0}" viewBox="0 0 {side:.0} {side:.0}" font-family="Helvetica, Arial, sans-serif"><rect width="100%" height="100%" fill="#faf8f3"/>"##);
    // A wall column is occupied over at least 60% of 0.2 to 2.0 m: a wall gap or door shows as a break in these.
    let is_wall = |i: usize, k: usize| {
        let hs: Vec<usize> = (0..ny).filter(|&j| (0.2..2.0).contains(&(o[1] + (j as f64 + 0.5) * v - floor_y))).collect();
        hs.iter().filter(|&&j| grid.is_occupied([i, j, k])).count() as f64 >= 0.6 * hs.len() as f64
    };
    for (b, (_, colour, _)) in bands.iter().enumerate() {
        let _ = write!(s, r#"<g fill="{colour}">"#);
        for k in 0..nz {
            for i in 0..nx {
                let top = (0..ny).filter(|&j| grid.is_occupied([i, j, k])).map(|j| o[1] + (j as f64 + 0.5) * v - floor_y);
                // The column shows the band of its highest voxel at or below the band's own ceiling, so walls stay dark.
                let in_band = if b == 4 { is_wall(i, k) } else { top.clone().any(|hgt| band_of(hgt) == b) };
                if in_band {
                    let _ = write!(s, r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}"/>"#, px(o[0] + i as f64 * v), pz(o[2] + k as f64 * v), v * PX_PER_M, v * PX_PER_M);
                }
            }
        }
        s.push_str("</g>");
    }
    // 1 m grid with scan coordinates (labels are scan x and z, metres).
    let (x0, x1, z0, z1) = (o[0].ceil() as i64, (o[0] + nx as f64 * v).floor() as i64, o[2].ceil() as i64, (o[2] + nz as f64 * v).floor() as i64);
    for x in x0..=x1 {
        let _ = write!(s, r##"<line x1="{0:.1}" y1="{1:.1}" x2="{0:.1}" y2="{2:.1}" stroke="#1f6fb2" stroke-opacity="0.35" stroke-width="1"/><text x="{0:.1}" y="{3:.1}" font-size="13" text-anchor="middle" fill="#1f6fb2">x={x}</text>"##, px(x as f64), pz(o[2]), pz(o[2] + nz as f64 * v), MARGIN_PX - 8.0);
    }
    for z in z0..=z1 {
        let _ = write!(s, r##"<line x1="{1:.1}" y1="{0:.1}" x2="{2:.1}" y2="{0:.1}" stroke="#1f6fb2" stroke-opacity="0.35" stroke-width="1"/><text x="{3:.1}" y="{4:.1}" font-size="13" text-anchor="end" fill="#1f6fb2">z={z}</text>"##, pz(z as f64), px(o[0]), px(o[0] + nx as f64 * v), MARGIN_PX - 6.0, pz(z as f64) + 4.0);
    }
    let _ = write!(s, r##"<text x="{:.1}" y="{:.1}" font-size="15" font-weight="bold" fill="#1f6fb2" text-anchor="middle">NORTH (-z) up</text>"##, w / 2.0, 18.0);
    for p in places::all() {
        let _ = write!(s, r##"<circle cx="{0:.1}" cy="{1:.1}" r="7" fill="#ff5a1f" stroke="#fff" stroke-width="2"/><text x="{2:.1}" y="{3:.1}" font-size="16" font-weight="bold" fill="#c2410c" stroke="#fff" stroke-width="3" paint-order="stroke">{4} (y={5:.2})</text>"##, px(p.x), pz(p.z), px(p.x) + 11.0, pz(p.z) + 5.0, p.name, p.y);
    }
    let ly = h - 24.0;
    for (n, (_, colour, label)) in bands.iter().enumerate() {
        let lx = MARGIN_PX + (n % 3) as f64 * 230.0; let ly = ly - (n / 3) as f64 * 20.0;
        let _ = write!(s, r##"<rect x="{lx:.0}" y="{:.0}" width="14" height="14" fill="{colour}" stroke="#777"/><text x="{:.0}" y="{:.0}" font-size="13" fill="#333">{label}</text>"##, ly, lx + 20.0, ly + 12.0);
    }
    s.push_str("</svg>\n");
    s
}
