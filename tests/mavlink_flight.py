#!/usr/bin/env python3
"""Ground-station check: pymavlink flies the simulated Drone through the PX4 offboard sequence.

Builds and starts `sim_drone --id 1`, listens on UDP 127.0.0.1:14550 like QGroundControl,
and checks refusal, arming, a climb, a reposition, telemetry rates, landing and auto-disarm.
"""
import os

os.environ['MAVLINK20'] = '1'  # must be set before pymavlink is imported

import math
import signal
import subprocess
import sys
import threading
import time

from pymavlink import mavutil

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MAV = mavutil.mavlink
STEP_TIMEOUT = 10.0
# Position-only setpoints: ignore velocity, acceleration, yaw and yaw rate (bits 3-11).
POSITION_ONLY_MASK = 4088
SETPOINT_HZ = 20.0
ARM_PARAM = 1.0
DISARM_PARAM = 0.0
CLIMB_TARGET = (0.0, 0.0, -1.0)
SECOND_TARGET = (1.0, 0.5, -1.0)
HOVER_SECONDS = 3.0
MIN_ATT_HZ = 45
MIN_POS_HZ = 25


class Fail(Exception):
    pass


class GroundStation:
    """Reads every message from system 1 and keeps the latest state the checks need."""

    def __init__(self):
        self.mav = mavutil.mavlink_connection('udpin:127.0.0.1:14550', source_system=255)
        self.heartbeat = None
        self.position = None
        self.sys_state = None
        self.acks = []
        self.att_count = 0
        self.pos_count = 0

    def _note(self, msg):
        if msg.get_srcSystem() != 1:
            return
        kind = msg.get_type()
        if kind == 'HEARTBEAT':
            self.heartbeat = msg
        elif kind == 'LOCAL_POSITION_NED':
            self.position = msg
            self.pos_count += 1
        elif kind == 'ATTITUDE':
            self.att_count += 1
        elif kind == 'EXTENDED_SYS_STATE':
            self.sys_state = msg
        elif kind == 'COMMAND_ACK':
            self.acks.append((msg.command, msg.result))

    def pump(self, seconds):
        end = time.monotonic() + seconds
        while True:
            left = end - time.monotonic()
            if left <= 0:
                return
            msg = self.mav.recv_match(blocking=True, timeout=min(left, 0.05))
            if msg is not None:
                self._note(msg)

    def wait_for(self, predicate, timeout, what):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            self.pump(0.05)
            if predicate():
                return
        raise Fail(f'timed out after {timeout:.0f} s waiting for {what}')

    def command(self, cmd, params, what):
        padded = list(params) + [0.0] * (7 - len(params))
        first = len(self.acks)
        self.mav.mav.command_long_send(1, 1, cmd, 0, *padded)
        end = time.monotonic() + STEP_TIMEOUT
        while time.monotonic() < end:
            self.pump(0.05)
            for ack_cmd, result in self.acks[first:]:
                if ack_cmd == cmd:
                    return result
        raise Fail(f'no COMMAND_ACK for {what} within {STEP_TIMEOUT:.0f} s')

    def armed(self):
        return self.heartbeat is not None and bool(self.heartbeat.base_mode & MAV.MAV_MODE_FLAG_SAFETY_ARMED)

    def mode_name(self):
        if self.heartbeat is None:
            return 'none'
        return mavutil.interpret_px4_mode(self.heartbeat.base_mode, self.heartbeat.custom_mode)

    def distance_to(self, target):
        if self.position is None:
            return math.inf
        dx = self.position.x - target[0]
        dy = self.position.y - target[1]
        dz = self.position.z - target[2]
        return math.sqrt(dx * dx + dy * dy + dz * dz)


class Setpoints:
    """Streams position setpoints at SETPOINT_HZ from its own thread until stopped."""

    def __init__(self, mav, target):
        self.mav = mav
        self.target = target
        self.running = False
        self.thread = None

    def set_target(self, target):
        self.target = target

    def start(self):
        self.running = True
        self.thread = threading.Thread(target=self._run, daemon=True)
        self.thread.start()

    def stop(self):
        self.running = False
        if self.thread is not None:
            self.thread.join(timeout=2)

    def _run(self):
        period = 1.0 / SETPOINT_HZ
        while self.running:
            x, y, z = self.target
            self.mav.mav.set_position_target_local_ned_send(
                0, 1, 1, MAV.MAV_FRAME_LOCAL_NED, POSITION_ONLY_MASK,
                x, y, z, 0, 0, 0, 0, 0, 0, 0, 0)
            time.sleep(period)


def start_sim():
    env = dict(os.environ)
    env['PATH'] = os.path.expanduser('~/.cargo/bin') + os.pathsep + env.get('PATH', '')
    subprocess.run(['cargo', 'build', '-q', '-p', 'aether-engine', '--bin', 'sim_drone'],
                   cwd=REPO, env=env, check=True)
    # start_new_session puts cargo and the sim in their own process group, so killpg stops both.
    return subprocess.Popen(
        ['cargo', 'run', '-q', '-p', 'aether-engine', '--bin', 'sim_drone', '--', '--id', '1'],
        cwd=REPO, env=env, start_new_session=True)


def stop_sim(proc):
    if proc is None:
        return
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        proc.wait()


def run_sequence(link):
    result = {'refused': 0, 'armed': 0, 'reached': 0, 'landed': 0, 'att_hz': 0, 'pos_hz': 0}

    link.wait_for(lambda: link.heartbeat is not None, 30, 'a heartbeat from system 1')
    if link.heartbeat.get_msgbuf()[0] != 0xFD:
        raise Fail('heartbeat is not MAVLink v2 (first byte is not 0xFD)')

    # 1. Offboard before any setpoint must be refused.
    ack = link.command(MAV.MAV_CMD_DO_SET_MODE,
                       [MAV.MAV_MODE_FLAG_CUSTOM_MODE_ENABLED,
                        mavutil.px4_map['OFFBOARD'][1], mavutil.px4_map['OFFBOARD'][2]],
                       'OFFBOARD without setpoints')
    if ack == MAV.MAV_RESULT_ACCEPTED:
        raise Fail('OFFBOARD was ACCEPTED without setpoints')
    result['refused'] = 1

    # 2. Stream setpoints, then request OFFBOARD.
    setpoints = Setpoints(link.mav, CLIMB_TARGET)
    setpoints.start()
    try:
        link.pump(1.0)
        ack = link.command(MAV.MAV_CMD_DO_SET_MODE,
                           [MAV.MAV_MODE_FLAG_CUSTOM_MODE_ENABLED,
                            mavutil.px4_map['OFFBOARD'][1], mavutil.px4_map['OFFBOARD'][2]],
                           'OFFBOARD with setpoints')
        if ack != MAV.MAV_RESULT_ACCEPTED:
            raise Fail(f'OFFBOARD with setpoints got result {ack}, expected ACCEPTED')
        link.wait_for(lambda: link.mode_name() == 'OFFBOARD', 5, 'OFFBOARD in heartbeat')

        # 3. Arm.
        ack = link.command(MAV.MAV_CMD_COMPONENT_ARM_DISARM, [ARM_PARAM], 'arm')
        if ack != MAV.MAV_RESULT_ACCEPTED:
            raise Fail(f'arm got result {ack}, expected ACCEPTED')
        link.wait_for(link.armed, 5, 'SAFETY_ARMED in heartbeat')
        result['armed'] = 1

        # 4. Climb to 1 m.
        link.wait_for(lambda: link.position is not None and abs(link.position.z + 1.0) <= 0.1,
                      STEP_TIMEOUT, 'climb to z = -1.0 m')

        # 5. Second setpoint.
        setpoints.set_target(SECOND_TARGET)
        link.wait_for(lambda: link.distance_to(SECOND_TARGET) <= 0.1,
                      STEP_TIMEOUT, 'reaching the second setpoint')
        result['reached'] = 1

        # 6. Telemetry rates while hovering.
        att0, pos0 = link.att_count, link.pos_count
        started = time.monotonic()
        link.pump(HOVER_SECONDS)
        elapsed = time.monotonic() - started
        result['att_hz'] = math.floor((link.att_count - att0) / elapsed)
        result['pos_hz'] = math.floor((link.pos_count - pos0) / elapsed)

        # 7. Land and auto-disarm.
        ack = link.command(MAV.MAV_CMD_NAV_LAND, [], 'land')
        if ack != MAV.MAV_RESULT_ACCEPTED:
            raise Fail(f'land got result {ack}, expected ACCEPTED')
        link.wait_for(
            lambda: (link.sys_state is not None
                     and link.sys_state.landed_state == MAV.MAV_LANDED_STATE_ON_GROUND
                     and not link.armed()),
            20, 'ON_GROUND and disarmed after landing')
        result['landed'] = 1
    finally:
        setpoints.stop()
    return result


def main():
    sim = None
    link = None
    try:
        sim = start_sim()
        link = GroundStation()
        result = run_sequence(link)
    except Fail as err:
        print(f'FAILED: {err}')
        return 1
    except subprocess.CalledProcessError as err:
        print(f'FAILED: build failed with exit code {err.returncode}')
        return 1
    finally:
        if link is not None:
            link.mav.close()
        stop_sim(sim)

    problems = []
    if result['att_hz'] < MIN_ATT_HZ:
        problems.append(f"att_hz={result['att_hz']} below {MIN_ATT_HZ}")
    if result['pos_hz'] < MIN_POS_HZ:
        problems.append(f"pos_hz={result['pos_hz']} below {MIN_POS_HZ}")
    if problems:
        print('FAILED: telemetry rates too low: ' + '; '.join(problems))
        return 1
    print(f"mavlink_ok refused={result['refused']} armed={result['armed']} "
          f"reached={result['reached']} landed={result['landed']} "
          f"att_hz={result['att_hz']} pos_hz={result['pos_hz']}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
