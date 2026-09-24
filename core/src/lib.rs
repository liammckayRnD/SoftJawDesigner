//! softjaw-core: turns a part mesh into a pair of machinable soft jaws.
//!
//! Pipeline: mesh -> signed distance grid -> offset by clearance ->
//! sweep up (lift-out direction) -> subtract from each jaw box ->
//! dual contouring -> binary STL.

pub mod dc;
pub mod grid;
pub mod jaw;
pub mod sdf;
pub mod stl;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use jaw::{generate, JawParams, JawResult};
