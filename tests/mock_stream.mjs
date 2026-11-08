// Stand-in for the T14 engine: speaks the Stream protocol (SPEC decisions log) with the real scan's points and six SIM Drones.
// Usage: node tests/mock_stream.mjs [--port 7700]   (--port 0 picks a free port; the listening line names it)
//        node tests/mock_stream.mjs --self-test     (asserts on the Command validator, no scan needed)
// Zero dependencies: the WebSocket handshake and framing (RFC 6455) are written here with node:http and node:crypto.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// These mirror aether_core::frame (PAD_XZ, FLOOR_Y, pad_slot), as T8 measured them on the scan. Scan axes: x, y up, z.
// Mission frame: n = -(z - PAD_XZ[1]), e = x - PAD_XZ[0], d = FLOOR_Y - y.
const PAD_XZ = [-0.1, 1.4];
const FLOOR_Y = 0.096;

const SWARM_SIZE = 6;
const MAX_POINTS = 200_000;
const HOVER_ABOVE_SLOT = 0.35; // m above the slot while hovering
const TARGET_LIFT = 0.5; // m: a clicked spot is a surface point; Drones hover this far above it
const SPACING = 0.6; // m between Drones at a target
const V_MAX = 0.9; // m/s
const A_MAX = 1.6; // m/s²
const G = 9.81;
const STATE_HZ = 30;

// Pad slot i on the floor (mirrors aether_core::frame::pad_slot).
const slot = (i) => [((i / 3) | 0) * 0.6 - 0.3, (i % 3) * 0.6 - 0.6, 0];

// ---------- Command wire shape (SPEC "Command wire format (T3)"; checks the shapes in the SPEC, not a copy of aether-core's parser) ----------

const isObj = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);
function onlyKeys(obj, keys, where) {
  for (const k of Object.keys(obj)) if (!keys.includes(k)) throw new Error(`unknown field ${JSON.stringify(k)} in ${where}`);
  for (const k of keys) if (!Object.hasOwn(obj, k)) throw new Error(`missing field ${JSON.stringify(k)} in ${where}`);
}
const inSwarm = (id) => {
  if (!Number.isInteger(id) || id < 1 || id > SWARM_SIZE) throw new Error(`drone ${id} outside 1-${SWARM_SIZE}`);
};
function checkDrones(drones) {
  if (drones === 'all') return;
  if (!isObj(drones)) throw new Error('drones must be "all", {"count":n} or {"ids":[…]}');
  if (Object.hasOwn(drones, 'count')) return onlyKeys(drones, ['count'], 'drones'), inSwarm(drones.count);
  onlyKeys(drones, ['ids'], 'drones');
  if (!Array.isArray(drones.ids) || drones.ids.length === 0) throw new Error('drone ids are empty');
  const seen = new Set();
  for (const id of drones.ids) {
    inSwarm(id);
    if (seen.has(id)) throw new Error(`drone id ${id} repeated`);
    seen.add(id);
  }
}
function checkTarget(target) {
  if (target === 'there') return;
  if (!isObj(target)) throw new Error('target must be "there", {"place":…} or {"point":{n,e,d}}');
  if (Object.hasOwn(target, 'place')) {
    onlyKeys(target, ['place'], 'target');
    const p = target.place;
    if (typeof p !== 'string' || !p.trim() || p !== p.trim().toLowerCase()) throw new Error(`place ${JSON.stringify(p)} must be non-empty, trimmed and lowercase`);
    return;
  }
  onlyKeys(target, ['point'], 'target');
  if (!isObj(target.point)) throw new Error('point must be {n,e,d}');
  onlyKeys(target.point, ['n', 'e', 'd'], 'point');
  if (!['n', 'e', 'd'].every((k) => typeof target.point[k] === 'number' && Number.isFinite(target.point[k]))) throw new Error('point coordinates must be finite');
}
/** Throws with a plain reason unless `c` is a well-formed Command. */
export function validateCommand(c) {
  if (!isObj(c)) throw new Error('command must be an object');
  const shapes = { dispatch: ['kind', 'drones', 'target'], advance: ['kind', 'drones'], recall: ['kind', 'drones'], abort: ['kind'] };
  if (typeof c.kind !== 'string' || !Object.hasOwn(shapes, c.kind)) throw new Error(`unknown command kind ${JSON.stringify(c.kind)}`);
  onlyKeys(c, shapes[c.kind], 'command');
  if (c.drones !== undefined) checkDrones(c.drones);
  if (c.target !== undefined) checkTarget(c.target);
}

function selfTest() {
  const ok = (c) => validateCommand(c);
  const bad = (c, re) => {
    try {
      validateCommand(c);
    } catch (e) {
      if (re.test(e.message)) return;
      throw new Error(`wrong reason for ${JSON.stringify(c)}: ${e.message}`);
    }
    throw new Error(`accepted ${JSON.stringify(c)}`);
  };
  ok({ kind: 'dispatch', drones: 'all', target: { point: { n: 1, e: 2, d: -1 } } });
  ok({ kind: 'dispatch', drones: { ids: [1, 6] }, target: 'there' });
  ok({ kind: 'recall', drones: { count: 2 } });
  ok({ kind: 'abort' });
  bad({ kind: 'abort', drones: 'all' }, /unknown field "drones"/);
  bad({ kind: 'dispatch', drones: { ids: [] }, target: 'there' }, /empty/);
  bad({ kind: 'dispatch', drones: { ids: [2, 2] }, target: 'there' }, /repeated/);
  bad({ kind: 'dispatch', drones: { ids: [7] }, target: 'there' }, /outside 1-6/);
  bad({ kind: 'dispatch', drones: 'all', target: { point: { n: 1, e: 2, d: 3, x: 0 } } }, /unknown field "x"/);
  bad({ kind: 'dispatch', drones: 'all', target: { place: 'Door' } }, /lowercase/);
  bad({ kind: 'fly' }, /unknown command kind/);
  bad({ kind: 'constructor' }, /unknown command kind/);
  bad({ kind: ['abort'] }, /unknown command kind/);
  bad({ kind: 'recall', drones: { count: 1.5 } }, /outside/);
  bad({ kind: 'recall', drones: { ids: [1.5] } }, /outside/);
  bad({ kind: 'recall', drones: 'toString' }, /drones must be/);
  bad({ kind: 'dispatch', drones: 'all', target: 'constructor' }, /target must be/);
  console.log('mock_stream_self_test ok');
}

// ---------- The Room Twin from the real scan ----------

function findEnvFile() {
  // Like dotenvy: the nearest .env from the repo root upward (a worktree under .claude/worktrees finds the main checkout's).
  let dir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  for (;;) {
    const f = path.join(dir, '.env');
    if (existsSync(f)) return f;
    const up = path.dirname(dir);
    if (up === dir) return null;
    dir = up;
  }
}

function loadRoom() {
  if (!process.env.ROOM_SCAN_PATH) {
    const env = findEnvFile();
    if (env) process.loadEnvFile(env);
  }
  const scan = process.env.ROOM_SCAN_PATH;
  if (!scan || !existsSync(scan)) throw new Error('ROOM_SCAN_PATH is not set or the file is missing (value not printed)');
  const text = readFileSync(scan, 'utf8');
  const xyz = [];
  for (let i = 0; i < text.length; ) {
    let j = text.indexOf('\n', i);
    if (j < 0) j = text.length;
    if (text.charCodeAt(i) === 118 && text.charCodeAt(i + 1) === 32) {
      // "v x y z [r g b]"
      const f = text.slice(i + 2, j).trim().split(/\s+/);
      const x = +f[0], y = +f[1], z = +f[2];
      if (Number.isFinite(x) && Number.isFinite(y) && Number.isFinite(z)) xyz.push(x, y, z);
    }
    i = j + 1;
  }
  const count = xyz.length / 3;
  if (count < 1000) throw new Error(`scan has only ${count} vertices`);
  const [xPad, zPad] = PAD_XZ;
  const floorY = FLOOR_Y;

  // Decimate evenly to at most MAX_POINTS, then convert to the Mission frame (SPEC T8).
  const stride = Math.ceil(count / MAX_POINTS);
  const kept = Math.ceil(count / stride);
  const buf = Buffer.alloc(kept * 12);
  const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity];
  for (let k = 0, o = 0; k < count; k += stride, o += 12) {
    const x = xyz[3 * k], y = xyz[3 * k + 1], z = xyz[3 * k + 2];
    const ned = [-(z - zPad), x - xPad, floorY - y];
    for (let a = 0; a < 3; a++) {
      buf.writeFloatLE(ned[a], o + 4 * a);
      const v = buf.readFloatLE(o + 4 * a); // the box is of the f32 values the browser receives
      if (v < min[a]) min[a] = v;
      if (v > max[a]) max[a] = v;
    }
  }
  return { points: buf, count: kept, min, max };
}

// ---------- Swarm simulation (all SIM) ----------

const drones = [];
function home(i) {
  const s = slot(i);
  return [s[0], s[1], s[2] - HOVER_ABOVE_SLOT];
}
function resetSwarm() {
  drones.length = 0;
  for (let i = 0; i < SWARM_SIZE; i++) {
    // Landed on the slot, disarmed; a dispatch takes off.
    drones.push({ id: i + 1, p: slot(i), v: [0, 0, 0], a: [0, 0, 0], yaw: 0, goal: slot(i), phase: 'landed', mode: 'LAND', armed: false, link: 12, bob: i * 1.7 });
  }
}

function resolveDrones(sel, target) {
  if (sel === 'all') return drones.slice();
  if (sel.ids) return sel.ids.map((id) => drones[id - 1]);
  const dist = (d) => Math.hypot(d.p[0] - target[0], d.p[1] - target[1]);
  return drones.slice().sort((a, b) => dist(a) - dist(b)).slice(0, sel.count);
}

const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

function dispatch(chosen, point, room) {
  const k = chosen.length;
  const r = k === 1 ? 0 : SPACING / (2 * Math.sin(Math.PI / k)); // neighbours on the ring are SPACING apart
  const c = [clamp(point.n, room.min[0] + 0.3, room.max[0] - 0.3), clamp(point.e, room.min[1] + 0.3, room.max[1] - 0.3), clamp(point.d - TARGET_LIFT, room.min[2] + 0.3, -0.3)];
  // Order Drones by bearing from the target so their paths to the ring do not cross.
  const bearing = (d) => Math.atan2(d.p[1] - c[1], d.p[0] - c[0]);
  const sorted = chosen.slice().sort((a, b) => bearing(a) - bearing(b));
  const start = sorted.length ? bearing(sorted[0]) : 0;
  sorted.forEach((d, j) => {
    const t = start + (2 * Math.PI * j) / k;
    d.goal = [c[0] + r * Math.cos(t), c[1] + r * Math.sin(t), c[2]];
    d.phase = 'fly';
    d.mode = 'OFFBOARD';
    d.armed = true;
  });
}

function verdictFor(cmd, room) {
  validateCommand(cmd);
  switch (cmd.kind) {
    case 'dispatch': {
      const t = cmd.target;
      if (t === 'there') throw new Error('no pointing ray yet: "there" needs the phone or a finger');
      if (t.place) throw new Error(`unknown Place ${JSON.stringify(t.place)}; no Places are known yet`);
      const p = [t.point.n, t.point.e, t.point.d];
      if (!p.every((v, a) => v >= room.min[a] && v <= room.max[a])) throw new Error('target is outside the Room Twin');
      dispatch(resolveDrones(cmd.drones, p), t.point, room);
      return;
    }
    case 'recall':
      for (const d of resolveDrones(cmd.drones, [0, 0])) {
        d.goal = home(d.id - 1);
        d.phase = 'rtl';
        d.mode = 'RTL';
      }
      return;
    case 'abort':
      for (const d of drones) {
        if (!d.armed) continue;
        d.goal = d.p.slice();
        d.phase = 'hold';
        d.mode = 'HOLD';
      }
      return;
    default:
      throw new Error(`${cmd.kind} is not simulated by the mock stream`);
  }
}

function step(dt, tSec) {
  for (const d of drones) {
    if (!d.armed) {
      d.v = [0, 0, 0];
      d.a = [0, 0, 0];
      continue;
    }
    // Hovering Drones drift a few centimetres around their goal (SIM).
    const g = d.goal.slice();
    if (d.phase === 'hover' || d.phase === 'hold') {
      g[0] += 0.02 * Math.sin(0.7 * tSec + d.bob);
      g[1] += 0.02 * Math.cos(0.9 * tSec + d.bob);
      g[2] += 0.015 * Math.sin(1.3 * tSec + 2 * d.bob);
    }
    const e = [g[0] - d.p[0], g[1] - d.p[1], g[2] - d.p[2]];
    const dist = Math.hypot(...e);
    const speed = Math.min(V_MAX, Math.sqrt(2 * A_MAX * dist) * 0.8, dist * 4);
    const vDes = dist > 1e-6 ? e.map((x) => (x / dist) * speed) : [0, 0, 0];
    // Keep SPACING between Drones in flight: push apart any pair that is closer.
    for (const o of drones) {
      if (o === d) continue;
      const s = [d.p[0] - o.p[0], d.p[1] - o.p[1], d.p[2] - o.p[2]];
      const sd = Math.hypot(...s);
      if (sd > 1e-6 && sd < SPACING) for (let a = 0; a < 3; a++) vDes[a] += (s[a] / sd) * (SPACING - sd) * 2.5;
    }
    const acc = vDes.map((v, a) => (v - d.v[a]) / dt);
    const am = Math.hypot(...acc);
    if (am > A_MAX) for (let a = 0; a < 3; a++) acc[a] *= A_MAX / am;
    for (let a = 0; a < 3; a++) {
      d.v[a] += acc[a] * dt;
      d.p[a] += d.v[a] * dt;
    }
    d.a = acc;
    const hs = Math.hypot(d.v[0], d.v[1]);
    if (hs > 0.15) {
      let dy = Math.atan2(d.v[1], d.v[0]) - d.yaw;
      dy = Math.atan2(Math.sin(dy), Math.cos(dy));
      d.yaw += clamp(dy, -2 * dt, 2 * dt); // yaw rate limit 2 rad/s
      d.yaw = Math.atan2(Math.sin(d.yaw), Math.cos(d.yaw));
    }
    if (d.phase === 'rtl' && dist < 0.05) {
      d.phase = 'land';
      d.mode = 'LAND';
      const s = slot(d.id - 1);
      d.goal = [s[0], s[1], s[2]];
    } else if (d.phase === 'land' && dist < 0.02) {
      d.phase = 'landed';
      d.armed = false;
      d.p = d.goal.slice();
    } else if (d.phase === 'fly' && dist < 0.05) {
      d.phase = 'hover';
      d.mode = 'HOLD';
    }
    d.link = clamp(d.link + (Math.random() - 0.5) * 6, 6, 40); // SIM link age, ms
  }
}

function stateFrame(tMs) {
  const r = (v, k = 3) => Number(v.toFixed(k));
  return JSON.stringify({
    type: 'state',
    t_ms: tMs,
    drones: drones.map((d) => {
      const cy = Math.cos(d.yaw), sy = Math.sin(d.yaw);
      const aF = d.a[0] * cy + d.a[1] * sy, aR = -d.a[0] * sy + d.a[1] * cy;
      return {
        id: d.id, n: r(d.p[0]), e: r(d.p[1]), d: r(d.p[2]),
        roll: r(Math.atan2(aR, G), 4), pitch: r(-Math.atan2(aF, G), 4), yaw: r(d.yaw, 4),
        vn: r(d.v[0]), ve: r(d.v[1]), vd: r(d.v[2]),
        mode: d.mode, armed: d.armed, link_ms: Math.round(d.link), provenance: 'SIM',
      };
    }),
  });
}

// ---------- WebSocket (RFC 6455), server side ----------

const WS_GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
const MAX_IN = 1 << 20; // browser → engine messages are small

function frame(opcode, payload) {
  const n = payload.length;
  const head = n < 126 ? Buffer.from([0x80 | opcode, n]) : n < 65536 ? Buffer.alloc(4) : Buffer.alloc(10);
  if (n >= 126 && n < 65536) {
    head[0] = 0x80 | opcode;
    head[1] = 126;
    head.writeUInt16BE(n, 2);
  } else if (n >= 65536) {
    head[0] = 0x80 | opcode;
    head[1] = 127;
    head.writeBigUInt64BE(BigInt(n), 2);
  }
  return Buffer.concat([head, payload]);
}

class Client {
  constructor(socket, id, onText) {
    this.socket = socket;
    this.id = id;
    this.onText = onText;
    this.buf = Buffer.alloc(0);
    this.parts = [];
    this.open = true;
    socket.on('data', (chunk) => this.read(chunk));
  }
  send(opcode, payload) {
    if (this.open) this.socket.write(frame(opcode, payload));
  }
  text(s) {
    this.send(1, Buffer.from(s));
  }
  close(code) {
    if (!this.open) return;
    const p = Buffer.alloc(2);
    p.writeUInt16BE(code);
    this.send(8, p);
    this.open = false;
    this.socket.end();
  }
  read(chunk) {
    this.buf = Buffer.concat([this.buf, chunk]);
    while (this.buf.length >= 2) {
      const b0 = this.buf[0], b1 = this.buf[1];
      const fin = b0 & 0x80, op = b0 & 0x0f;
      let len = b1 & 0x7f, off = 2;
      if (len === 126) {
        if (this.buf.length < 4) return;
        len = this.buf.readUInt16BE(2);
        off = 4;
      } else if (len === 127) {
        if (this.buf.length < 10) return;
        const big = this.buf.readBigUInt64BE(2);
        if (big > BigInt(MAX_IN)) return this.close(1009);
        len = Number(big);
        off = 10;
      }
      if (!(b1 & 0x80)) return this.close(1002); // client frames must be masked
      if (len > MAX_IN) return this.close(1009);
      if (this.buf.length < off + 4 + len) return;
      const mask = this.buf.subarray(off, off + 4);
      const data = Buffer.from(this.buf.subarray(off + 4, off + 4 + len));
      for (let i = 0; i < len; i++) data[i] ^= mask[i & 3];
      this.buf = this.buf.subarray(off + 4 + len);
      if (op === 8) return this.close(len >= 2 ? data.readUInt16BE(0) : 1000);
      if (op === 9) this.send(10, data);
      else if (op === 0 || op === 1 || op === 2) {
        this.parts.push(data);
        if (Buffer.concat(this.parts).length > MAX_IN) return this.close(1009);
        if (fin) {
          const msg = Buffer.concat(this.parts);
          const textual = op === 1 || this.firstOp === 1;
          this.parts = [];
          this.firstOp = undefined;
          if (textual) this.onText(this, msg.toString('utf8'));
        } else if (op !== 0) this.firstOp = op;
      }
    }
  }
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--self-test')) return selfTest();
  const pi = args.indexOf('--port');
  const port = pi >= 0 ? Number(args[pi + 1]) : 7700;

  const room = loadRoom();
  const hello = JSON.stringify({
    type: 'hello',
    swarm_size: SWARM_SIZE,
    room: { provenance: 'LIVE', min: room.min, max: room.max },
    pad: Array.from({ length: SWARM_SIZE }, (_, i) => slot(i)),
    places: [],
  });
  resetSwarm();
  const clients = new Set();
  let nextId = 1;
  let commands = 0;

  function onText(c, text) {
    let msg;
    try {
      msg = JSON.parse(text);
    } catch {
      return; // not JSON: ignored like an unknown type
    }
    if (msg?.type === 'reset') {
      resetSwarm();
      console.error(`reset from #${c.id}`);
      // Same explicit acknowledgment as T14; Recall remains a separate flight command.
      c.text(JSON.stringify({ type: 'reset', ok: true }));
    } else if (msg?.type === 'command') {
      const n = ++commands;
      const source = ['click', 'key'].includes(msg.source) ? msg.source : 'click';
      let reason = null;
      try {
        verdictFor(msg.command, room);
      } catch (e) {
        reason = e.message;
      }
      console.error(`command ${n} from #${c.id} source=${source} kind=${msg.command?.kind} ${reason ? `rejected: ${reason}` : 'approved'}`);
      const v = JSON.stringify({ type: 'verdict', source, command: msg.command ?? null, approved: !reason, reason });
      for (const o of clients) o.text(v);
    }
  }

  const server = createServer((req, res) => {
    res.writeHead(404, { 'content-type': 'text/plain' });
    res.end('mock_stream: WebSocket only, at /ws\n');
  });
  server.on('upgrade', (req, socket) => {
    const key = req.headers['sec-websocket-key'];
    if (new URL(req.url, 'http://x').pathname !== '/ws' || !key || String(req.headers.upgrade).toLowerCase() !== 'websocket') {
      socket.end('HTTP/1.1 400 Bad Request\r\n\r\n');
      return;
    }
    const accept = createHash('sha1').update(key + WS_GUID).digest('base64');
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    socket.setNoDelay(true);
    const c = new Client(socket, nextId++, onText);
    clients.add(c);
    console.error(`connect #${c.id} from ${socket.remoteAddress}`);
    const gone = () => {
      if (clients.delete(c)) console.error(`disconnect #${c.id}`);
      c.open = false;
    };
    socket.on('close', gone);
    socket.on('error', gone);
    c.text(hello);
    c.send(2, room.points);
  });

  const t0 = performance.now();
  let last = t0;
  setInterval(() => {
    const now = performance.now();
    step(Math.min(0.1, (now - last) / 1000), (now - t0) / 1000);
    last = now;
    const s = stateFrame(Math.round(now - t0));
    for (const c of clients) if (c.socket.writableLength < 1 << 20) c.text(s); // skip a frame for a client that is not reading
  }, 1000 / STATE_HZ);

  server.listen(port, '127.0.0.1', () => {
    console.error(`mock_stream listening ws://127.0.0.1:${server.address().port}/ws points=${room.count}`);
  });
}

main();
