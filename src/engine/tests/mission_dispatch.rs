//! T13.G2: mission control flies sim_swarm through the real Room Twin. Two Drones dispatched to a free point both
//! arrive; no pair comes closer than 0.5 m and no airborne sample is in an occupied voxel; an occupied target is
//! rejected with its reason; Abort while two Drones move holds every Drone within 1 s, then lands and disarms them.
//! Simulated Drones only; nothing here is flown on hardware.

use std::collections::BTreeMap;
use std::process::{Child, Command as Process};
use std::sync::Arc;
use std::time::{Duration, Instant};

use aether_core::command::{Command, Source, Verdict};
use aether_core::frame;
use aether_core::planner::DistanceField;
use aether_core::policy::{CLEARANCE_M, SURFACE_LIFT_M};
use aether_core::room_twin::{load_obj, OccupancyGrid};
use aether_engine::gcs::{DroneLink, Gcs};
use aether_engine::mission::{self, Inbox};
use mavlink::dialects::common::MavLandedState;
use tokio::sync::oneshot;

/// The free point (Mission frame): 1 m up, 2.5 m west and 0.5 m north of the pad centre. The clearance map of the scan
/// at 1 m shows at least 0.9 m clearance from e = −3 to +0.5 along n = 0.5; the test checks it below.
const TARGET: [f64; 3] = [0.5, -2.5, -1.0];
/// Where the two Drones are sent next, back over the pad's east side, so they are moving when Abort comes.
const SECOND: [f64; 3] = [0.0, 0.5, -1.0];
/// T10's probe crossed this scan on a path within 2% of the straight line (9.85 m for 9.73 m), so the ~2.6 m legs here
/// take about 3 s at the 1 m/s cap; with the take-off handshake, the 1 m climb and the slow-down, about 7 s. The bound
/// is about three times that, for a machine loaded with other builds.
const ARRIVE_WITHIN: Duration = Duration::from_secs(20);
const ARRIVED_M: f64 = 0.3;
const MIN_SEPARATION_M: f64 = 0.5;
/// Below this height (d > −0.15 m) a Drone is on or at the floor, whose voxels it touches by design.
const AIRBORNE_D: f64 = -0.15;
const HEARTBEATS_WITHIN: Duration = Duration::from_secs(30);
const MOVING_WITHIN: Duration = Duration::from_secs(20);
const STILL_WITHIN: Duration = Duration::from_secs(1);
/// 1.5 s hold, about 1.5 s down at 0.7 m/s, PX4's 2 s disarm delay and the 1 Hz heartbeat; the rest is margin.
const LANDED_WITHIN: Duration = Duration::from_secs(20);

/// Kills sim_swarm on every path out of the test, a panic included.
struct Sim(Child);

impl Drop for Sim {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn pos(d: &DroneLink) -> Option<[f64; 3]> {
    d.position.as_ref().map(|p| [p.x, p.y, p.z].map(f64::from))
}

fn speed(d: &DroneLink) -> f64 {
    d.position.as_ref().map_or(0.0, |p| (p.vx * p.vx + p.vy * p.vy + p.vz * p.vz).sqrt() as f64)
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

/// Safety on every snapshot: separation and occupancy.
struct Watch {
    gcs: Arc<Gcs>,
    grid: OccupancyGrid,
    field: Arc<DistanceField>,
    closest: f64,
    samples: usize,
}

impl Watch {
    /// Samples every 50 ms until `done` holds, asserting the safety rules on each sample; returns how long it took.
    async fn until(&mut self, within: Duration, what: &str, done: impl Fn(&BTreeMap<u8, DroneLink>) -> bool) -> Duration {
        let start = Instant::now();
        loop {
            let drones = self.gcs.drones();
            self.check(&drones);
            if done(&drones) {
                return start.elapsed();
            }
            assert!(start.elapsed() < within, "{what}: not within {within:?}: {drones:#?}");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn check(&mut self, drones: &BTreeMap<u8, DroneLink>) {
        self.samples += 1;
        let at: Vec<(u8, [f64; 3])> = drones.iter().filter_map(|(id, d)| Some((*id, pos(d)?))).collect();
        for (i, (a, pa)) in at.iter().enumerate() {
            for (b, pb) in &at[i + 1..] {
                let gap = dist(*pa, *pb);
                self.closest = self.closest.min(gap);
                assert!(gap >= MIN_SEPARATION_M, "Drones {a} and {b} are {gap:.3} m apart at {pa:.3?} and {pb:.3?}");
            }
            if pa[2] < AIRBORNE_D {
                let cell = self.field.cell_of(frame::mission_to_scan(*pa));
                assert!(cell.is_some_and(|c| !self.grid.is_occupied(c)), "Drone {a} at {pa:.3?} is in an occupied voxel or outside the Room Twin");
            }
        }
    }
}

async fn ask(inbox: &Inbox, wire: &str) -> Verdict {
    let command = Command::from_json(wire).expect("test Commands are valid");
    let (tx, rx) = oneshot::channel();
    inbox.send((Source::Key, command, tx)).await.expect("mission is running");
    rx.await.expect("mission answers every Command")
}

fn point(p: [f64; 3]) -> String {
    format!(r#"{{"point":{{"n":{},"e":{},"d":{}}}}}"#, p[0], p[1], p[2])
}

fn airborne(drones: &BTreeMap<u8, DroneLink>) -> Vec<u8> {
    drones.iter().filter(|(_, d)| d.armed).map(|(id, _)| *id).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_drones_fly_to_a_free_point_and_abort_holds_then_lands_them() {
    let t = Instant::now();
    dotenvy::dotenv().ok();
    let scan = std::env::var("ROOM_SCAN_PATH").expect("ROOM_SCAN_PATH in .env");
    let mesh = load_obj(&std::fs::read(&scan).expect("read the scan")).expect("parse the scan");
    let grid = OccupancyGrid::from_mesh(&mesh, 0.10).expect("occupancy grid");
    let field = Arc::new(DistanceField::from_grid(&grid));
    let loaded = t.elapsed();

    // The chosen points are free, and neither the goal spread nor the parked Drones change with the pad.
    for p in [TARGET, SECOND] {
        let clear = field.at_point(frame::mission_to_scan(p));
        assert!(clear >= 0.6, "{p:?} has only {clear:.2} m clearance");
    }
    let goals = mission::spread_goals(&field, mission::ROOM, TARGET, 2, &[]).expect("room for two Drones at the target");
    for g in &goals {
        assert!((0..6).all(|i| dist(*g, frame::pad_slot(i)) > 1.0), "goal {g:?} is near the pad, so parked Drones could move it");
    }
    // An occupied voxel about 1 m up whose spot 0.5 m above is blocked too, so it is not a surface hit: the one
    // nearest the pad centre (a wall or a tall piece of furniture).
    let [nx, ny, nz] = grid.dims();
    let occupied = (0..nz)
        .flat_map(|k| (0..ny).flat_map(move |j| (0..nx).map(move |i| [i, j, k])))
        .filter(|&c| grid.is_occupied(c))
        .map(|c| frame::scan_to_mission(field.centre(c)))
        .filter(|p| (-1.1..=-0.9).contains(&p[2]) && field.at_point(frame::mission_to_scan([p[0], p[1], p[2] - SURFACE_LIFT_M])) < CLEARANCE_M)
        .min_by(|a, b| a[0].hypot(a[1]).total_cmp(&b[0].hypot(b[1])))
        .expect("an occupied voxel about 1 m up");

    let gcs = Arc::new(Gcs::bind("127.0.0.1:0").await.expect("bind the ground station on a free port"));
    let port = gcs.local_addr().unwrap().port().to_string();
    let _sim = Sim(
        Process::new(env!("CARGO_BIN_EXE_sim_swarm"))
            .args(["--count", "6", "--gcs-port", &port])
            .spawn()
            .expect("start sim_swarm"),
    );
    let mission = mission::spawn(Arc::clone(&gcs), Arc::clone(&field), |name| Err(format!("No Places in this test ({name}).")));
    let mut watch = Watch { gcs: Arc::clone(&gcs), grid, field, closest: f64::INFINITY, samples: 0 };
    let heard = watch
        .until(HEARTBEATS_WITHIN, "six Drones with heartbeat and position", |d| d.len() == 6 && d.values().all(|d| d.mode.is_some() && pos(d).is_some()))
        .await;

    // Dispatch two Drones to the free point.
    let t = Instant::now();
    let verdict = ask(&mission.inbox, &format!(r#"{{"kind":"dispatch","drones":{{"count":2}},"target":{}}}"#, point(TARGET))).await;
    assert_eq!(verdict, Verdict { approved: true, reason: None });
    let planned = t.elapsed();
    let arrived = watch
        .until(ARRIVE_WITHIN, "both Drones within 0.3 m of their goals", |d| {
            goals.iter().all(|g| d.values().any(|d| d.armed && pos(d).is_some_and(|p| dist(p, *g) <= ARRIVED_M)))
        })
        .await;
    // The mission takes new Commands for them once they hover, within mission::ARRIVED_M of their goals.
    let settled = watch
        .until(ARRIVE_WITHIN, "both Drones hovering at their goals", |d| {
            goals.iter().all(|g| d.values().any(|d| d.armed && pos(d).is_some_and(|p| dist(p, *g) < mission::ARRIVED_M)))
        })
        .await;
    let flown = airborne(&gcs.drones());
    assert_eq!(flown.len(), 2, "exactly two Drones flew: {flown:?}");
    println!("arrived: Drones {flown:?} at goals {goals:.2?}");

    // An occupied target is rejected with its reason.
    let verdict = ask(&mission.inbox, &format!(r#"{{"kind":"dispatch","drones":{{"count":1}},"target":{}}}"#, point(occupied))).await;
    assert_eq!(verdict, Verdict { approved: false, reason: Some("The target is inside an obstacle.".into()) }, "target {occupied:?}");

    // Send the two on, and Abort once both are moving.
    let ids = format!("{flown:?}").replace(' ', "");
    let t2 = Instant::now();
    let verdict = ask(&mission.inbox, &format!(r#"{{"kind":"dispatch","drones":{{"ids":{ids}}},"target":{}}}"#, point(SECOND))).await;
    assert_eq!(verdict, Verdict { approved: true, reason: None });
    let replanned = t2.elapsed();
    let moving_after = watch
        .until(MOVING_WITHIN, "both Drones moving faster than 0.7 m/s", |d| flown.iter().all(|id| d.get(id).is_some_and(|d| speed(d) > 0.7)))
        .await;
    let speeds: Vec<f64> = flown.iter().map(|id| speed(&gcs.drones()[id])).collect();
    let t = Instant::now();
    let verdict = ask(&mission.inbox, r#"{"kind":"abort"}"#).await;
    assert_eq!(verdict, Verdict { approved: true, reason: None });
    watch.until(STILL_WITHIN, "every Drone slower than 0.1 m/s", |d| d.values().all(|d| speed(d) < 0.1)).await;
    let still = t.elapsed(); // from before the Abort was sent
    assert!(still <= STILL_WITHIN, "still after {still:?}");
    // They hold: still slower than 0.1 m/s 1.2 s after the Abort, before the mission lands them at 1.5 s.
    tokio::time::sleep(Duration::from_millis(1200).saturating_sub(t.elapsed())).await;
    let fast: Vec<(u8, f64)> = gcs.drones().iter().map(|(id, d)| (*id, speed(d))).filter(|(_, s)| *s >= 0.1).collect();
    assert!(fast.is_empty(), "moving again during the hold: {fast:?}");
    let down = watch
        .until(LANDED_WITHIN, "every Drone disarmed on the ground", |d| {
            d.values().all(|d| !d.armed && d.landed_state == Some(MavLandedState::MAV_LANDED_STATE_ON_GROUND))
        })
        .await;
    let landed = t.elapsed();

    println!(
        "mission_dispatch phases: load_scan={:.2}s heartbeats={:.2}s plan_verdict={:.2}s arrive={:.2}s (bound {}s) hover_after={:.2}s \
         second_verdict={:.2}s moving={:.2}s (speeds {speeds:.2?} m/s) abort_to_still={:.2}s (bound 1s) \
         abort_to_landed_disarmed={:.2}s down_phase={:.2}s; closest_pair={:.3}m samples={}",
        loaded.as_secs_f64(),
        heard.as_secs_f64(),
        planned.as_secs_f64(),
        arrived.as_secs_f64(),
        ARRIVE_WITHIN.as_secs(),
        settled.as_secs_f64(),
        replanned.as_secs_f64(),
        moving_after.as_secs_f64(),
        still.as_secs_f64(),
        landed.as_secs_f64(),
        down.as_secs_f64(),
        watch.closest,
        watch.samples
    );
}
