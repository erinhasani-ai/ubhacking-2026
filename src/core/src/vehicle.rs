//! Tier A Drone model: a kinematic quadrotor in north-east-down metres, floor at z = 0.
//! Pure computation with time passed in as `dt`, so it also builds for WebAssembly.

/// Tier A values; Tier B uses a published airframe spec.
pub const MAX_HORIZONTAL_SPEED: f64 = 1.5;
/// Tier A values; Tier B uses a published airframe spec.
pub const MAX_VERTICAL_SPEED: f64 = 1.0;
/// Tier A values; Tier B uses a published airframe spec.
pub const MAX_ACCELERATION: f64 = 2.0;
/// Tier A values; Tier B uses a published airframe spec (PX4's default MPC_LAND_SPEED).
pub const LAND_SPEED: f64 = 0.7;
/// Standard gravity, turns commanded horizontal acceleration into tilt.
const GRAVITY: f64 = 9.80665;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drone {
    /// North, east, down in metres. z = 0 is the floor; z > 0 never happens.
    pub position: [f64; 3],
    /// North, east, down in metres per second.
    pub velocity: [f64; 3],
    /// Radians. Yaw stays 0 in Tier A.
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub armed: bool,
    /// True while the Drone sits on the floor.
    pub landed: bool,
}

impl Drone {
    /// A disarmed Drone on the floor at the origin.
    pub fn on_floor() -> Self {
        Self {
            position: [0.0; 3],
            velocity: [0.0; 3],
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            armed: false,
            landed: true,
        }
    }

    pub fn arm(&mut self) {
        self.armed = true;
    }

    /// Stops the Drone and zeroes its tilt. Shortcut: in flight it stops where it is, not a controlled stop; upgrade with Tier B.
    pub fn disarm(&mut self) {
        self.armed = false;
        self.velocity = [0.0; 3];
        self.roll = 0.0;
        self.pitch = 0.0;
    }

    /// Moves toward a position target with a braking profile. Does nothing while disarmed.
    pub fn step(&mut self, dt: f64, target: [f64; 3]) {
        if !self.armed {
            return;
        }
        // A target below the floor is clamped onto the floor.
        let tz = target[2].min(0.0);
        let (ex, ey) = (target[0] - self.position[0], target[1] - self.position[1]);
        let dist = ex.hypot(ey);
        let speed = MAX_HORIZONTAL_SPEED.min((2.0 * MAX_ACCELERATION * dist).sqrt());
        let want_h = if dist > 0.0 {
            [ex / dist * speed, ey / dist * speed]
        } else {
            [0.0, 0.0]
        };
        let dz = tz - self.position[2];
        let want_vz =
            MAX_VERTICAL_SPEED.min((2.0 * MAX_ACCELERATION * dz.abs()).sqrt()) * dz.signum();
        let accel_h = self.steer_horizontal(dt, want_h);
        self.steer_vertical(dt, want_vz);
        self.integrate(dt, accel_h);
    }

    /// Descends straight down at LAND_SPEED, holding x and y, until it touches the floor.
    pub fn land_step(&mut self, dt: f64) {
        if !self.armed || self.landed {
            return;
        }
        let accel_h = self.steer_horizontal(dt, [0.0, 0.0]);
        self.steer_vertical(dt, LAND_SPEED);
        self.integrate(dt, accel_h);
        if self.landed {
            self.velocity = [0.0; 3];
        }
    }

    /// Moves the horizontal velocity toward `want` by at most a_max·dt; returns the commanded acceleration.
    fn steer_horizontal(&mut self, dt: f64, want: [f64; 2]) -> [f64; 2] {
        let dv = [want[0] - self.velocity[0], want[1] - self.velocity[1]];
        let max_dv = MAX_ACCELERATION * dt;
        let norm = dv[0].hypot(dv[1]);
        let scale = if norm > max_dv { max_dv / norm } else { 1.0 };
        let change = [dv[0] * scale, dv[1] * scale];
        self.velocity[0] += change[0];
        self.velocity[1] += change[1];
        [change[0] / dt, change[1] / dt]
    }

    /// Moves the vertical velocity toward `want` by at most a_max·dt.
    fn steer_vertical(&mut self, dt: f64, want: f64) {
        let max_dv = MAX_ACCELERATION * dt;
        self.velocity[2] += (want - self.velocity[2]).clamp(-max_dv, max_dv);
    }

    /// Moves by the velocity, stops on the floor, and sets the tilt from the horizontal acceleration.
    fn integrate(&mut self, dt: f64, accel_h: [f64; 2]) {
        for axis in 0..3 {
            self.position[axis] += self.velocity[axis] * dt;
        }
        if self.position[2] >= 0.0 {
            self.position[2] = 0.0;
            self.velocity[2] = self.velocity[2].min(0.0);
            self.landed = true;
        } else {
            self.landed = false;
        }
        self.pitch = -(accel_h[0] / GRAVITY).atan();
        self.roll = (accel_h[1] / GRAVITY).atan();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 0.004;

    fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn step_reaches_two_metre_target_at_analytic_time() {
        let mut drone = Drone::on_floor();
        drone.arm();
        let target = [2.0, 0.0, 0.0];
        // Trapezoid profile: d/v + v/a = 2/1.5 + 1.5/2 ≈ 2.083 s.
        let analytic = 2.0 / MAX_HORIZONTAL_SPEED + MAX_HORIZONTAL_SPEED / MAX_ACCELERATION;
        let steps = |t: f64| (t / DT).round() as usize;
        let n_09 = steps(0.9 * analytic);
        for _ in 0..n_09 {
            drone.step(DT, target);
        }
        // Braking phase at 0.9 T: remaining distance is a·τ²/2 with τ = T − t.
        let t_09 = n_09 as f64 * DT;
        let braking = MAX_ACCELERATION * (analytic - t_09).powi(2) / 2.0;
        let remaining = distance(drone.position, target);
        assert!(
            (remaining - braking).abs() <= 0.01,
            "remaining {remaining} at 0.9 T, expected braking distance {braking}"
        );
        for _ in n_09..steps(1.05 * analytic) {
            drone.step(DT, target);
        }
        assert!(
            distance(drone.position, target) <= 0.02,
            "{:?} not at target by 1.05 T",
            drone.position
        );
    }

    #[test]
    fn tilt_follows_commanded_acceleration() {
        let expected = (2.0 / GRAVITY).atan().to_degrees();
        let mut north = Drone::on_floor();
        north.arm();
        for _ in 0..50 {
            north.step(DT, [100.0, 0.0, 0.0]);
        }
        assert!((north.pitch.to_degrees() + expected).abs() < 0.1, "pitch {}", north.pitch.to_degrees());
        assert!(north.roll.abs() < 1e-12);

        let mut east = Drone::on_floor();
        east.arm();
        for _ in 0..50 {
            east.step(DT, [0.0, 100.0, 0.0]);
        }
        assert!((east.roll.to_degrees() - expected).abs() < 0.1, "roll {}", east.roll.to_degrees());
        assert!(east.pitch.abs() < 1e-12);
    }

    #[test]
    fn target_below_floor_never_goes_above_floor() {
        let mut drone = Drone::on_floor();
        drone.arm();
        for _ in 0..1250 {
            drone.step(DT, [0.0, 0.0, -1.0]);
        }
        assert!((drone.position[2] + 1.0).abs() < 0.05, "did not climb");
        for _ in 0..1250 {
            drone.step(DT, [0.0, 0.0, 1.0]);
            assert!(drone.position[2] <= 1e-9, "z above floor: {}", drone.position[2]);
        }
        assert_eq!(drone.position[2], 0.0);
    }

    #[test]
    fn disarmed_drone_does_not_move() {
        let mut drone = Drone::on_floor();
        let before = drone.position;
        for _ in 0..2500 {
            drone.step(DT, [5.0, 5.0, -1.0]);
        }
        assert_eq!(drone.position, before);
    }

    #[test]
    fn land_descends_at_land_speed_and_touches_floor() {
        let mut drone = Drone::on_floor();
        drone.arm();
        for _ in 0..1250 {
            drone.step(DT, [0.0, 0.0, -1.0]);
        }
        let mut steps = 0;
        while !drone.landed {
            drone.land_step(DT);
            assert!(drone.velocity[2] <= LAND_SPEED + 1e-9, "descent {}", drone.velocity[2]);
            steps += 1;
            assert!(steps < 5000, "never touched down");
        }
        assert_eq!(drone.position[2], 0.0);
        assert_eq!(drone.velocity, [0.0; 3]);
    }
}
