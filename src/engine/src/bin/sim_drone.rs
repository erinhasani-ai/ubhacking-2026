//! `sim_drone --id N`: serves one simulated PX4-style quadrotor as a MAVLink v2 endpoint.

use anyhow::{Context, Result, bail};
use aether_engine::mavlink_link;

fn main() -> Result<()> {
    let mut id: u8 = 1;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--id" => {
                let value = args.next().context("--id needs a number")?;
                id = value
                    .parse()
                    .with_context(|| format!("--id {value} is not a number"))?;
            }
            other => bail!("unknown argument {other}; usage: sim_drone [--id N]"),
        }
    }
    if !(1..=254).contains(&id) {
        bail!("--id must be 1 to 254, got {id}");
    }
    mavlink_link::run(id)
}
