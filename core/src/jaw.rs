//! The soft jaw pipeline.
//!
//! Frame (the caller places the part in it):
//!   X  along the jaw length
//!   Y  clamp direction; the parting plane is Y = 0
//!   Z  up; the jaw tops are at Z = 0, the part is lifted out along +Z
//!
//! Jaw A sits at +Y, jaw B at -Y. Each is `thickness` deep, with `gap`
//! between them at the parting plane.

use crate::dc::{dual_contour, Mesh};
use crate::grid::{Grid, V3};
use crate::sdf::{mesh_to_sdf, offset, sweep_up};

/// Stop before the browser runs out of memory (4 bytes per node).
pub const MAX_NODES: usize = 60_000_000;

#[derive(Clone, Copy, Debug)]
pub struct JawParams {
    /// Grid spacing in mm. Smaller = finer and slower.
    pub voxel: f32,
    /// Extra room around the part in mm, per side.
    pub clearance: f32,
    /// Space between the two jaws at the parting plane, in mm.
    pub gap: f32,
    /// Jaw length along X, mm.
    pub width: f32,
    /// Jaw height along Z, mm.
    pub height: f32,
    /// Jaw depth along Y (each jaw), mm.
    pub thickness: f32,
}

impl Default for JawParams {
    fn default() -> Self {
        JawParams {
            voxel: 0.4,
            clearance: 0.05,
            gap: 0.5,
            width: 152.4,
            height: 38.1,
            thickness: 25.4,
        }
    }
}

pub struct JawResult {
    pub a: Mesh,
    pub b: Mesh,
    pub grid_dims: [usize; 3],
}

fn sd_box(p: V3, lo: V3, hi: V3) -> f32 {
    let mut q = [0.0f32; 3];
    for d in 0..3 {
        let c = 0.5 * (lo[d] + hi[d]);
        let e = 0.5 * (hi[d] - lo[d]);
        q[d] = (p[d] - c).abs() - e;
    }
    let outside = (q[0].max(0.0).powi(2) + q[1].max(0.0).powi(2) + q[2].max(0.0).powi(2)).sqrt();
    let inside = q[0].max(q[1]).max(q[2]).min(0.0);
    outside + inside
}

pub fn generate(pos: &[f32], idx: &[u32], p: &JawParams) -> Result<JawResult, String> {
    if idx.len() < 3 || pos.len() < 9 {
        return Err("The part has no triangles.".into());
    }
    if !(p.voxel > 0.0) {
        return Err("Resolution must be greater than 0.".into());
    }
    if p.clearance < 0.0 || p.gap < 0.0 {
        return Err("Clearance and gap can't be negative.".into());
    }
    if p.width <= 0.0 || p.height <= 0.0 || p.thickness <= 0.0 {
        return Err("Jaw size must be greater than 0.".into());
    }

    let h = p.voxel;
    let pad = 2.0 * h;
    let half_gap = 0.5 * p.gap;
    let y_out = half_gap + p.thickness;

    let part_min_z = pos.chunks(3).map(|v| v[2]).fold(f32::INFINITY, f32::min);
    let z_lo = (-p.height).min(part_min_z - p.clearance) - pad;

    let lo = [-0.5 * p.width - pad, -y_out - pad, z_lo];
    let hi = [0.5 * p.width + pad, y_out + pad, pad];
    let n = [
        ((hi[0] - lo[0]) / h).ceil() as usize + 1,
        ((hi[1] - lo[1]) / h).ceil() as usize + 1,
        ((hi[2] - lo[2]) / h).ceil() as usize + 1,
    ];
    let nodes = n[0] * n[1] * n[2];
    if nodes > MAX_NODES {
        return Err(format!(
            "Grid too large ({} x {} x {} = {:.1}M points). Increase the resolution value or shrink the jaws.",
            n[0], n[1], n[2], nodes as f64 / 1e6
        ));
    }

    // Part SDF -> grow by clearance -> sweep up. The result is the pocket.
    let mut g = Grid::new(lo, h, n, 0.0);
    let band = p.clearance + 3.0 * h;
    mesh_to_sdf(&mut g, pos, idx, band);
    offset(&mut g, p.clearance);
    sweep_up(&mut g);

    let jaw = |y0: f32, y1: f32| -> Mesh {
        let blo = [-0.5 * p.width, y0, -p.height];
        let bhi = [0.5 * p.width, y1, 0.0];
        let field = |i: usize, j: usize, k: usize| {
            let q = g.pos(i, j, k);
            sd_box(q, blo, bhi).max(-g.data[g.idx(i, j, k)])
        };
        // Only mesh the slab of grid around this jaw.
        let (j0, j1) = g.range(1, y0 - pad, y1 + pad).unwrap_or((0, n[1] - 1));
        dual_contour(&g, field, [0, j0, 0], [n[0], j1 + 1, n[2]])
    };

    let a = jaw(half_gap, y_out);
    let b = jaw(-y_out, -half_gap);
    Ok(JawResult { a, b, grid_dims: n })
}
