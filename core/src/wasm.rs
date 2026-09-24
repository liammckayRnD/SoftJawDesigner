//! Browser bindings.

use crate::jaw::{generate, JawParams};
use crate::stl::to_binary_stl;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct JawOutput {
    a: Vec<u8>,
    b: Vec<u8>,
    tris_a: u32,
    tris_b: u32,
    grid: Vec<u32>,
}

#[wasm_bindgen]
impl JawOutput {
    /// Binary STL for the +Y jaw.
    pub fn jaw_a(&self) -> Vec<u8> {
        self.a.clone()
    }
    /// Binary STL for the -Y jaw.
    pub fn jaw_b(&self) -> Vec<u8> {
        self.b.clone()
    }
    pub fn tris_a(&self) -> u32 {
        self.tris_a
    }
    pub fn tris_b(&self) -> u32 {
        self.tris_b
    }
    /// Grid size used, [nx, ny, nz].
    pub fn grid(&self) -> Vec<u32> {
        self.grid.clone()
    }
}

/// `positions`: flat xyz, already placed in the jaw frame.
/// `indices`: flat triangle indices.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn generate_jaws(
    positions: &[f32],
    indices: &[u32],
    voxel: f32,
    clearance: f32,
    gap: f32,
    width: f32,
    height: f32,
    thickness: f32,
) -> Result<JawOutput, JsValue> {
    let p = JawParams { voxel, clearance, gap, width, height, thickness };
    let r = generate(positions, indices, &p).map_err(|e| JsValue::from_str(&e))?;
    Ok(JawOutput {
        tris_a: r.a.tris.len() as u32,
        tris_b: r.b.tris.len() as u32,
        a: to_binary_stl(&r.a, "jaw A"),
        b: to_binary_stl(&r.b, "jaw B"),
        grid: r.grid_dims.iter().map(|&x| x as u32).collect(),
    })
}
