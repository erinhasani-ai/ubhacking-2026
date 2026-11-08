//! T12.G2: the ground station flies the six Drones of one sim_swarm process through the PX4
//! offboard sequence: setpoints, offboard, arm, climb to d = −1.0 m, hold 2 s, land, auto-disarm.
//! Simulated Drones only; nothing here is flown on hardware.

use std::process::{Child, Command};
use std::time::{Duration, Instant};

use aether_engine::gcs::{DroneLink, Gcs};
use futures_util::future::try_join_all;
use mavlink::dialects::common::MavLandedState;

const COUNT: usize = 6;
const CLIMB_D: f32 = -1.0;
const TOLERANCE: f32 = 0.1;
/// Process start to six heartbeats, with margin for a loaded machine.
const HEARTBEATS_WITHIN: Duration = Duration::from_secs(30);
/// 1 m at 1 m/s with 2 m/s² acceleration is about 1.5 s; the rest is margin.
const CLIMB_WITHIN: Duration = Duration::from_secs(10);
const HOLD: Duration = Duration::from_secs(2);
/// 1 m at 0.7 m/s is about 1.5 s, then PX4's 2 s disarm delay and the 1 Hz heartbeat; the rest is margin.
const LAND_WITHIN: Duration = Duration::from_secs(15);

/// Kills sim_swarm on every path out of the test, a panic included.
struct Sim(Child);

impl Drop for Sim {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn at_climb_height(d: &DroneLink) -> bool {
    d.position.as_ref().is_some_and(|p| (p.z - CLIMB_D).abs() <= TOLERANCE)
}

/// Waits until all six Drones satisfy `ok`; returns how long that took.
async fn all_six(gcs: &Gcs, within: Duration, what: &str, ok: impl Fn(&DroneLink) -> bool) -> Duration {
    let start = Instant::now();
    loop {
        let drones = gcs.drones();
        if drones.len() == COUNT && drones.values().all(&ok) {
            return start.elapsed();
        }
        assert!(start.elapsed() < within, "{what}: not all {COUNT} Drones within {within:?}: {drones:#?}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn six_drones_arm_climb_hold_land_and_disarm() {
    let gcs = Gcs::bind("127.0.0.1:0").await.expect("bind the ground station on a free port");
    let port = gcs.local_addr().unwrap().port().to_string();
    let _sim = Sim(
        Command::new(env!("CARGO_BIN_EXE_sim_swarm"))
            .args(["--count", "6", "--gcs-port", &port])
            .spawn()
            .expect("start sim_swarm"),
    );
    let ids: Vec<u8> = (1..=COUNT as u8).collect();

    let heard = all_six(&gcs, HEARTBEATS_WITHIN, "heartbeat and position", |d| {
        d.mode.is_some() && d.position.is_some() && !d.armed
    })
    .await;

    // Straight up from each pad slot. PX4 accepts offboard only while setpoints already arrive.
    let t = Instant::now();
    for (id, d) in gcs.drones() {
        let p = d.position.unwrap();
        gcs.set_target(id, Some([p.x, p.y, CLIMB_D]));
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    try_join_all(ids.iter().map(|&id| gcs.set_offboard(id))).await.expect("offboard on all six");
    try_join_all(ids.iter().map(|&id| gcs.arm(id))).await.expect("arm all six");
    let commanded = t.elapsed();

    let climbed = all_six(&gcs, CLIMB_WITHIN, "armed in OFFBOARD at d = -1.0 m", |d| {
        d.armed && d.mode == Some("OFFBOARD") && at_climb_height(d)
    })
    .await;

    let t = Instant::now();
    while t.elapsed() < HOLD {
        let drones = gcs.drones();
        let off: Vec<_> = drones.iter().filter(|(_, d)| !at_climb_height(d)).map(|(id, _)| id).collect();
        assert!(off.is_empty(), "Drones {off:?} left d = -1.0 ± 0.1 m during the hold");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let t = Instant::now();
    try_join_all(ids.iter().map(|&id| gcs.land(id))).await.expect("land all six");
    all_six(&gcs, LAND_WITHIN, "disarmed and ON_GROUND", |d| {
        !d.armed && d.landed_state == Some(MavLandedState::MAV_LANDED_STATE_ON_GROUND)
    })
    .await;
    let landed = t.elapsed();

    println!(
        "gcs_swarm phases: heartbeats={:.2}s setpoints_offboard_arm={:.2}s climb={:.2}s hold={:.2}s land_disarm={:.2}s",
        heard.as_secs_f64(),
        commanded.as_secs_f64(),
        climbed.as_secs_f64(),
        HOLD.as_secs_f64(),
        landed.as_secs_f64()
    );
}
