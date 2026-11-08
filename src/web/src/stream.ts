// Stream protocol client (SPEC decisions log, "Stream protocol"): hello, one binary point cloud, 30 Hz state, Verdicts.

export interface Hello {
  type: 'hello';
  swarm_size: number;
  room: { provenance: 'LIVE' | 'SIM' | 'REPLAY'; min: [number, number, number]; max: [number, number, number] };
  pad: [number, number, number][];
  places: { name: string; n: number; e: number; d: number }[];
}

export interface DroneState {
  id: number;
  n: number;
  e: number;
  d: number;
  roll: number;
  pitch: number;
  yaw: number;
  vn: number;
  ve: number;
  vd: number;
  mode: string;
  armed: boolean;
  link_ms: number;
  provenance: 'LIVE' | 'SIM' | 'REPLAY';
}

export interface State {
  type: 'state';
  t_ms: number;
  drones: DroneState[];
}

export interface Verdict {
  type: 'verdict';
  source: string;
  command: any;
  approved: boolean;
  reason: string | null;
}

export interface StreamHandlers {
  hello(h: Hello): void;
  points(nedTriples: Float32Array): void;
  state(s: State): void;
  verdict(v: Verdict): void;
  reset(ok: boolean): void;
  link(state: 'connecting' | 'open' | 'waiting', retryAt?: number, attempt?: number): void;
}

export const DEFAULT_WS = 'ws://127.0.0.1:7700/ws';

const LOOPBACK = ['127.0.0.1', 'localhost', '[::1]'];

/** The stream URL: `?ws=<url>` overrides the default, if it is a ws:// or wss:// URL on a loopback host. */
export function streamUrl(search: string): string {
  const raw = new URLSearchParams(search).get('ws');
  if (!raw) return DEFAULT_WS;
  try {
    const u = new URL(raw);
    if ((u.protocol === 'ws:' || u.protocol === 'wss:') && LOOPBACK.includes(u.hostname)) return u.href;
  } catch {
    /* fall through to the default */
  }
  console.warn('ignoring ?ws= that is not a ws:// or wss:// URL on 127.0.0.1, localhost or ::1');
  return DEFAULT_WS;
}

const HOST_IS_LE = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;
/** The binary frame is little-endian f32 (Stream protocol); one Float32Array, a byte swap only on a big-endian host. */
function littleEndianFloats(buf: ArrayBuffer): Float32Array {
  if (HOST_IS_LE) return new Float32Array(buf);
  const out = new Float32Array(buf.byteLength / 4);
  const dv = new DataView(buf);
  for (let i = 0; i < out.length; i++) out[i] = dv.getFloat32(i * 4, true);
  return out;
}

export function connect(url: string, on: StreamHandlers) {
  let ws: WebSocket | null = null;
  let attempt = 0;
  let timer = 0;

  const open = () => {
    on.link('connecting');
    ws = new WebSocket(url);
    ws.binaryType = 'arraybuffer';
    ws.onopen = () => {
      attempt = 0;
      on.link('open');
    };
    ws.onmessage = (ev) => {
      if (typeof ev.data !== 'string') {
        const buf = ev.data as ArrayBuffer;
        if (buf.byteLength % 12 === 0) on.points(littleEndianFloats(buf));
        else console.warn('point cloud frame is not whole (n, e, d) f32 triples; ignored');
        return;
      }
      let msg: any;
      try {
        msg = JSON.parse(ev.data);
      } catch {
        return;
      }
      // Each side ignores types it does not know (Stream protocol).
      if (msg?.type === 'state' && Array.isArray(msg.drones)) on.state(msg);
      else if (msg?.type === 'hello' && msg.room) on.hello(msg);
      else if (msg?.type === 'verdict') on.verdict(msg);
      else if (msg?.type === 'reset' && typeof msg.ok === 'boolean') on.reset(msg.ok);
    };
    ws.onclose = () => {
      ws = null;
      // Backoff 0.5, 1, 2, 4, 8 s (capped), with a little jitter so many tabs do not retry in step.
      const delay = Math.min(8000, 500 * 2 ** attempt) * (0.9 + Math.random() * 0.2);
      attempt++;
      on.link('waiting', performance.now() + delay, attempt);
      timer = window.setTimeout(open, delay);
    };
  };
  open();

  return {
    /** Sends one message now; false when there is no open connection. */
    send(msg: object): boolean {
      if (!ws || ws.readyState !== WebSocket.OPEN) return false;
      ws.send(JSON.stringify(msg));
      return true;
    },
    retryNow() {
      if (ws) return;
      clearTimeout(timer);
      open();
    },
  };
}
