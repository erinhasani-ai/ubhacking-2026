// Clean architectural layer from the raw scan: walls as flat vertical planes along smoothed outlines, the floor as one
// flat surface. The raw LiDAR mesh is noisy; this layer is what the eye reads, and the point cloud stays the truth.
// Input is the scan in Three.js coordinates (x = E, y = height above the floor, z = −N).

const CELL_M = 0.1;
/** Wall band above the floor: above desks and chairs, below door heads, so furniture does not become walls. */
const WALL_LO_M = 1.1;
const WALL_HI_M = 2.1;
const FLOOR_BAND_M = 0.15;

export interface Plan {
  /** Wall quads as triangles (x, y, z), and each vertex's height 0..1 for shading. */
  walls: Float32Array;
  wallHeight: Float32Array;
  /** Line segments: wall tops and feet, and the floor outline. */
  edges: Float32Array;
  /** Floor mask texture source and the world rectangle it covers. */
  floor: { canvas: HTMLCanvasElement; x0: number; z0: number; w: number; h: number };
}

export function buildPlan(pos: Float32Array, index: Uint32Array, wallTop: number): Plan {
  let x0 = Infinity, x1 = -Infinity, z0 = Infinity, z1 = -Infinity;
  for (let i = 0; i < pos.length; i += 3) {
    x0 = Math.min(x0, pos[i]);
    x1 = Math.max(x1, pos[i]);
    z0 = Math.min(z0, pos[i + 2]);
    z1 = Math.max(z1, pos[i + 2]);
  }
  x0 -= 2 * CELL_M;
  z0 -= 2 * CELL_M;
  const W = Math.ceil((x1 - x0) / CELL_M) + 4, H = Math.ceil((z1 - z0) / CELL_M) + 4;
  const wall = new Uint16Array(W * H), floor = new Uint16Array(W * H);
  const mark = (x: number, y: number, z: number) => {
    const c = Math.floor((x - x0) / CELL_M) + W * Math.floor((z - z0) / CELL_M);
    if (y > WALL_LO_M && y < WALL_HI_M) wall[c] = Math.min(65535, wall[c] + 1);
    else if (Math.abs(y) < FLOOR_BAND_M) floor[c] = Math.min(65535, floor[c] + 1);
  };
  // Sample each triangle's surface every ~7 cm, so large flat triangles count as much as small noisy ones.
  for (let t = 0; t < index.length; t += 3) {
    const a = 3 * index[t], b = 3 * index[t + 1], c = 3 * index[t + 2];
    const ux = pos[b] - pos[a], uy = pos[b + 1] - pos[a + 1], uz = pos[b + 2] - pos[a + 2];
    const vx = pos[c] - pos[a], vy = pos[c + 1] - pos[a + 1], vz = pos[c + 2] - pos[a + 2];
    const longest = Math.sqrt(Math.max(ux * ux + uy * uy + uz * uz, vx * vx + vy * vy + vz * vz));
    const steps = Math.min(40, Math.max(1, Math.ceil(longest / 0.07)));
    for (let i = 0; i <= steps; i++) {
      for (let j = 0; j <= steps - i; j++) {
        const s = i / steps, r = j / steps;
        mark(pos[a] + ux * s + vx * r, pos[a + 1] + uy * s + vy * r, pos[a + 2] + uz * s + vz * r);
      }
    }
  }
  const wallField = blur(blur(despeckle(wall, W, H), W, H), W, H);
  const floorField = blur(blur(close(floor, W, H), W, H), W, H);

  const wallSegs = contour(wallField, W, H, 0.3, x0, z0);
  const floorSegs = contour(floorField, W, H, 0.45, x0, z0);

  const walls = new Float32Array((wallSegs.length / 4) * 18);
  const wallHeight = new Float32Array((wallSegs.length / 4) * 6);
  const edges = new Float32Array((wallSegs.length / 4) * 12 + (floorSegs.length / 4) * 6);
  let w = 0, hgt = 0, e = 0;
  for (let i = 0; i < wallSegs.length; i += 4) {
    const ax = wallSegs[i], az = wallSegs[i + 1], bx = wallSegs[i + 2], bz = wallSegs[i + 3];
    const quad = [
      [ax, 0, az, 0], [bx, 0, bz, 0], [bx, wallTop, bz, 1],
      [ax, 0, az, 0], [bx, wallTop, bz, 1], [ax, wallTop, az, 1],
    ];
    for (const [x, y, z, k] of quad) {
      walls[w++] = x;
      walls[w++] = y;
      walls[w++] = z;
      wallHeight[hgt++] = k;
    }
    edges.set([ax, wallTop, az, bx, wallTop, bz, ax, 0.02, az, bx, 0.02, bz], e);
    e += 12;
  }
  for (let i = 0; i < floorSegs.length; i += 4) {
    edges.set([floorSegs[i], 0.01, floorSegs[i + 1], floorSegs[i + 2], 0.01, floorSegs[i + 3]], e);
    e += 6;
  }

  const canvas = document.createElement('canvas');
  canvas.width = W;
  canvas.height = H;
  const g = canvas.getContext('2d')!;
  const img = g.createImageData(W, H);
  for (let c = 0; c < W * H; c++) {
    const v = Math.round(Math.min(1, floorField[c] / 0.9) * 255);
    img.data[4 * c] = img.data[4 * c + 1] = img.data[4 * c + 2] = v;
    img.data[4 * c + 3] = 255;
  }
  g.putImageData(img, 0, 0);
  return { walls, wallHeight, edges, floor: { canvas, x0, z0, w: W * CELL_M, h: H * CELL_M } };
}

/** Occupied cells with at least two occupied neighbours: drops single stray samples. */
function despeckle(m: Uint16Array, W: number, H: number): Float32Array {
  const out = new Float32Array(W * H);
  for (let y = 1; y < H - 1; y++)
    for (let x = 1; x < W - 1; x++) {
      const c = x + W * y;
      if (!m[c]) continue;
      let n = 0;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) n += m[c + dx + W * dy] ? 1 : 0;
      out[c] = n >= 3 ? 1 : 0;
    }
  return out;
}

/** Morphological close (dilate twice, erode twice): fills the scan's small floor holes. */
function close(m: Uint16Array, W: number, H: number): Float32Array {
  let a = new Float32Array(W * H);
  for (let c = 0; c < W * H; c++) a[c] = m[c] ? 1 : 0;
  const pass = (src: Float32Array, grow: boolean) => {
    const out = new Float32Array(W * H);
    for (let y = 1; y < H - 1; y++)
      for (let x = 1; x < W - 1; x++) {
        const c = x + W * y;
        let any = 0, all = 1;
        for (let dy = -1; dy <= 1; dy++)
          for (let dx = -1; dx <= 1; dx++) {
            const v = src[c + dx + W * dy];
            any = Math.max(any, v);
            all = Math.min(all, v);
          }
        out[c] = grow ? any : all;
      }
    return out;
  };
  a = pass(pass(a, true), true);
  return pass(pass(a, false), false);
}

function blur(m: Float32Array, W: number, H: number): Float32Array {
  const out = new Float32Array(W * H);
  for (let y = 1; y < H - 1; y++)
    for (let x = 1; x < W - 1; x++) {
      const c = x + W * y;
      out[c] = (m[c - W - 1] + m[c - W] + m[c - W + 1] + m[c - 1] + m[c] + m[c + 1] + m[c + W - 1] + m[c + W] + m[c + W + 1]) / 9;
    }
  return out;
}

// Marching squares: segments (ax, az, bx, bz) where the field crosses `iso`, between cell centres.
// Corners 0..3 = (x, y), (x+1, y), (x+1, y+1), (x, y+1); edges 0..3 = 0-1, 1-2, 3-2, 0-3.
const CASES: number[][] = [[], [3, 0], [0, 1], [3, 1], [1, 2], [3, 0, 1, 2], [0, 2], [3, 2], [2, 3], [0, 2], [0, 1, 2, 3], [1, 2], [1, 3], [0, 1], [3, 0], []];
const EDGES = [
  [0, 1, 0, 0, 1, 0],
  [1, 2, 1, 0, 0, 1],
  [3, 2, 0, 1, 1, 0],
  [0, 3, 0, 0, 0, 1],
];
function contour(f: Float32Array, W: number, H: number, iso: number, x0: number, z0: number): number[] {
  const out: number[] = [];
  const v = [0, 0, 0, 0];
  for (let y = 0; y < H - 1; y++)
    for (let x = 0; x < W - 1; x++) {
      v[0] = f[x + W * y];
      v[1] = f[x + 1 + W * y];
      v[2] = f[x + 1 + W * (y + 1)];
      v[3] = f[x + W * (y + 1)];
      const segs = CASES[(v[0] > iso ? 1 : 0) | (v[1] > iso ? 2 : 0) | (v[2] > iso ? 4 : 0) | (v[3] > iso ? 8 : 0)];
      for (let s = 0; s < segs.length; s++) {
        const [i, j, ox, oy, dx, dy] = EDGES[segs[s]];
        const t = (iso - v[i]) / (v[j] - v[i] || 1e-6);
        out.push(x0 + (x + ox + dx * t + 0.5) * CELL_M, z0 + (y + oy + dy * t + 0.5) * CELL_M);
      }
    }
  return out;
}
