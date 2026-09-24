//! Dev helper: run the pipeline on raw f32 positions / u32 indices and write STLs.
//! cargo run --release --example from_raw -- pos.bin idx.bin out_dir
use softjaw_core::stl::to_binary_stl;
use softjaw_core::{generate, JawParams};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pb = std::fs::read(&a[1]).unwrap();
    let ib = std::fs::read(&a[2]).unwrap();
    let pos: Vec<f32> = pb.chunks(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
    let idx: Vec<u32> = ib.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    let p = JawParams { width: 100.0, height: 30.0, thickness: 25.0, voxel: 0.3, ..Default::default() };
    let t = std::time::Instant::now();
    let r = generate(&pos, &idx, &p).unwrap();
    println!("grid {:?} in {:.2}s, tris {} + {}", r.grid_dims, t.elapsed().as_secs_f32(), r.a.tris.len(), r.b.tris.len());
    std::fs::write(format!("{}/jaw_a.stl", a[3]), to_binary_stl(&r.a, "jaw A")).unwrap();
    std::fs::write(format!("{}/jaw_b.stl", a[3]), to_binary_stl(&r.b, "jaw B")).unwrap();
}
