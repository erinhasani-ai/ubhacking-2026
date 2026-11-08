//! The policy check that gives every Command its Verdict (T13). Pure: the caller passes the Room Twin's distance
//! field (scan frame), the Mission-frame conversions, the Place lookup and what each Drone is doing.
//! WebAssembly-safe: no file access, no threads, no clock.
//!
//! Rules (SPEC T13 and the decisions log):
//! - A target, a Place or a point, must be inside the Room Twin and in free space at least CLEARANCE_M from every
//!   occupied voxel centre.
//! - Surface hit: a point closer than CLEARANCE_M to an obstacle whose spot SURFACE_LIFT_M above (along −D) is free
//!   becomes that hover point. Clicks and pointing rays end on a surface, so this is how they turn into targets;
//!   a point already free is kept as it is.
//! - An unknown Place is rejected with the lookup's message, which names the known Places.
//! - "there" is rejected until T20/T30 resolve it to a point before it reaches the policy.
//! - Only idle Drones (Parked, Landed or Hovering) take a new Command. A Busy Drone (taking off, flying, stopping or
//!   landing) is never redirected; it finishes, or Abort stops it. Named or counted Drones must be idle, and "all"
//!   is rejected while any Drone it covers is Busy, so a Command never runs on part of the Swarm without saying so.
//! - Abort is always approved.

use crate::command::{Command, CountBody, Drones, IdsBody, PlaceBody, Point, PointBody, Target};
use crate::planner::DistanceField;

/// Distance every Drone keeps from every occupied voxel centre (SPEC T10).
pub const CLEARANCE_M: f64 = 0.35;
/// A surface hit becomes a hover point this far above it (SPEC decisions log, Surface targets).
pub const SURFACE_LIFT_M: f64 = 0.5;
/// How far one Advance moves the Drones (SPEC decisions log, Gestures).
pub const ADVANCE_STEP_M: f64 = 1.0;

/// What a Drone is doing, as the mission sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// On the ground on its Launch pad slot.
    Parked,
    /// On the ground somewhere else, after an Abort.
    Landed,
    /// In the air, holding at its hold point.
    Hovering,
    /// Taking off, flying, stopping or landing.
    Busy,
}

impl Activity {
    pub fn is_idle(self) -> bool {
        self != Activity::Busy
    }
}

/// One connected Drone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DroneView {
    pub id: u8,
    /// Mission frame (north, east, down, metres). A Hovering Drone's hold point.
    pub position: [f64; 3],
    pub activity: Activity,
}

/// Everything the policy check reads.
pub struct Context<'a> {
    /// The Room Twin's distance field, scan frame.
    pub field: &'a DistanceField,
    pub mission_to_scan: fn([f64; 3]) -> [f64; 3],
    pub scan_to_mission: fn([f64; 3]) -> [f64; 3],
    /// A Place's hover point in scan coordinates, or a message naming the known Places.
    pub place: &'a dyn Fn(&str) -> Result<[f64; 3], String>,
    /// Every connected Drone.
    pub drones: &'a [DroneView],
    /// The view's horizontal forward as a unit (north, east) vector; Advance moves along it.
    pub view_heading: [f64; 2],
}

/// What an approved Command does.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Send these Drones to the target (Mission frame). Counted Drones are listed nearest first.
    Dispatch { ids: Vec<u8>, target: [f64; 3] },
    /// Move each Drone to its goal (Mission frame), ADVANCE_STEP_M along the view heading.
    Advance { goals: Vec<(u8, [f64; 3])> },
    /// Fly these Drones back to their pad slots and land them.
    Recall { ids: Vec<u8> },
    Abort,
}

/// The Verdict for one Command: what it does, or why it is rejected, in a sentence a judge can read on screen.
pub fn check(command: &Command, ctx: &Context) -> Result<Decision, String> {
    match command {
        Command::Abort {} => Ok(Decision::Abort),
        Command::Dispatch { drones, target } => {
            let target = resolve_target(target, ctx)?;
            let picked = select(drones, ctx, "idle", |_| Ok(()), |d| dist(d.position, target))?;
            Ok(Decision::Dispatch { ids: picked, target })
        }
        Command::Advance { drones } => {
            let ids = select(drones, ctx, "hovering", on_the_ground, |d| d.id as f64)?;
            let [hn, he] = ctx.view_heading;
            let step = [hn * ADVANCE_STEP_M, he * ADVANCE_STEP_M, 0.0];
            let mut goals = Vec::with_capacity(ids.len());
            for id in ids {
                let from = view(ctx, id).position;
                let goal = [0, 1, 2].map(|a| from[a] + step[a]);
                if let Some(why) = blocked(ctx, goal, &format!("Drone {id}'s spot {ADVANCE_STEP_M} m ahead")) {
                    return Err(why);
                }
                goals.push((id, goal));
            }
            Ok(Decision::Advance { goals })
        }
        Command::Recall { drones } => {
            let ids = select(drones, ctx, "away from the Launch pad", already_parked, |d| d.id as f64)?;
            Ok(Decision::Recall { ids })
        }
    }
}

/// The target in the Mission frame, checked: a Place, a point (lifted if it is a surface hit), never "there" yet.
fn resolve_target(target: &Target, ctx: &Context) -> Result<[f64; 3], String> {
    match target {
        Target::Word(_) => Err("No pointing ray yet.".into()),
        Target::Place(PlaceBody { place }) => {
            let at = (ctx.scan_to_mission)((ctx.place)(place)?);
            match blocked(ctx, at, &format!("The {place}")) {
                Some(why) => Err(why),
                None => Ok(at),
            }
        }
        Target::Point(PointBody { point: Point { n, e, d } }) => {
            let at = [*n, *e, *d];
            let Some(why) = blocked(ctx, at, "The target") else { return Ok(at) };
            let lifted = [*n, *e, *d - SURFACE_LIFT_M];
            if inside(ctx, at) && blocked(ctx, lifted, "").is_none() {
                Ok(lifted)
            } else {
                Err(why)
            }
        }
    }
}

/// Why a Drone cannot hold at `at` (Mission frame), or None if it can. `what` starts the sentence.
fn blocked(ctx: &Context, at: [f64; 3], what: &str) -> Option<String> {
    if !inside(ctx, at) {
        return Some(format!("{what} is outside the Room Twin."));
    }
    let clear = ctx.field.at_point((ctx.mission_to_scan)(at));
    if clear == 0.0 {
        Some(format!("{what} is inside an obstacle."))
    } else if clear < CLEARANCE_M {
        Some(format!("{what} is {clear:.2} m from an obstacle; a Drone needs {CLEARANCE_M} m."))
    } else {
        None
    }
}

fn inside(ctx: &Context, at: [f64; 3]) -> bool {
    ctx.field.cell_of((ctx.mission_to_scan)(at)).is_some()
}

/// The Drones a Command covers. Each must be connected and idle and pass `fits` (Err is the reason). A count takes the
/// fitting Drones with the lowest `rank`, "all" takes every Drone that passes `fits` or is Busy, and is rejected while
/// one of them is Busy. `state` names the fitting Drones in the shortfall message.
fn select(
    drones: &Drones,
    ctx: &Context,
    state: &str,
    fits: impl Fn(&DroneView) -> Result<(), String>,
    rank: impl Fn(&DroneView) -> f64,
) -> Result<Vec<u8>, String> {
    let ready = |d: &DroneView| d.activity.is_idle() && fits(d).is_ok();
    match drones {
        Drones::Ids(IdsBody { ids }) => {
            for &id in ids {
                let Some(d) = ctx.drones.iter().find(|d| d.id == id) else {
                    return Err(format!("Drone {id} is not connected."));
                };
                if !d.activity.is_idle() {
                    return Err(format!("Drone {id} is busy."));
                }
                fits(d)?;
            }
            Ok(ids.clone())
        }
        Drones::Count(CountBody { count }) => {
            let mut fitting: Vec<&DroneView> = ctx.drones.iter().filter(|d| ready(d)).collect();
            if fitting.len() < *count as usize {
                return Err(shortfall(fitting.len(), *count, state));
            }
            fitting.sort_by(|a, b| rank(a).total_cmp(&rank(b)).then(a.id.cmp(&b.id)));
            Ok(fitting[..*count as usize].iter().map(|d| d.id).collect())
        }
        Drones::Word(_) => {
            let mut covered: Vec<&DroneView> =
                ctx.drones.iter().filter(|d| !d.activity.is_idle() || fits(d).is_ok()).collect();
            covered.sort_by_key(|d| d.id);
            if let Some(busy) = covered.iter().find(|d| !d.activity.is_idle()) {
                return Err(format!("Drone {} is busy.", busy.id));
            }
            if covered.is_empty() {
                return Err(shortfall(0, 1, state));
            }
            Ok(covered.iter().map(|d| d.id).collect())
        }
    }
}

fn on_the_ground(d: &DroneView) -> Result<(), String> {
    match d.activity {
        Activity::Parked | Activity::Landed => Err(format!("Drone {} is on the ground.", d.id)),
        _ => Ok(()),
    }
}

fn already_parked(d: &DroneView) -> Result<(), String> {
    match d.activity {
        Activity::Parked => Err(format!("Drone {} is already on the Launch pad.", d.id)),
        _ => Ok(()),
    }
}

fn shortfall(have: usize, want: u8, state: &str) -> String {
    match have {
        0 => format!("No Drone is {state}."),
        1 => format!("Only 1 Drone is {state}; {want} were asked for."),
        n => format!("Only {n} Drones are {state}; {want} were asked for."),
    }
}

fn view<'a>(ctx: &'a Context, id: u8) -> &'a DroneView {
    ctx.drones.iter().find(|d| d.id == id).expect("select returns connected Drones only")
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The test room's Mission frame: the same axes as T8's (N = −z, E = x, D = −y), floor at y = 0.
    fn to_scan([n, e, d]: [f64; 3]) -> [f64; 3] {
        [e, -d, -n]
    }

    fn to_mission([x, y, z]: [f64; 3]) -> [f64; 3] {
        [-z, x, -y]
    }

    /// A 4 × 3 × 4 m room at 0.1 m: x and z from −2 to 2, y from −0.05 to 2.95 (outside counts as occupied, so those
    /// are the walls). The floor layer j = 0 has its centres at y = 0. A pillar fills column i = 10, k = 10 (centre
    /// x = z = −0.95, so n = 0.95, e = −0.95). A table top fills j = 8 (centre y = 0.8, d = −0.8) over i, k in 25..=30
    /// (x and z from 0.55 to 1.05, so n from −1.05 to −0.55, e from 0.55 to 1.05).
    fn room() -> DistanceField {
        DistanceField::from_occupancy([40, 30, 40], [-2.0, -0.05, -2.0], 0.1, |[i, j, k]| {
            j == 0 || (i == 10 && k == 10) || (j == 8 && (25..=30).contains(&i) && (25..=30).contains(&k))
        })
    }

    fn places(name: &str) -> Result<[f64; 3], String> {
        match name {
            "door" => Ok([1.55, 1.25, -1.55]),
            _ => Err(format!("There is no Place called \"{name}\". Known Places: door.")),
        }
    }

    /// Six Drones on slots 0.6 m apart around the origin, as T8's pad: n = (i / 3) 0.6 − 0.3, e = (i % 3) 0.6 − 0.6.
    fn pad(activity: impl Fn(u8) -> Activity) -> Vec<DroneView> {
        (0..6u8)
            .map(|i| DroneView {
                id: i + 1,
                position: [(i / 3) as f64 * 0.6 - 0.3, (i % 3) as f64 * 0.6 - 0.6, 0.0],
                activity: activity(i + 1),
            })
            .collect()
    }

    fn verdict(field: &DistanceField, drones: &[DroneView], wire: serde_json::Value) -> Result<Decision, String> {
        let command = Command::from_json(&wire.to_string()).expect("test Commands are valid");
        let ctx = Context {
            field,
            mission_to_scan: to_scan,
            scan_to_mission: to_mission,
            place: &places,
            drones,
            view_heading: [1.0, 0.0],
        };
        check(&command, &ctx)
    }

    fn dispatch_point(n: f64, e: f64, d: f64) -> serde_json::Value {
        json!({"kind": "dispatch", "drones": {"count": 2}, "target": {"point": {"n": n, "e": e, "d": d}}})
    }

    #[test]
    fn free_target_with_enough_idle_drones_is_approved_with_the_nearest() {
        let field = room();
        // North-east of the pad, 1.2 m up: slots 6 (0.3, 0.6) and 5 (0.3, 0.0) are nearest.
        let got = verdict(&field, &pad(|_| Activity::Parked), dispatch_point(1.0, 1.0, -1.2));
        assert_eq!(got, Ok(Decision::Dispatch { ids: vec![6, 5], target: [1.0, 1.0, -1.2] }));
        // Busy Drones are skipped by a count.
        let got = verdict(&field, &pad(|id| if id >= 5 { Activity::Busy } else { Activity::Parked }), dispatch_point(1.0, 1.0, -1.2));
        assert_eq!(got, Ok(Decision::Dispatch { ids: vec![3, 2], target: [1.0, 1.0, -1.2] }));
    }

    #[test]
    fn occupied_target_is_rejected() {
        let field = room();
        // Inside the pillar at 1 m, and still inside it 0.5 m higher, so it is not a surface hit.
        let got = verdict(&field, &pad(|_| Activity::Parked), dispatch_point(0.95, -0.95, -1.0));
        assert_eq!(got, Err("The target is inside an obstacle.".into()));
    }

    #[test]
    fn target_too_close_to_an_obstacle_is_rejected_with_the_distance() {
        let field = room();
        // Two cells east of the pillar (e = −0.75, cell i = 12) at 1 m: 0.2 m from it, here and 0.5 m higher.
        let got = verdict(&field, &pad(|_| Activity::Parked), dispatch_point(0.95, -0.75, -1.0));
        assert_eq!(got, Err("The target is 0.20 m from an obstacle; a Drone needs 0.35 m.".into()));
    }

    #[test]
    fn target_outside_the_room_twin_is_rejected() {
        let field = room();
        for (n, e, d) in [(5.0, 0.0, -1.0), (0.0, -2.5, -1.0), (0.0, 0.0, -3.5), (0.0, 0.0, 0.3)] {
            let got = verdict(&field, &pad(|_| Activity::Parked), dispatch_point(n, e, d));
            assert_eq!(got, Err("The target is outside the Room Twin.".into()), "({n}, {e}, {d})");
        }
    }

    #[test]
    fn surface_hit_becomes_a_hover_point_half_a_metre_above() {
        let field = room();
        let parked = pad(|_| Activity::Parked);
        // A hit on the table top (its voxel centre, d = −0.8) hovers 0.5 m higher, 0.5 m from the table.
        let got = verdict(&field, &parked, dispatch_point(-0.8, 0.8, -0.8));
        assert_eq!(got, Ok(Decision::Dispatch { ids: vec![3, 2], target: [-0.8, 0.8, -0.8 - 0.5] }));
        // A hit on the floor hovers 0.5 m above it.
        let got = verdict(&field, &parked, dispatch_point(1.0, 1.0, 0.0));
        assert_eq!(got, Ok(Decision::Dispatch { ids: vec![6, 5], target: [1.0, 1.0, -0.5] }));
        // A hit 0.1 m above the table, still too close, is lifted too.
        let got = verdict(&field, &parked, dispatch_point(-0.8, 0.8, -0.9));
        assert_eq!(got, Ok(Decision::Dispatch { ids: vec![3, 2], target: [-0.8, 0.8, -0.9 - 0.5] }));
    }

    #[test]
    fn places_resolve_and_an_unknown_place_names_the_known_ones() {
        let field = room();
        let parked = pad(|_| Activity::Parked);
        let to_door = json!({"kind": "dispatch", "drones": {"count": 1}, "target": {"place": "door"}});
        assert_eq!(verdict(&field, &parked, to_door), Ok(Decision::Dispatch { ids: vec![6], target: [1.55, 1.55, -1.25] }));
        let to_kitchen = json!({"kind": "dispatch", "drones": {"count": 1}, "target": {"place": "kitchen"}});
        assert_eq!(verdict(&field, &parked, to_kitchen), Err("There is no Place called \"kitchen\". Known Places: door.".into()));
    }

    #[test]
    fn there_is_rejected_until_it_is_resolved_to_a_point() {
        let got = verdict(&room(), &pad(|_| Activity::Parked), json!({"kind": "dispatch", "drones": "all", "target": "there"}));
        assert_eq!(got, Err("No pointing ray yet.".into()));
    }

    #[test]
    fn too_few_idle_drones_are_rejected() {
        let field = room();
        let one_idle = pad(|id| if id == 4 { Activity::Hovering } else { Activity::Busy });
        assert_eq!(verdict(&field, &one_idle, dispatch_point(1.0, 1.0, -1.2)), Err("Only 1 Drone is idle; 2 were asked for.".into()));
        let none_idle = pad(|_| Activity::Busy);
        assert_eq!(verdict(&field, &none_idle, dispatch_point(1.0, 1.0, -1.2)), Err("No Drone is idle.".into()));
        let only_three = &pad(|_| Activity::Parked)[..3];
        let six = json!({"kind": "dispatch", "drones": {"count": 6}, "target": {"point": {"n": 1.0, "e": 1.0, "d": -1.2}}});
        assert_eq!(verdict(&field, only_three, six), Err("Only 3 Drones are idle; 6 were asked for.".into()));
    }

    #[test]
    fn named_drones_must_be_connected_and_idle() {
        let field = room();
        let drones = pad(|id| if id == 3 { Activity::Busy } else { Activity::Parked });
        let to = |ids: serde_json::Value| json!({"kind": "dispatch", "drones": {"ids": ids}, "target": {"point": {"n": 1.0, "e": 1.0, "d": -1.2}}});
        assert_eq!(verdict(&field, &drones, to(json!([2, 1]))), Ok(Decision::Dispatch { ids: vec![2, 1], target: [1.0, 1.0, -1.2] }));
        assert_eq!(verdict(&field, &drones, to(json!([1, 3]))), Err("Drone 3 is busy.".into()));
        assert_eq!(verdict(&field, &drones[..5], to(json!([6]))), Err("Drone 6 is not connected.".into()));
    }

    #[test]
    fn all_is_rejected_while_a_drone_is_busy() {
        let field = room();
        let all = json!({"kind": "dispatch", "drones": "all", "target": {"point": {"n": 1.0, "e": 1.0, "d": -1.2}}});
        let ids = verdict(&field, &pad(|_| Activity::Parked), all.clone());
        assert_eq!(ids, Ok(Decision::Dispatch { ids: vec![1, 2, 3, 4, 5, 6], target: [1.0, 1.0, -1.2] }));
        let got = verdict(&field, &pad(|id| if id == 2 { Activity::Busy } else { Activity::Parked }), all.clone());
        assert_eq!(got, Err("Drone 2 is busy.".into()));
        assert_eq!(verdict(&field, &[], all), Err("No Drone is idle.".into()));
    }

    #[test]
    fn abort_is_always_approved() {
        let abort = json!({"kind": "abort"});
        assert_eq!(verdict(&room(), &pad(|_| Activity::Busy), abort.clone()), Ok(Decision::Abort));
        assert_eq!(verdict(&room(), &[], abort), Ok(Decision::Abort));
    }

    /// Drones 1 and 2 hover 1 m up at (0, 0) and (0, 1); the rest are parked.
    fn two_hovering() -> Vec<DroneView> {
        let mut drones = pad(|_| Activity::Parked);
        drones[0] = DroneView { id: 1, position: [0.0, 0.0, -1.0], activity: Activity::Hovering };
        drones[1] = DroneView { id: 2, position: [0.0, 1.0, -1.0], activity: Activity::Hovering };
        drones
    }

    #[test]
    fn advance_moves_the_hovering_drones_one_metre_along_the_view() {
        let field = room();
        let all = json!({"kind": "advance", "drones": "all"});
        let goals = vec![(1, [1.0, 0.0, -1.0]), (2, [1.0, 1.0, -1.0])];
        assert_eq!(verdict(&field, &two_hovering(), all), Ok(Decision::Advance { goals }));
        let one = json!({"kind": "advance", "drones": {"count": 1}});
        assert_eq!(verdict(&field, &two_hovering(), one), Ok(Decision::Advance { goals: vec![(1, [1.0, 0.0, -1.0])] }));
    }

    #[test]
    fn advance_toward_an_obstacle_or_from_the_ground_is_rejected() {
        let field = room();
        let mut drones = two_hovering();
        // 1 m north of (−0.05, −0.95) is the pillar's cell.
        drones[0].position = [-0.05, -0.95, -1.0];
        let got = verdict(&field, &drones, json!({"kind": "advance", "drones": {"ids": [1]}}));
        assert_eq!(got, Err("Drone 1's spot 1 m ahead is inside an obstacle.".into()));
        let got = verdict(&field, &drones, json!({"kind": "advance", "drones": {"ids": [3]}}));
        assert_eq!(got, Err("Drone 3 is on the ground.".into()));
        let got = verdict(&field, &pad(|_| Activity::Parked), json!({"kind": "advance", "drones": "all"}));
        assert_eq!(got, Err("No Drone is hovering.".into()));
    }

    #[test]
    fn recall_takes_the_drones_away_from_the_pad() {
        let field = room();
        let mut drones = two_hovering();
        drones[5].activity = Activity::Landed;
        let all = json!({"kind": "recall", "drones": "all"});
        assert_eq!(verdict(&field, &drones, all.clone()), Ok(Decision::Recall { ids: vec![1, 2, 6] }));
        assert_eq!(verdict(&field, &pad(|_| Activity::Parked), all), Err("No Drone is away from the Launch pad.".into()));
        let got = verdict(&field, &drones, json!({"kind": "recall", "drones": {"ids": [2, 4]}}));
        assert_eq!(got, Err("Drone 4 is already on the Launch pad.".into()));
    }
}
