//! Rough timing at the default 6" vise jaw size.
use softjaw_core::{generate, JawParams};
use std::time::Instant;

fn main() {
    // A 40 mm cylinder-ish part as a 256-sided prism, 30 mm tall, sunk 15 mm.
    let (seg, r, z0, z1) = (256usize, 20.0f32, -15.0f32, 15.0f32);
    let mut pos = vec![0.0, 0.0, z0, 0.0, 0.0, z1];
    for i in 0..seg {
        let a = 2.0 * std::f32::consts::PI * i as f32 / seg as f32;
        pos.extend_from_slice(&[r * a.cos(), r * a.sin(), z0, r * a.cos(), r * a.sin(), z1]);
    }
    let mut idx = Vec::new();
    for i in 0..seg as u32 {
        let (b0, t0) = (2 + 2 * i, 3 + 2 * i);
        let n = (i + 1) % seg as u32;
        let (b1, t1) = (2 + 2 * n, 3 + 2 * n);
        idx.extend_from_slice(&[0, b1, b0, 1, t0, t1, b0, b1, t1, b0, t1, t0]);
    }
    for voxel in [0.8f32, 0.4, 0.25] {
        let p = JawParams { voxel, ..Default::default() };
        let t = Instant::now();
        let r = generate(&pos, &idx, &p).unwrap();
        println!(
            "voxel {voxel} mm: grid {:?}, {:.2}s, {} + {} tris",
            r.grid_dims, t.elapsed().as_secs_f32(), r.a.tris.len(), r.b.tris.len()
        );
    }
}
