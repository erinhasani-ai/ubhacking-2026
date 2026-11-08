// Visual resampling only. The engine's original occupancy grid remains the flight authority.
// Assumed display thresholds are recorded in SPEC; no added point is a new LiDAR measurement.
export function enhanceCloud(input, { ceiling = 2.7, maxPoints = 320_000 } = {}) {
  if (input.length % 3) throw new Error('Point cloud must contain NED triples');
  if (!Number.isInteger(maxPoints) || maxPoints < 1) throw new Error('Invalid point budget');
  const cellSize = 0.12, duplicateBin = 0.008;
  const key = (x, y, z, size) => `${Math.floor(x / size)},${Math.floor(y / size)},${Math.floor(z / size)}`;
  const bins = new Map(), unique = new Set(), offsets = [];
  for (let offset = 0; offset < input.length; offset += 3) {
    const [n, e, d] = input.subarray(offset, offset + 3);
    if (![n, e, d].every(Number.isFinite) || -d > ceiling) continue;
    const id = key(n, e, d, duplicateBin);
    if (unique.has(id)) continue;
    unique.add(id);
    offsets.push(offset);
    const cell = key(n, e, d, cellSize);
    const bucket = bins.get(cell);
    if (bucket) bucket.push(offset); else bins.set(cell, [offset]);
  }
  const neighbours = new Map();
  for (const offset of offsets) {
    const n = input[offset], e = input[offset + 1], d = input[offset + 2];
    const ix = Math.floor(n / cellSize), iy = Math.floor(e / cellSize), iz = Math.floor(d / cellSize);
    let first = -1, second = -1, firstD = Infinity, secondD = Infinity;
    for (let x = ix - 1; x <= ix + 1; x++) for (let y = iy - 1; y <= iy + 1; y++) for (let z = iz - 1; z <= iz + 1; z++) {
      const bucket = bins.get(`${x},${y},${z}`);
      if (!bucket) continue;
      for (const other of bucket) {
        if (other === offset) continue;
        const distance = (n - input[other]) ** 2 + (e - input[other + 1]) ** 2 + (d - input[other + 2]) ** 2;
        if (distance > cellSize ** 2) continue;
        if (distance < firstD) { second = first; secondD = firstD; first = other; firstD = distance; }
        else if (distance < secondD) { second = other; secondD = distance; }
      }
    }
    if (second >= 0) neighbours.set(offset, [[first, firstD], [second, secondD]]);
  }
  const retained = [...neighbours.keys()];
  const additions = [];
  for (const [offset, near] of neighbours) {
    for (const [other, distance] of near) {
      if (other < offset || !neighbours.has(other) || distance < 0.024 ** 2 || distance > 0.08 ** 2) continue;
      const n = (input[offset] + input[other]) / 2;
      const e = (input[offset + 1] + input[other + 1]) / 2;
      const d = (input[offset + 2] + input[other + 2]) / 2;
      const id = key(n, e, d, duplicateBin);
      if (unique.has(id)) continue;
      unique.add(id);
      additions.push(n, e, d);
    }
  }
  const originalCount = Math.min(retained.length, maxPoints);
  const addedCount = Math.min(additions.length / 3, maxPoints - originalCount);
  const output = new Float32Array((originalCount + addedCount) * 3);
  // Evenly sample each pool if capped, so the input's ordering does not crowd out the end of the room.
  for (let i = 0; i < originalCount; i++) {
    const at = retained[Math.floor(i * retained.length / originalCount)];
    output.set(input.subarray(at, at + 3), i * 3);
  }
  for (let i = 0; i < addedCount; i++) {
    const at = Math.floor(i * (additions.length / 3) / addedCount) * 3;
    output.set(additions.slice(at, at + 3), (originalCount + i) * 3);
  }
  return { points: output, received: input.length / 3, retained: originalCount, added: addedCount };
}
