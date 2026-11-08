//! `sim_swarm [--count N] [--ground-m H] [--gcs-port P]`: N simulated Drones in one process,
//! system ids 1..N, each on its own UDP socket, resting on the pad slots, sending to the ground
//! station at 127.0.0.1:P (default 14550, QGroundControl's port).

use std::thread;

use aether_engine::mavlink_link;
use anyhow::{Context, Result, bail};

/// Pad slot i in the Mission frame (shared with the Room Twin): 2 × 3 grid, 0.6 m apart, centred on the pad (SPEC T8, floor pad).
// shortcut: pad slot layout duplicated from T8's brief, use frame::pad_slot once T8 lands (T9b)
fn slot(i: usize, height: f64) -> [f64; 3] {
    [(i / 3) as f64 * 0.6 - 0.3, (i % 3) as f64 * 0.6 - 0.6, -height]
}

fn main() -> Result<()> {
    let mut count: u8 = 6;
    let mut ground_m: f64 = 0.0;
    let mut ground_station = mavlink_link::GROUND_STATION.to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--count" => {
                let v = args.next().context("--count needs a number")?;
                count = v.parse().with_context(|| format!("--count {v} is not a number"))?;
            }
            "--ground-m" => {
                let v = args.next().context("--ground-m needs a number")?;
                ground_m = v.parse().with_context(|| format!("--ground-m {v} is not a number"))?;
                if !ground_m.is_finite() || ground_m < 0.0 {
                    bail!("--ground-m must be a finite number of metres, 0 or more, got {v}");
                }
            }
            "--gcs-port" => {
                let v = args.next().context("--gcs-port needs a number")?;
                let port: u16 = v.parse().with_context(|| format!("--gcs-port {v} is not a port"))?;
                if port == 0 {
                    bail!("--gcs-port must be 1 to 65535");
                }
                ground_station = format!("127.0.0.1:{port}");
            }
            other => bail!("unknown argument {other}; usage: sim_swarm [--count N] [--ground-m H] [--gcs-port P]"),
        }
    }
    if !(1..=254).contains(&count) {
        bail!("--count must be 1 to 254, got {count}");
    }
    let handles: Vec<_> = (0..count as usize)
        .map(|i| {
            // shortcut: one ground height for every Drone, upgrade in T9b (the Room Twin's ground under each Drone, after T8 lands)
            let start = slot(i, ground_m);
            let gcs = ground_station.clone();
            thread::spawn(move || mavlink_link::run_at(i as u8 + 1, start, -ground_m, &gcs))
        })
        .collect();
    for h in handles {
        h.join().map_err(|_| anyhow::anyhow!("a Drone thread panicked"))??;
    }
    Ok(())
}
