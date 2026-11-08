//! Collision avoidance between Drones (ORCA), filled in by T11.
//!
//! Optimal Reciprocal Collision Avoidance in 3D, implemented from J. van den Berg, S. J. Guy,
//! M. C. Lin and D. Manocha, "Reciprocal n-Body Collision Avoidance", Robotics Research (ISRR 2009),
//! Springer Tracts in Advanced Robotics 70, 2011, and the RVO2-3D reference algorithm
//! (`Agent::computeNewVelocity` and linear programs 1–4, UNC GAMMA group).
//!
//! Each neighbour adds one ORCA half-space in velocity space: the plane through
//! `velocity + u / 2` with normal `n`, where `u` is the smallest change of the relative velocity
//! that leaves the velocity obstacle truncated at the time horizon. Each Drone takes half (`u / 2`),
//! trusting the other to take the other half. The new velocity is the one closest to the preferred
//! velocity inside the speed sphere and every half-space (linear program 3); if none exists, it is
//! the one that minimises the largest violation (linear program 4).
//!
//! Frame-agnostic: positions in metres, velocities in m/s, any right-handed frame.

use glam::DVec3;

/// One Drone as ORCA sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DroneState {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    /// Safety radius (SPEC: 0.3 m).
    pub radius: f64,
    pub max_speed: f64,
    pub pref_velocity: [f64; 3],
}

/// RVO2-3D's RVO_EPSILON: squared lengths at or below it count as zero (parallel planes and lines).
const EPSILON: f64 = 1e-5;

/// Velocities `v` with `(v - point) · normal >= 0` are allowed.
struct Plane {
    point: DVec3,
    normal: DVec3,
}

struct Line {
    point: DVec3,
    direction: DVec3,
}

/// The velocity `drone` should fly next, avoiding `neighbours` (which exclude `drone`; the caller
/// picks who is near) for `time_horizon` seconds. `time_step` is the control step; it only matters
/// when two Drones already overlap, where the overlap is resolved within one step. Never faster
/// than `drone.max_speed`.
pub fn new_velocity(drone: &DroneState, neighbours: &[DroneState], time_horizon: f64, time_step: f64) -> [f64; 3] {
    debug_assert!(time_horizon > 0.0 && time_step > 0.0);
    let planes: Vec<Plane> = neighbours.iter().map(|other| orca_plane(drone, other, 1.0 / time_horizon, 1.0 / time_step)).collect();
    let (failed, mut result) = linear_program3(&planes, drone.max_speed, DVec3::from_array(drone.pref_velocity), false);
    if failed < planes.len() {
        result = linear_program4(&planes, failed, drone.max_speed, result);
    }
    result.to_array()
}

fn orca_plane(drone: &DroneState, other: &DroneState, inv_time_horizon: f64, inv_time_step: f64) -> Plane {
    let velocity = DVec3::from_array(drone.velocity);
    let rel_pos = DVec3::from_array(other.position) - DVec3::from_array(drone.position);
    let rel_vel = velocity - DVec3::from_array(other.velocity);
    let dist_sq = rel_pos.length_squared();
    let combined_radius = drone.radius + other.radius;
    let combined_radius_sq = combined_radius * combined_radius;

    // `u` = magnitude * normal.
    let (normal, magnitude) = if dist_sq > combined_radius_sq {
        // No collision yet. `w` runs from the centre of the cutoff sphere to the relative velocity.
        let w = rel_vel - inv_time_horizon * rel_pos;
        let w_len_sq = w.length_squared();
        let dot = w.dot(rel_pos);
        if dot < 0.0 && dot * dot > combined_radius_sq * w_len_sq {
            // Nearest boundary point is on the cutoff sphere.
            let w_len = w_len_sq.sqrt();
            (w / w_len, combined_radius * inv_time_horizon - w_len)
        } else {
            // Nearest boundary point is on the cone's side: the sphere of radius t·R at t·rel_pos
            // whose outward normal through the relative velocity is perpendicular to the cone.
            let a = dist_sq;
            let b = rel_pos.dot(rel_vel);
            let c = rel_vel.length_squared() - rel_pos.cross(rel_vel).length_squared() / (dist_sq - combined_radius_sq);
            let t = (b + (b * b - a * c).max(0.0).sqrt()) / a;
            let w = rel_vel - t * rel_pos;
            let w_len = w.length();
            if w_len > 1e-9 * rel_vel.length() {
                (w / w_len, combined_radius * t - w_len)
            } else {
                // Not in RVO2-3D: the relative velocity lies on the cone's axis (an exact head-on), where `w`
                // is zero or rounding noise and could even point at the neighbour. Use the plane at the front
                // of the cutoff sphere instead; the whole truncated cone lies behind it.
                let dist = dist_sq.sqrt();
                (-rel_pos / dist, rel_vel.dot(rel_pos) / dist - (dist - combined_radius) * inv_time_horizon)
            }
        }
    } else {
        // Already overlapping: leave the velocity obstacle of a one-step horizon.
        let w = rel_vel - inv_time_step * rel_pos;
        let w_len = w.length();
        (w / w_len, combined_radius * inv_time_step - w_len)
    };
    Plane { point: velocity + 0.5 * magnitude * normal, normal }
}

/// Best point on `line` inside the speed sphere and planes `0..plane_no`, or None if there is none.
fn linear_program1(planes: &[Plane], plane_no: usize, line: &Line, radius: f64, opt: DVec3, direction_opt: bool) -> Option<DVec3> {
    let dot = line.point.dot(line.direction);
    let discriminant = dot * dot + radius * radius - line.point.length_squared();
    if discriminant < 0.0 {
        return None; // the speed sphere misses the line
    }
    let sqrt_discriminant = discriminant.sqrt();
    let mut t_left = -dot - sqrt_discriminant;
    let mut t_right = -dot + sqrt_discriminant;

    for plane in &planes[..plane_no] {
        let numerator = (plane.point - line.point).dot(plane.normal);
        let denominator = line.direction.dot(plane.normal);
        if denominator * denominator <= EPSILON {
            // Line (almost) parallel to the plane: wholly inside or wholly outside.
            if numerator > 0.0 {
                return None;
            }
            continue;
        }
        let t = numerator / denominator;
        if denominator >= 0.0 {
            t_left = t_left.max(t);
        } else {
            t_right = t_right.min(t);
        }
        if t_left > t_right {
            return None;
        }
    }

    let t = if direction_opt {
        if opt.dot(line.direction) > 0.0 { t_right } else { t_left }
    } else {
        line.direction.dot(opt - line.point).max(t_left).min(t_right) // not clamp(): it panics on NaN input
    };
    Some(line.point + t * line.direction)
}

/// Best point on plane `plane_no` inside the speed sphere and planes `0..plane_no`, or None.
fn linear_program2(planes: &[Plane], plane_no: usize, radius: f64, opt: DVec3, direction_opt: bool) -> Option<DVec3> {
    let plane = &planes[plane_no];
    let plane_dist = plane.point.dot(plane.normal);
    let plane_dist_sq = plane_dist * plane_dist;
    let radius_sq = radius * radius;
    if plane_dist_sq > radius_sq {
        return None; // the speed sphere misses the plane
    }
    let plane_radius_sq = radius_sq - plane_dist_sq;
    let plane_center = plane_dist * plane.normal;

    let mut result = if direction_opt {
        let plane_opt = opt - opt.dot(plane.normal) * plane.normal;
        let plane_opt_len_sq = plane_opt.length_squared();
        if plane_opt_len_sq <= EPSILON {
            plane_center
        } else {
            plane_center + (plane_radius_sq / plane_opt_len_sq).sqrt() * plane_opt
        }
    } else {
        let on_plane = opt + (plane.point - opt).dot(plane.normal) * plane.normal;
        if on_plane.length_squared() > radius_sq {
            let from_center = on_plane - plane_center;
            plane_center + (plane_radius_sq / from_center.length_squared()).sqrt() * from_center
        } else {
            on_plane
        }
    };

    for i in 0..plane_no {
        if planes[i].normal.dot(planes[i].point - result) > 0.0 {
            // Violates plane i: the best point now lies on the line where planes i and plane_no meet.
            let cross = planes[i].normal.cross(plane.normal);
            if cross.length_squared() <= EPSILON {
                return None; // parallel, and plane i excludes all of plane_no
            }
            let direction = cross.normalize();
            let line_normal = direction.cross(plane.normal);
            let point = plane.point + ((planes[i].point - plane.point).dot(planes[i].normal) / line_normal.dot(planes[i].normal)) * line_normal;
            result = linear_program1(planes, i, &Line { point, direction }, radius, opt, direction_opt)?;
        }
    }
    Some(result)
}

/// Velocity closest to `opt` (or furthest along the unit direction `opt` when `direction_opt`) inside
/// the speed sphere and all planes. Returns the index of the first plane that could not be met
/// (`planes.len()` on success) and the best result found before it.
fn linear_program3(planes: &[Plane], radius: f64, opt: DVec3, direction_opt: bool) -> (usize, DVec3) {
    let mut result = if direction_opt {
        opt * radius
    } else if opt.length_squared() > radius * radius {
        opt.normalize() * radius
    } else {
        opt
    };
    for (i, plane) in planes.iter().enumerate() {
        if plane.normal.dot(plane.point - result) > 0.0 {
            match linear_program2(planes, i, radius, opt, direction_opt) {
                Some(r) => result = r,
                None => return (i, result),
            }
        }
    }
    (planes.len(), result)
}

/// Velocity inside the speed sphere that minimises the largest violation of planes `begin..`,
/// starting from `result`, which already satisfies planes `..begin`.
fn linear_program4(planes: &[Plane], begin: usize, radius: f64, mut result: DVec3) -> DVec3 {
    let mut distance = 0.0;
    for i in begin..planes.len() {
        let plane = &planes[i];
        if plane.normal.dot(plane.point - result) > distance {
            // Planes 0..i projected onto plane i: where each meets plane i, tilted halfway between them.
            let projected: Vec<Plane> = planes[..i]
                .iter()
                .filter_map(|other| {
                    let cross = other.normal.cross(plane.normal);
                    let point = if cross.length_squared() <= EPSILON {
                        if plane.normal.dot(other.normal) > 0.0 {
                            return None; // parallel, same direction
                        }
                        0.5 * (plane.point + other.point)
                    } else {
                        let line_normal = cross.cross(plane.normal);
                        plane.point + ((other.point - plane.point).dot(other.normal) / line_normal.dot(other.normal)) * line_normal
                    };
                    Some(Plane { point, normal: (other.normal - plane.normal).normalize() })
                })
                .collect();
            let (failed, r) = linear_program3(&projected, radius, plane.normal, true);
            // Failure can only come from rounding here (the current result is feasible); keep the current result then.
            if failed == projected.len() {
                result = r;
            }
            distance = plane.normal.dot(plane.point - result);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    const DT: f64 = 0.05;
    const TAU: f64 = 2.0;
    const R: f64 = 0.3;
    const MAX_SPEED: f64 = 1.5;
    /// Preferred speed is distance / SETTLE_TIME, capped at max speed, so a Drone slows inside the last 0.75 m.
    const SETTLE_TIME: f64 = 0.5;
    /// A Drone has arrived once its centre is this close to its goal.
    const ARRIVED: f64 = 0.01;
    /// Mission frame is NED, so z is down: z = -1.2 is 1.2 m above the floor.
    const HEIGHT_Z: f64 = -1.2;

    fn pref_toward(position: DVec3, goal: DVec3) -> DVec3 {
        let to_goal = goal - position;
        let dist = to_goal.length();
        if dist == 0.0 {
            return DVec3::ZERO;
        }
        to_goal * ((dist / SETTLE_TIME).min(MAX_SPEED) / dist)
    }

    struct Run {
        min_separation: f64,
        arrival_s: Vec<f64>,
    }

    /// Plain integrator: every step all Drones pick their ORCA velocity from the same state, then move.
    /// Also checks that no chosen velocity is non-finite or above max speed.
    fn simulate(starts: &[DVec3], goals: &[DVec3], max_time_s: f64) -> Run {
        let mut drones: Vec<DroneState> = starts
            .iter()
            .map(|p| DroneState { position: p.to_array(), velocity: [0.0; 3], radius: R, max_speed: MAX_SPEED, pref_velocity: [0.0; 3] })
            .collect();
        let n = drones.len();
        let mut min_separation = f64::INFINITY;
        let mut arrival_s = vec![f64::INFINITY; n];
        let steps = (max_time_s / DT).ceil() as usize;
        for step in 0..=steps {
            let t = step as f64 * DT;
            for i in 0..n {
                let p = DVec3::from_array(drones[i].position);
                for other in &drones[i + 1..] {
                    min_separation = min_separation.min(p.distance(DVec3::from_array(other.position)));
                }
                if arrival_s[i].is_infinite() && p.distance(goals[i]) < ARRIVED {
                    arrival_s[i] = t;
                }
            }
            if arrival_s.iter().all(|a| a.is_finite()) {
                break;
            }
            for (i, d) in drones.iter_mut().enumerate() {
                d.pref_velocity = pref_toward(DVec3::from_array(d.position), goals[i]).to_array();
            }
            let chosen: Vec<[f64; 3]> = (0..n)
                .map(|i| {
                    let others: Vec<DroneState> = (0..n).filter(|&j| j != i).map(|j| drones[j]).collect();
                    new_velocity(&drones[i], &others, TAU, DT)
                })
                .collect();
            for (d, v) in drones.iter_mut().zip(chosen) {
                let v = DVec3::from_array(v);
                assert!(v.is_finite(), "non-finite velocity at t = {t}");
                assert!(v.length() <= MAX_SPEED + 1e-9, "speed {} above max at t = {t}", v.length());
                d.velocity = v.to_array();
                d.position = (DVec3::from_array(d.position) + v * DT).to_array();
            }
        }
        Run { min_separation, arrival_s }
    }

    #[test]
    fn head_on_pair_swaps_without_touching() {
        // Exact symmetry deadlocks reciprocal avoidance (both Drones brake along the same line forever),
        // so the second Drone starts 1 mm to the side. Both stay at the same height.
        let a = DVec3::new(-2.0, 0.0, HEIGHT_Z);
        let b = DVec3::new(2.0, 0.001, HEIGHT_Z);
        let run = simulate(&[a, b], &[b, a], 20.0);
        println!("head-on: min separation {:.6} m, arrivals {:?} s", run.min_separation, run.arrival_s);
        assert!(run.min_separation >= 2.0 * R - 1e-6, "came {} m close", run.min_separation);
        // 4 m straight at 1.5 m/s is 2.7 s; 8 s leaves room for the detour and the slow final approach.
        for &t in &run.arrival_s {
            assert!(t <= 8.0, "arrived at {t} s");
        }
    }

    #[test]
    fn six_drone_circle_swaps_to_antipodes_without_touching() {
        // Exact six-fold symmetry deadlocks in the centre, so Drone k starts (and ends) k mm lower (z is down),
        // which lets ORCA use the third dimension. Measured: with every Drone at one height the swap is planar
        // and stalls for 25-68 s even with angle offsets of 0.03-0.1 rad round the circle.
        let starts: Vec<DVec3> = (0..6)
            .map(|k| {
                let angle = k as f64 * std::f64::consts::FRAC_PI_3;
                DVec3::new(3.0 * angle.cos(), 3.0 * angle.sin(), HEIGHT_Z + k as f64 * 1e-3)
            })
            .collect();
        let goals: Vec<DVec3> = starts.iter().map(|s| DVec3::new(-s.x, -s.y, s.z)).collect();
        let run = simulate(&starts, &goals, 30.0);
        println!("circle: min separation {:.6} m, arrivals {:?} s", run.min_separation, run.arrival_s);
        assert!(run.min_separation >= 2.0 * R - 1e-6, "came {} m close", run.min_separation);
        // 6 m straight at 1.5 m/s is 4 s, plus about 2 s of slowing at the goal; 10 s leaves room for the crowd.
        for &t in &run.arrival_s {
            assert!(t <= 10.0, "arrived at {t} s");
        }
    }

    fn lone(pref_velocity: [f64; 3]) -> DroneState {
        DroneState { position: [1.0, 2.0, HEIGHT_Z], velocity: [0.2, 0.0, 0.0], radius: R, max_speed: MAX_SPEED, pref_velocity }
    }

    #[test]
    fn lone_drone_keeps_its_preferred_velocity() {
        let pref = [0.3, -0.4, 0.5];
        assert_eq!(new_velocity(&lone(pref), &[], TAU, DT), pref);
    }

    #[test]
    fn lone_drone_preferred_velocity_above_max_speed_is_scaled_to_max_speed() {
        let v = DVec3::from_array(new_velocity(&lone([3.0, 0.0, -4.0]), &[], TAU, DT));
        assert!((v - DVec3::new(0.9, 0.0, -1.2)).length() < 1e-12, "{v}");
    }

    #[test]
    fn exact_head_on_at_speed_brakes_instead_of_failing() {
        // Relative velocity exactly on the cone axis: RVO2-3D would divide by zero here.
        let me = DroneState { position: [0.0, 0.0, HEIGHT_Z], velocity: [1.5, 0.0, 0.0], radius: R, max_speed: MAX_SPEED, pref_velocity: [1.5, 0.0, 0.0] };
        let other = DroneState { position: [2.0, 0.0, HEIGHT_Z], velocity: [-1.5, 0.0, 0.0], pref_velocity: [-1.5, 0.0, 0.0], ..me };
        let v = DVec3::from_array(new_velocity(&me, &[other], TAU, DT));
        // Half of the closing speed that reaches the front of the cutoff sphere: ((2 - 0.6) / 2 s) / 2.
        assert!((v - DVec3::new(0.35, 0.0, 0.0)).length() < 1e-12, "{v}");
    }

    #[test]
    fn overlapping_on_both_sides_minimises_the_worst_penetration() {
        // Neighbours overlap at +x (0.35 m) and -x (0.5 m): no velocity satisfies both planes, so
        // linear program 4 balances them: v_x <= -2.5 and v_x >= 1 are each missed by 1.75 at v_x = -0.75.
        let me = DroneState { position: [0.0, 0.0, HEIGHT_Z], velocity: [0.0; 3], radius: R, max_speed: MAX_SPEED, pref_velocity: [0.0; 3] };
        let right = DroneState { position: [0.35, 0.0, HEIGHT_Z], ..me };
        let left = DroneState { position: [-0.5, 0.0, HEIGHT_Z], ..me };
        let v = DVec3::from_array(new_velocity(&me, &[right, left], TAU, DT));
        assert!((v - DVec3::new(-0.75, 0.0, 0.0)).length() < 1e-9, "{v}");
    }

    #[test]
    fn crowd_never_exceeds_max_speed() {
        // A fixed pseudo-random crowd (some overlapping, fast, from every side) around one Drone.
        let mut seed: u64 = 7;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
        };
        for _ in 0..200 {
            let mut rand3 = |scale: f64| [next() * scale, next() * scale, next() * scale];
            let me = DroneState { position: [0.0; 3], velocity: rand3(1.5), radius: R, max_speed: MAX_SPEED, pref_velocity: rand3(3.0) };
            let crowd: Vec<DroneState> = (0..8).map(|_| DroneState { position: rand3(1.5), velocity: rand3(1.5), ..me }).collect();
            let v = DVec3::from_array(new_velocity(&me, &crowd, TAU, DT));
            assert!(v.is_finite() && v.length() <= MAX_SPEED + 1e-9, "{v}");
        }
    }
}
