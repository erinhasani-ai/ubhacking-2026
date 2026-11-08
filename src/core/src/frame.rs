//! Mission frame (local NED) and the Launch pad (a clear patch of floor).
//!
//! Scan frame: Scaniverse OBJ, metres, Y up. Mission frame: N = -z, E = x, D = floor_y - y,
//! origin on the room floor directly below the Launch pad centre.

/// Launch pad centre (x, z) in the scan frame, measured by `probe_pad --search` on the 2026-11-07 Scaniverse scan.
pub const PAD_XZ: [f64; 2] = [-0.10, 1.40];
/// Scanned floor height under the pad (the floor is not level, so it is measured under the pad only), same scan.
pub const FLOOR_Y: f64 = 0.096;

pub fn scan_to_mission(p: [f64; 3]) -> [f64; 3] {
    [-(p[2] - PAD_XZ[1]), p[0] - PAD_XZ[0], FLOOR_Y - p[1]]
}

pub fn mission_to_scan(m: [f64; 3]) -> [f64; 3] {
    [PAD_XZ[0] + m[1], FLOOR_Y - m[2], PAD_XZ[1] - m[0]]
}

/// Mission position of pad slot `i` (0..6): two rows 0.6 m apart in N, three columns 0.6 m apart in E, on the floor.
pub fn pad_slot(i: usize) -> [f64; 3] {
    assert!(i < 6, "pad slot {i} out of range 0..6");
    [(i / 3) as f64 * 0.6 - 0.3, (i % 3) as f64 * 0.6 - 0.6, 0.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    }

    #[test]
    fn round_trip() {
        for p in [[0.0, 0.0, 0.0], [1.5, 2.0, -3.25], [-7.1, 0.4, 5.5], [PAD_XZ[0], FLOOR_Y, PAD_XZ[1]]] {
            let q = mission_to_scan(scan_to_mission(p));
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 1e-12, "{p:?} -> {q:?}");
            }
        }
    }

    #[test]
    fn north_cross_east_is_down() {
        // Unit Mission axes written in scan coordinates (direction only: difference of two mapped points).
        let axis = |m: [f64; 3]| {
            let (a, o) = (mission_to_scan(m), mission_to_scan([0.0; 3]));
            [a[0] - o[0], a[1] - o[1], a[2] - o[2]]
        };
        let (n, e, d) = (axis([1.0, 0.0, 0.0]), axis([0.0, 1.0, 0.0]), axis([0.0, 0.0, 1.0]));
        assert_eq!(n, [0.0, 0.0, -1.0]);
        assert_eq!(e, [1.0, 0.0, 0.0]);
        assert_eq!(d, [0.0, -1.0, 0.0]);
        let c = cross(n, e);
        for k in 0..3 {
            assert!((c[k] - d[k]).abs() < 1e-12);
        }
    }

    #[test]
    fn pad_centre_maps_to_origin() {
        let m = scan_to_mission([PAD_XZ[0], FLOOR_Y, PAD_XZ[1]]);
        assert!(m.iter().all(|v| v.abs() < 1e-12), "{m:?}");
    }

    #[test]
    fn slots_are_0_6_m_apart_and_distinct() {
        let dist = |a: [f64; 3], b: [f64; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert!(dist(pad_slot(i), pad_slot(j)) > 0.59, "slots {i} and {j}");
            }
        }
        for (a, b) in [(0, 1), (1, 2), (3, 4), (4, 5), (0, 3), (1, 4), (2, 5)] {
            assert!((dist(pad_slot(a), pad_slot(b)) - 0.6).abs() < 1e-12);
        }
        assert_eq!(pad_slot(0), [-0.3, -0.6, 0.0]);
        assert_eq!(pad_slot(5), [0.3, 0.6, 0.0]);
    }
}
