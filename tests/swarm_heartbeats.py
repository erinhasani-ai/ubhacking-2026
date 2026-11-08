#!/usr/bin/env python3
"""Ground-station check: one sim_swarm process (six Drones) shows up as systems 1-6 on UDP 14550.

Prints `swarm_ok systems=6 min_att_hz=<int> min_pos_hz=<int>` when every system sent MAVLink v2
heartbeats; otherwise `swarm_fail ...` and exit 1.
"""
import os

os.environ['MAVLINK20'] = '1'  # must be set before pymavlink is imported

import subprocess
import sys
import time

from pymavlink import mavutil

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
COUNT = 6
MEASURE_SECONDS = 5.0
START_TIMEOUT = 120.0  # includes a cold cargo build


def stop(proc):
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()


def measure(mav):
    """Counts per system from the first heartbeat of any system until MEASURE_SECONDS later."""
    stats = {i: {'hb_v2': 0, 'hb_bad': 0, 'att': 0, 'pos': 0} for i in range(1, COUNT + 1)}
    t_start = None
    deadline = time.time() + START_TIMEOUT
    while time.time() < deadline:
        msg = mav.recv_match(blocking=True, timeout=1.0)
        if msg is None:
            continue
        now = time.time()
        if t_start is None:
            if msg.get_type() != 'HEARTBEAT':
                continue
            t_start = now
            deadline = now + MEASURE_SECONDS
        sysid = msg.get_srcSystem()
        if sysid not in stats:
            continue
        kind = msg.get_type()
        if kind == 'HEARTBEAT':
            key = 'hb_v2' if msg.get_msgbuf()[0] == 0xFD else 'hb_bad'
            stats[sysid][key] += 1
        elif kind == 'ATTITUDE':
            stats[sysid]['att'] += 1
        elif kind == 'LOCAL_POSITION_NED':
            stats[sysid]['pos'] += 1
    return stats, (time.time() - t_start) if t_start else 0.0


def main():
    mav = mavutil.mavlink_connection('udpin:127.0.0.1:14550', source_system=255)  # bind first
    env = dict(os.environ)
    env['PATH'] = os.path.expanduser('~/.cargo/bin') + os.pathsep + env.get('PATH', '')
    proc = None
    try:
        subprocess.run(['cargo', 'build', '-q', '-p', 'aether-engine', '--bin', 'sim_swarm'],
                       cwd=REPO, env=env, check=True)
        # Something else already sending to 14550 (for example an orphaned swarm) would be counted as ours.
        if mav.recv_match(blocking=True, timeout=1.0) is not None:
            print('swarm_fail error=packets arrived on 14550 before the swarm started')
            return 1
        # The built binary directly, in our process group, so a group kill also kills it.
        proc = subprocess.Popen([os.path.join(REPO, 'target', 'debug', 'sim_swarm'),
                                 '--count', str(COUNT)], cwd=REPO)
        stats, elapsed = measure(mav)
    except Exception as e:
        print(f'swarm_fail error={e!r}')
        return 1
    finally:
        if proc is not None:
            stop(proc)
    missing = [i for i, s in stats.items() if s['hb_v2'] == 0]
    bad = [i for i, s in stats.items() if s['hb_bad']]
    if missing or bad or elapsed <= 0:
        print(f'swarm_fail systems={COUNT - len(missing)} no_v2_heartbeat={missing} non_v2={bad}')
        return 1
    min_att = int(min(s['att'] for s in stats.values()) / elapsed)
    min_pos = int(min(s['pos'] for s in stats.values()) / elapsed)
    print(f'swarm_ok systems={COUNT} min_att_hz={min_att} min_pos_hz={min_pos}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
