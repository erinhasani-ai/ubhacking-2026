//! Ground station for the Swarm over MAVLink v2 on UDP. It tracks each Drone by system id from
//! the source address of its packets, streams position setpoints at 20 Hz to the Drones that have
//! a target, and sends commands that wait for their COMMAND_ACK.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mavlink::dialects::common::{
    ATTITUDE_DATA, COMMAND_LONG_DATA, LOCAL_POSITION_NED_DATA, MavCmd, MavFrame, MavLandedState,
    MavMessage, MavModeFlag, MavResult, PositionTargetTypemask, SET_POSITION_TARGET_LOCAL_NED_DATA,
};
use mavlink::error::MessageReadError;
use mavlink::{MavHeader, MavlinkReader, MavlinkVersion};
use tokio::net::{ToSocketAddrs, UdpSocket};
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;

/// QGroundControl's ids: system 255, component MAV_COMP_ID_MISSIONPLANNER.
const GCS_SYSTEM_ID: u32 = 255;
const GCS_COMPONENT_ID: u8 = 190;
/// MAV_COMP_ID_AUTOPILOT1, where each Drone takes commands.
const AUTOPILOT_COMPONENT_ID: u8 = 1;
const SETPOINT_PERIOD: Duration = Duration::from_millis(50); // 20 Hz
/// The mask tests/mavlink_flight.py sends: bits 3-11 set, so only x, y, z count.
const POSITION_ONLY: u16 = 4088;
/// PX4_CUSTOM_MAIN_MODE_OFFBOARD, sent in param2 of MAV_CMD_DO_SET_MODE as pymavlink's set_mode_px4 does.
const PX4_MAIN_MODE_OFFBOARD: f32 = 6.0;
/// First values: the simulated Drone answers within milliseconds; three tries ride out a stalled thread.
const ACK_TIMEOUT: Duration = Duration::from_secs(1);
const ATTEMPTS: u8 = 3;
const MESSAGE_BACKLOG: usize = 1024;

/// What the ground station knows about one Drone, from its latest messages.
#[derive(Debug, Clone, PartialEq)]
pub struct DroneLink {
    /// Where its packets come from; commands and setpoints go back there.
    pub addr: SocketAddr,
    pub position: Option<LOCAL_POSITION_NED_DATA>,
    pub attitude: Option<ATTITUDE_DATA>,
    /// PX4 mode name from the last HEARTBEAT (see `mode_name`); None before the first one.
    pub mode: Option<&'static str>,
    pub armed: bool,
    pub landed_state: Option<MavLandedState>,
    /// When the last packet of any kind arrived.
    pub last_seen: Instant,
}

impl DroneLink {
    /// Time since the last packet from this Drone.
    pub fn link_age(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.last_seen)
    }
}

/// Why a command did not go through.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandError {
    /// No COMMAND_ACK after every attempt.
    Timeout,
    /// The Drone answered with this result instead of ACCEPTED.
    Denied(MavResult),
    /// Nothing heard from this system id yet, so there is no address to send to.
    UnknownDrone(u8),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Timeout => write!(f, "no COMMAND_ACK after every attempt"),
            CommandError::Denied(result) => write!(f, "command denied: {result:?}"),
            CommandError::UnknownDrone(id) => write!(f, "no packet from Drone {id} yet"),
        }
    }
}

impl std::error::Error for CommandError {}

/// PX4 mode name of a HEARTBEAT custom_mode (main mode in bits 16-23, sub mode in 24-31), named as
/// pymavlink's interpret_px4_mode names them.
/// Shortcut: only the modes T7's Drone uses; others read UNKNOWN until a real PX4 joins the Swarm.
pub fn mode_name(custom_mode: u32) -> &'static str {
    match ((custom_mode >> 16) & 0xFF, custom_mode >> 24) {
        (1, _) => "MANUAL",
        (6, _) => "OFFBOARD",
        (4, 6) => "LAND",
        _ => "UNKNOWN",
    }
}

/// Every MAVLink v2 message in one datagram; bytes that are not a valid frame are skipped.
fn parse(datagram: &[u8]) -> Vec<(MavHeader, MavMessage)> {
    let mut reader = MavlinkReader::new(datagram);
    let mut out = Vec::new();
    loop {
        match reader.read_message::<MavMessage>(MavlinkVersion::V2) {
            Ok(message) => out.push(message),
            Err(MessageReadError::Io(_)) => return out, // end of the datagram
            Err(MessageReadError::Parse(_)) => {}       // a frame this dialect cannot decode
        }
    }
}

#[derive(Default)]
struct State {
    drones: BTreeMap<u8, DroneLink>,
    targets: BTreeMap<u8, [f32; 3]>,
    /// Commands waiting for their ACK, oldest first: system id, command id, where the result goes.
    pending: Vec<(u8, MavCmd, oneshot::Sender<MavResult>)>,
}

impl State {
    /// Records one message that arrived from `from` at `now`.
    fn note(&mut self, header: &MavHeader, msg: &MavMessage, from: SocketAddr, now: Instant) {
        let Ok(system) = u8::try_from(header.system_id) else { return };
        let drone = self.drones.entry(system).or_insert_with(|| DroneLink {
            addr: from,
            position: None,
            attitude: None,
            mode: None,
            armed: false,
            landed_state: None,
            last_seen: now,
        });
        drone.addr = from;
        drone.last_seen = now;
        match msg {
            MavMessage::HEARTBEAT(d) => {
                drone.mode = Some(mode_name(d.custom_mode));
                drone.armed = d.base_mode.contains(MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED);
            }
            MavMessage::LOCAL_POSITION_NED(d) => drone.position = Some(d.clone()),
            MavMessage::ATTITUDE(d) => drone.attitude = Some(d.clone()),
            MavMessage::EXTENDED_SYS_STATE(d) => drone.landed_state = Some(d.landed_state),
            MavMessage::COMMAND_ACK(d) => {
                // Waiters that gave up have dropped their receiver.
                self.pending.retain(|(_, _, tx)| !tx.is_closed());
                if let Some(i) = self.pending.iter().position(|(s, c, _)| *s == system && *c == d.command) {
                    let (_, _, tx) = self.pending.remove(i);
                    let _ = tx.send(d.result);
                }
            }
            _ => {}
        }
    }

    /// Registers a waiter for the ACK of `command` from `system`.
    fn expect_ack(&mut self, system: u8, command: MavCmd) -> oneshot::Receiver<MavResult> {
        let (tx, rx) = oneshot::channel();
        self.pending.push((system, command, tx));
        rx
    }
}

/// The part shared by the ground station and its two background tasks.
struct Inner {
    socket: UdpSocket,
    boot: Instant,
    sequence: AtomicU8,
    state: Mutex<State>,
    messages: broadcast::Sender<(MavHeader, MavMessage)>,
}

impl Inner {
    /// Shortcut: a send error is dropped like a lost packet; commands retry and setpoints repeat.
    async fn send(&self, to: SocketAddr, msg: &MavMessage) {
        let header = MavHeader {
            system_id: GCS_SYSTEM_ID,
            component_id: GCS_COMPONENT_ID,
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
        };
        let mut buf = Vec::new();
        let _ = mavlink::write_v2_msg(&mut buf, header, msg); // writing into a Vec cannot fail
        let _ = self.socket.send_to(&buf, to).await;
    }

    async fn receive_loop(self: Arc<Self>) {
        let mut buf = vec![0u8; 65_536]; // the largest UDP datagram
        loop {
            let Ok((n, from)) = self.socket.recv_from(&mut buf).await else {
                tokio::time::sleep(Duration::from_millis(1)).await;
                continue;
            };
            let now = Instant::now();
            for (header, msg) in parse(&buf[..n]) {
                self.state.lock().unwrap().note(&header, &msg, from, now);
                let _ = self.messages.send((header, msg)); // having no subscriber is fine
            }
        }
    }

    async fn setpoint_loop(self: Arc<Self>) {
        let mut tick = tokio::time::interval(SETPOINT_PERIOD);
        loop {
            tick.tick().await;
            let due: Vec<(SocketAddr, u8, [f32; 3])> = {
                let s = self.state.lock().unwrap();
                s.targets
                    .iter()
                    .filter_map(|(id, target)| s.drones.get(id).map(|d| (d.addr, *id, *target)))
                    .collect()
            };
            for (addr, system, [x, y, z]) in due {
                let setpoint = SET_POSITION_TARGET_LOCAL_NED_DATA {
                    time_boot_ms: self.boot.elapsed().as_millis() as u32,
                    x,
                    y,
                    z,
                    type_mask: PositionTargetTypemask::from_bits_truncate(POSITION_ONLY),
                    target_system: system,
                    target_component: AUTOPILOT_COMPONENT_ID,
                    coordinate_frame: MavFrame::MAV_FRAME_LOCAL_NED,
                    ..Default::default()
                };
                self.send(addr, &MavMessage::SET_POSITION_TARGET_LOCAL_NED(setpoint)).await;
            }
        }
    }
}

/// The ground station. Dropping it stops its background tasks and closes the socket.
pub struct Gcs {
    inner: Arc<Inner>,
    tasks: [JoinHandle<()>; 2],
    /// How long each attempt of a command waits for its COMMAND_ACK.
    pub ack_timeout: Duration,
    /// How many times a command is sent before Timeout is reported.
    pub attempts: u8,
}

impl Gcs {
    /// Listens on `addr` (`mavlink_link::GROUND_STATION` is QGroundControl's address; tests use
    /// port 0) and starts tracking and setpoint streaming. Call inside a tokio runtime.
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let inner = Arc::new(Inner {
            socket: UdpSocket::bind(addr).await?,
            boot: Instant::now(),
            sequence: AtomicU8::new(0),
            state: Mutex::default(),
            messages: broadcast::channel(MESSAGE_BACKLOG).0,
        });
        let tasks = [
            tokio::spawn(Arc::clone(&inner).receive_loop()),
            tokio::spawn(Arc::clone(&inner).setpoint_loop()),
        ];
        Ok(Self { inner, tasks, ack_timeout: ACK_TIMEOUT, attempts: ATTEMPTS })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.socket.local_addr()
    }

    /// A snapshot of every Drone heard from so far, by system id.
    pub fn drones(&self) -> BTreeMap<u8, DroneLink> {
        self.inner.state.lock().unwrap().drones.clone()
    }

    /// Every message received from now on, with its header. A subscriber that falls more than
    /// MESSAGE_BACKLOG messages behind loses the oldest ones.
    pub fn subscribe(&self) -> broadcast::Receiver<(MavHeader, MavMessage)> {
        self.inner.messages.subscribe()
    }

    /// Streams `target` (Mission frame: north, east, down in metres) to the Drone at 20 Hz;
    /// None stops its stream.
    pub fn set_target(&self, system: u8, target: Option<[f32; 3]>) {
        let mut s = self.inner.state.lock().unwrap();
        match target {
            Some(t) => s.targets.insert(system, t),
            None => s.targets.remove(&system),
        };
    }

    /// Sends COMMAND_LONG and waits for the COMMAND_ACK with the same system id and command id,
    /// resending up to `attempts` times.
    pub async fn command(&self, system: u8, command: MavCmd, params: [f32; 7]) -> Result<(), CommandError> {
        let (addr, mut ack) = {
            let mut s = self.inner.state.lock().unwrap();
            let addr = s.drones.get(&system).ok_or(CommandError::UnknownDrone(system))?.addr;
            (addr, s.expect_ack(system, command))
        };
        let [param1, param2, param3, param4, param5, param6, param7] = params;
        for confirmation in 0..self.attempts {
            let msg = MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
                param1,
                param2,
                param3,
                param4,
                param5,
                param6,
                param7,
                command,
                target_system: system,
                target_component: AUTOPILOT_COMPONENT_ID,
                confirmation,
            });
            self.inner.send(addr, &msg).await;
            match tokio::time::timeout(self.ack_timeout, &mut ack).await {
                Ok(Ok(MavResult::MAV_RESULT_ACCEPTED)) => return Ok(()),
                Ok(Ok(result)) => return Err(CommandError::Denied(result)),
                _ => {} // no ACK yet: send again
            }
        }
        Err(CommandError::Timeout)
    }

    /// PX4 offboard mode. The Drone accepts it only while setpoints are arriving (see `set_target`).
    pub async fn set_offboard(&self, system: u8) -> Result<(), CommandError> {
        let custom = MavModeFlag::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED.bits() as f32;
        let params = [custom, PX4_MAIN_MODE_OFFBOARD, 0.0, 0.0, 0.0, 0.0, 0.0];
        self.command(system, MavCmd::MAV_CMD_DO_SET_MODE, params).await
    }

    pub async fn arm(&self, system: u8) -> Result<(), CommandError> {
        let params = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        self.command(system, MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, params).await
    }

    pub async fn disarm(&self, system: u8) -> Result<(), CommandError> {
        self.command(system, MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, [0.0; 7]).await
    }

    /// Lands where the Drone is; PX4 disarms on its own after touchdown.
    pub async fn land(&self, system: u8) -> Result<(), CommandError> {
        self.command(system, MavCmd::MAV_CMD_NAV_LAND, [0.0; 7]).await
    }
}

impl Drop for Gcs {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::common::{
        COMMAND_ACK_DATA, EXTENDED_SYS_STATE_DATA, HEARTBEAT_DATA, MavAutopilot, MavState,
        MavType, MavVtolState,
    };

    const OFFBOARD: u32 = 6 << 16;
    const MANUAL: u32 = 1 << 16;

    /// One MAVLink v2 packet from `system`, component 1, encoded by the mavlink crate.
    fn packet(system: u8, msg: &MavMessage) -> Vec<u8> {
        let header = MavHeader { system_id: system as u32, component_id: 1, sequence: 7 };
        let mut buf = Vec::new();
        mavlink::write_v2_msg(&mut buf, header, msg).unwrap();
        assert_eq!(buf[0], 0xFD, "MAVLink v2 start byte");
        buf
    }

    fn feed(state: &mut State, datagram: &[u8], from: SocketAddr, now: Instant) {
        for (header, msg) in parse(datagram) {
            state.note(&header, &msg, from, now);
        }
    }

    fn heartbeat(custom_mode: u32, armed: bool) -> MavMessage {
        let mut base_mode = MavModeFlag::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED;
        if armed {
            base_mode |= MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED;
        }
        MavMessage::HEARTBEAT(HEARTBEAT_DATA {
            custom_mode,
            mavtype: MavType::MAV_TYPE_QUADROTOR,
            autopilot: MavAutopilot::MAV_AUTOPILOT_PX4,
            base_mode,
            system_status: MavState::MAV_STATE_ACTIVE,
            mavlink_version: 3,
        })
    }

    fn ack(command: MavCmd, result: MavResult) -> MavMessage {
        MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command, result })
    }

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    #[test]
    fn tracks_each_drone_by_system_id() {
        let mut state = State::default();
        let now = Instant::now();
        let position = LOCAL_POSITION_NED_DATA { x: 1.0, y: 2.0, z: -1.0, vz: -0.5, ..Default::default() };
        let attitude = ATTITUDE_DATA { roll: 0.1, pitch: -0.2, ..Default::default() };
        let sys_state = MavMessage::EXTENDED_SYS_STATE(EXTENDED_SYS_STATE_DATA {
            vtol_state: MavVtolState::MAV_VTOL_STATE_UNDEFINED,
            landed_state: MavLandedState::MAV_LANDED_STATE_IN_AIR,
        });
        feed(&mut state, &packet(1, &heartbeat(OFFBOARD, true)), addr(5001), now);
        // Two messages in one datagram are both read.
        let mut both = packet(1, &MavMessage::LOCAL_POSITION_NED(position.clone()));
        both.extend(packet(1, &MavMessage::ATTITUDE(attitude.clone())));
        feed(&mut state, &both, addr(5001), now);
        feed(&mut state, &packet(1, &sys_state), addr(5001), now);
        feed(&mut state, &packet(2, &heartbeat(MANUAL, false)), addr(5002), now);

        assert_eq!(state.drones.len(), 2);
        let one = &state.drones[&1];
        assert_eq!(one.addr, addr(5001));
        assert_eq!(one.mode, Some("OFFBOARD"));
        assert!(one.armed);
        assert_eq!(one.position, Some(position));
        assert_eq!(one.attitude, Some(attitude));
        assert_eq!(one.landed_state, Some(MavLandedState::MAV_LANDED_STATE_IN_AIR));
        let two = &state.drones[&2];
        assert_eq!(two.addr, addr(5002));
        assert_eq!(two.mode, Some("MANUAL"));
        assert!(!two.armed);
        assert_eq!(two.position, None);
        assert_eq!(two.landed_state, None);
    }

    #[test]
    fn garbage_and_truncated_bytes_are_skipped() {
        let mut state = State::default();
        let mut datagram = vec![0x00, 0x13, 0x37];
        datagram.extend(packet(3, &heartbeat(MANUAL, false)));
        feed(&mut state, &datagram, addr(5003), Instant::now());
        feed(&mut state, &[0xFD, 0x09], addr(5004), Instant::now());
        assert_eq!(state.drones.keys().copied().collect::<Vec<_>>(), vec![3]);
    }

    #[test]
    fn link_age_counts_from_the_last_packet() {
        let mut state = State::default();
        let t0 = Instant::now();
        feed(&mut state, &packet(1, &heartbeat(MANUAL, false)), addr(5001), t0);
        assert_eq!(state.drones[&1].link_age(t0 + Duration::from_millis(300)), Duration::from_millis(300));
        let pos = MavMessage::LOCAL_POSITION_NED(LOCAL_POSITION_NED_DATA::default());
        feed(&mut state, &packet(1, &pos), addr(5001), t0 + Duration::from_millis(200));
        assert_eq!(state.drones[&1].link_age(t0 + Duration::from_millis(500)), Duration::from_millis(300));
    }

    #[test]
    fn mode_names_follow_px4_custom_mode() {
        assert_eq!(mode_name(1 << 16), "MANUAL");
        assert_eq!(mode_name(6 << 16), "OFFBOARD");
        assert_eq!(mode_name((4 << 16) | (6 << 24)), "LAND");
        assert_eq!(mode_name((4 << 16) | (4 << 24)), "UNKNOWN");
        assert_eq!(mode_name(0), "UNKNOWN");
    }

    #[test]
    fn ack_goes_only_to_the_command_it_answers() {
        let mut state = State::default();
        let now = Instant::now();
        let arm = MavCmd::MAV_CMD_COMPONENT_ARM_DISARM;
        let mut waiting = state.expect_ack(1, arm);
        // Stale ACKs: right command from another system, another command from the right system.
        feed(&mut state, &packet(2, &ack(arm, MavResult::MAV_RESULT_ACCEPTED)), addr(5002), now);
        feed(&mut state, &packet(1, &ack(MavCmd::MAV_CMD_NAV_LAND, MavResult::MAV_RESULT_ACCEPTED)), addr(5001), now);
        assert_eq!(waiting.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        feed(&mut state, &packet(1, &ack(arm, MavResult::MAV_RESULT_DENIED)), addr(5001), now);
        assert_eq!(waiting.try_recv(), Ok(MavResult::MAV_RESULT_DENIED));
        // An ACK nobody waits for is dropped.
        feed(&mut state, &packet(1, &ack(arm, MavResult::MAV_RESULT_ACCEPTED)), addr(5001), now);
        assert!(state.pending.is_empty());
    }

    async fn wait_until(ok: impl Fn() -> bool) {
        let end = Instant::now() + Duration::from_secs(5);
        while !ok() {
            assert!(Instant::now() < end, "timed out waiting");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// Every message the fake Drone socket receives during `window`.
    async fn received(socket: &UdpSocket, window: Duration) -> Vec<MavMessage> {
        let end = tokio::time::Instant::now() + window;
        let mut buf = [0u8; 512];
        let mut out = Vec::new();
        while let Ok(Ok((n, _))) = tokio::time::timeout_at(end, socket.recv_from(&mut buf)).await {
            out.extend(parse(&buf[..n]).into_iter().map(|(_, m)| m));
        }
        out
    }

    /// A ground station on a free port and a fake Drone 1 it has heard from.
    async fn station_and_drone() -> (Gcs, UdpSocket) {
        let gcs = Gcs::bind("127.0.0.1:0").await.unwrap();
        let drone = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        drone.send_to(&packet(1, &heartbeat(MANUAL, false)), gcs.local_addr().unwrap()).await.unwrap();
        wait_until(|| gcs.drones().contains_key(&1)).await;
        (gcs, drone)
    }

    #[tokio::test]
    async fn command_without_ack_times_out_after_every_attempt() {
        let (mut gcs, drone) = station_and_drone().await;
        gcs.ack_timeout = Duration::from_millis(100);
        gcs.attempts = 3;
        let (result, seen) = tokio::join!(gcs.arm(1), received(&drone, Duration::from_millis(600)));
        assert_eq!(result, Err(CommandError::Timeout));
        let confirmations: Vec<u8> = seen
            .iter()
            .filter_map(|m| match m {
                MavMessage::COMMAND_LONG(d) if d.command == MavCmd::MAV_CMD_COMPONENT_ARM_DISARM => Some(d.confirmation),
                _ => None,
            })
            .collect();
        assert_eq!(confirmations, vec![0, 1, 2]);
        assert_eq!(gcs.arm(9).await, Err(CommandError::UnknownDrone(9)));
    }

    #[tokio::test]
    async fn command_returns_the_result_of_its_own_ack() {
        let (gcs, drone) = station_and_drone().await;
        let fake = async {
            let mut buf = [0u8; 512];
            let mut answered = 0;
            while answered < 2 {
                let (n, from) = drone.recv_from(&mut buf).await.unwrap();
                for (_, msg) in parse(&buf[..n]) {
                    let MavMessage::COMMAND_LONG(d) = msg else { continue };
                    if d.target_system != 1 || d.target_component != 1 {
                        continue;
                    }
                    let result = match d.command {
                        MavCmd::MAV_CMD_COMPONENT_ARM_DISARM if d.param1 == 1.0 => MavResult::MAV_RESULT_ACCEPTED,
                        MavCmd::MAV_CMD_DO_SET_MODE if (d.param1, d.param2) == (1.0, 6.0) => {
                            MavResult::MAV_RESULT_TEMPORARILY_REJECTED
                        }
                        _ => continue,
                    };
                    // A stale ACK for another command first; it must not be taken.
                    let stale = ack(MavCmd::MAV_CMD_NAV_LAND, MavResult::MAV_RESULT_ACCEPTED);
                    drone.send_to(&packet(1, &stale), from).await.unwrap();
                    drone.send_to(&packet(1, &ack(d.command, result)), from).await.unwrap();
                    answered += 1;
                }
            }
        };
        let station = async {
            let armed = gcs.arm(1).await;
            let offboard = gcs.set_offboard(1).await;
            (armed, offboard)
        };
        let ((armed, offboard), ()) = tokio::join!(station, fake);
        assert_eq!(armed, Ok(()));
        assert_eq!(offboard, Err(CommandError::Denied(MavResult::MAV_RESULT_TEMPORARILY_REJECTED)));
    }

    #[tokio::test]
    async fn setpoints_stream_at_20_hz_until_cleared() {
        let (gcs, drone) = station_and_drone().await;
        gcs.set_target(1, Some([0.5, -0.4, -1.0]));
        let seen = received(&drone, Duration::from_secs(1)).await;
        let setpoints: Vec<_> = seen
            .iter()
            .filter_map(|m| match m {
                MavMessage::SET_POSITION_TARGET_LOCAL_NED(d) => Some(d),
                _ => None,
            })
            .collect();
        // 20 Hz for 1 s; the margin covers a loaded machine.
        assert!((15..=22).contains(&setpoints.len()), "{} setpoints in 1 s", setpoints.len());
        for d in &setpoints {
            assert_eq!((d.target_system, d.target_component), (1, 1));
            assert_eq!(d.coordinate_frame, MavFrame::MAV_FRAME_LOCAL_NED);
            assert_eq!(d.type_mask.bits(), 4088);
            assert_eq!([d.x, d.y, d.z], [0.5, -0.4, -1.0]);
        }
        gcs.set_target(1, None);
        received(&drone, Duration::from_millis(150)).await;
        assert!(received(&drone, Duration::from_millis(300)).await.is_empty());
    }
}
