import assert from 'node:assert/strict';
import { enhanceCloud } from '../src/web/src/enhance-cloud.js';

const grid = [];
for (let x = 0; x < 4; x++) for (let y = 0; y < 4; y++) grid.push(x * 0.04, y * 0.04, -1);
const input = new Float32Array([...grid, 0, 0, -1, 10, 10, -1, NaN, 0, 0, 0, 0, -4]);
const before = input.slice();
const result = enhanceCloud(input);
assert.deepEqual(input, before, 'input mutated');
assert.equal(result.retained, 16);
assert(result.added > 0, 'short gaps were not filled');
assert.deepEqual(result.points.slice(0, grid.length), new Float32Array(grid));
for (let i = 0; i < result.points.length; i += 3) {
  assert(result.points[i] >= 0 && result.points[i] <= 0.120001);
  assert(result.points[i + 1] >= 0 && result.points[i + 1] <= 0.120001);
  assert.equal(result.points[i + 2], -1);
}
assert.equal(enhanceCloud(input, { maxPoints: 18 }).points.length, 54);
assert.equal(enhanceCloud(input, { maxPoints: 8 }).points.length, 24);
assert.equal(enhanceCloud(new Float32Array([0, 0, 0, 1, 0, 0, 2, 0, 0])).points.length, 0, 'bridged distant islands');
assert.throws(() => enhanceCloud(new Float32Array([1])));
assert.throws(() => enhanceCloud(input, { maxPoints: 0 }));
console.log(`cloud_enhancement_ok retained=${result.retained} added=${result.added}`);
