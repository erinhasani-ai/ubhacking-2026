import '@fontsource-variable/geist-mono';
import '@fontsource/instrument-serif';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { hud, type DroneRow, type VerdictLine } from './hud.svelte';
import { CEILING_CUT_M, createScene } from './scene';
import { connect, streamUrl, type State } from './stream';

const VERDICT_SHOWN_MS = 6000;
const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)');

const canvas = document.createElement('canvas');
canvas.className = 'map';
canvas.setAttribute('aria-label', 'Room Twin map. Click a Drone to select it; click a spot to send the selected Drones there.');
Object.assign(canvas.style, { position: 'fixed', inset: '0', width: '100%', height: '100%', display: 'block', touchAction: 'none' });
document.body.prepend(canvas);

const url = streamUrl(location.search);
hud.url = url;

const minus = (v: number) => v.toFixed(2).replace('-', '−');
function droneWords(d: any): string {
  if (d === 'all') return 'all Drones';
  if (d?.ids) return `${d.ids.length === 1 ? 'Drone' : 'Drones'} ${d.ids.join(', ')}`;
  if (d?.count) return `${d.count} ${d.count === 1 ? 'Drone' : 'Drones'}`;
  return 'Drones';
}
/** One line for a Command, in the Mission frame for points. */
function summarize(c: any): string {
  switch (c?.kind) {
    case 'dispatch': {
      const t = c.target;
      const where = t === 'there' ? 'there' : t?.place ? `Place ${t.place}` : t?.point ? `n ${minus(t.point.n)}  e ${minus(t.point.e)}  d ${minus(t.point.d)}` : '?';
      return `Dispatch ${droneWords(c.drones)} → ${where}`;
    }
    case 'advance':
      return `Advance ${droneWords(c.drones)}`;
    case 'recall':
      return `Recall ${droneWords(c.drones)}`;
    case 'abort':
      return 'Abort';
    default:
      return 'Command';
  }
}

let seq = 0;
let hideTimer = 0;
function showLine(state: VerdictLine['state'], summary: string, reason: string | null, source: string) {
  hud.verdict = { seq: ++seq, state, summary, reason, source };
  hud.verdictVisible = true;
  clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => (hud.verdictVisible = false), VERDICT_SHOWN_MS);
}

let lastVerdict: { approved: boolean; reason: string | null } | null = null;

/** Sent Commands waiting for their Verdict; the engine echoes the Command unchanged, so a Verdict is matched on it and the source. */
const pending: { command: unknown; source: string; summary?: string }[] = [];
function sameJson(a: any, b: any): boolean {
  if (a === b) return true;
  if (!a || !b || typeof a !== 'object' || typeof b !== 'object' || Array.isArray(a) !== Array.isArray(b)) return false;
  const ka = Object.keys(a);
  return ka.length === Object.keys(b).length && ka.every((k) => Object.hasOwn(b, k) && sameJson(a[k], b[k]));
}
const RESET_SUMMARY = 'Reset: a new Mission, Drones back on the Launch pad';

const scene = createScene(
  canvas,
  {
    drone: (id, shift) => select(id, shift),
    spot(p) {
      const drones = hud.selected.length ? { ids: [...hud.selected].sort((a, b) => a - b) } : 'all';
      return sendCommand('click', { kind: 'dispatch', drones, target: { point: { n: p.n, e: p.e, d: p.d } } });
    },
  },
  () => reduceMotion.matches,
);

let scanJob = 0;
let meshRequested = false;
let originalScan: Float32Array | null = null;
const enhancement = new Worker(new URL('./enhance-cloud.worker.js', import.meta.url), { type: 'module' });
enhancement.onmessage = ({ data }) => {
  if (data.id !== scanJob) return;
  const { drawn } = scene.setPoints(data.points);
  if (hud.room) Object.assign(hud.room, { points: data.received, drawn, processing: false, enhanced: data.enhanced });
  originalScan = null;
};
enhancement.onerror = () => {
  if (originalScan && hud.room) {
    const { drawn } = scene.setPoints(originalScan);
    Object.assign(hud.room, { drawn, processing: false, enhanced: false });
  }
};

const stream = connect(url, {
  hello(h) {
    scanJob++;
    scene.setRoom(h);
    hud.places = h.places.map((p) => p.name);
    if (!meshRequested) {
      meshRequested = true;
      // The holographic layer is optional: without it the point cloud alone is the map.
      fetch('/api/mesh')
        .then((r) => (r.ok ? r.arrayBuffer() : null))
        .then((b) => {
          if (!b) return;
          scene.setMesh(b);
          scene.setHolo(hud.holo);
        })
        .catch((e) => console.warn('No holographic layer:', e));
    }
    hud.room = {
      provenance: h.room.provenance,
      size: [h.room.max[1] - h.room.min[1], h.room.max[0] - h.room.min[0], h.room.max[2] - h.room.min[2]],
      points: 0,
      drawn: 0,
      cut: CEILING_CUT_M,
      processing: false,
      enhanced: false,
    };
  },
  points(ned) {
    originalScan = ned;
    if (hud.room) Object.assign(hud.room, { points: ned.length / 3, processing: true });
    enhancement.postMessage({ id: ++scanJob, points: ned, ceiling: CEILING_CUT_M });
  },
  state(s) {
    if (hud.stale) {
      hud.stale = false;
      scene.setStale(false);
    }
    scene.pushState(s); // straight to the scene; the HUD copies the newest frame at about 10 Hz
    latest = s;
  },
  verdict(v) {
    lastVerdict = { approved: v.approved, reason: v.reason };
    const i = pending.findIndex((p) => p.source === v.source && sameJson(p.command, v.command));
    const mine = i >= 0 ? pending.splice(i, 1)[0] : null;
    if (mine && v.source === 'click' && v.command?.kind === 'dispatch') scene.resolveMarker(v.approved);
    if (v.approved && (v.command?.kind === 'abort' || v.command?.kind === 'recall')) scene.clearActive();
    showLine(v.approved ? 'approved' : 'rejected', mine?.summary ?? summarize(v.command), v.reason, v.source);
  },
  reset(ok) {
    if (ok) {
      pending.length = 0;
      hud.selected = [];
      hud.follow = false;
      scene.setSelected(new Set());
      scene.setFollow(false);
      scene.clearActive();
    }
    showLine(ok ? 'approved' : 'rejected', RESET_SUMMARY, ok ? null : 'The engine could not reset the Mission.', 'key');
  },
  link(state, retryAt, attempt) {
    if (state !== 'open') {
      // Nothing sent before the drop will be answered; the Drones stay drawn where they were, dimmed.
      if (pending.some((p) => p.source === 'click' && (p.command as any)?.kind === 'dispatch')) scene.resolveMarker(false);
      pending.length = 0;
      hud.stale = true;
      scene.setStale(true);
    }
    hud.link = state === 'waiting' ? { state, retryAt: retryAt!, attempt: attempt! } : { state };
    refreshRetry();
  },
});

function sendCommand(source: 'click' | 'key', command: object): boolean {
  if (stream.send({ type: 'command', source, command })) {
    pending.push({ command, source });
    return true;
  }
  showLine('not_sent', summarize(command), 'no connection to the engine', source);
  return false;
}

function select(id: number, shift: boolean) {
  const s = new Set(hud.selected);
  if (shift || s.has(id)) s.has(id) ? s.delete(id) : s.add(id);
  else {
    s.clear();
    s.add(id);
  }
  hud.selected = [...s];
  scene.setSelected(s);
}

const actions = {
  select,
  reset() {
    if (!stream.send({ type: 'reset' })) return showLine('not_sent', 'Reset', 'no connection to the engine', 'key');
    showLine('pending', RESET_SUMMARY, null, 'key');
  },
  abort(source: 'click' | 'key') {
    sendCommand(source, { kind: 'abort' }); // never waits on anything else
  },
  toggleFollow() {
    hud.follow = !hud.follow;
    scene.setFollow(hud.follow);
  },
  retryNow: () => stream.retryNow(),
  toggleHolo() {
    hud.holo = !hud.holo;
    scene.setHolo(hud.holo);
  },
  zoomIn: () => scene.zoomBy(0.7),
  zoomOut: () => scene.zoomBy(1.45),
  resetView() {
    if (hud.follow) actions.toggleFollow();
    scene.resetView();
  },
  sendPlace(place: string) {
    const drones = hud.selected.length ? { ids: [...hud.selected].sort((a, b) => a - b) } : 'all';
    sendCommand('click', { kind: 'dispatch', drones, target: { place } });
  },
};

addEventListener('keydown', (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey || e.repeat) return;
  if (e.key === 'Escape') actions.abort('key');
  else if (e.key === 'r' || e.key === 'R') actions.reset();
  else if (e.key === 'f' || e.key === 'F') actions.toggleFollow();
  else if (e.key === 'h' || e.key === 'H') actions.toggleHolo();
  else if (e.key === 'v' || e.key === 'V') actions.resetView();
  else if (e.key === '+' || e.key === '=') actions.zoomIn();
  else if (e.key === '-' || e.key === '_') actions.zoomOut();
  else return;
  e.preventDefault();
});

// HUD at about 10 Hz: copy only what it shows from the newest state frame.
let latest: State | null = null;
let shown: State | null = null;
function refreshRetry() {
  hud.retryIn = hud.link.state === 'waiting' ? Math.max(0, Math.ceil((hud.link.retryAt - performance.now()) / 1000)) : 0;
}
setInterval(() => {
  refreshRetry();
  if (!latest || latest === shown) return;
  shown = latest;
  const rows = hud.drones;
  const ds = [...latest.drones].sort((a, b) => a.id - b.id);
  if (rows.length !== ds.length) {
    hud.drones = ds.map((d): DroneRow => ({ id: d.id, mode: d.mode, armed: d.armed, link_ms: d.link_ms, provenance: d.provenance }));
    return;
  }
  ds.forEach((d, i) => {
    const r = rows[i];
    if (r.id !== d.id) r.id = d.id;
    if (r.mode !== d.mode) r.mode = d.mode;
    if (r.armed !== d.armed) r.armed = d.armed;
    if (r.link_ms !== d.link_ms) r.link_ms = d.link_ms;
    if (r.provenance !== d.provenance) r.provenance = d.provenance;
  });
}, 100);

mount(App, { target: document.getElementById('app')!, props: { actions } });

// Keep the view centred in the space left of the Drone list.
const column = document.querySelector('.column');
if (column) {
  new ResizeObserver(() => {
    const r = column.getBoundingClientRect();
    scene.setRightInset(innerWidth - r.left);
  }).observe(column);
}

document.fonts.load('600 11px "Geist Mono Variable"').then(() => scene.refreshLabels());

// Test-only, read-only debug hook for tests/web_ui.mjs (T15.G2). The UI never reads it.
Object.defineProperty(window, '__aether', {
  value: Object.freeze({
    get points() {
      return scene.stats().points;
    },
    get drones() {
      return scene.stats().drones;
    },
    get frames() {
      return scene.stats().frames;
    },
    get renderer() {
      return scene.stats().renderer;
    },
    get markerVisible() {
      return scene.stats().markerVisible;
    },
    get lastVerdict() {
      return lastVerdict;
    },
    project: (n: number, e: number, d: number) => scene.project(n, e, d),
  }),
});
