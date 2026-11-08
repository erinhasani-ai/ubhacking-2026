//! The Room Twin: a scan parsed from bytes and an occupancy grid filled from its triangles.
//! WebAssembly-safe: no file paths, no std::fs, no threads. Scan frame is kept as it is (Y up).

use std::io::Cursor;

/// Location and heading from the leading `#` comment lines of a Scaniverse OBJ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanHeader {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub elevation_m: Option<f64>,
    pub heading_deg: Option<f64>,
}

impl ScanHeader {
    /// Reads `# Key: value` lines before the first non-comment line; missing keys stay None.
    pub fn parse(bytes: &[u8]) -> ScanHeader {
        let mut header = ScanHeader { latitude: None, longitude: None, elevation_m: None, heading_deg: None };
        for raw in bytes.split(|&b| b == b'\n') {
            let line = std::str::from_utf8(raw).unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some(comment) = line.strip_prefix('#') else { break };
            let Some((key, value)) = comment.split_once(':') else { continue };
            let number = value.split_whitespace().next().and_then(|t| t.parse::<f64>().ok());
            match key.trim() {
                "Latitude" => header.latitude = number,
                "Longitude" => header.longitude = number,
                "Elevation" => header.elevation_m = number,
                "Rotation" => header.heading_deg = number,
                _ => {}
            }
        }
        header
    }
}

/// Vertices and triangles of a scan, in its own frame.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomMesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// Parses an OBJ from bytes, triangulated. Materials are ignored, so a missing .mtl is not an error.
pub fn load_obj(bytes: &[u8]) -> Result<RoomMesh, tobj::LoadError> {
    let options = tobj::LoadOptions { triangulate: true, ..Default::default() };
    let (models, _materials) = tobj::load_obj_buf(&mut Cursor::new(bytes), &options, |_| {
        Err(tobj::LoadError::OpenFileFailed)
    })?;
    let mut mesh = RoomMesh { vertices: Vec::new(), triangles: Vec::new() };
    for model in models {
        let base = mesh.vertices.len() as u32;
        mesh.vertices.extend(model.mesh.positions.chunks_exact(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]));
        mesh.triangles.extend(model.mesh.indices.chunks_exact(3).map(|t| [t[0] + base, t[1] + base, t[2] + base]));
    }
    Ok(mesh)
}

/// Boolean voxel grid over the mesh's bounding box. Voxel centres start at the bounding-box minimum.
#[derive(Debug, Clone)]
pub struct OccupancyGrid {
    voxel: f64,
    origin: [f64; 3],
    size: [f64; 3],
    dims: [usize; 3],
    cells: Vec<bool>,
    occupied: usize,
}

impl OccupancyGrid {
    /// Fills the grid by sampling every triangle at no more than half a voxel spacing.
    pub fn from_mesh(mesh: &RoomMesh, voxel: f64) -> Result<OccupancyGrid, &'static str> {
        if !(voxel > 0.0) {
            return Err("voxel size must be positive");
        }
        let first = *mesh.vertices.first().ok_or("mesh has no vertices")?;
        let (mut min, mut max) = (first, first);
        for v in &mesh.vertices {
            for k in 0..3 {
                min[k] = min[k].min(v[k]);
                max[k] = max[k].max(v[k]);
            }
        }
        let mut size = [0.0; 3];
        let mut origin = [0.0; 3];
        let mut dims = [0usize; 3];
        for k in 0..3 {
            size[k] = max[k] - min[k];
            origin[k] = min[k] - voxel / 2.0;
            dims[k] = (size[k] / voxel).floor() as usize + 1;
        }
        let mut grid = OccupancyGrid { voxel, origin, size, dims, cells: vec![false; dims[0] * dims[1] * dims[2]], occupied: 0 };
        for t in &mesh.triangles {
            let (a, b, c) = (mesh.vertices[t[0] as usize], mesh.vertices[t[1] as usize], mesh.vertices[t[2] as usize]);
            grid.fill_triangle(a, b, c);
        }
        Ok(grid)
    }

    fn fill_triangle(&mut self, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
        let edge = |p: [f64; 3], q: [f64; 3]| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
        let longest = edge(a, b).max(edge(b, c)).max(edge(c, a));
        let n = ((longest / (self.voxel / 2.0)).ceil() as usize).max(1);
        for i in 0..=n {
            for j in 0..=(n - i) {
                let (ti, tj) = (i as f64 / n as f64, j as f64 / n as f64);
                self.mark([0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * ti + (c[k] - a[k]) * tj));
            }
        }
    }

    fn mark(&mut self, p: [f64; 3]) {
        let mut idx = [0usize; 3];
        for k in 0..3 {
            let cell = ((p[k] - self.origin[k]) / self.voxel).floor().clamp(0.0, (self.dims[k] - 1) as f64);
            idx[k] = cell as usize;
        }
        let flat = idx[0] + self.dims[0] * (idx[1] + self.dims[1] * idx[2]);
        if !self.cells[flat] {
            self.cells[flat] = true;
            self.occupied += 1;
        }
    }

    pub fn dims(&self) -> [usize; 3] {
        self.dims
    }

    pub fn origin(&self) -> [f64; 3] {
        self.origin
    }

    /// Bounding-box extent in metres, per axis (scan frame).
    pub fn size(&self) -> [f64; 3] {
        self.size
    }

    pub fn voxel(&self) -> f64 {
        self.voxel
    }

    /// True if the voxel at grid index [i, j, k] is occupied. Panics if the index is outside `dims()`.
    pub fn is_occupied(&self, idx: [usize; 3]) -> bool {
        assert!((0..3).all(|k| idx[k] < self.dims[k]), "voxel index {idx:?} outside {:?}", self.dims);
        self.cells[idx[0] + self.dims[0] * (idx[1] + self.dims[1] * idx[2])]
    }

    pub fn total_cells(&self) -> usize {
        self.cells.len()
    }

    pub fn occupied_count(&self) -> usize {
        self.occupied
    }

    // shortcut: free = every unoccupied voxel in the bounding box (no inside/outside test), upgrade when the planner needs unknown space
    pub fn free_count(&self) -> usize {
        self.cells.len() - self.occupied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Closed box from (0,0,0) to (2.0,1.5,1.0): 8 vertices, 12 triangles.
    fn box_mesh() -> RoomMesh {
        let v = vec![
            [0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.5, 0.0], [0.0, 1.5, 0.0],
            [0.0, 0.0, 1.0], [2.0, 0.0, 1.0], [2.0, 1.5, 1.0], [0.0, 1.5, 1.0],
        ];
        let t = vec![
            [0, 2, 1], [0, 3, 2], // z = 0
            [4, 5, 6], [4, 6, 7], // z = 1
            [0, 1, 5], [0, 5, 4], // y = 0
            [2, 3, 7], [2, 7, 6], // y = 1.5
            [0, 4, 7], [0, 7, 3], // x = 0
            [1, 2, 6], [1, 6, 5], // x = 2
        ];
        RoomMesh { vertices: v, triangles: t }
    }

    #[test]
    fn box_room_counts_are_exact() {
        let grid = OccupancyGrid::from_mesh(&box_mesh(), 0.25).unwrap();
        assert_eq!(grid.dims(), [9, 7, 5]);
        assert_eq!(grid.total_cells(), 315);
        assert_eq!(grid.occupied_count(), 210);
        assert_eq!(grid.free_count(), 105);
    }

    #[test]
    fn occupancy_query_matches_the_box_walls() {
        let grid = OccupancyGrid::from_mesh(&box_mesh(), 0.25).unwrap();
        let [nx, ny, nz] = grid.dims();
        let mut occupied = 0;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let on_wall = i == 0 || j == 0 || k == 0 || i == nx - 1 || j == ny - 1 || k == nz - 1;
                    assert_eq!(grid.is_occupied([i, j, k]), on_wall, "voxel {:?}", [i, j, k]);
                    occupied += on_wall as usize;
                }
            }
        }
        assert_eq!(occupied, grid.occupied_count());
    }

    #[test]
    fn large_flat_wall_leaves_no_holes() {
        let mesh = RoomMesh {
            vertices: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let grid = OccupancyGrid::from_mesh(&mesh, 0.25).unwrap();
        assert_eq!(grid.dims(), [9, 5, 1]);
        assert_eq!(grid.occupied_count(), 9 * 5);
    }

    #[test]
    fn header_parses_scaniverse_comments() {
        let text = b"# Unit: meter\n# Latitude: 43.0025265\n# Longitude: -78.7874606\n# Elevation: 189.963\n# Rotation: 345.94275 deg\nmtllib x.mtl\nv 0 0 0\n";
        let h = ScanHeader::parse(text);
        assert_eq!(h.latitude, Some(43.0025265));
        assert_eq!(h.longitude, Some(-78.7874606));
        assert_eq!(h.elevation_m, Some(189.963));
        assert_eq!(h.heading_deg, Some(345.94275));
    }

    #[test]
    fn header_missing_lines_are_none() {
        let h = ScanHeader::parse(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n");
        assert_eq!(h, ScanHeader { latitude: None, longitude: None, elevation_m: None, heading_deg: None });
    }

    #[test]
    fn obj_without_material_file_loads() {
        let mesh = load_obj(b"mtllib missing.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.triangles, vec![[0, 1, 2]]);
    }
}
