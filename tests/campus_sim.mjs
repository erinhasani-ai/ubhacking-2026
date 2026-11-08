// T32/T33 check: campus command grammar and the simulated fire response, headless. Run: node tests/campus_sim.mjs
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { addressed, apply, buildWorld, createSim, findBuilding, parse, status, step, TEAMS, TIME_SCALE } from '../src/web/src/campus/sim.js';

const data = JSON.parse(readFileSync(new URL('../src/web/public/campus/ub-north.json', import.meta.url), 'utf8'));
const world = buildWorld(data);
const sim = createSim(world);
assert.equal(sim.drones.length, 30);
for (const t of TEAMS) {
  const ds = sim.drones.filter((d) => d.team === t.id);
  assert.equal(ds.filter((d) => d.role === 'alert').length, 2, `${t.name} has 2 alert Drones`);
  assert.equal(ds.filter((d) => d.role === 'suppress').length, 4, `${t.name} has 4 suppression Drones`);
}
const padGap = Math.min(...world.pads.flatMap((a, i) => world.pads.slice(i + 1).map((b) => Math.hypot(a.at[0] - b.at[0], a.at[1] - b.at[1]))));
assert.ok(padGap > 250, `pads are spread across campus (closest pair ${padGap.toFixed(0)} m)`);
assert.ok(world.cruise > 35, 'cruise above the tallest roof');

// Grammar: team + verb + building, and the errors a judge might trigger.
assert.equal(findBuilding(world, 'go to capen').name, 'Capen Hall');
assert.equal(findBuilding(world, 'the library').name, 'Lockwood Memorial Library');
assert.equal(findBuilding(world, "O'Brian Hall").name, "O'Brian Hall");
const cmd = (t) => parse(sim, t);
assert.deepEqual(cmd('Alpha, patrol the campus.').cmd.verb, 'patrol');
assert.deepEqual(cmd('Bravo and Charlie go to Davis Hall').cmd.teams, ['bravo', 'charlie']);
assert.equal(cmd('All teams, return to base').cmd.teams.length, 5);
assert.ok(cmd('so what did you have for lunch').unheard, 'chatter is ignored in hands-free mode');
assert.deepEqual(cmd('Delta and Echo, scout the campus').cmd, { kind: 'team', teams: ['delta', 'echo'], verb: 'patrol', target: null });
assert.match(cmd('go to Capen Hall').error, /Which team/);
assert.match(cmd('Bravo, put out the fire').error, /no active fire/);
assert.equal(cmd('Abort').cmd.kind, 'abort');
assert.equal(cmd('status report').cmd.kind, 'status');

// Scenario: patrol up, fire at Capen, detection, automatic response, fire out.
assert.match(apply(sim, cmd('All teams, patrol the campus').cmd), /scouting/);
const ig = cmd('Simulate a fire at Capen Hall').cmd;
assert.equal(ig.building.name, 'Capen Hall');
apply(sim, ig);
const seen = {};
const frame = 1 / 30;
for (let i = 0; i < 30 * 240 && !seen.out; i++) for (const e of step(sim, frame)) seen[e.kind] ??= { t: sim.t, text: e.text };
for (const k of ['detected', 'alerted', 'contained', 'out']) assert.ok(seen[k], `event ${k} happened`);
assert.ok(seen.detected.t < seen.contained.t && seen.contained.t < seen.out.t);
const realS = (seen.out.t - seen.detected.t) / TIME_SCALE;
assert.ok(realS > 10 && realS < 90, `detection to out takes ${realS.toFixed(0)} s on screen`);
assert.ok(sim.fire.alerted.size >= 1);
const [first] = sim.fire.responders;
assert.equal(first, 'delta', 'the pad nearest Capen Hall (Natural Sciences Complex) responds');
assert.match(status(sim), /out/);

console.log(`campus ok: ${world.buildings.length} buildings, pads near ${world.pads.map((p) => p.near).join(", ")}, cruise ${world.cruise} m`);
for (const [k, v] of Object.entries(seen)) console.log(`  ${k} at sim ${v.t.toFixed(0)} s (${(v.t / TIME_SCALE).toFixed(0)} s real): ${v.text}`);

// Earthquake: simulated accelerometer nodes trigger it, Drones inspect the hardest-hit buildings.
apply(sim, cmd('Reset').cmd);
assert.equal(cmd('Simulate an earthquake').cmd.kind, 'quake');
apply(sim, cmd('All teams, patrol the campus').cmd);
apply(sim, cmd('Simulate an earthquake').cmd);
const q = {};
for (let i = 0; i < 30 * 300 && !q.surveyed; i++) for (const e of step(sim, frame)) q[e.kind] ??= { t: sim.t, text: e.text };
assert.ok(q.quake && q.surveyed, 'quake detected and survey completed');
assert.ok(sim.quake.affected.length >= 3 && sim.quake.inspected.size === sim.quake.affected.length);
console.log(`  quake detected at ${(q.quake.t / TIME_SCALE).toFixed(0)} s real: ${q.quake.text}`);
console.log(`  survey done at ${(q.surveyed.t / TIME_SCALE).toFixed(0)} s real: ${q.surveyed.text}`);

// Random events, and orders that send a team to an event instead of starting a new one.
{
  const s2 = createSim(world);
  const rf = parse(s2, 'Simulate a fire somewhere random');
  assert.equal(rf.cmd.kind, 'ignite');
  assert.ok(rf.cmd.building?.name, 'a random fire starts at a named building');
  const rq = parse(s2, 'Simulate a random earthquake');
  assert.equal(rq.cmd.kind, 'quake');
  assert.ok(rq.cmd.building?.name, 'a random quake is centred on a named building');
  apply(s2, rq.cmd);
  const go = parse(s2, 'Delta, go to the earthquake');
  assert.equal(go.cmd?.kind, 'team', 'an order about the quake does not start a new one');
  assert.deepEqual(go.cmd.teams, ['delta']);
  assert.deepEqual(go.cmd.target.at, s2.quake.at);
  apply(s2, rf.cmd);
  const out = parse(s2, 'Echo, put out the fire');
  assert.equal(out.cmd?.verb, 'suppress');
  console.log('random events and manual orders: ok');
}

// Hands-free speech acts only when it addresses Aether.
assert.equal(addressed('this simulation is so cool'), null);
assert.equal(addressed('reset the simulation'), null);
assert.equal(addressed('Aether, simulate a random earthquake'), 'simulate a random earthquake');
assert.equal(addressed('Ether all teams patrol the campus'), 'all teams patrol the campus');
assert.equal(addressed('hey Aether. Status report'), 'Status report');
assert.equal(addressed('Okay Aether'), '');
console.log('wake word: ok');
