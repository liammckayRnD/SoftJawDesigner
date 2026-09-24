use softjaw_core::dc::Mesh;
use softjaw_core::grid::{cross, dot};
use softjaw_core::{generate, JawParams};
use std::collections::HashMap;

/// Axis-aligned box as 12 outward-facing triangles.
fn box_mesh(lo: [f32; 3], hi: [f32; 3]) -> (Vec<f32>, Vec<u32>) {
    let mut pos = Vec::new();
    for c in 0..8 {
        pos.push(if c & 1 != 0 { hi[0] } else { lo[0] });
        pos.push(if c & 2 != 0 { hi[1] } else { lo[1] });
        pos.push(if c & 4 != 0 { hi[2] } else { lo[2] });
    }
    let idx = vec![
        0, 2, 1, 1, 2, 3, // -z
        4, 5, 6, 5, 7, 6, // +z
        0, 1, 4, 1, 5, 4, // -y
        2, 6, 3, 3, 6, 7, // +y
        0, 4, 2, 2, 4, 6, // -x
        1, 3, 5, 3, 7, 5, // +x
    ];
    (pos, idx)
}

/// UV sphere, outward winding.
fn sphere_mesh(c: [f32; 3], r: f32, seg: usize) -> (Vec<f32>, Vec<u32>) {
    let rings = seg / 2;
    let mut pos = Vec::new();
    for i in 0..=rings {
        let th = std::f32::consts::PI * i as f32 / rings as f32;
        for j in 0..seg {
            let ph = 2.0 * std::f32::consts::PI * j as f32 / seg as f32;
            pos.extend_from_slice(&[
                c[0] + r * th.sin() * ph.cos(),
                c[1] + r * th.sin() * ph.sin(),
                c[2] + r * th.cos(),
            ]);
        }
    }
    let mut idx = Vec::new();
    let at = |i: usize, j: usize| (i * seg + j % seg) as u32;
    for i in 0..rings {
        for j in 0..seg {
            let (a, b, cc, d) = (at(i, j), at(i, j + 1), at(i + 1, j), at(i + 1, j + 1));
            idx.extend_from_slice(&[a, cc, b, b, cc, d]);
        }
    }
    (pos, idx)
}

fn volume(m: &Mesh) -> f64 {
    m.tris
        .iter()
        .map(|t| {
            let (a, b, c) = (m.verts[t[0] as usize], m.verts[t[1] as usize], m.verts[t[2] as usize]);
            dot(a, cross(b, c)) as f64 / 6.0
        })
        .sum()
}

/// Every directed edge must be matched by exactly one reverse edge.
fn assert_closed(m: &Mesh, name: &str) {
    let mut e: HashMap<(u32, u32), i32> = HashMap::new();
    for t in &m.tris {
        for s in 0..3 {
            let (a, b) = (t[s], t[(s + 1) % 3]);
            *e.entry((a, b)).or_default() += 1;
        }
    }
    let mut bad = 0;
    for (&(a, b), &n) in &e {
        if n != 1 || e.get(&(b, a)).copied().unwrap_or(0) != 1 {
            bad += 1;
        }
    }
    // Dual contouring can make rare non-manifold spots at ambiguous cells.
    let frac = bad as f64 / e.len() as f64;
    assert!(frac < 1e-3, "{name}: {bad} of {} edges not closed", e.len());
}

#[test]
fn box_part_pocket_volume() {
    let p = JawParams { voxel: 0.5, clearance: 0.1, gap: 0.5, width: 80.0, height: 30.0, thickness: 20.0 };
    let depth = 12.0;
    // 20 x 20 x 30 block centered on the parting plane, sunk 12 mm.
    let (pos, idx) = box_mesh([-10.0, -10.0, -depth], [10.0, 10.0, 18.0]);
    let r = generate(&pos, &idx, &p).unwrap();
    for (m, name) in [(&r.a, "A"), (&r.b, "B")] {
        assert_closed(m, name);
        let v = volume(m);
        let jaw = (p.width * p.thickness * p.height) as f64;
        let c = p.clearance;
        let pocket = ((10.0 + c - p.gap / 2.0) * (20.0 + 2.0 * c) * (depth + c)) as f64;
        let expect = jaw - pocket;
        let err = (v - expect).abs() / expect;
        println!("jaw {name}: vol {v:.1} expect {expect:.1} err {:.4}% tris {}", err * 100.0, m.tris.len());
        assert!(v > 0.0, "{name} is inside out");
        assert!(err < 0.005, "{name} volume off by {:.3}%", err * 100.0);
    }
}

#[test]
fn sphere_part_is_swept_not_undercut() {
    // A sphere with its center 5 mm below the jaw top. Without the sweep,
    // the pocket would trap it. With the sweep, the pocket above the equator
    // is a straight cylinder.
    let p = JawParams { voxel: 0.4, clearance: 0.2, gap: 0.5, width: 60.0, height: 25.0, thickness: 20.0 };
    let (r0, zc) = (10.0f32, -5.0f32);
    let (pos, idx) = sphere_mesh([0.0, 0.0, zc], r0, 96);
    let r = generate(&pos, &idx, &p).unwrap();
    assert_closed(&r.a, "A");

    // Monte Carlo the expected jaw A volume.
    let rr = r0 + p.clearance;
    let (x0, x1) = (-p.width / 2.0, p.width / 2.0);
    let (y0, y1) = (p.gap / 2.0, p.gap / 2.0 + p.thickness);
    let (z0, z1) = (-p.height, 0.0);
    let mut s: u64 = 12345;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let n = 2_000_000;
    let mut hit = 0;
    for _ in 0..n {
        let x = x0 as f64 + rnd() * (x1 - x0) as f64;
        let y = y0 as f64 + rnd() * (y1 - y0) as f64;
        let z = z0 as f64 + rnd() * (z1 - z0) as f64;
        let rxy2 = x * x + y * y;
        let dz = z - zc as f64;
        let in_pocket = rxy2 + dz * dz <= (rr * rr) as f64 || (dz >= 0.0 && rxy2 <= (rr * rr) as f64);
        if !in_pocket {
            hit += 1;
        }
    }
    let box_v = ((x1 - x0) * (y1 - y0) * (z1 - z0)) as f64;
    let expect = box_v * hit as f64 / n as f64;
    let v = volume(&r.a);
    let err = (v - expect).abs() / expect;
    println!("sphere jaw A: vol {v:.1} expect {expect:.1} err {:.3}%", err * 100.0);
    assert!(err < 0.005);
}

#[test]
fn rejects_huge_grid() {
    let p = JawParams { voxel: 0.01, ..Default::default() };
    let (pos, idx) = box_mesh([-5.0; 3], [5.0; 3]);
    assert!(generate(&pos, &idx, &p).is_err());
}
