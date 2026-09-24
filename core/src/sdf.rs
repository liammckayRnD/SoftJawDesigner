//! Triangle mesh -> band-limited signed distance field on a dense grid.
//!
//! Distance: each triangle writes exact point-triangle distance into the
//! grid nodes inside its bounding box grown by `band`. Nodes farther than
//! `band` from the surface keep the value `band`.
//!
//! Sign: one vertical ray per grid column. Crossings are counted by parity,
//! so the input mesh needs to be closed (true for STEP tessellations).

use crate::grid::{add, dot, length, scale, sub, Grid, V3};

/// Closest point on triangle (a, b, c) to p. Ericson, Real-Time Collision Detection 5.1.5.
fn closest_on_tri(p: V3, a: V3, b: V3, c: V3) -> V3 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return add(a, scale(ab, v));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return add(a, scale(ac, w));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add(b, scale(sub(c, b), w));
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    add(a, add(scale(ab, v), scale(ac, w)))
}

/// Z where the vertical line through (x, y) crosses the triangle, if it does.
fn vertical_hit(x: f32, y: f32, a: V3, b: V3, c: V3) -> Option<f32> {
    let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if d.abs() < 1e-12 {
        return None; // triangle is edge-on to the ray
    }
    let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / d;
    let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / d;
    let l3 = 1.0 - l1 - l2;
    if l1 < 0.0 || l2 < 0.0 || l3 < 0.0 {
        return None;
    }
    Some(l1 * a[2] + l2 * b[2] + l3 * c[2])
}

/// Fill `grid` with the signed distance to the mesh, clamped to [-band, band].
/// Negative inside.
pub fn mesh_to_sdf(grid: &mut Grid, pos: &[f32], idx: &[u32], band: f32) {
    let [nx, ny, nz] = grid.n;
    let tri = |t: usize| -> (V3, V3, V3) {
        let v = |i: u32| -> V3 {
            let i = i as usize * 3;
            [pos[i], pos[i + 1], pos[i + 2]]
        };
        (v(idx[t * 3]), v(idx[t * 3 + 1]), v(idx[t * 3 + 2]))
    };
    let ntri = idx.len() / 3;

    // 1. Unsigned distance inside a band around each triangle.
    for d in grid.data.iter_mut() {
        *d = band;
    }
    for t in 0..ntri {
        let (a, b, c) = tri(t);
        let lo = |ax: usize| a[ax].min(b[ax]).min(c[ax]) - band;
        let hi = |ax: usize| a[ax].max(b[ax]).max(c[ax]) + band;
        let (Some(rx), Some(ry), Some(rz)) = (
            grid.range(0, lo(0), hi(0)),
            grid.range(1, lo(1), hi(1)),
            grid.range(2, lo(2), hi(2)),
        ) else {
            continue;
        };
        for k in rz.0..=rz.1 {
            for j in ry.0..=ry.1 {
                for i in rx.0..=rx.1 {
                    let p = grid.pos(i, j, k);
                    let dist = length(sub(p, closest_on_tri(p, a, b, c)));
                    let id = grid.idx(i, j, k);
                    if dist < grid.data[id] {
                        grid.data[id] = dist;
                    }
                }
            }
        }
    }

    // 2. Sign by ray parity, one vertical ray per column.
    // A tiny, irregular jitter keeps rays off shared edges and vertices.
    let jx = grid.h * 1.234_567e-3;
    let jy = grid.h * 2.718_281e-3;
    let mut hits: Vec<Vec<f32>> = vec![Vec::new(); nx * ny];
    for t in 0..ntri {
        let (a, b, c) = tri(t);
        let lo = |ax: usize| a[ax].min(b[ax]).min(c[ax]);
        let hi = |ax: usize| a[ax].max(b[ax]).max(c[ax]);
        let (Some(rx), Some(ry)) = (
            grid.range(0, lo(0) - jx - grid.h, hi(0) + grid.h),
            grid.range(1, lo(1) - jy - grid.h, hi(1) + grid.h),
        ) else {
            continue;
        };
        for j in ry.0..=ry.1 {
            for i in rx.0..=rx.1 {
                let p = grid.pos(i, j, 0);
                if let Some(z) = vertical_hit(p[0] + jx, p[1] + jy, a, b, c) {
                    hits[i + nx * j].push(z);
                }
            }
        }
    }
    for j in 0..ny {
        for i in 0..nx {
            let col = &mut hits[i + nx * j];
            if col.is_empty() {
                continue;
            }
            col.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut below = 0usize; // crossings below the current node
            for k in 0..nz {
                let z = grid.origin[2] + grid.h * k as f32;
                while below < col.len() && col[below] < z {
                    below += 1;
                }
                if below % 2 == 1 {
                    let id = grid.idx(i, j, k);
                    grid.data[id] = -grid.data[id];
                }
            }
        }
    }
}

/// Grow the shape by `clearance` (subtract a constant from the SDF).
pub fn offset(grid: &mut Grid, clearance: f32) {
    for d in grid.data.iter_mut() {
        *d -= clearance;
    }
}

/// Sweep the shape straight up (+Z) to infinity, so the part can be lifted out.
///
/// The distance to the swept set at (x, y, z) is the min of the original
/// distance over all z' <= z in that column. This is exact outside the shape;
/// inside, the sign is right and the value is a usable bound.
pub fn sweep_up(grid: &mut Grid) {
    let [nx, ny, nz] = grid.n;
    for j in 0..ny {
        for i in 0..nx {
            let mut m = f32::INFINITY;
            for k in 0..nz {
                let id = grid.idx(i, j, k);
                m = m.min(grid.data[id]);
                grid.data[id] = m;
            }
        }
    }
}
