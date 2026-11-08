//! Mission control (T13). Takes Commands from the Mission inbox and answers each with its Verdict: the policy check
//! (aether-core `policy`), then goals spread around the target, paths planned through the Room Twin (T10) and Drones
//! assigned to goals on path length (Kuhn-Munkres). A 20 Hz control loop flies the Swarm through the ground station:
//! each Drone follows its path with a preferred velocity that ORCA (T11) shapes against the other Drones, sent as
//! position setpoints. Abort is handled before any queued Command and never waits on planning, the AI or the network.
//! Simulated Drones only; nothing here is flown on hardware.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aether_core::command::{Command, Source, Verdict};
use aether_core::frame;
use aether_core::orca::{self, DroneState};
use aether_core::planner::{self, DistanceField, PlanError};
use aether_core::policy::{self, Activity, Context, Decision, DroneView, CLEARANCE_M};
use aether_core::vehicle::MAX_ACCELERATION;
use mavlink::dialects::common::MavLandedState;
use pathfinding::prelude::{kuhn_munkres_min, Matrix};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::gcs::{DroneLink, Gcs};

/// The Mission inbox (SPEC decisions log): every Command with the channel its Verdict goes back on.
pub type Inbox = mpsc::Sender<(Source, Command, oneshot::Sender<Verdict>)>;
/// Every answered Command with its Verdict, for the browser (T14) and the Mission timeline (T22).
pub type Verdicts = broadcast::Sender<(Source, Command, Verdict)>;
pub type Resets = mpsc::Sender<oneshot::Sender<()>>;

const TICK: Duration = Duration::from_millis(50); // 20 Hz
/// Cruise speed cap, m/s (Tier A Drones fly up to 1.5 horizontally and 1.0 vertically).
const MAX_SPEED: f64 = 1.0;
/// Near the goal the speed is this gain times the path still to fly (1/s), so a Drone slows down smoothly.
const SLOW_GAIN: f64 = 1.5;
/// ORCA: each Drone's safety radius (SPEC: 0.3 m) and how far ahead it looks.
const RADIUS: f64 = 0.3;
const TIME_HORIZON: f64 = 2.0;
const NEIGHBOUR_RANGE: f64 = 3.0;
/// Near another Drone (within BIAS_RANGE) each Drone adds a sideways part to its preferred velocity, about this
/// fraction of it, to break ORCA's symmetric deadlocks.
const SIDE_BIAS: f64 = 0.05;
const BIAS_RANGE: f64 = 1.5;
/// Take-off climbs this far straight up before the planned path starts.
const CLIMB_M: f64 = 1.0;
/// Goals of one Command and the spots other Drones hold are at least this far apart: 2 × RADIUS plus slack for ORCA.
const GOAL_SPACING_M: f64 = 0.8;
/// A waypoint counts as passed this close.
const WAYPOINT_M: f64 = 0.15;
/// A Drone this close to its goal hovers there and is idle again (takes new Commands).
pub const ARRIVED_M: f64 = 0.1;
/// A setpoint whose spot is closer than this to an obstacle is replaced by the Drone's stopping point.
const GUARD_M: f64 = 0.2;
/// A Drone on the ground this close (horizontally) to its slot is Parked.
const ON_SLOT_M: f64 = 0.2;
/// After an Abort every Drone holds for this long, then lands.
const HOLD_FOR: Duration = Duration::from_millis(1500);
/// PX4 accepts offboard only while setpoints already arrive, so the first ones go out this long before the mode change.
const SETPOINTS_FIRST: Duration = Duration::from_millis(150);
const TAKEOFF_TIMEOUT: Duration = Duration::from_secs(10);
const LAND_RETRY: Duration = Duration::from_secs(5);
const ABORT_FIRST: &str = "Abort came first.";

/// The handle to a running mission.
pub struct Mission {
    pub inbox: Inbox,
    pub verdicts: Verdicts,
    pub resets: Resets,
    view_heading: Arc<Mutex<[f64; 2]>>,
}

impl Mission {
    /// The map view's horizontal forward (north, east), which Advance moves along; normalised. Default +N.
    pub fn set_view_heading(&self, north: f64, east: f64) {
        let len = north.hypot(east);
        if len.is_finite() && len > 1e-9 {
            *self.view_heading.lock().unwrap() = [north / len, east / len];
        }
    }
}

/// Starts mission control for the Drones `gcs` hears from. `field` is the Room Twin's distance field (scan frame);
/// `place` gives a Place's hover point in scan coordinates or a message naming the known Places
/// (`places::find` from T31). Call inside a tokio runtime. The control loop ends when every inbox Sender is dropped.
pub fn spawn(
    gcs: Arc<Gcs>,
    field: Arc<DistanceField>,
    place: impl Fn(&str) -> Result<[f64; 3], String> + Send + 'static,
) -> Mission {
    spawn_with_reset(gcs, field, place, || {})
}

/// Adds the local simulator reset callback; never used as a physical flight command.
pub fn spawn_with_reset(
    gcs: Arc<Gcs>,
    field: Arc<DistanceField>,
    place: impl Fn(&str) -> Result<[f64; 3], String> + Send + 'static,
    reset_simulation: impl Fn() + Send + 'static,
) -> Mission {
    let (inbox, rx) = mpsc::channel(64);
    let (resets, reset_rx) = mpsc::channel(8);
    let verdicts = broadcast::channel(256).0;
    let view_heading = Arc::new(Mutex::new([1.0, 0.0]));
    let (planned, planned_rx) = mpsc::unbounded_channel();
    let control = Control {
        gcs,
        field,
        place: Box::new(place),
        view_heading: Arc::clone(&view_heading),
        verdicts: verdicts.clone(),
        tasks: BTreeMap::new(),
        epoch: 0,
        planning: false,
        waiting: VecDeque::new(),
        planned,
    };
    tokio::spawn(control.run(rx, planned_rx, reset_rx, reset_simulation));
    Mission { inbox, verdicts, resets, view_heading }
}

/// Scan ↔ Mission conversions, passed in so the pure helpers also run on small test rooms.
#[derive(Clone, Copy)]
pub struct Frame {
    pub to_scan: fn([f64; 3]) -> [f64; 3],
    pub to_mission: fn([f64; 3]) -> [f64; 3],
}

/// The Room Twin's frame (T8).
pub const ROOM: Frame = Frame { to_scan: frame::mission_to_scan, to_mission: frame::scan_to_mission };

/// Goals for `n` Drones around `target` (Mission frame), at the target's height: the target itself, then 6 spots on
/// a ring GOAL_SPACING_M out, then 12 on a ring twice as far, north first, clockwise. A spot is taken if it is inside
/// the Room Twin with CLEARANCE_M and at least GOAL_SPACING_M from every goal already taken and from `others`
/// (where other Drones are, or hold).
/// shortcut: a spot behind a thin wall counts too (the path then goes around it); check line of sight if that shows up.
pub fn spread_goals(field: &DistanceField, frame: Frame, target: [f64; 3], n: usize, others: &[[f64; 3]]) -> Result<Vec<[f64; 3]>, String> {
    let mut spots = vec![target];
    for (ring, count) in [(1.0, 6), (2.0, 12)] {
        for k in 0..count {
            let angle = std::f64::consts::TAU * k as f64 / count as f64;
            let r = ring * GOAL_SPACING_M;
            spots.push([target[0] + r * angle.cos(), target[1] + r * angle.sin(), target[2]]);
        }
    }
    let mut goals: Vec<[f64; 3]> = Vec::with_capacity(n);
    for spot in spots {
        if goals.len() == n {
            break;
        }
        let clear = field.at_point((frame.to_scan)(spot)) >= CLEARANCE_M;
        if clear && goals.iter().chain(others).all(|&o| dist(o, spot) >= GOAL_SPACING_M) {
            goals.push(spot);
        }
    }
    if goals.len() < n {
        return Err(format!("No room for {n} Drones around the target."));
    }
    Ok(goals)
}

/// A planned path for each Drone (Mission frame, from its start to its goal). With `assign`, Drones are matched to
/// goals so the total path length is smallest (Kuhn-Munkres on millimetres); without it, Drone i flies to goal i.
pub fn plan_routes(field: &DistanceField, frame: Frame, starts: &[(u8, [f64; 3])], goals: &[[f64; 3]], assign: bool) -> Result<Vec<(u8, Vec<[f64; 3]>)>, String> {
    assert_eq!(starts.len(), goals.len(), "one goal per Drone");
    let plan_one = |id: u8, from: [f64; 3], to: [f64; 3]| -> Result<Option<Vec<[f64; 3]>>, String> {
        match planner::plan(field, (frame.to_scan)(from), (frame.to_scan)(to), CLEARANCE_M) {
            Ok(path) => Ok(Some(path.into_iter().map(frame.to_mission).collect())),
            Err(PlanError::StartBlocked) => Err(format!("Drone {id} has no clear space to start from.")),
            Err(PlanError::GoalBlocked | PlanError::Unreachable) => Ok(None),
        }
    };
    let no_path = |id: u8| format!("No clear path for Drone {id}.");
    if !assign {
        return starts
            .iter()
            .zip(goals)
            .map(|(&(id, from), &to)| plan_one(id, from, to)?.map(|p| (id, p)).ok_or_else(|| no_path(id)))
            .collect();
    }
    let n = starts.len();
    let mut paths: Vec<Vec<Option<Vec<[f64; 3]>>>> = Vec::with_capacity(n);
    for &(id, from) in starts {
        paths.push(goals.iter().map(|&to| plan_one(id, from, to)).collect::<Result<_, _>>()?);
    }
    // Unreachable pairs cost more than any real assignment, so they are chosen only when nothing else works.
    const UNREACHABLE_MM: i64 = 1_000_000_000;
    let cost = Matrix::from_fn(n, n, |(r, c)| paths[r][c].as_deref().map_or(UNREACHABLE_MM, |p| (length(p) * 1000.0).round() as i64));
    let (_, columns) = kuhn_munkres_min(&cost);
    starts
        .iter()
        .zip(columns)
        .enumerate()
        .map(|(r, (&(id, _), c))| paths[r][c].take().map(|p| (id, p)).ok_or_else(|| no_path(id)))
        .collect()
}

/// The setpoint that makes a Tier A Drone at `pos` fly at velocity `v` (Mission frame): its braking profile
/// (vehicle.rs) asks for speed √(2·a·d) toward a target d away, so a target v·|v| / 2a ahead gives speed |v|,
/// horizontally and vertically. With the Drone's own velocity it is its stopping point under full braking.
/// shortcut: inverts the Tier A braking profile; for PX4 or Tier B send velocity setpoints instead (a gcs type_mask).
pub fn lead(pos: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    let k = 1.0 / (2.0 * MAX_ACCELERATION);
    let h = v[0].hypot(v[1]);
    [pos[0] + v[0] * h * k, pos[1] + v[1] * h * k, pos[2] + v[2] * v[2].abs() * k]
}

/// One planned flight: the path and what happens at its end.
struct Route {
    path: Vec<[f64; 3]>,
    then: Then,
}

#[derive(Clone, Copy, PartialEq)]
enum Then {
    Hover,
    Land,
}

enum Task {
    /// Disarmed on the ground; nothing streamed.
    Ground,
    /// Selected by a Command that is being planned. Its hold point if it hovers, None on the ground.
    Planning(Option<[f64; 3]>),
    /// Offboard and arm sent; climbing straight up to the path's start.
    TakeOff { route: Route, since: Instant, handshake: JoinHandle<()> },
    Fly { route: Route, next: usize },
    Hover([f64; 3]),
    /// Abort: braking to its stopping point, then it lands.
    Stop { since: Instant },
    /// Land sent; until it is disarmed on the ground.
    Land { since: Instant, handshake: JoinHandle<()> },
}

/// What the latest Telemetry says about one Drone, Mission frame.
#[derive(Clone, Copy)]
struct Seen {
    pos: [f64; 3],
    vel: [f64; 3],
    armed: bool,
    on_ground: bool,
    landing: bool,
}

fn seen(link: &DroneLink) -> Option<Seen> {
    let p = link.position.as_ref()?;
    Some(Seen {
        pos: [p.x, p.y, p.z].map(f64::from),
        vel: [p.vx, p.vy, p.vz].map(f64::from),
        armed: link.armed,
        on_ground: link.landed_state == Some(MavLandedState::MAV_LANDED_STATE_ON_GROUND),
        landing: link.mode == Some("LAND"),
    })
}

/// A planning result coming back to the control loop.
struct Planned {
    epoch: u64,
    source: Source,
    command: Command,
    reply: oneshot::Sender<Verdict>,
    result: Result<Vec<(u8, Route)>, String>,
}

/// The control loop's state; only its own task touches it.
struct Control {
    gcs: Arc<Gcs>,
    field: Arc<DistanceField>,
    place: Box<dyn Fn(&str) -> Result<[f64; 3], String> + Send>,
    view_heading: Arc<Mutex<[f64; 2]>>,
    verdicts: Verdicts,
    tasks: BTreeMap<u8, Task>,
    /// Counts Aborts, so a plan that was started before one is thrown away.
    epoch: u64,
    /// One Command is planned at a time, so its goals are known when the next one spreads its own.
    planning: bool,
    waiting: VecDeque<(Source, Command, oneshot::Sender<Verdict>)>,
    planned: mpsc::UnboundedSender<Planned>,
}

impl Control {
    async fn run(
        mut self,
        mut rx: mpsc::Receiver<(Source, Command, oneshot::Sender<Verdict>)>,
        mut planned_rx: mpsc::UnboundedReceiver<Planned>,
        mut resets: mpsc::Receiver<oneshot::Sender<()>>,
        reset_simulation: impl Fn() + Send,
    ) {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                Some(reply) = resets.recv() => {
                    self.reset(&mut rx).await;
                    reset_simulation();
                    // Let old loopback packets drain and fresh reset telemetry arrive before new Commands.
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    let _ = reply.send(());
                }
                message = rx.recv() => {
                    let Some(first) = message else { break };
                    let mut batch = vec![first];
                    while let Ok(more) = rx.try_recv() {
                        batch.push(more);
                    }
                    self.inbox(batch);
                }
                Some(planned) = planned_rx.recv() => self.planned(planned),
                _ = tick.tick() => self.tick(),
            }
        }
    }

    async fn reset(&mut self, rx: &mut mpsc::Receiver<(Source, Command, oneshot::Sender<Verdict>)>) {
        self.epoch += 1;
        self.planning = false;
        for task in self.tasks.values_mut() {
            match task {
                Task::TakeOff { handshake, .. } | Task::Land { handshake, .. } => {
                    handshake.abort();
                    let _ = handshake.await;
                }
                _ => {},
            }
            *task = Task::Ground;
        }
        self.gcs.reset_control();
        while let Ok(command) = rx.try_recv() { self.waiting.push_back(command); }
        let dropped: Vec<_> = self.waiting.drain(..).collect();
        for (source, command, reply) in dropped {
            self.answer(source, command, reply, Verdict { approved: false, reason: Some("Mission reset.".into()) });
        }
    }

    /// Everything that was queued in the inbox. An Abort in it goes first, and every other Command not yet answered
    /// (queued with it, waiting, or being planned) is rejected, so nothing queued before an Abort moves a Drone after it.
    fn inbox(&mut self, batch: Vec<(Source, Command, oneshot::Sender<Verdict>)>) {
        let (aborts, rest): (Vec<_>, Vec<_>) = batch.into_iter().partition(|(_, c, _)| matches!(c, Command::Abort {}));
        if aborts.is_empty() {
            for (source, command, reply) in rest {
                self.waiting.push_back((source, command, reply));
            }
            self.next_waiting();
            return;
        }
        self.abort();
        for (source, command, reply) in aborts {
            self.answer(source, command, reply, Verdict { approved: true, reason: None });
        }
        let dropped: Vec<_> = self.waiting.drain(..).chain(rest).collect();
        for (source, command, reply) in dropped {
            self.answer(source, command, reply, Verdict { approved: false, reason: Some(ABORT_FIRST.into()) });
        }
    }

    /// Takes waiting Commands in order until one needs planning.
    fn next_waiting(&mut self) {
        while !self.planning {
            let Some((source, command, reply)) = self.waiting.pop_front() else { return };
            self.command(source, command, reply);
        }
    }

    fn answer(&self, source: Source, command: Command, reply: oneshot::Sender<Verdict>, verdict: Verdict) {
        let _ = reply.send(verdict.clone()); // the asker may have stopped waiting
        let _ = self.verdicts.send((source, command, verdict)); // having no subscriber is fine
    }

    fn snapshot(&mut self) -> BTreeMap<u8, Seen> {
        let seen: BTreeMap<u8, Seen> = self.gcs.drones().iter().filter_map(|(id, link)| Some((*id, seen(link)?))).collect();
        for (id, s) in &seen {
            self.tasks.entry(*id).or_insert_with(|| {
                if s.armed {
                    // Already flying when the mission started: hold it where it is.
                    self.gcs.set_target(*id, Some(s.pos.map(|x| x as f32)));
                    Task::Hover(s.pos)
                } else {
                    Task::Ground
                }
            });
        }
        seen
    }

    /// The policy check, then planning off the control loop for a Command that moves Drones.
    fn command(&mut self, source: Source, command: Command, reply: oneshot::Sender<Verdict>) {
        // A position update can arrive between ticks: finish an arrived route before checking whether it is busy.
        self.tick();
        let seen = self.snapshot();
        let views: Vec<DroneView> = seen
            .iter()
            .map(|(&id, s)| {
                let (position, activity) = match &self.tasks[&id] {
                    Task::Ground if slot(id).is_some_and(|p| (p[0] - s.pos[0]).hypot(p[1] - s.pos[1]) <= ON_SLOT_M) => (s.pos, Activity::Parked),
                    Task::Ground => (s.pos, Activity::Landed),
                    Task::Hover(hold) => (*hold, Activity::Hovering),
                    _ => (s.pos, Activity::Busy),
                };
                DroneView { id, position, activity }
            })
            .collect();
        let ctx = Context {
            field: &self.field,
            mission_to_scan: ROOM.to_scan,
            scan_to_mission: ROOM.to_mission,
            place: &*self.place,
            drones: &views,
            view_heading: *self.view_heading.lock().unwrap(),
        };
        let decision = match policy::check(&command, &ctx) {
            Ok(decision) => decision,
            Err(reason) => return self.answer(source, command, reply, Verdict { approved: false, reason: Some(reason) }),
        };
        let (ids, goals, then) = match decision {
            Decision::Dispatch { ids, target } => (ids, Goals::Around(target), Then::Hover),
            Decision::Advance { goals } => (goals.iter().map(|g| g.0).collect(), Goals::Given(goals.iter().map(|g| g.1).collect()), Then::Hover),
            Decision::Recall { ids } => {
                let goals: Option<Vec<[f64; 3]>> = ids.iter().map(|&id| slot(id).map(|s| [s[0], s[1], s[2] - CLIMB_M])).collect();
                let Some(goals) = goals else {
                    return self.answer(source, command, reply, Verdict { approved: false, reason: Some("A Drone has no pad slot.".into()) });
                };
                (ids, Goals::Given(goals), Then::Land)
            }
            Decision::Abort => unreachable!("Abort is handled in inbox()"),
        };
        // Where each selected Drone starts its path: its hold point, or straight above it after the climb.
        let starts: Vec<(u8, [f64; 3])> = ids
            .iter()
            .map(|&id| match self.tasks[&id] {
                Task::Hover(hold) => (id, hold),
                _ => {
                    let p = seen[&id].pos;
                    (id, [p[0], p[1], p[2] - CLIMB_M])
                }
            })
            .collect();
        // Spots other Drones hold or fly to, which the new goals keep away from.
        let others: Vec<[f64; 3]> = seen
            .iter()
            .filter(|(id, _)| !ids.contains(*id))
            .map(|(id, s)| match &self.tasks[id] {
                Task::Hover(hold) => *hold,
                Task::Fly { route, .. } | Task::TakeOff { route, .. } => *route.path.last().unwrap(),
                _ => s.pos,
            })
            .collect();
        for &id in &ids {
            let hold = match self.tasks[&id] {
                Task::Hover(hold) => Some(hold),
                _ => None,
            };
            self.tasks.insert(id, Task::Planning(hold));
        }
        self.planning = true;
        let job = Job { starts, goals, others, then };
        let (field, tx, epoch) = (Arc::clone(&self.field), self.planned.clone(), self.epoch);
        tokio::task::spawn_blocking(move || {
            let result = plan_job(&field, job);
            let _ = tx.send(Planned { epoch, source, command, reply, result });
        });
    }

    fn planned(&mut self, p: Planned) {
        if p.epoch != self.epoch {
            // abort() already reset every Drone this plan selected and cleared `planning`, which may now belong to a
            // newer plan, so neither is touched here.
            self.answer(p.source, p.command, p.reply, Verdict { approved: false, reason: Some("A newer Abort or reset cancelled this plan.".into()) });
            return;
        }
        self.planning = false;
        match p.result {
            Err(reason) => {
                for task in self.tasks.values_mut() {
                    if let Task::Planning(hold) = *task {
                        *task = hold.map_or(Task::Ground, Task::Hover);
                    }
                }
                self.answer(p.source, p.command, p.reply, Verdict { approved: false, reason: Some(reason) });
            }
            Ok(routes) => {
                let now = Instant::now();
                for (id, route) in routes {
                    let task = match self.tasks.remove(&id) {
                        Some(Task::Planning(Some(_))) => Task::Fly { route, next: 1 },
                        _ => {
                            self.gcs.set_target(id, Some(route.path[0].map(|x| x as f32)));
                            let handshake = tokio::spawn(take_off(Arc::clone(&self.gcs), id));
                            Task::TakeOff { route, since: now, handshake }
                        }
                    };
                    self.tasks.insert(id, task);
                }
                self.answer(p.source, p.command, p.reply, Verdict { approved: true, reason: None });
            }
        }
        self.next_waiting();
    }

    /// Every Drone that is not disarmed on the ground brakes to its stopping point, holds HOLD_FOR, then lands.
    fn abort(&mut self) {
        self.epoch += 1;
        self.planning = false;
        let seen = self.snapshot();
        let now = Instant::now();
        for (id, task) in self.tasks.iter_mut() {
            match task {
                Task::Ground | Task::Stop { .. } | Task::Land { .. } => continue,
                Task::Planning(None) => {
                    *task = Task::Ground;
                    continue;
                }
                Task::TakeOff { handshake, .. } => handshake.abort(),
                Task::Planning(Some(_)) | Task::Fly { .. } | Task::Hover(_) => {}
            }
            // A Drone without Telemetry keeps its last setpoint, which is a spot on its path, until it reports again.
            if let Some(s) = seen.get(id) {
                self.gcs.set_target(*id, Some(lead(s.pos, s.vel).map(|x| x as f32)));
            }
            *task = Task::Stop { since: now };
        }
    }

    /// One control step: phase changes, then ORCA-shaped setpoints for every Drone that follows a path or hovers.
    fn tick(&mut self) {
        let seen = self.snapshot();
        let now = Instant::now();
        let mut preferred: Vec<(u8, [f64; 3])> = Vec::new();
        for (&id, task) in self.tasks.iter_mut() {
            let Some(s) = seen.get(&id) else { continue };
            match task {
                Task::Ground | Task::Planning(None) => {}
                Task::Planning(Some(hold)) | Task::Hover(hold) => preferred.push((id, toward(s.pos, *hold))),
                Task::TakeOff { route, since, .. } => {
                    if s.armed && dist(s.pos, route.path[0]) < WAYPOINT_M {
                        let route = std::mem::replace(route, Route { path: Vec::new(), then: Then::Hover });
                        *task = Task::Fly { route, next: 1 };
                    } else if !s.armed && since.elapsed() > TAKEOFF_TIMEOUT {
                        eprintln!("mission: Drone {id} did not arm within {TAKEOFF_TIMEOUT:?}; back on the ground");
                        self.gcs.set_target(id, None);
                        *task = Task::Ground;
                    }
                }
                Task::Fly { route, next } => {
                    let path = &route.path;
                    let last = path.len() - 1;
                    while *next < last && dist(s.pos, path[*next]) < WAYPOINT_M {
                        *next += 1;
                    }
                    let goal = path[last];
                    if *next == last && dist(s.pos, goal) < ARRIVED_M {
                        if route.then == Then::Hover {
                            preferred.push((id, toward(s.pos, goal)));
                            *task = Task::Hover(goal);
                        } else {
                            self.gcs.set_target(id, Some(goal.map(|x| x as f32)));
                            *task = Task::Land { since: now, handshake: land(&self.gcs, id) };
                        }
                        continue;
                    }
                    let to_next = dist(s.pos, path[*next]);
                    let remaining = to_next + path[*next..].windows(2).map(|w| dist(w[0], w[1])).sum::<f64>();
                    let speed = MAX_SPEED.min(SLOW_GAIN * remaining);
                    let v = if to_next > 1e-9 { [0, 1, 2].map(|a| (path[*next][a] - s.pos[a]) / to_next * speed) } else { [0.0; 3] };
                    preferred.push((id, v));
                }
                Task::Stop { since } => {
                    if since.elapsed() >= HOLD_FOR {
                        *task = Task::Land { since: now, handshake: land(&self.gcs, id) };
                    }
                }
                Task::Land { since, handshake } => {
                    if !s.armed && s.on_ground {
                        self.gcs.set_target(id, None);
                        *task = Task::Ground;
                    } else if !s.landing && since.elapsed() > LAND_RETRY {
                        handshake.abort();
                        *handshake = land(&self.gcs, id);
                        *since = now;
                    }
                }
            }
        }
        for (id, mut pref) in preferred {
            let me = seen[&id];
            let neighbours: Vec<DroneState> = seen
                .iter()
                .filter(|&(&other, o)| other != id && dist(o.pos, me.pos) < NEIGHBOUR_RANGE)
                .map(|(_, o)| state(o, [0.0; 3]))
                .collect();
            // ORCA deadlocks on exact head-ons and symmetric groups (the pad is a grid), so near another Drone each one
            // leans a little to its right, by a different amount per Drone.
            if neighbours.iter().any(|o| dist(o.position, me.pos) < BIAS_RANGE) {
                let lean = SIDE_BIAS * (1.0 + 0.2 * id as f64);
                pref = [pref[0] - lean * pref[1], pref[1] + lean * pref[0], pref[2]];
            }
            let v = orca::new_velocity(&state(&me, pref), &neighbours, TIME_HORIZON, TICK.as_secs_f64());
            let mut target = lead(me.pos, v);
            if self.field.at_point((ROOM.to_scan)(target)) < GUARD_M {
                target = lead(me.pos, me.vel);
            }
            self.gcs.set_target(id, Some(target.map(|x| x as f32)));
        }
    }
}

/// Where the goals of a planned Command come from.
enum Goals {
    /// Spread around this target, and Drones assigned to them.
    Around([f64; 3]),
    /// One per Drone, in order.
    Given(Vec<[f64; 3]>),
}

struct Job {
    starts: Vec<(u8, [f64; 3])>,
    goals: Goals,
    others: Vec<[f64; 3]>,
    then: Then,
}

/// Goals, paths and assignment for one Command; runs off the control loop.
fn plan_job(field: &DistanceField, job: Job) -> Result<Vec<(u8, Route)>, String> {
    let (goals, assign) = match job.goals {
        Goals::Around(target) => (spread_goals(field, ROOM, target, job.starts.len(), &job.others)?, true),
        Goals::Given(goals) => {
            // A Drone that will hover there must not crowd one that holds nearby (slots are for landing, 0.6 m apart).
            if job.then == Then::Hover {
                let crowded = job.starts.iter().zip(&goals).find(|(_, &g)| job.others.iter().any(|&o| dist(o, g) < GOAL_SPACING_M));
                if let Some((&(id, _), _)) = crowded {
                    return Err(format!("Drone {id} would end too close to another Drone."));
                }
            }
            (goals, false)
        }
    };
    let routes = plan_routes(field, ROOM, &job.starts, &goals, assign)?;
    Ok(routes.into_iter().map(|(id, path)| (id, Route { path, then: job.then })).collect())
}

/// PX4's offboard sequence: setpoints first (already streaming), then offboard, then arm.
async fn take_off(gcs: Arc<Gcs>, id: u8) {
    tokio::time::sleep(SETPOINTS_FIRST).await;
    let result = async {
        let mut offboard = gcs.set_offboard(id).await;
        for _ in 0..2 {
            if offboard.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
            offboard = gcs.set_offboard(id).await;
        }
        offboard?;
        gcs.arm(id).await
    }
    .await;
    if let Err(e) = result {
        eprintln!("mission: Drone {id} did not take off: {e}");
    }
}

/// Sends land without holding up the control loop; PX4 disarms on its own after touchdown.
fn land(gcs: &Arc<Gcs>, id: u8) -> JoinHandle<()> {
    let gcs = Arc::clone(gcs);
    tokio::spawn(async move {
        if let Err(e) = gcs.land(id).await {
            eprintln!("mission: Drone {id} did not take the land command: {e}");
        }
    })
}

/// Velocity back to a hold point, slowing as it gets close.
fn toward(pos: [f64; 3], hold: [f64; 3]) -> [f64; 3] {
    let d = dist(pos, hold);
    let speed = MAX_SPEED.min(SLOW_GAIN * d);
    if d > 1e-9 { [0, 1, 2].map(|a| (hold[a] - pos[a]) / d * speed) } else { [0.0; 3] }
}

fn state(s: &Seen, pref_velocity: [f64; 3]) -> DroneState {
    DroneState { position: s.pos, velocity: s.vel, radius: RADIUS, max_speed: MAX_SPEED, pref_velocity }
}

/// Drone `id`'s Launch pad slot (system ids 1–6 sit on slots 0–5, as sim_swarm places them).
fn slot(id: u8) -> Option<[f64; 3]> {
    (1..=6).contains(&id).then(|| frame::pad_slot(id as usize - 1))
}

fn length(path: &[[f64; 3]]) -> f64 {
    path.windows(2).map(|w| dist(w[0], w[1])).sum()
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_core::vehicle::Drone;

    /// The policy tests' axes (N = −z, E = x, D = −y) on a 4 × 3 × 4 m room at 0.1 m with a floor at y = 0.
    const TEST: Frame = Frame { to_scan: |[n, e, d]| [e, -d, -n], to_mission: |[x, y, z]| [-z, x, -y] };

    fn room(wall: impl Fn([usize; 3]) -> bool) -> DistanceField {
        DistanceField::from_occupancy([40, 30, 40], [-2.0, -0.05, -2.0], 0.1, |c| c[1] == 0 || wall(c))
    }

    #[test]
    fn goals_spread_from_the_target_keep_their_spacing_and_clearance() {
        let field = room(|_| false);
        let goals = spread_goals(&field, TEST, [0.0, 0.0, -1.0], 3, &[]).unwrap();
        assert_eq!(goals[0], [0.0, 0.0, -1.0], "the target itself first");
        assert!((dist(goals[1], [GOAL_SPACING_M, 0.0, -1.0])) < 1e-9, "then north on the first ring: {goals:?}");
        for (i, a) in goals.iter().enumerate() {
            assert!(field.at_point((TEST.to_scan)(*a)) >= CLEARANCE_M);
            for b in &goals[i + 1..] {
                assert!(dist(*a, *b) >= GOAL_SPACING_M - 1e-9, "{a:?} {b:?}");
            }
        }
        // A Drone holding at the target pushes every goal off it.
        let goals = spread_goals(&field, TEST, [0.0, 0.0, -1.0], 2, &[[0.0, 0.0, -1.0]]).unwrap();
        assert!(goals.iter().all(|g| dist(*g, [0.0, 0.0, -1.0]) >= GOAL_SPACING_M - 1e-9), "{goals:?}");
        // 1.2 m from the north wall (n = 2) only the target and spots up to 0.85 m north of it are clear.
        let goals = spread_goals(&field, TEST, [0.8, 0.0, -1.0], 7, &[]).unwrap();
        assert!(goals.iter().all(|g| g[0] <= 2.0 - CLEARANCE_M), "{goals:?}");
        assert!(spread_goals(&field, TEST, [0.0, 0.0, -1.0], 19, &[[1.6, 0.0, -1.0]]).is_err());
    }

    #[test]
    fn assignment_picks_the_shorter_total_path() {
        let field = room(|_| false);
        // Drone 1 is north, Drone 2 south; the goals are listed south first, so the crossing assignment is the naive one.
        let starts = [(1, [1.0, 0.0, -1.0]), (2, [-1.0, 0.0, -1.0])];
        let goals = [[-1.0, 1.0, -1.0], [1.0, 1.0, -1.0]];
        let routes = plan_routes(&field, TEST, &starts, &goals, true).unwrap();
        assert_eq!(routes.len(), 2);
        assert_eq!((routes[0].0, *routes[0].1.last().unwrap()), (1, [1.0, 1.0, -1.0]));
        assert_eq!((routes[1].0, *routes[1].1.last().unwrap()), (2, [-1.0, 1.0, -1.0]));
        // Without assignment Drone i takes goal i.
        let routes = plan_routes(&field, TEST, &starts, &goals, false).unwrap();
        assert_eq!(*routes[0].1.last().unwrap(), [-1.0, 1.0, -1.0]);
    }

    #[test]
    fn blocked_starts_and_walled_off_goals_are_rejected() {
        // A full wall across n = 0 (scan z cell 20) splits the room.
        let field = room(|[_, _, k]| k == 20);
        let starts = [(3, [1.0, 0.0, -1.0])];
        assert_eq!(plan_routes(&field, TEST, &starts, &[[-1.0, 0.0, -1.0]], true).unwrap_err(), "No clear path for Drone 3.");
        assert_eq!(plan_routes(&field, TEST, &starts, &[[-1.0, 0.0, -1.0]], false).unwrap_err(), "No clear path for Drone 3.");
        let on_floor = [(4, [1.0, 0.0, -0.1])];
        assert_eq!(plan_routes(&field, TEST, &on_floor, &[[1.0, 1.0, -1.0]], true).unwrap_err(), "Drone 4 has no clear space to start from.");
    }

    /// Steps a Tier A Drone for `seconds` toward the target `aim` picks each 50 ms control tick.
    fn fly(drone: &mut Drone, seconds: f64, aim: impl Fn(&Drone) -> [f64; 3]) -> Vec<f64> {
        let mut speeds = Vec::new();
        for _ in 0..(seconds / 0.05).round() as usize {
            let target = aim(drone);
            for _ in 0..12 {
                drone.step(0.004, target); // the vehicle's 250 Hz physics, about one 50 ms tick
            }
            speeds.push(drone.velocity.iter().map(|v| v * v).sum::<f64>().sqrt());
        }
        speeds
    }

    #[test]
    fn lead_setpoints_fly_the_tier_a_drone_at_the_asked_velocity() {
        let mut drone = Drone::on_floor();
        drone.position = [0.0, 0.0, -1.0];
        drone.landed = false;
        drone.arm();
        let speeds = fly(&mut drone, 3.0, |d| lead(d.position, [0.6, 0.8, 0.0]));
        let cruise = &speeds[20..];
        assert!(cruise.iter().all(|s| (0.85..=1.05).contains(s)), "cruise speeds {cruise:?}");
        assert!((drone.velocity[1] / drone.velocity[0] - 0.8 / 0.6).abs() < 0.01, "heading kept: {:?}", drone.velocity);
    }

    #[test]
    fn stopping_point_halts_the_drone_within_a_second_without_coming_back() {
        let mut drone = Drone::on_floor();
        drone.position = [0.0, 0.0, -1.0];
        drone.velocity = [1.0, 0.0, 0.0];
        drone.landed = false;
        drone.arm();
        let hold = lead(drone.position, drone.velocity);
        let speeds = fly(&mut drone, 2.0, |_| hold);
        assert!(speeds[..20].iter().any(|&s| s < 0.1) && speeds[19] < 0.1, "still within 1 s: {speeds:?}");
        assert!(speeds[19..].iter().all(|&s| s < 0.1), "and stays still: {speeds:?}");
        assert!(dist(drone.position, hold) < 0.02, "held at {hold:?}, ended at {:?}", drone.position);
    }

    #[tokio::test]
    async fn old_plan_after_abort_cannot_release_or_replace_a_newer_plan() {
        let gcs = Arc::new(Gcs::bind("127.0.0.1:0").await.unwrap());
        let mut control = Control {
            gcs, field: Arc::new(room(|_| false)), place: Box::new(|_| Err("unknown".into())),
            view_heading: Arc::new(Mutex::new([1.0, 0.0])), verdicts: broadcast::channel(16).0,
            tasks: BTreeMap::new(), epoch: 0, planning: true, waiting: VecDeque::new(),
            planned: mpsc::unbounded_channel().0,
        };
        let (abort_tx, abort_rx) = oneshot::channel();
        let (dispatch_tx, dispatch_rx) = oneshot::channel();
        let dispatch = Command::from_json(r#"{"kind":"dispatch","drones":{"count":1},"target":{"place":"door"}}"#).unwrap();
        control.inbox(vec![(Source::Key, dispatch.clone(), dispatch_tx), (Source::Key, Command::Abort {}, abort_tx)]);
        assert!(abort_rx.await.unwrap().approved);
        assert_eq!(dispatch_rx.await.unwrap().reason.as_deref(), Some(ABORT_FIRST));
        assert!(!control.planning);
        // A post-Abort command now owns the planning slot when the older worker finally finishes.
        control.planning = true;
        let (old_tx, old_rx) = oneshot::channel();
        control.planned(Planned {
            epoch: 0, source: Source::Key, command: dispatch, reply: old_tx, result: Ok(vec![]),
        });
        assert_eq!(old_rx.await.unwrap().reason.as_deref(), Some("A newer Abort or reset cancelled this plan."));
        assert!(control.planning, "the old result released the newer planning slot");
        assert!(control.tasks.is_empty(), "the old result started a route");
    }
}
