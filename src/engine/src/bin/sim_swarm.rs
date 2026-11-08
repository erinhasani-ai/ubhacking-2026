//! Six simulated MAVLink Drones on the shared pad, optionally using the scanned room's ground.

use std::{sync::Arc, thread};

use aether_core::{frame, room_twin::{load_obj, OccupancyGrid}};
use aether_engine::mavlink_link::{self, Ground};
use anyhow::{Context, Result, bail};

#[derive(Debug)]
struct Options {
    count: u8,
    ground_m: f64,
    room: bool,
    ground_station: String,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self {
            count: 6,
            ground_m: 0.0,
            room: false,
            ground_station: mavlink_link::GROUND_STATION.to_string(),
        };
        let mut explicit_ground = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--count" => {
                    options.count = args.next().context("--count needs a number")?.parse()?;
                }
                "--ground-m" => {
                    explicit_ground = true;
                    options.ground_m = args.next().context("--ground-m needs a number")?.parse()?;
                    if !options.ground_m.is_finite() || options.ground_m < 0.0 {
                        bail!("--ground-m must be a finite number of metres, 0 or more");
                    }
                }
                "--room" => options.room = true,
                "--gcs-port" => {
                    let port: u16 = args.next().context("--gcs-port needs a number")?.parse()?;
                    if port == 0 { bail!("--gcs-port must be 1 to 65535"); }
                    options.ground_station = format!("127.0.0.1:{port}");
                }
                other => bail!("unknown argument {other}; usage: sim_swarm [--count N] [--ground-m H | --room] [--gcs-port P]"),
            }
        }
        if !(1..=6).contains(&options.count) { bail!("--count must be 1 to 6 (the six pad slots)"); }
        if options.room && explicit_ground { bail!("--room cannot be combined with --ground-m"); }
        Ok(options)
    }

    fn start(&self, i: usize) -> [f64; 3] {
        let mut start = frame::pad_slot(i);
        start[2] = -self.ground_m;
        start
    }
}

fn main() -> Result<()> {
    let options = Options::parse(std::env::args().skip(1))?;
    let ground = if options.room {
        dotenvy::dotenv().ok();
        let path = std::env::var("ROOM_SCAN_PATH").context("ROOM_SCAN_PATH is not set")?;
        let mesh = load_obj(&std::fs::read(path)?).context("loading the Room Twin OBJ")?;
        Ground::Room(Arc::new(OccupancyGrid::from_mesh(&mesh, 0.1).map_err(anyhow::Error::msg)?))
    } else {
        Ground::Flat(-options.ground_m)
    };
    let handles: Vec<_> = (0..options.count as usize)
        .map(|i| {
            let start = options.start(i);
            let gcs = options.ground_station.clone();
            let ground = ground.clone();
            thread::spawn(move || mavlink_link::run_on_ground(i as u8 + 1, start, ground, &gcs))
        })
        .collect();
    for h in handles {
        h.join().map_err(|_| anyhow::anyhow!("a Drone thread panicked"))??;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options> {
        Options::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn starts_on_the_shared_six_pad_slots() {
        let options = parse(&[]).unwrap();
        assert_eq!(options.count, 6);
        for i in 0..6 { assert_eq!(options.start(i), frame::pad_slot(i)); }
        assert_eq!(parse(&["--ground-m", "0.8"]).unwrap().start(0), [-0.3, -0.6, -0.8]);
    }

    #[test]
    fn accepts_room_and_custom_ground_station() {
        let options = parse(&["--room", "--count", "2", "--gcs-port", "15000"]).unwrap();
        assert!(options.room);
        assert_eq!(options.count, 2);
        assert_eq!(options.ground_station, "127.0.0.1:15000");
    }

    #[test]
    fn rejects_conflicting_or_invalid_options() {
        for args in [
            vec!["--count", "0"], vec!["--count", "7"], vec!["--count"],
            vec!["--ground-m", "NaN"], vec!["--ground-m", "-1"], vec!["--ground-m", "inf"],
            vec!["--gcs-port", "0"], vec!["--gcs-port", "65536"], vec!["--unknown"],
            vec!["--room", "--ground-m", "0"], vec!["--ground-m", "1", "--room"],
        ] { assert!(parse(&args).is_err(), "accepted {args:?}"); }
    }
}
