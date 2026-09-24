//! Binary STL writer.

use crate::dc::Mesh;
use crate::grid::{cross, normalize, sub};

pub fn to_binary_stl(mesh: &Mesh, name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + mesh.tris.len() * 50);
    let mut header = [0u8; 80];
    let tag = format!("softjaw {}", name);
    let n = tag.len().min(80);
    header[..n].copy_from_slice(&tag.as_bytes()[..n]);
    out.extend_from_slice(&header);
    out.extend_from_slice(&(mesh.tris.len() as u32).to_le_bytes());
    for t in &mesh.tris {
        let a = mesh.verts[t[0] as usize];
        let b = mesh.verts[t[1] as usize];
        let c = mesh.verts[t[2] as usize];
        let nrm = normalize(cross(sub(b, a), sub(c, a)));
        for v in [nrm, a, b, c] {
            for x in v {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}
