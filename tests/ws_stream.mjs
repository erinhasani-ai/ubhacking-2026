// T14.G2: actual engine, scan, MAVLink simulation and policy; no mock mission or external services.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const cargo = path.join(homedir(), '.cargo/bin/cargo');
execFileSync(cargo, ['build', '-q', '-p', 'aether-engine', '--bin', 'aether'], { cwd: root, stdio: 'inherit' });
const engine = spawn(path.join(root, 'target/debug/aether'), ['--port', '0', '--gcs-port', '0'], { cwd: root });
let log = '';
engine.stdout.on('data', b => { log += b; });
engine.stderr.on('data', b => { log += b; });
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(fn, ms, label) {
  const deadline = performance.now() + ms;
  while (performance.now() < deadline) {
    const result = fn();
    if (result) return result;
    assert.equal(engine.exitCode, null, `engine exited: ${log}`);
    await delay(10);
  }
  throw new Error(`timed out: ${label}\n${log}`);
}

let ws;
try {
  const match = await until(() => log.match(/aether_ready url=(http:\/\/127\.0\.0\.1:\d+)/), 120_000, 'engine ready');
  ws = new WebSocket(match[1].replace('http:', 'ws:') + '/ws');
  ws.binaryType = 'arraybuffer';
  const frames = [];
  const states = [];
  const verdicts = [];
  ws.onmessage = ({ data }) => {
    const value = typeof data === 'string' ? JSON.parse(data) : data;
    frames.push(value);
    if (value.type === 'state') states.push({ value, at: performance.now() });
    if (value.type === 'verdict') verdicts.push(value);
  };
  await until(() => frames.length >= 3, 5000, 'hello, cloud, state');
  const hello = frames[0];
  assert.equal(hello.type, 'hello');
  assert.equal(hello.swarm_size, 6);
  assert.equal(hello.room.provenance, 'LIVE');
  assert.equal(hello.pad.length, 6);
  assert(hello.places.some(p => p.name === 'door'));
  assert(frames[1] instanceof ArrayBuffer);
  assert(frames[1].byteLength > 0 && frames[1].byteLength <= 200_000 * 12);
  assert.equal(frames[1].byteLength % 12, 0);
  const cloud = new DataView(frames[1]);
  for (let i = 0; i < cloud.byteLength; i += 4) {
    const axis = (i / 4) % 3;
    const value = cloud.getFloat32(i, true);
    assert(value >= hello.room.min[axis] - 1e-5 && value <= hello.room.max[axis] + 1e-5);
  }
  const first = states.length;
  await delay(2200);
  const observed = states.slice(first);
  const hz = (observed.length - 1) * 1000 / (observed.at(-1).at - observed[0].at);
  assert(hz >= 25, `state rate ${hz}`);
  for (const { value } of observed) {
    assert.deepEqual(value.drones.map(d => d.id), [1, 2, 3, 4, 5, 6]);
    for (const d of value.drones) {
      assert.equal(d.provenance, 'SIM');
      assert.equal(typeof d.armed, 'boolean');
      for (const key of ['n', 'e', 'd', 'roll', 'pitch', 'yaw', 'vn', 've', 'vd', 'link_ms']) assert(Number.isFinite(d[key]), key);
    }
  }
  async function command(value, approved) {
    const start = verdicts.length;
    ws.send(JSON.stringify({ type: 'command', source: 'click', command: value }));
    const verdict = await until(() => verdicts.slice(start).find(v => JSON.stringify(v.command) === JSON.stringify(value)
      || v.command.kind === value.kind), 10_000, 'real mission Verdict');
    assert.equal(verdict.approved, approved, verdict.reason);
    assert.equal(verdict.source, 'click');
    if (!approved) assert(verdict.reason.length > 0);
  }
  await command({ kind: 'dispatch', drones: { count: 2 }, target: { place: 'the moon' } }, false);
  await command({ kind: 'dispatch', drones: { count: 2 }, target: { place: 'door' } }, true);
  await until(() => states.at(-1).value.drones.filter(d => d.armed && d.d < -0.5).length === 2,
    10_000, 'two dispatched Drones actually climb');
  const resetAt = performance.now();
  const beforeReset = frames.length;
  ws.send(JSON.stringify({ type: 'reset' }));
  await until(() => frames.slice(beforeReset).some(f => f.type === 'reset' && f.ok), 5000, 'reset acknowledgment');
  const parked = () => states.at(-1).value.drones.every((d, i) => !d.armed
    && Math.hypot(d.n - hello.pad[i][0], d.e - hello.pad[i][1], d.d - hello.pad[i][2]) < 0.01);
  await until(parked, 5000 - (performance.now() - resetAt), 'all six reset to pad');
  const resetMs = Math.round(performance.now() - resetAt);
  // Old path/arm/land work must not restart after reset, and a new mission must still fly.
  await delay(1200);
  assert(parked(), 'stale work moved a Drone after reset');
  await command({ kind: 'dispatch', drones: { count: 2 }, target: { place: 'door' } }, true);
  await until(() => states.at(-1).value.drones.filter(d => d.armed && d.d < -0.5).length === 2,
    10_000, 'fresh mission climbs after reset');
  await command({ kind: 'abort' }, true);
  assert.equal(frames.filter(f => f instanceof ArrayBuffer).length, 1);
  console.log(`ws_ok drones=6 hz=${Math.floor(hz)} verdict=1 movement=1 rejection=1 reset_ms=${resetMs} redispatch=1`);
} finally {
  ws?.close();
  if (engine.exitCode === null) {
    const exited = once(engine, 'exit');
    engine.kill('SIGTERM');
    await exited;
  }
}
