//! Distance field and 3D path search over the Room Twin, in scan-frame metres (Y up). T13 converts to the Mission frame.
//! WebAssembly-safe: no file access, no threads.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::room_twin::OccupancyGrid;

/// Euclidean distance from every cell centre to the nearest occupied cell centre. Cells outside the grid count as
/// occupied, because the scan box ends at the walls. Layout matches the grid: x fastest, then y, then z.
#[derive(Debug, Clone)]
pub struct DistanceField {
    voxel: f64,
    origin: [f64; 3],
    dims: [usize; 3],
    dist: Vec<f64>,
}

impl DistanceField {
    /// The distance field of a Room Twin occupancy grid.
    pub fn from_grid(grid: &OccupancyGrid) -> DistanceField {
        DistanceField::from_occupancy(grid.dims(), grid.origin(), grid.voxel(), |c| grid.is_occupied(c))
    }

    /// Exact squared Euclidean transform, one axis at a time (Felzenszwalb and Huttenlocher's lower envelope of
    /// parabolas). `origin` is the low corner of cell [0, 0, 0], as in OccupancyGrid.
    pub fn from_occupancy(dims: [usize; 3], origin: [f64; 3], voxel: f64, occupied: impl Fn([usize; 3]) -> bool) -> DistanceField {
        let [nx, ny, nz] = dims;
        let mut sq = Vec::with_capacity(nx * ny * nz);
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    sq.push(if occupied([i, j, k]) { 0.0 } else { f64::INFINITY });
                }
            }
        }
        let mut line = Line::new(nx.max(ny).max(nz));
        let strides = [1, nx, nx * ny];
        for axis in 0..3 {
            let (n, stride) = (dims[axis], strides[axis]);
            // A line along this axis starts at every cell whose coordinate on this axis is 0.
            for start in (0..sq.len()).filter(|s| (s / stride) % n == 0) {
                line.transform(&mut sq, start, stride, n);
            }
        }
        let dist = sq.into_iter().map(|s| s.sqrt() * voxel).collect();
        DistanceField { voxel, origin, dims, dist }
    }

    pub fn dims(&self) -> [usize; 3] {
        self.dims
    }

    pub fn origin(&self) -> [f64; 3] {
        self.origin
    }

    pub fn voxel(&self) -> f64 {
        self.voxel
    }

    /// Distance in metres at a cell. Panics outside the grid, like OccupancyGrid::is_occupied.
    pub fn at(&self, idx: [usize; 3]) -> f64 {
        assert!((0..3).all(|a| idx[a] < self.dims[a]), "cell {idx:?} outside {:?}", self.dims);
        self.dist[self.flat(idx)]
    }

    /// The cell whose centre is nearest to `p`, or None outside the grid.
    pub fn cell_of(&self, p: [f64; 3]) -> Option<[usize; 3]> {
        let mut idx = [0; 3];
        for a in 0..3 {
            let c = ((p[a] - self.origin[a]) / self.voxel).floor();
            if !(c >= 0.0 && c < self.dims[a] as f64) {
                return None;
            }
            idx[a] = c as usize;
        }
        Some(idx)
    }

    /// Distance at the cell nearest to `p`; 0 outside the grid, where everything counts as occupied.
    pub fn at_point(&self, p: [f64; 3]) -> f64 {
        self.cell_of(p).map_or(0.0, |c| self.dist[self.flat(c)])
    }

    pub fn centre(&self, idx: [usize; 3]) -> [f64; 3] {
        [0, 1, 2].map(|a| self.origin[a] + (idx[a] as f64 + 0.5) * self.voxel)
    }

    fn flat(&self, [i, j, k]: [usize; 3]) -> usize {
        i + self.dims[0] * (j + self.dims[1] * k)
    }

    fn unflat(&self, cell: usize) -> [usize; 3] {
        let [nx, ny, _] = self.dims;
        [cell % nx, (cell / nx) % ny, cell / (nx * ny)]
    }
}

/// Scratch buffers for the 1D transform, reused for every line.
struct Line {
    f: Vec<f64>,
    v: Vec<usize>,
    z: Vec<f64>,
}

impl Line {
    fn new(longest: usize) -> Line {
        Line { f: vec![0.0; longest + 2], v: vec![0; longest + 2], z: vec![0.0; longest + 3] }
    }

    /// Squared distance along one line of `n` cells, in place. Positions 0 and n + 1 are the outside cells at either
    /// end, occupied; a free cell (infinite) is not a parabola, so the envelope never mixes infinities.
    fn transform(&mut self, sq: &mut [f64], start: usize, stride: usize, n: usize) {
        let Line { f, v, z } = self;
        f[0] = 0.0;
        f[n + 1] = 0.0;
        for q in 0..n {
            f[q + 1] = sq[start + q * stride];
        }
        let mut k = 0;
        v[0] = 0;
        z[0] = f64::NEG_INFINITY;
        z[1] = f64::INFINITY;
        for q in 1..n + 2 {
            if f[q].is_infinite() {
                continue;
            }
            let s = loop {
                let p = v[k];
                let s = ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2 * (q - p)) as f64;
                // z[0] is -inf, so k never drops below 0.
                if s <= z[k] {
                    k -= 1;
                } else {
                    break s;
                }
            };
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
        }
        k = 0;
        for q in 1..=n {
            while z[k + 1] < q as f64 {
                k += 1;
            }
            let p = v[k];
            sq[start + (q - 1) * stride] = (q.abs_diff(p) * q.abs_diff(p)) as f64 + f[p];
        }
    }
}

/// Why no path was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanError {
    /// The start is outside the grid or closer than the clearance to an occupied cell.
    StartBlocked,
    /// The goal is outside the grid or closer than the clearance to an occupied cell.
    GoalBlocked,
    /// No chain of traversable cells joins the start to the goal.
    Unreachable,
}

/// A path from `start` to `goal` that keeps `clearance_m` from every occupied cell centre: A* over the 26 neighbours
/// of each cell, then line-of-sight smoothing. It begins at `start` and ends at `goal` exactly; every point on it lies
/// in a cell whose distance is at least the clearance.
pub fn plan(field: &DistanceField, start: [f64; 3], goal: [f64; 3], clearance_m: f64) -> Result<Vec<[f64; 3]>, PlanError> {
    let free: Vec<bool> = field.dist.iter().map(|&d| d >= clearance_m).collect();
    let traversable = |p: [f64; 3]| field.cell_of(p).filter(|&c| free[field.flat(c)]);
    let s = traversable(start).ok_or(PlanError::StartBlocked)?;
    let g = traversable(goal).ok_or(PlanError::GoalBlocked)?;
    let cells = search(field, &free, s, g).ok_or(PlanError::Unreachable)?;
    let mut raw = Vec::with_capacity(cells.len() + 2);
    raw.push(start);
    raw.extend(cells.iter().map(|&c| field.centre(c)));
    raw.push(goal);
    Ok(smooth(field, &free, &raw))
}

/// A cell on the A* frontier, ordered so the BinaryHeap pops the lowest estimated total cost first.
#[derive(Clone, Copy)]
struct Open {
    f: f64,
    cell: usize,
}

impl PartialEq for Open {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Open {}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Open {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.total_cmp(&self.f)
    }
}

/// One of the 26 moves: its offset, its length in voxels, and the flat offsets of the other cells in the block it spans.
#[derive(Clone, Copy)]
struct Step {
    d: [i64; 3],
    len: f64,
    to: isize,
    corners: [isize; 6],
    corner_count: usize,
}

/// A* in voxel units with the Euclidean heuristic, which never overestimates. A diagonal step is allowed only if every
/// cell of the block it spans is traversable, so the straight step between two centres never enters a blocked cell.
fn search(field: &DistanceField, free: &[bool], start: [usize; 3], goal: [usize; 3]) -> Option<Vec<[usize; 3]>> {
    let dims = field.dims;
    let strides = [1, dims[0] as isize, (dims[0] * dims[1]) as isize];
    let mut steps = [Step { d: [0; 3], len: 0.0, to: 0, corners: [0; 6], corner_count: 0 }; 26];
    let mut n = 0;
    for dz in -1..=1i64 {
        for dy in -1..=1i64 {
            for dx in -1..=1i64 {
                let d = [dx, dy, dz];
                if d == [0, 0, 0] {
                    continue;
                }
                let moved = (0..3).filter(|&a| d[a] != 0).fold(0, |m, a| m | 1 << a);
                let mut step = Step { d, len: ((dx * dx + dy * dy + dz * dz) as f64).sqrt(), to: 0, corners: [0; 6], corner_count: 0 };
                for mask in 1..8 {
                    let offset = (0..3).filter(|&a| mask & (1 << a) != 0).map(|a| d[a] as isize * strides[a]).sum();
                    if mask == moved {
                        step.to = offset;
                    } else if mask & !moved == 0 {
                        step.corners[step.corner_count] = offset;
                        step.corner_count += 1;
                    }
                }
                steps[n] = step;
                n += 1;
            }
        }
    }

    let h = |c: [usize; 3]| (0..3).map(|a| (c[a] as f64 - goal[a] as f64).powi(2)).sum::<f64>().sqrt();
    let mut cost = vec![f64::INFINITY; free.len()];
    let mut parent = vec![usize::MAX; free.len()];
    let mut closed = vec![false; free.len()];
    let mut open = BinaryHeap::new();
    let (s, g) = (field.flat(start), field.flat(goal));
    cost[s] = 0.0;
    open.push(Open { f: h(start), cell: s });
    while let Some(Open { cell, .. }) = open.pop() {
        if closed[cell] {
            continue;
        }
        if cell == g {
            let mut path = vec![goal];
            let mut at = g;
            while at != s {
                at = parent[at];
                path.push(field.unflat(at));
            }
            path.reverse();
            return Some(path);
        }
        closed[cell] = true;
        let c = field.unflat(cell);
        for step in &steps {
            if (0..3).any(|a| (c[a] == 0 && step.d[a] < 0) || (c[a] + 1 == dims[a] && step.d[a] > 0)) {
                continue;
            }
            let next = (cell as isize + step.to) as usize;
            if closed[next] || !free[next] {
                continue;
            }
            if !step.corners[..step.corner_count].iter().all(|&o| free[(cell as isize + o) as usize]) {
                continue;
            }
            let through = cost[cell] + step.len;
            if through < cost[next] {
                cost[next] = through;
                parent[next] = cell;
                open.push(Open { f: through + h(field.unflat(next)), cell: next });
            }
        }
    }
    None
}

/// Line-of-sight smoothing: drop each waypoint whose neighbours on the smoothed path see each other with clearance.
fn smooth(field: &DistanceField, free: &[bool], raw: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut out = vec![raw[0]];
    for k in 1..raw.len() - 1 {
        if !segment_clear(field, free, out[out.len() - 1], raw[k + 1]) {
            out.push(raw[k]);
        }
    }
    out.push(raw[raw.len() - 1]);
    out
}

/// True if every point of the segment lies in a traversable cell. Samples are at most half a voxel apart and each one
/// checks every cell within a quarter voxel of it, so the stretches between samples are covered too.
fn segment_clear(field: &DistanceField, free: &[bool], a: [f64; 3], b: [f64; 3]) -> bool {
    let len = (0..3).map(|k| (b[k] - a[k]).powi(2)).sum::<f64>().sqrt();
    let n = (len / (field.voxel / 2.0)).ceil().max(1.0) as usize;
    // The extra millionth of a voxel absorbs rounding at the quarter-voxel edge.
    let reach = field.voxel * (0.25 + 1e-6);
    for s in 0..=n {
        let t = s as f64 / n as f64;
        let p = [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t);
        let (Some(lo), Some(hi)) = (field.cell_of(p.map(|x| x - reach)), field.cell_of(p.map(|x| x + reach))) else {
            return false;
        };
        for k in lo[2]..=hi[2] {
            for j in lo[1]..=hi[1] {
                for i in lo[0]..=hi[0] {
                    if !free[field.flat([i, j, k])] {
                        return false;
                    }
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room_twin::RoomMesh;

    /// xorshift64*: deterministic, so every run sees the same grids.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn chance(&mut self, p: f64) -> bool {
            ((self.next() >> 11) as f64 / (1u64 << 53) as f64) < p
        }
    }

    fn flat(dims: [usize; 3], [i, j, k]: [usize; 3]) -> usize {
        i + dims[0] * (j + dims[1] * k)
    }

    fn cells(dims: [usize; 3]) -> impl Iterator<Item = [usize; 3]> {
        (0..dims[2]).flat_map(move |k| (0..dims[1]).flat_map(move |j| (0..dims[0]).map(move |i| [i, j, k])))
    }

    /// Nearest occupied cell centre by checking every one, with the outside of the grid occupied.
    fn brute_force(dims: [usize; 3], voxel: f64, occ: &[bool]) -> Vec<f64> {
        let occupied: Vec<[usize; 3]> = cells(dims).filter(|&c| occ[flat(dims, c)]).collect();
        cells(dims)
            .map(|c| {
                let mut best = (0..3).map(|a| (c[a] + 1).min(dims[a] - c[a]).pow(2)).min().unwrap();
                for o in &occupied {
                    let sq: usize = (0..3).map(|a| c[a].abs_diff(o[a]).pow(2)).sum();
                    best = best.min(sq);
                }
                (best as f64).sqrt() * voxel
            })
            .collect()
    }

    fn field_from(dims: [usize; 3], voxel: f64, occ: &[bool]) -> DistanceField {
        DistanceField::from_occupancy(dims, [0.0; 3], voxel, |c| occ[flat(dims, c)])
    }

    fn length(path: &[[f64; 3]]) -> f64 {
        path.windows(2).map(|w| dist(w[0], w[1])).sum()
    }

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
    }

    /// Every point sampled along the path, at most an eighth of a voxel apart, with its distance-field value.
    fn samples(field: &DistanceField, path: &[[f64; 3]]) -> Vec<([f64; 3], f64)> {
        let mut out = Vec::new();
        for w in path.windows(2) {
            let n = (dist(w[0], w[1]) / (field.voxel() / 8.0)).ceil().max(1.0) as usize;
            for s in 0..=n {
                let t = s as f64 / n as f64;
                let p = [0, 1, 2].map(|k| w[0][k] + (w[1][k] - w[0][k]) * t);
                out.push((p, field.at_point(p)));
            }
        }
        out
    }

    fn assert_keeps_clearance(field: &DistanceField, path: &[[f64; 3]], clearance: f64) {
        for (p, d) in samples(field, path) {
            assert!(d >= clearance, "sample {p:?} has distance {d} below clearance {clearance}");
        }
    }

    #[test]
    fn distance_field_equals_brute_force_on_random_grids() {
        let shapes = [[1, 1, 1], [2, 3, 1], [5, 4, 3], [7, 1, 6], [6, 6, 6], [9, 5, 7], [4, 11, 3]];
        let mut checked = 0;
        for seed in 1..=5u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            for dims in shapes {
                for p in [0.0, 1.0, 0.05, 0.2, 0.5] {
                    let occ: Vec<bool> = (0..dims[0] * dims[1] * dims[2]).map(|_| rng.chance(p)).collect();
                    let voxel = 0.1;
                    let field = field_from(dims, voxel, &occ);
                    let expected = brute_force(dims, voxel, &occ);
                    for c in cells(dims) {
                        let (got, want) = (field.at(c), expected[flat(dims, c)]);
                        assert!((got - want).abs() < 1e-12, "seed {seed} dims {dims:?} p {p} cell {c:?}: {got} != {want}");
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 10_000, "only {checked} cells checked");
    }

    #[test]
    fn empty_and_full_grids_have_known_distances() {
        let dims = [5, 3, 4];
        let empty = field_from(dims, 0.5, &vec![false; 60]);
        assert_eq!(empty.at([0, 0, 0]), 0.5, "a corner cell is one voxel from the outside");
        assert_eq!(empty.at([2, 1, 1]), 1.0, "the middle cell is two voxels from the nearest outside cell");
        let full = field_from(dims, 0.5, &vec![true; 60]);
        assert!(cells(dims).all(|c| full.at(c) == 0.0));
    }

    /// Closed box from (0,0,0) to (2.0,1.5,1.0): 8 vertices, 12 triangles (the same room as the room_twin tests).
    fn box_mesh() -> RoomMesh {
        let vertices = vec![
            [0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.5, 0.0], [0.0, 1.5, 0.0],
            [0.0, 0.0, 1.0], [2.0, 0.0, 1.0], [2.0, 1.5, 1.0], [0.0, 1.5, 1.0],
        ];
        let triangles = vec![
            [0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7], [0, 1, 5], [0, 5, 4],
            [2, 3, 7], [2, 7, 6], [0, 4, 7], [0, 7, 3], [1, 2, 6], [1, 6, 5],
        ];
        RoomMesh { vertices, triangles }
    }

    #[test]
    fn from_grid_reads_the_room_twin_and_maps_points_to_cells() {
        let grid = OccupancyGrid::from_mesh(&box_mesh(), 0.25).unwrap();
        let field = DistanceField::from_grid(&grid);
        assert_eq!(field.dims(), [9, 7, 5]);
        assert_eq!(field.voxel(), 0.25);
        assert_eq!(field.origin(), grid.origin());
        // The centre cell is two voxels from the z = 0 and z = 1 walls, three from the floor and ceiling, four from x walls.
        assert_eq!(field.centre([4, 3, 2]), [1.0, 0.75, 0.5]);
        assert_eq!(field.at([4, 3, 2]), 0.5);
        assert_eq!(field.at([0, 3, 2]), 0.0, "wall cells are occupied");
        assert_eq!(field.cell_of([1.0, 0.75, 0.5]), Some([4, 3, 2]));
        assert_eq!(field.cell_of([1.12, 0.86, 0.62]), Some([4, 3, 2]), "nearest cell, not the cell below");
        assert_eq!(field.at_point([1.1, 0.8, 0.45]), 0.5);
        assert_eq!(field.cell_of([3.0, 0.5, 0.5]), None);
        assert_eq!(field.at_point([3.0, 0.5, 0.5]), 0.0, "outside the grid counts as occupied");
        assert_eq!(field.at_point([1.0, -0.5, 0.5]), 0.0);
    }

    #[test]
    fn empty_box_path_is_the_straight_line() {
        let dims = [30, 12, 20];
        let field = field_from(dims, 0.1, &vec![false; 30 * 12 * 20]);
        let (start, goal) = (field.centre([4, 4, 4]), field.centre([25, 7, 15]));
        let path = plan(&field, start, goal, 0.25).unwrap();
        assert_eq!(path.len(), 2, "smoothing leaves only the ends: {path:?}");
        assert_eq!((path[0], path[1]), (start, goal));
        assert!((length(&path) - dist(start, goal)).abs() < 1e-9);
        assert_keeps_clearance(&field, &path, 0.25);
    }

    /// A wall across x = 20 (cell index), with a gap three cells wide in z centred on z = 20 and open full height.
    /// Clearance 0.2 m (two voxels) leaves exactly one traversable column in the gap: z = 20.
    fn wall_with_gap(gap: bool) -> (DistanceField, [usize; 3]) {
        let dims = [41, 9, 31];
        let mut occ = vec![false; dims[0] * dims[1] * dims[2]];
        for c in cells(dims) {
            if c[0] == 20 && !(gap && (19..=21).contains(&c[2])) {
                occ[flat(dims, c)] = true;
            }
        }
        (field_from(dims, 0.1, &occ), dims)
    }

    #[test]
    fn wall_with_one_gap_forces_the_known_detour() {
        let (field, _) = wall_with_gap(true);
        let clearance = 0.2;
        let (start, goal) = (field.centre([5, 4, 10]), field.centre([35, 4, 10]));
        let gap_centre = field.centre([20, 4, 20]);
        let path = plan(&field, start, goal, clearance).unwrap();
        let detour = dist(start, gap_centre) + dist(gap_centre, goal);
        let len = length(&path);
        // The path crosses the wall plane somewhere in the one free column, at most half a voxel from the gap centre in z,
        // so it is at most one voxel shorter than the detour through the centre; smoothing keeps it within two voxels above.
        assert!(len >= detour - 0.1 && len <= detour + 0.2, "length {len} vs detour {detour}, path {path:?}");
        assert!(len > dist(start, goal) + 0.5, "the straight line is blocked, so the path must be much longer");
        let crossings: Vec<[usize; 3]> = samples(&field, &path).iter().filter_map(|(p, _)| field.cell_of(*p)).filter(|c| c[0] == 20).collect();
        assert!(!crossings.is_empty(), "the path must cross the wall plane");
        assert!(crossings.iter().all(|c| c[2] == 20), "every crossing is in the gap's free column: {crossings:?}");
        assert_keeps_clearance(&field, &path, clearance);
    }

    #[test]
    fn smoothed_paths_never_go_below_clearance() {
        let dims = [32, 10, 28];
        let clearance = 0.15;
        let (s, g) = ([3, 5, 3], [28, 4, 24]);
        let mut bent = 0;
        for seed in 1..=8u64 {
            let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03));
            let mut occ: Vec<bool> = (0..dims[0] * dims[1] * dims[2]).map(|_| rng.chance(0.02)).collect();
            for c in cells(dims) {
                let near = |e: [usize; 3]| (0..3).all(|a| c[a].abs_diff(e[a]) <= 2);
                if near(s) || near(g) {
                    occ[flat(dims, c)] = false;
                }
            }
            let field = field_from(dims, 0.1, &occ);
            let (start, goal) = (field.centre(s), field.centre(g));
            let path = match plan(&field, start, goal, clearance) {
                Ok(path) => path,
                Err(e) => {
                    assert_eq!(e, PlanError::Unreachable, "seed {seed}");
                    continue;
                }
            };
            assert_eq!((path[0], *path.last().unwrap()), (start, goal), "seed {seed}");
            for w in &path {
                assert!(field.at_point(*w) >= clearance, "seed {seed}: waypoint {w:?}");
            }
            assert_keeps_clearance(&field, &path, clearance);
            bent += (path.len() > 2) as usize;
        }
        assert!(bent >= 4, "only {bent} seeds needed a bend, so the clutter is not testing anything");
    }

    #[test]
    fn exact_off_centre_endpoints_are_the_path_ends() {
        let (field, _) = wall_with_gap(true);
        let (start, goal) = ([0.537, 0.461, 1.012], [3.548, 0.433, 0.968]);
        let path = plan(&field, start, goal, 0.2).unwrap();
        assert_eq!((path[0], *path.last().unwrap()), (start, goal));
        assert_keeps_clearance(&field, &path, 0.2);
    }

    #[test]
    fn a_full_wall_is_unreachable() {
        let (field, _) = wall_with_gap(false);
        let (start, goal) = (field.centre([5, 4, 10]), field.centre([35, 4, 10]));
        assert_eq!(plan(&field, start, goal, 0.2), Err(PlanError::Unreachable));
    }

    #[test]
    fn blocked_start_and_goal_are_reported() {
        let (field, _) = wall_with_gap(true);
        let free = field.centre([5, 4, 10]);
        let in_wall = field.centre([20, 4, 5]);
        let beside_wall = field.centre([21, 4, 5]); // one voxel from the wall, below the 0.2 m clearance
        assert_eq!(plan(&field, in_wall, free, 0.2), Err(PlanError::StartBlocked));
        assert_eq!(plan(&field, free, beside_wall, 0.2), Err(PlanError::GoalBlocked));
        assert_eq!(plan(&field, [-1.0, 0.4, 1.0], free, 0.2), Err(PlanError::StartBlocked), "outside the grid");
        assert_eq!(plan(&field, free, [1.0, 0.4, 9.0], 0.2), Err(PlanError::GoalBlocked), "outside the grid");
    }
}
