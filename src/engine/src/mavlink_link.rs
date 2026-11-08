//! MAVLink v2 endpoint for one simulated Drone: telemetry out, commands and position
//! setpoints in, PX4-style modes. The ground station is reached over UDP at GROUND_STATION.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use aether_core::vehicle::Drone;
use aether_core::{frame, room_twin::OccupancyGrid};
use mavlink::dialects::common::{
    ATTITUDE_DATA, COMMAND_ACK_DATA, COMMAND_LONG_DATA, EXTENDED_SYS_STATE_DATA, HEARTBEAT_DATA,
    LOCAL_POSITION_NED_DATA, MINOR_MAVLINK_VERSION, MavAutopilot, MavCmd, MavFrame,
    MavLandedState, MavMessage, MavModeFlag, MavResult, MavState, MavType, MavVtolState,
};
use mavlink::{Connection, MavConnection, MavHeader, MavlinkVersion, connect};

/// The address QGroundControl listens on.
pub const GROUND_STATION: &str = "127.0.0.1:14550";
/// MAV_COMP_ID_AUTOPILOT1.
const AUTOPILOT_COMPONENT_ID: u8 = 1;

const PHYSICS_STEP: Duration = Duration::from_millis(4); // 250 Hz
const PHYSICS_DT: f64 = 0.004;
const HEARTBEAT_PERIOD: Duration = Duration::from_secs(1);
const ATTITUDE_PERIOD: Duration = Duration::from_millis(20); // 50 Hz
const POSITION_PERIOD: Duration = Duration::from_micros(33_334); // 30 Hz
const SYS_STATE_PERIOD: Duration = Duration::from_millis(200); // 5 Hz
/// Offboard is accepted only if the last setpoint is newer than this (more than 2 Hz).
const SETPOINT_FRESH: Duration = Duration::from_millis(500);
/// PX4's COM_DISARM_LAND default.
const DISARM_AFTER_LAND: Duration = Duration::from_secs(2);
/// Below this height an airborne Drone reports TAKEOFF rather than IN_AIR (approximation).
const AIRBORNE_ALTITUDE: f64 = 0.5;

// PX4 custom modes, as pymavlink's mavutil.py defines them (PX4_CUSTOM_MAIN_MODE_*, PX4_CUSTOM_SUB_MODE_*).
const PX4_MAIN_MODE_MANUAL: u32 = 1;
const PX4_MAIN_MODE_AUTO: u32 = 4;
const PX4_MAIN_MODE_OFFBOARD: u32 = 6;
const PX4_SUB_MODE_AUTO_LAND: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Manual,
    Offboard,
    AutoLand,
}

impl Mode {
    /// PX4 custom_mode: main mode in bits 16-23, sub mode in bits 24-31.
    fn custom_mode(self) -> u32 {
        let (main, sub) = match self {
            Mode::Manual => (PX4_MAIN_MODE_MANUAL, 0),
            Mode::Offboard => (PX4_MAIN_MODE_OFFBOARD, 0),
            Mode::AutoLand => (PX4_MAIN_MODE_AUTO, PX4_SUB_MODE_AUTO_LAND),
        };
        (main << 16) | (sub << 24)
    }
}

struct State {
    drone: Drone,
    mode: Mode,
    /// Last position setpoint and when it arrived.
    setpoint: Option<([f64; 3], Instant)>,
    /// When the Drone auto-disarms after touchdown.
    disarm_at: Option<Instant>,
}

/// One Drone as a MAVLink endpoint. Sending and receiving share the connection; the model is behind a Mutex.
pub struct Endpoint {
    system_id: u8,
    boot: Instant,
    conn: Connection<MavMessage>,
    state: Mutex<State>,
    ground: Ground,
}

/// Flat ground for standalone probes, or the shared scanned room for the demo.
#[derive(Clone)]
pub enum Ground {
    Flat(f64),
    Room(Arc<OccupancyGrid>),
}

impl Ground {
    /// Surface top below a Mission-frame position. Scan floor drift maps to the pad plane.
    pub fn below(&self, position: [f64; 3]) -> f64 {
        let grid = match self {
            Self::Flat(d) => return *d,
            Self::Room(grid) => grid,
        };
        let p = frame::mission_to_scan(position);
        let origin = grid.origin();
        let dims = grid.dims();
        let v = grid.voxel();
        let x = ((p[0] - origin[0]) / v).floor();
        let z = ((p[2] - origin[2]) / v).floor();
        if x < 0.0 || z < 0.0 || x >= dims[0] as f64 || z >= dims[2] as f64 {
            return 0.0;
        }
        for y in (0..dims[1]).rev() {
            let top = origin[1] + (y + 1) as f64 * v;
            if top > p[1] + 1e-9 || !grid.is_occupied([x as usize, y, z as usize]) {
                continue;
            }
            let centre = top - v / 2.0;
            // Assumed 20 cm band covers the measured floor drift (SPEC T9b).
            return if (centre - frame::FLOOR_Y).abs() <= 0.20 {
                0.0
            } else {
                frame::FLOOR_Y - top
            };
        }
        0.0
    }
}

/// Fires once per period on the wall clock; a late tick resets the schedule instead of bursting.
struct Due {
    next: Instant,
    period: Duration,
}

impl Due {
    fn new(start: Instant, period: Duration) -> Self {
        Self { next: start, period }
    }

    fn due(&mut self, now: Instant) -> bool {
        if now < self.next {
            return false;
        }
        self.next += self.period;
        if self.next < now {
            self.next = now + self.period;
        }
        true
    }
}

impl Endpoint {
    fn header(&self) -> MavHeader {
        MavHeader {
            system_id: self.system_id as u32,
            component_id: AUTOPILOT_COMPONENT_ID,
            sequence: 0,
        }
    }

    /// Shortcut: send errors (for example ECONNREFUSED before the ground station listens) are dropped; the next message retries.
    fn send(&self, msg: &MavMessage) {
        let _ = self.conn.send(&self.header(), msg);
    }

    fn boot_ms(&self) -> u32 {
        self.boot.elapsed().as_millis() as u32
    }

    fn send_heartbeat(&self) {
        let s = self.state.lock().unwrap();
        let mut base = MavModeFlag::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED;
        if s.drone.armed {
            base = base | MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        }
        let msg = MavMessage::HEARTBEAT(HEARTBEAT_DATA {
            custom_mode: s.mode.custom_mode(),
            mavtype: MavType::MAV_TYPE_QUADROTOR,
            autopilot: MavAutopilot::MAV_AUTOPILOT_PX4,
            base_mode: base,
            system_status: if s.drone.armed {
                MavState::MAV_STATE_ACTIVE
            } else {
                MavState::MAV_STATE_STANDBY
            },
            mavlink_version: MINOR_MAVLINK_VERSION,
        });
        drop(s);
        self.send(&msg);
    }

    fn send_attitude(&self) {
        let s = self.state.lock().unwrap();
        let msg = MavMessage::ATTITUDE(ATTITUDE_DATA {
            time_boot_ms: self.boot_ms(),
            roll: s.drone.roll as f32,
            pitch: s.drone.pitch as f32,
            yaw: s.drone.yaw as f32,
            // Tier A has no body rates, so they are sent as zero.
            rollspeed: 0.0,
            pitchspeed: 0.0,
            yawspeed: 0.0,
        });
        drop(s);
        self.send(&msg);
    }

    fn send_position(&self) {
        let s = self.state.lock().unwrap();
        let [x, y, z] = s.drone.position;
        let [vx, vy, vz] = s.drone.velocity;
        let msg = MavMessage::LOCAL_POSITION_NED(LOCAL_POSITION_NED_DATA {
            time_boot_ms: self.boot_ms(),
            x: x as f32,
            y: y as f32,
            z: z as f32,
            vx: vx as f32,
            vy: vy as f32,
            vz: vz as f32,
        });
        drop(s);
        self.send(&msg);
    }

    fn send_sys_state(&self) {
        let s = self.state.lock().unwrap();
        let landed_state = if s.drone.landed {
            MavLandedState::MAV_LANDED_STATE_ON_GROUND
        } else if s.mode == Mode::AutoLand {
            MavLandedState::MAV_LANDED_STATE_LANDING
        } else if s.drone.ground - s.drone.position[2] < AIRBORNE_ALTITUDE {
            MavLandedState::MAV_LANDED_STATE_TAKEOFF
        } else {
            MavLandedState::MAV_LANDED_STATE_IN_AIR
        };
        drop(s);
        self.send(&MavMessage::EXTENDED_SYS_STATE(EXTENDED_SYS_STATE_DATA {
            vtol_state: MavVtolState::MAV_VTOL_STATE_UNDEFINED,
            landed_state,
        }));
    }

    /// Advances the model by one physics step under the current mode.
    fn physics(&self, now: Instant) {
        let mut s = self.state.lock().unwrap();
        let ground = self.ground.below(s.drone.position);
        s.drone.set_ground(ground);
        match s.mode {
            Mode::Offboard => {
                let target = s.setpoint.map(|(t, _)| t).unwrap_or(s.drone.position);
                s.drone.step(PHYSICS_DT, target);
            }
            Mode::Manual => {
                let hold = s.drone.position;
                s.drone.step(PHYSICS_DT, hold);
            }
            Mode::AutoLand => {
                s.drone.land_step(PHYSICS_DT);
                if s.drone.landed && s.drone.armed && s.disarm_at.is_none() {
                    s.disarm_at = Some(now + DISARM_AFTER_LAND);
                }
                if let Some(at) = s.disarm_at {
                    if now >= at {
                        s.drone.disarm();
                        s.disarm_at = None;
                    }
                }
            }
        }
    }

    fn receive_loop(&self) {
        loop {
            match self.conn.recv() {
                Ok((_, msg)) => self.handle(msg),
                Err(_) => thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    fn handle(&self, msg: MavMessage) {
        match msg {
            MavMessage::SET_POSITION_TARGET_LOCAL_NED(d) => {
                if d.target_system != self.system_id
                    || d.coordinate_frame != MavFrame::MAV_FRAME_LOCAL_NED
                {
                    return;
                }
                let target = [d.x as f64, d.y as f64, d.z as f64];
                self.state.lock().unwrap().setpoint = Some((target, Instant::now()));
            }
            MavMessage::COMMAND_LONG(d) => {
                if d.target_system != self.system_id {
                    return;
                }
                let result = self.command(&d);
                self.send(&MavMessage::COMMAND_ACK(COMMAND_ACK_DATA {
                    command: d.command,
                    result,
                }));
            }
            _ => {}
        }
    }

    fn command(&self, d: &COMMAND_LONG_DATA) -> MavResult {
        let now = Instant::now();
        let mut s = self.state.lock().unwrap();
        match d.command {
            MavCmd::MAV_CMD_DO_SET_MODE => {
                // pymavlink sends the PX4 main mode in param2 and the sub mode in param3 (set_mode_px4).
                if d.param2 as u32 != PX4_MAIN_MODE_OFFBOARD {
                    return MavResult::MAV_RESULT_UNSUPPORTED;
                }
                // Unverified against PX4: TEMPORARILY_REJECTED is the enum's "retry later" result; PX4's own refusal code is not in the sources available here.
                let fresh = matches!(s.setpoint, Some((_, at)) if now.duration_since(at) < SETPOINT_FRESH);
                if !fresh {
                    return MavResult::MAV_RESULT_TEMPORARILY_REJECTED;
                }
                s.mode = Mode::Offboard;
                MavResult::MAV_RESULT_ACCEPTED
            }
            MavCmd::MAV_CMD_COMPONENT_ARM_DISARM => {
                if d.param1 >= 0.5 {
                    s.drone.arm();
                } else {
                    s.drone.disarm();
                }
                MavResult::MAV_RESULT_ACCEPTED
            }
            MavCmd::MAV_CMD_NAV_LAND => {
                s.mode = Mode::AutoLand;
                MavResult::MAV_RESULT_ACCEPTED
            }
            _ => MavResult::MAV_RESULT_UNSUPPORTED,
        }
    }
}

/// Serves one Drone with the given system id to the ground station until the process ends.
pub fn run(system_id: u8) -> Result<()> {
    run_at(system_id, [0.0; 3], 0.0, GROUND_STATION)
}

/// Like `run`, with the Drone resting at `start` (NED metres) on ground at NED d `ground`,
/// sending to the ground station at `ground_station` (host:port).
/// Each call opens its own UDP socket, so several can run in one process on separate threads.
pub fn run_at(system_id: u8, start: [f64; 3], ground: f64, ground_station: &str) -> Result<()> {
    run_on_ground(system_id, start, Ground::Flat(ground), ground_station)
}

/// Like `run_at`, with the ground sampled from the Room Twin on each physics step.
pub fn run_on_ground(system_id: u8, start: [f64; 3], ground: Ground, ground_station: &str) -> Result<()> {
    let mut conn = connect::<MavMessage>(&format!("udpout:{ground_station}"))
        .with_context(|| format!("opening MAVLink UDP to {ground_station}"))?;
    conn.set_protocol_version(MavlinkVersion::V2);
    let mut drone = Drone::on_floor();
    drone.position = start;
    drone.ground = ground.below(start);
    let endpoint = Arc::new(Endpoint {
        system_id,
        ground,
        boot: Instant::now(),
        conn,
        state: Mutex::new(State {
            drone,
            mode: Mode::Manual,
            setpoint: None,
            disarm_at: None,
        }),
    });
    let receiver = Arc::clone(&endpoint);
    thread::spawn(move || receiver.receive_loop());

    let start = Instant::now();
    let mut physics_at = start;
    let mut heartbeat = Due::new(start, HEARTBEAT_PERIOD);
    let mut attitude = Due::new(start, ATTITUDE_PERIOD);
    let mut position = Due::new(start, POSITION_PERIOD);
    let mut sys_state = Due::new(start, SYS_STATE_PERIOD);
    loop {
        let now = Instant::now();
        // A stall longer than 100 ms restarts the physics clock instead of replaying every missed step.
        if now.duration_since(physics_at) > Duration::from_millis(100) {
            physics_at = now;
        }
        while physics_at <= now {
            endpoint.physics(now);
            physics_at += PHYSICS_STEP;
        }
        if heartbeat.due(now) {
            endpoint.send_heartbeat();
        }
        if attitude.due(now) {
            endpoint.send_attitude();
        }
        if position.due(now) {
            endpoint.send_position();
        }
        if sys_state.due(now) {
            endpoint.send_sys_state();
        }
        let next = [physics_at, heartbeat.next, attitude.next, position.next, sys_state.next]
            .into_iter()
            .min()
            .unwrap_or(now);
        thread::sleep(next.saturating_duration_since(Instant::now()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_core::room_twin::RoomMesh;

    fn room_ground_fixture() -> Ground {
        let mut mesh = RoomMesh { vertices: vec![], triangles: vec![] };
        // Floor, table, ceiling. Coordinates are local to the pad, then mapped to the scan.
        for (half, height) in [(2.0, 0.0), (0.5, 0.8), (2.0, 2.5)] {
            let base = mesh.vertices.len() as u32;
            for [n, e] in [[-half, -half], [-half, half], [half, half], [half, -half]] {
                mesh.vertices.push(frame::mission_to_scan([n, e, -height]));
            }
            mesh.triangles.extend([[base, base + 1, base + 2], [base, base + 2, base + 3]]);
        }
        Ground::Room(Arc::new(OccupancyGrid::from_mesh(&mesh, 0.1).unwrap()))
    }

    #[test]
    fn room_ground_ignores_ceiling_and_handles_floor_and_unknown_columns() {
        let ground = room_ground_fixture();
        assert!((ground.below([0.0, 0.0, -1.5]) + 0.85).abs() < 1e-9);
        assert_eq!(ground.below([0.0, 0.0, -0.4]), 0.0); // Under the table.
        assert_eq!(ground.below([1.0, 0.0, -1.5]), 0.0);
        assert_eq!(ground.below([10.0, 0.0, -1.5]), 0.0);
        assert!((ground.below([0.0, 0.0, -0.85]) + 0.85).abs() < 1e-9);
        assert_eq!(Ground::Flat(-0.7).below([0.0; 3]), -0.7);
    }

    #[test]
    fn room_ground_lands_on_table_then_flies_off_and_lands_on_floor() {
        let ground = room_ground_fixture();
        let mut drone = Drone::on_floor();
        drone.position = [0.0, 0.0, -1.5];
        drone.landed = false;
        drone.arm();
        for _ in 0..1500 {
            drone.set_ground(ground.below(drone.position));
            drone.land_step(PHYSICS_DT);
        }
        assert!(drone.landed);
        assert!((drone.position[2] + 0.85).abs() < 1e-9);
        for target in [[0.0, 0.0, -1.5], [1.0, 0.0, -1.5]] {
            for _ in 0..1500 {
                drone.set_ground(ground.below(drone.position));
                drone.step(PHYSICS_DT, target);
            }
        }
        assert!((drone.position[0] - 1.0).abs() < 0.02);
        for _ in 0..1500 {
            drone.set_ground(ground.below(drone.position));
            drone.land_step(PHYSICS_DT);
        }
        assert!(drone.landed);
        assert_eq!(drone.position[2], 0.0);
    }

    #[test]
    fn room_ground_floor_drift_stays_on_the_pad_plane() {
        let mesh = RoomMesh {
            vertices: [[-1.0, -1.0, -0.12], [-1.0, 1.0, -0.12], [1.0, 1.0, -0.12], [1.0, -1.0, -0.12]]
                .map(frame::mission_to_scan).to_vec(),
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let ground = Ground::Room(Arc::new(OccupancyGrid::from_mesh(&mesh, 0.1).unwrap()));
        assert_eq!(ground.below([0.0, 0.0, -1.0]), 0.0);
        assert_eq!(ground.below([0.0; 3]), 0.0);
    }
}
