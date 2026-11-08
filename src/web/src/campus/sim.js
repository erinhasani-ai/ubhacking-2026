// Campus simulation (T32/T33, all SIM): five teams of six kinematic Drones on pads across UB North Campus, and a
// simulated fire. Every Drone scouts; in each team two are alert Drones and four are suppression Drones.
// Plain JS so tests/campus_sim.mjs can import it without a build. Map units: metres, x east, y north, z up.

export const TIME_SCALE = 5; // sim seconds per real second, shown on screen
export const DETECT_RANGE_M = 250; // stand-in for a thermal camera's useful range (SIM)
export const ALERT_MARGIN_M = 150; // alert radius = burning radius + this
export const PER_TEAM = 6;
export const ALERT_PER_TEAM = 2; // slots 0-1 alert, the rest suppress
const MAX_V = 16; // m/s horizontal
const MAX_VZ = 5;
const ACCEL = 6;
const PATROL_V = 11;
const PATROL_R = 280;
const NEAR_M = 40; // closer than this horizontally, a Drone descends to its task height
const GROWTH = 0.004; // fire intensity growth per sim second, scaled by (1 - intensity)
const SUPPRESS = 0.0025; // intensity removed per sim second per suppression Drone on station
export const QUAKE_SPEED = 350; // m per sim second: a visual sweep, far slower than real seismic waves
export const QUAKE_TRIGGER_G = 0.05; // a sensor node counts as shaking above this
const SHAKE_S = 14;

/** Teams and the building each team's pad is next to. */
export const TEAMS = [
  { id: 'alpha', name: 'Alpha', near: 'Davis Hall' },
  { id: 'bravo', name: 'Bravo', near: 'Fargo Quadrangle' },
  { id: 'charlie', name: 'Charlie', near: 'Center for the Arts' },
  { id: 'delta', name: 'Delta', near: 'Natural Sciences Complex' },
  { id: 'echo', name: 'Echo', near: 'Flint Village Community Center' },
];

const hyp = (x, y) => Math.hypot(x, y);
const hdist = (a, b) => hyp(a[0] - b[0], a[1] - b[1]);

export function inside(p, ring) {
  let hit = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [xi, yi] = ring[i];
    const [xj, yj] = ring[j];
    if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi) hit = !hit;
  }
  return hit;
}

const GENERIC = new Set('hall center centre building complex the of for and memorial quadrangle lecture community house laboratory plant court village suites inn at ub university apartments luxury field campus'.split(' '));
const ALIASES = { capon: 'capen hall', kaplan: 'capen hall', capin: 'capen hall', su: 'student union', nsc: 'natural sciences complex' };
export const norm = (s) => ` ${s.toLowerCase().replace(/['’]/g, '').replace(/[^a-z0-9]+/g, ' ').trim()} `;

/** Campus world from the cached OpenStreetMap file. */
export function buildWorld(data) {
  const buildings = data.buildings.map((b, i) => {
    const ring = b.ring;
    let cx = 0;
    let cy = 0;
    for (const [x, y] of ring) {
      cx += x;
      cy += y;
    }
    cx /= ring.length;
    cy /= ring.length;
    let r = 0;
    for (const [x, y] of ring) r = Math.max(r, hyp(x - cx, y - cy));
    return { i, name: b.name, h: b.h, est: b.est, ring, c: [cx, cy], r };
  });
  const cruise = Math.ceil(Math.max(...buildings.map((b) => b.h)) + 20);

  // One Place per distinct name: the largest footprint with that name.
  const byName = new Map();
  for (const b of buildings) {
    if (!b.name) continue;
    const k = b.name.toLowerCase();
    if (!byName.has(k) || byName.get(k).r < b.r) byName.set(k, b);
  }
  const places = [...byName.values()].map((b) => ({ b, tokens: norm(b.name).trim().split(' ').filter((t) => t.length > 2 && !GENERIC.has(t)) }));

  // Academic core centre: named buildings within 900 m of Davis Hall.
  const core = places.map((p) => p.b).filter((b) => hyp(...b.c) < 900);
  const centre = [core.reduce((s, b) => s + b.c[0], 0) / core.length, core.reduce((s, b) => s + b.c[1], 0) / core.length];

  // Each pad: the open spot nearest its anchor building, 30 m clear of every footprint (centroid radius, conservative).
  const clearAt = (p) => Math.min(...buildings.map((b) => hdist(p, b.c) - b.r));
  const pads = TEAMS.map((t) => {
    const anchor = byName.get(t.near.toLowerCase())?.c ?? [0, 0];
    let at = anchor;
    let best = Infinity;
    for (let x = -250; x <= 250; x += 10)
      for (let y = -250; y <= 250; y += 10) {
        const p = [anchor[0] + x, anchor[1] + y];
        if (hyp(x, y) < best && clearAt(p) >= 30) {
          best = hyp(x, y);
          at = p;
        }
      }
    // Each team patrols a loop pulled from its pad toward the core, so five loops cover the campus.
    const patrol = [at[0] + (centre[0] - at[0]) * 0.4, at[1] + (centre[1] - at[1]) * 0.4];
    return { team: t.id, near: t.near, at, patrol };
  });

  // Accelerometer nodes (SIM): one on each named building within 900 m of Davis Hall.
  const sensors = core.slice(0, 28);

  return { buildings, places, cruise, pads, centre, sensors, attribution: data.attribution, heights: data.heights };
}

/** The best-matching named building in a transcript, or null. */
export function findBuilding(world, text) {
  let t = norm(text);
  for (const [k, v] of Object.entries(ALIASES)) t = t.replaceAll(` ${k} `, ` ${v} `);
  let top = null;
  let topScore = 0;
  for (const p of world.places) {
    const hits = p.tokens.filter((tok) => t.includes(` ${tok} `)).length;
    if (!hits) continue;
    const score = hits * 10 + (t.includes(norm(p.b.name)) ? 5 : 0) - hyp(...p.b.c) / 1000;
    if (score > topScore) {
      topScore = score;
      top = p.b;
    }
  }
  return top;
}

export function buildingAt(world, p) {
  return world.buildings.find((b) => hdist(p, b.c) <= b.r && inside(p, b.ring)) ?? null;
}

export function createSim(world) {
  const sim = { world, t: 0, drones: [], fire: null, autoResponse: true };
  TEAMS.forEach((team, ti) => {
    const base = world.pads[ti].at;
    for (let k = 0; k < PER_TEAM; k++) {
      const pad = [base[0] + ((k % 3) - 1) * 8, base[1] + (k < 3 ? 4 : -4)];
      const role = k < ALERT_PER_TEAM ? 'alert' : 'suppress';
      sim.drones.push({ id: `${team.name[0]}${k + 1}`, team: team.id, slot: k, role, pad, pos: [pad[0], pad[1], 0], vel: [0, 0, 0], batt: 100, task: { kind: 'idle' } });
    }
  });
  return sim;
}

export function resetSim(sim) {
  const fresh = createSim(sim.world);
  sim.t = 0;
  sim.drones = fresh.drones;
  sim.fire = null;
  sim.quake = null;
}

const teamDrones = (sim, id) => sim.drones.filter((d) => d.team === id);
const airborne = (d) => d.pos[2] > 2;
const padOf = (sim, id) => sim.world.pads.find((p) => p.team === id);

function ring(at, slot, n, radius) {
  const a = (slot / n) * Math.PI * 2;
  return [at[0] + Math.cos(a) * radius, at[1] + Math.sin(a) * radius];
}

/** Where a Drone wants to be now: [x, y, z]. */
function aim(sim, d) {
  const { world } = sim;
  const k = d.task;
  const cruise = world.cruise;
  switch (k.kind) {
    case 'idle':
    case 'return':
      return [d.pad[0], d.pad[1], 0];
    case 'hold':
      return k.p;
    case 'goto':
      return [...ring(k.at, d.slot, PER_TEAM, 25), cruise];
    case 'patrol': {
      const c = padOf(sim, d.team).patrol;
      const a = (sim.t * PATROL_V) / PATROL_R + (d.slot / PER_TEAM) * Math.PI * 2;
      return [c[0] + Math.cos(a) * PATROL_R, c[1] + Math.sin(a) * PATROL_R, cruise];
    }
    case 'orbit': {
      const a = (sim.t * PATROL_V) / k.radius + (d.slot / PER_TEAM) * Math.PI * 2;
      return [k.at[0] + Math.cos(a) * k.radius, k.at[1] + Math.sin(a) * k.radius, cruise];
    }
    case 'suppress': {
      const f = sim.fire;
      if (!f) return d.pos;
      return [...ring(f.at, d.slot, PER_TEAM, 15), f.roof + 12];
    }
    case 'alert': {
      const b = world.buildings[k.building];
      return [b.c[0], b.c[1], b.h + 15];
    }
    case 'inspect': {
      const b = world.buildings[k.building];
      return [...ring(b.c, d.slot, PER_TEAM, 12), b.h + 20];
    }
  }
  return d.pos;
}

function fly(sim, d, dt) {
  let [tx, ty, tz] = aim(sim, d);
  const dx = tx - d.pos[0];
  const dy = ty - d.pos[1];
  const hd = hyp(dx, dy);
  // Travel at cruise height (above every roof); climb before moving off, descend only near the target.
  if (hd > NEAR_M) tz = sim.world.cruise;
  const climbing = hd > NEAR_M && d.pos[2] < sim.world.cruise - 8;
  const vh = climbing ? Math.min(2, hd) : Math.min(MAX_V, hd * 0.6);
  const want = [hd > 0.01 ? (dx / hd) * vh : 0, hd > 0.01 ? (dy / hd) * vh : 0, Math.max(-MAX_VZ, Math.min(MAX_VZ, (tz - d.pos[2]) * 0.8))];
  for (let i = 0; i < 3; i++) {
    const dv = want[i] - d.vel[i];
    d.vel[i] += Math.max(-ACCEL * dt, Math.min(ACCEL * dt, dv));
    d.pos[i] += d.vel[i] * dt;
  }
  if (d.pos[2] < 0) {
    d.pos[2] = 0;
    d.vel[2] = 0;
  }
  d.batt = airborne(d) ? Math.max(0, d.batt - 0.04 * dt) : Math.min(100, d.batt + 0.5 * dt);
  if (d.task.kind === 'return' && hd < 1.5 && d.pos[2] < 0.3) {
    d.task = { kind: 'idle' };
    d.vel = [0, 0, 0];
  }
}

/** Buildings inside the alert radius, nearest first. */
function zoneOf(sim) {
  const f = sim.fire;
  const R = f.radius + ALERT_MARGIN_M;
  return sim.world.buildings
    .map((b) => ({ b, dist: hdist(b.c, f.at) }))
    .filter(({ b, dist }) => dist - b.r * 0.5 <= R)
    .sort((a, b) => a.dist - b.dist)
    .map(({ b }) => b.i);
}

export function ignite(sim, building) {
  sim.fire = {
    at: [...building.c],
    name: building.name ?? 'an unnamed building',
    building: building.i,
    roof: building.h,
    t0: sim.t,
    intensity: 0.55,
    radius: 12,
    status: 'burning',
    detected: false,
    detectedBy: null,
    detectedAfter: null,
    zone: [],
    alertTargets: [],
    alerted: new Set(),
    monitor: null,
    responders: [],
  };
  sim.fire.zone = zoneOf(sim);
  return `Simulated fire started at ${sim.fire.name}. Undetected until a Drone is within ${DETECT_RANGE_M} metres.`;
}

/** Teams ordered by how close their pad is to a point. */
const nearestTeams = (sim, at) => [...sim.world.pads].sort((a, b) => hdist(a.at, at) - hdist(b.at, at)).map((p) => p.team);
export const teamTitle = (id) => TEAMS.find((t) => t.id === id).name;
export const spoken = (droneId) => `${TEAMS.find((t) => t.name[0] === droneId[0]).name} ${droneId.slice(1)}`;

/**
 * The automatic response to a detected fire: the nearest team's four suppression Drones suppress and its two alert
 * Drones alert; the next team's alert Drones alert the far side; the detecting Drone (or the nearest free one) monitors.
 */
export function respond(sim) {
  const f = sim.fire;
  if (!f) return null;
  const [first, second] = nearestTeams(sim, f.at);
  const suppressors = teamDrones(sim, first).filter((d) => d.role === 'suppress');
  const alerters = [...teamDrones(sim, first), ...teamDrones(sim, second)].filter((d) => d.role === 'alert');
  suppressors.forEach((d) => (d.task = { kind: 'suppress' }));
  sendAlerts(sim, alerters);
  const busy = new Set([...suppressors, ...alerters]);
  const free = sim.drones.filter((d) => !busy.has(d) && airborne(d));
  const watcher = free.find((d) => d.id === f.detectedBy) ?? free.sort((a, b) => hdist(a.pos, f.at) - hdist(b.pos, f.at))[0];
  if (watcher) {
    watcher.task = { kind: 'orbit', at: f.at, radius: 70, monitor: true };
    f.monitor = watcher.id;
  }
  f.responders = [first, second];
  return `${teamTitle(first)} is suppressing with four Drones; ${teamTitle(first)} and ${teamTitle(second)} are alerting ${f.alertTargets.length} buildings${watcher ? `; ${spoken(watcher.id)} is monitoring` : ''}.`;
}

function sendAlerts(sim, drones) {
  const f = sim.fire;
  const named = f.zone.filter((i) => i !== f.building && sim.world.buildings[i].name);
  const pool = (named.length ? named : f.zone).slice(0, Math.max(1, drones.length));
  f.alertTargets = pool;
  drones.forEach((d, k) => (d.task = { kind: 'alert', building: pool[k % pool.length] }));
}

// ---------- Earthquake (SIM campus effects; the trigger can be the LIVE phone accelerometer) ----------

const jitter = (i) => {
  const x = Math.sin(i * 12.9898) * 43758.5453;
  return x - Math.floor(x);
};

/** Starts an earthquake felt across campus. `live` marks a trigger measured by the phone's accelerometer. */
export function quake(sim, at, peakG, { name = null, live = false } = {}) {
  const pga = new Map();
  for (const b of sim.world.buildings) {
    const g = peakG * Math.exp(-hdist(b.c, at) / 900) * (0.75 + 0.5 * jitter(b.i));
    if (g >= 0.05) pga.set(b.i, g);
  }
  const affected = [...pga.entries()]
    .filter(([i, g]) => g >= 0.1 && sim.world.buildings[i].name)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 12)
    .map(([i]) => i);
  sim.quake = { at: [...at], name, peakG, live, t0: sim.t, detected: false, status: 'shaking', pga, affected, inspected: new Set(), detectedBy: 0 };
  return `${live ? 'Shaking measured by the phone accelerometer. ' : ''}Simulated earthquake${name ? ` near ${name}` : ''}, peak ${peakG.toFixed(2)} g.`;
}

/** Ground acceleration in g at point p now (SIM), for sensor node i. */
export function groundAccel(sim, p, i = 0) {
  const base = 0.002 + 0.001 * jitter(i + sim.t);
  const q = sim.quake;
  if (!q) return base;
  const d = hdist(p, q.at);
  const tt = sim.t - q.t0 - d / QUAKE_SPEED;
  if (tt < 0 || tt > SHAKE_S * 2) return base;
  return base + q.peakG * Math.exp(-d / 900) * Math.exp(-tt / 5) * (0.6 + 0.4 * Math.abs(Math.sin(tt * 9 + i)));
}

/** Automatic quake response: suppression Drones inspect the hardest-hit buildings, alert Drones broadcast evacuation. */
export function respondQuake(sim) {
  const q = sim.quake;
  const free = sim.drones.filter((d) => d.role === 'suppress');
  for (const i of q.affected) {
    const b = sim.world.buildings[i];
    free.sort((a, c) => hdist(a.pos, b.c) - hdist(c.pos, b.c));
    const d = free.shift();
    if (d) d.task = { kind: 'inspect', building: i };
  }
  const alerters = sim.drones.filter((d) => d.role === 'alert');
  alerters.forEach((d, k) => q.affected.length && (d.task = { kind: 'alert', building: q.affected[k % q.affected.length] }));
  return `${Math.min(q.affected.length, 20)} Drones are inspecting the hardest-hit buildings; ${alerters.length} alert Drones are broadcasting evacuation.`;
}

/** Advances the sim by `realDt` seconds of wall time; returns events for the HUD and voice. */
export function step(sim, realDt) {
  const events = [];
  let left = Math.min(realDt, 0.25) * TIME_SCALE;
  while (left > 1e-6) {
    const dt = Math.min(0.1, left);
    left -= dt;
    sim.t += dt;
    for (const d of sim.drones) fly(sim, d, dt);
    stepQuake(sim, events);
    const f = sim.fire;
    if (!f || f.status === 'out') continue;

    if (!f.detected) {
      const spotter = sim.drones.find((d) => d.pos[2] > 10 && hdist(d.pos, f.at) <= DETECT_RANGE_M);
      if (spotter) {
        f.detected = true;
        f.detectedBy = spotter.id;
        f.detectedAfter = sim.t - f.t0;
        let text = `Fire detected at ${f.name} by ${spoken(spotter.id)}.`;
        if (sim.autoResponse) text += ` ${respond(sim)}`;
        events.push({ kind: 'detected', text });
      }
    }

    const onStation = sim.drones.filter((d) => d.task.kind === 'suppress' && hdist(d.pos, f.at) < 40 && airborne(d)).length;
    f.onStation = onStation;
    f.intensity = Math.min(1, Math.max(0, f.intensity + (GROWTH * (1 - f.intensity) - SUPPRESS * onStation) * dt));
    f.radius = Math.min(90, Math.max(6, f.radius + (f.intensity > 0.35 ? 0.35 : -0.8) * dt));
    const zone = zoneOf(sim);
    if (zone.length !== f.zone.length) f.zone = zone;

    for (const d of sim.drones) {
      if (d.task.kind !== 'alert' || f.alerted.has(d.task.building)) continue;
      const b = sim.world.buildings[d.task.building];
      if (hdist(d.pos, b.c) < 25) {
        f.alerted.add(b.i);
        if (f.alertTargets.length && f.alertTargets.every((i) => f.alerted.has(i))) events.push({ kind: 'alerted', text: `${f.alerted.size} ${f.alerted.size === 1 ? 'building' : 'buildings'} near ${f.name} alerted.` });
      }
    }

    if (f.status === 'burning' && f.detected && f.intensity < 0.35) {
      f.status = 'contained';
      events.push({ kind: 'contained', text: `Fire at ${f.name} is contained.` });
    }
    if (f.intensity <= 0.02) {
      f.status = 'out';
      f.outAfter = sim.t - f.t0;
      let text = `Fire at ${f.name} is out.`;
      if (sim.autoResponse) {
        for (const d of sim.drones) {
          if (d.task.kind === 'suppress' || d.task.kind === 'alert') d.task = { kind: 'return' };
          if (d.task.monitor) d.task = { kind: 'patrol' };
        }
        text += ' Responders are returning to their pads.';
      }
      events.push({ kind: 'out', text });
    }
  }
  return events;
}

function stepQuake(sim, events) {
  const q = sim.quake;
  if (!q || q.status === 'clear') return;
  if (!q.detected) {
    const shaking = sim.world.sensors.filter((b, i) => groundAccel(sim, b.c, i) > QUAKE_TRIGGER_G).length;
    if (shaking >= 3) {
      q.detected = true;
      q.detectedBy = shaking;
      q.status = 'surveying';
      let text = `Earthquake detected by ${shaking} campus accelerometers. ${q.affected.length} buildings felt over 0.1 g.`;
      if (sim.autoResponse) text += ` ${respondQuake(sim)}`;
      events.push({ kind: 'quake', text });
    }
    return;
  }
  for (const d of sim.drones) {
    if (d.task.kind !== 'inspect') continue;
    const b = sim.world.buildings[d.task.building];
    if (hdist(d.pos, b.c) > 20 || Math.abs(d.pos[2] - (b.h + 20)) > 6) continue;
    q.inspected.add(b.i);
    const taken = new Set(sim.drones.filter((x) => x.task.kind === 'inspect').map((x) => x.task.building));
    const next = q.affected.find((i) => !q.inspected.has(i) && !taken.has(i));
    d.task = next != null ? { kind: 'inspect', building: next } : { kind: 'return' };
  }
  if (q.affected.every((i) => q.inspected.has(i))) {
    q.status = 'clear';
    for (const d of sim.drones) if (d.task.kind === 'inspect' || (d.task.kind === 'alert' && !(sim.fire && sim.fire.status !== 'out'))) d.task = { kind: 'return' };
    events.push({ kind: 'surveyed', text: `Survey complete: ${q.inspected.size} buildings inspected from the air. Drones returning to their pads.` });
  }
}

/** One telemetry reading for a Drone (SIM): thermal camera, battery, radio link to its pad. */
export function telemetry(sim, d) {
  const f = sim.fire && sim.fire.status !== 'out' ? sim.fire : null;
  const fd = f ? Math.hypot(d.pos[0] - f.at[0], d.pos[1] - f.at[1], d.pos[2] - f.roof) : Infinity;
  const pd = Math.hypot(d.pos[0] - d.pad[0], d.pos[1] - d.pad[1], d.pos[2]);
  return {
    speed: Math.hypot(d.vel[0], d.vel[1], d.vel[2]),
    thermal: 21 + (f ? 380 * f.intensity * Math.exp(-fd / 60) : 0),
    dbm: -38 - 20 * Math.log10(Math.max(10, pd) / 10),
  };
}

// ---------- Commands (fixed grammar: team + verb + building) ----------

const TEAM_WORDS = {
  alpha: /\b(alpha|alfa|alpa)\b/,
  bravo: /\b(bravo|brava|bravos)\b/,
  charlie: /\b(charlie|charley|charly|charli)\b/,
  delta: /\b(delta|delt)\b/,
  echo: /\b(echo|ecco|eko)\b/,
};
const ALL = /\b(all teams|all units|all drones|all squads|everyone|everybody)\b/;
const VERBS = [
  ['suppress', /\b(suppress|extinguish|put out|put it out|fight|douse|contain)\b/],
  ['alert', /\b(alert|warn|evacuate|notify)\b/],
  ['monitor', /\b(monitor|orbit|watch|circle|overwatch|survey|observe)\b/],
  ['patrol', /\b(patrol|scout|search|sweep)/],
  ['return', /\b(return|come back|come home|go home|head home|land|rtb|base|pad|recall)\b/],
  ['hold', /\b(hold|stop|hover|wait|freeze|halt|stay)\b/],
  ['goto', /\b(go|fly|move|head|send|proceed|dispatch|launch|deploy)\b/],
];

/** Hands-free speech counts only when it addresses Aether. Returns the words after the wake word, or null. Scribe often writes "Ether". */
export function addressed(text) {
  const m = /\b(?:hey |ok |okay )?(?:aether|ether)\b[\s,.:!?-]*/i.exec(text);
  return m ? text.slice(m.index + m[0].length).trim() : null;
}

/** Parses one transcript. Returns {cmd} or {error, unheard}; `unheard` means nothing command-like was said. */
export function parse(sim, text) {
  const t = norm(text);
  if (/^ reset\b| reset (the )?(sim|simulation|campus)\b/.test(t)) return { cmd: { kind: 'reset' } };
  if (/\b(abort|all stop|stop all|emergency stop|stop everything)\b/.test(t)) return { cmd: { kind: 'abort' } };
  const teams = ALL.test(t) ? TEAMS.map((x) => x.id) : TEAMS.filter((x) => TEAM_WORDS[x.id].test(t)).map((x) => x.id);
  // "random", "somewhere": the event starts at a named building chosen at random, so the operator has to find it.
  const anywhere = /\b(random|randomly|somewhere|anywhere|surprise)\b/.test(t);
  const randomBuilding = () => sim.world.places[Math.floor(Math.random() * sim.world.places.length)].b;
  // A sentence that names a team is an order about the quake ("Delta, go to the earthquake"), not a new one.
  if (/\b(earthquake|quake|tremor|seismic)\b/.test(t) && !/\bstatus\b/.test(t) && !teams.length) {
    const b = anywhere ? randomBuilding() : findBuilding(sim.world, text);
    return { cmd: { kind: 'quake', building: b } };
  }
  if (/\b(simulate|start|ignite|light|set)\b.*\bfire\b|\b(test|practice) fire\b|\bfire drill\b|\b(random|surprise) fire\b/.test(t) && !teams.length) {
    const b = anywhere ? randomBuilding() : findBuilding(sim.world, text) ?? sim.world.places.find((p) => p.b.name === 'Capen Hall')?.b ?? sim.world.places[0].b;
    return { cmd: { kind: 'ignite', building: b } };
  }
  const verb = VERBS.find(([, re]) => re.test(t))?.[0];
  if (/\b(status|report|sitrep|situation|update)\b/.test(t) && (!verb || !teams.length)) return { cmd: { kind: 'status' } };
  const fire = sim.fire && sim.fire.status !== 'out' ? sim.fire : null;
  const building = findBuilding(sim.world, text);
  const shaken = sim.quake && sim.quake.status !== 'clear' ? sim.quake : null;
  const quakeTarget = shaken && /\b(earthquake|quake|epicent(er|re)|tremor|damage|shaking)\b/.test(t)
    ? { name: shaken.name ? `the earthquake near ${shaken.name}` : 'the earthquake epicentre', at: shaken.at }
    : null;
  const target = /\bfire\b/.test(t) && fire ? { name: `the fire at ${fire.name}`, at: fire.at, fire: true } : building ? { name: building.name, at: building.c, building } : quakeTarget;
  const v = verb ?? (target && teams.length ? 'goto' : null);
  if (!v) return { error: 'No command heard. Try "Alpha, patrol the campus".', unheard: !teams.length };
  if (!teams.length) return { error: 'Which team? Say Alpha, Bravo, Charlie, Delta or Echo.' };
  if (v === 'suppress' && !fire) return { error: 'There is no active fire to suppress.' };
  if (v === 'alert' && !fire && !target) return { error: 'Alert where? Name a building, or wait for a fire.' };
  if ((v === 'goto' || v === 'monitor') && !target && !fire) return { error: 'Where to? Name a building, like Capen Hall.' };
  return { cmd: { kind: 'team', teams, verb: v, target: target ?? (fire ? { name: `the fire at ${fire.name}`, at: fire.at, fire: true } : null) } };
}

const teamName = (ids) => (ids.length === TEAMS.length ? 'All teams' : ids.map(teamTitle).join(' and '));

/** Applies a parsed command; returns the sentence Aether speaks back. */
export function apply(sim, cmd) {
  switch (cmd.kind) {
    case 'reset':
      resetSim(sim);
      return 'Simulation reset. All Drones are on their pads.';
    case 'abort':
      for (const d of sim.drones) d.task = airborne(d) ? { kind: 'hold', p: [...d.pos] } : { kind: 'idle' };
      return 'Abort. All Drones holding position.';
    case 'ignite':
      return ignite(sim, cmd.building);
    case 'quake': {
      // Default epicentre: 800 m south-west of the campus centre, off campus.
      const at = cmd.building ? cmd.building.c : [sim.world.centre[0] - 560, sim.world.centre[1] - 560];
      return quake(sim, at, 0.35, { name: cmd.building?.name ?? null });
    }
    case 'status':
      return status(sim);
  }
  const who = teamName(cmd.teams);
  const drones = cmd.teams.flatMap((id) => teamDrones(sim, id));
  const tgt = cmd.target;
  switch (cmd.verb) {
    case 'patrol':
      drones.forEach((d) => (d.task = { kind: 'patrol' }));
      return `${who}, scouting the campus.`;
    case 'return':
      drones.forEach((d) => (d.task = { kind: 'return' }));
      return `${who}, returning to ${cmd.teams.length > 1 ? 'their pads' : 'the pad'}.`;
    case 'hold':
      drones.forEach((d) => (d.task = airborne(d) ? { kind: 'hold', p: [...d.pos] } : { kind: 'idle' }));
      return `${who}, holding position.`;
    case 'goto':
      drones.forEach((d) => (d.task = { kind: 'goto', at: tgt.at }));
      return `${who}, en route to ${tgt.name}.`;
    case 'monitor':
      drones.forEach((d) => (d.task = { kind: 'orbit', at: tgt.at, radius: tgt.fire ? 70 : 60 }));
      return `${who}, monitoring ${tgt.name}.`;
    case 'suppress': {
      const s = drones.filter((d) => d.role === 'suppress');
      s.forEach((d) => (d.task = { kind: 'suppress' }));
      return `${who}, ${s.length} suppression Drones moving to the fire at ${sim.fire.name}.`;
    }
    case 'alert': {
      const a = drones.filter((d) => d.role === 'alert');
      if (sim.fire && sim.fire.status !== 'out' && (!tgt || tgt.fire)) {
        sendAlerts(sim, a);
        return `${who}, ${a.length} alert Drones warning ${sim.fire.alertTargets.length} buildings near ${sim.fire.name}.`;
      }
      a.forEach((d) => (d.task = { kind: 'alert', building: tgt.building.i }));
      return `${who}, alert Drones warning ${tgt.name}.`;
    }
  }
  return 'Command not understood.';
}

const TASK_WORDS = { inspect: 'Inspecting buildings', idle: 'On the pad', return: 'Returning to pad', hold: 'Holding', goto: 'En route', patrol: 'Scouting', orbit: 'Monitoring', suppress: 'Suppressing fire', alert: 'Alerting buildings' };

/** One line per team for the HUD: the main task, plus what the rest are doing when the team is split. */
export function teamStatus(sim, id) {
  const ds = teamDrones(sim, id);
  const counts = {};
  for (const d of ds) {
    const k = d.task.monitor ? 'monitor' : d.task.kind;
    counts[k] = (counts[k] ?? 0) + 1;
  }
  const ranked = Object.entries(counts).sort((a, b) => b[1] - a[1]);
  const word = (k, n) => {
    if (k === 'monitor') return `${n} monitoring fire`;
    if (k === 'goto') {
      const lead = ds.find((d) => d.task.kind === 'goto');
      const b = buildingAt(sim.world, lead.task.at);
      return `${n} ${hdist(lead.pos, lead.task.at) < 50 ? 'over' : 'en route to'} ${b?.name ?? 'target'}`;
    }
    return `${n} ${TASK_WORDS[k].toLowerCase()}`;
  };
  const words = ranked.length === 1 ? (ranked[0][0] === 'goto' ? word('goto', 'All').replace(/^All /, '').replace(/^./, (c) => c.toUpperCase()) : ranked[0][0] === 'monitor' ? 'Monitoring fire' : TASK_WORDS[ranked[0][0]]) : ranked.map(([k, n]) => word(k, n)).join(', ');
  return { words, airborne: ds.filter(airborne).length, total: ds.length, pad: padOf(sim, id).near };
}

export function status(sim) {
  const parts = TEAMS.map((t) => {
    const s = teamStatus(sim, t.id);
    return `${t.name}: ${s.words.toLowerCase()}.`;
  });
  const f = sim.fire;
  if (!f && !sim.quake) parts.push('No active incidents.');
  else if (!f) {
    /* fire line omitted */
  }
  else if (f.status === 'out') parts.push(`The fire at ${f.name} is out.`);
  else parts.push(`Fire at ${f.name}: ${f.detected ? f.status : 'not yet detected'}, about ${Math.round(f.radius * 2)} metres across.`);
  const q = sim.quake;
  if (q) parts.push(q.status === 'clear' ? `Earthquake survey complete, ${q.inspected.size} buildings inspected.` : `Earthquake: ${q.detected ? `${q.inspected.size} of ${q.affected.length} buildings inspected` : 'shaking, not yet detected'}.`);
  return parts.join(' ');
}
