// Phone sensor test page: the motion classifier on each simulated feed. Run: node tests/phone_classify.mjs
import assert from 'node:assert/strict';
import { classify, simulate } from '../src/web/src/phone/classify.js';

let seed = 7;
const rand = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
const feed = (kind, from, to) => {
  const out = [];
  for (let t = from; t < to; t += 0.01) out.push({ t, m: simulate(kind, t, rand) });
  return out;
};

assert.equal(classify(feed('still', 0, 1).slice(0, 10)).kind, 'waiting');
for (const [kind, from, to] of [['still', 0, 6], ['quake', 0, 6], ['jump', 0, 3], ['handling', 4, 10]]) {
  const v = classify(feed(kind, from, to));
  console.log(kind.padEnd(9), '->', v.kind.padEnd(9), `rms ${v.rms.toFixed(2)} peak ${v.peak.toFixed(2)} freq ${v.freq.toFixed(1)} Hz active ${(v.active * 100).toFixed(0)}%`);
  assert.equal(v.kind, kind, `${kind} feed classified as ${v.kind}`);
}
// A quiet window after the jump has passed is still again.
assert.equal(classify(feed('jump', 1.8, 4.9)).kind, 'still');
console.log('phone classifier: all scenarios pass');
