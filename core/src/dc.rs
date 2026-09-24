//! Dual contouring on a dense grid.
//!
//! One vertex per cell that the surface passes through, placed by solving a
//! small least-squares problem (QEF) built from the edge crossings and their
//! normals. This keeps sharp edges and corners, which matter for jaw faces.

use crate::grid::{normalize, Grid, V3};
use std::collections::HashMap;

pub struct Mesh {
    pub verts: Vec<V3>,
    pub tris: Vec<[u32; 3]>,
}

/// Pull toward the mass point. Keeps flat regions stable; small enough to
/// leave corners sharp.
const QEF_REG: f64 = 0.05;

/// Solve the 3x3 system with Gaussian elimination and partial pivoting.
fn solve3(mut a: [[f64; 3]; 3], mut b: [f64; 3]) -> Option<[f64; 3]> {
    for col in 0..3 {
        let mut piv = col;
        for r in col + 1..3 {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        for r in col + 1..3 {
            let f = a[r][col] / a[col][col];
            for c in col..3 {
                a[r][c] -= f * a[col][c];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = [0.0; 3];
    for r in (0..3).rev() {
        let mut s = b[r];
        for c in r + 1..3 {
            s -= a[r][c] * x[c];
        }
        x[r] = s / a[r][r];
    }
    Some(x)
}

/// Contour the zero level of `f` over nodes `lo..hi` (exclusive hi).
/// The caller must make `f` positive on the border nodes of that region,
/// so the output is closed.
pub fn dual_contour<F>(g: &Grid, f: F, lo: [usize; 3], hi: [usize; 3]) -> Mesh
where
    F: Fn(usize, usize, usize) -> f32,
{
    let h = g.h;
    let cell_id = |i: usize, j: usize, k: usize| g.idx(i, j, k);

    // Central-difference gradient at a node (one-sided at the region edge).
    let grad = |i: usize, j: usize, k: usize| -> V3 {
        let p = [i, j, k];
        let mut out = [0.0f32; 3];
        for ax in 0..3 {
            let mut a = p;
            let mut b = p;
            if p[ax] > lo[ax] {
                a[ax] -= 1;
            }
            if p[ax] + 1 < hi[ax] {
                b[ax] += 1;
            }
            let span = (b[ax] - a[ax]) as f32 * h;
            out[ax] = if span > 0.0 {
                (f(b[0], b[1], b[2]) - f(a[0], a[1], a[2])) / span
            } else {
                0.0
            };
        }
        out
    };

    // Cell corner c (0..8) as (dx, dy, dz) bits.
    let corner = |c: usize| [c & 1, (c >> 1) & 1, (c >> 2) & 1];

    let mut verts: Vec<V3> = Vec::new();
    let mut cell_vert: HashMap<usize, u32> = HashMap::new();

    // Pass 1: one vertex per surface cell.
    for k in lo[2]..hi[2] - 1 {
        for j in lo[1]..hi[1] - 1 {
            for i in lo[0]..hi[0] - 1 {
                let mut vals = [0.0f32; 8];
                let mut inside = 0u8;
                for c in 0..8 {
                    let v = f(i + (c & 1), j + ((c >> 1) & 1), k + ((c >> 2) & 1));
                    vals[c] = v;
                    if v < 0.0 {
                        inside |= 1 << c;
                    }
                }
                if inside == 0 || inside == 0xFF {
                    continue;
                }

                let base = g.pos(i, j, k);
                let mut ata = [[0.0f64; 3]; 3];
                let mut pts: Vec<([f64; 3], [f64; 3])> = Vec::with_capacity(6);
                let mut mass = [0.0f64; 3];

                for c0 in 0..8usize {
                    for ax in 0..3 {
                        if c0 & (1 << ax) != 0 {
                            continue;
                        }
                        let c1 = c0 | (1 << ax);
                        let (v0, v1) = (vals[c0], vals[c1]);
                        if (v0 < 0.0) == (v1 < 0.0) {
                            continue;
                        }
                        let t = (v0 / (v0 - v1)).clamp(0.0, 1.0);
                        let (o0, o1) = (corner(c0), corner(c1));
                        let n0 = [i + o0[0], j + o0[1], k + o0[2]];
                        let n1 = [i + o1[0], j + o1[1], k + o1[2]];
                        let mut p = [0.0f64; 3];
                        for d in 0..3 {
                            let a = o0[d] as f32;
                            let b = o1[d] as f32;
                            p[d] = (base[d] + h * (a + t * (b - a))) as f64;
                        }
                        let g0 = grad(n0[0], n0[1], n0[2]);
                        let g1 = grad(n1[0], n1[1], n1[2]);
                        let n = normalize([
                            g0[0] + t * (g1[0] - g0[0]),
                            g0[1] + t * (g1[1] - g0[1]),
                            g0[2] + t * (g1[2] - g0[2]),
                        ]);
                        let n = [n[0] as f64, n[1] as f64, n[2] as f64];
                        for r in 0..3 {
                            for c in 0..3 {
                                ata[r][c] += n[r] * n[c];
                            }
                            mass[r] += p[r];
                        }
                        pts.push((p, n));
                    }
                }

                let cnt = pts.len() as f64;
                for m in mass.iter_mut() {
                    *m /= cnt;
                }
                // Solve for y = x - mass:  (sum n n^T + reg I) y = sum n (n . (p - mass))
                let mut rhs = [0.0f64; 3];
                for (p, n) in &pts {
                    let d = n[0] * (p[0] - mass[0]) + n[1] * (p[1] - mass[1]) + n[2] * (p[2] - mass[2]);
                    for r in 0..3 {
                        rhs[r] += n[r] * d;
                    }
                }
                for r in 0..3 {
                    ata[r][r] += QEF_REG;
                }
                let y = solve3(ata, rhs).unwrap_or([0.0; 3]);

                // Keep the vertex inside its cell.
                let mut x = [0.0f32; 3];
                for d in 0..3 {
                    let v = (mass[d] + y[d]) as f32;
                    x[d] = v.clamp(base[d], base[d] + h);
                }
                cell_vert.insert(cell_id(i, j, k), verts.len() as u32);
                verts.push(x);
            }
        }
    }

    // Pass 2: one quad per grid edge with a sign change, joining the 4 cells around it.
    let mut tris: Vec<[u32; 3]> = Vec::new();
    for k in lo[2]..hi[2] {
        for j in lo[1]..hi[1] {
            for i in lo[0]..hi[0] {
                let p = [i, j, k];
                let f0 = f(i, j, k);
                for ax in 0..3 {
                    if p[ax] + 1 >= hi[ax] {
                        continue;
                    }
                    let mut q = p;
                    q[ax] += 1;
                    let f1 = f(q[0], q[1], q[2]);
                    if (f0 < 0.0) == (f1 < 0.0) {
                        continue;
                    }
                    let u = (ax + 1) % 3;
                    let v = (ax + 2) % 3;
                    if p[u] == lo[u] || p[v] == lo[v] {
                        continue;
                    }
                    // Cells around the edge, counter-clockwise about +ax.
                    let mut quad = [0u32; 4];
                    let mut ok = true;
                    for (s, (du, dv)) in [(1, 1), (0, 1), (0, 0), (1, 0)].iter().enumerate() {
                        let mut c = p;
                        c[u] -= du;
                        c[v] -= dv;
                        match cell_vert.get(&cell_id(c[0], c[1], c[2])) {
                            Some(&vi) => quad[s] = vi,
                            None => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok {
                        continue;
                    }
                    // Normal points from inside to outside.
                    if f0 >= 0.0 {
                        quad.reverse();
                    }
                    tris.push([quad[0], quad[1], quad[2]]);
                    tris.push([quad[0], quad[2], quad[3]]);
                }
            }
        }
    }

    Mesh { verts, tris }
}
