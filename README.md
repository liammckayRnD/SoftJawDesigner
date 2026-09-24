# SoftJaw Designer

Load a part (STEP or STL). Get a pair of vise soft jaws that fit it, as STL.

Everything runs in the browser. The site is static and hosts free on GitHub Pages.

## How it works

1. **Import.** STEP files are tessellated by OpenCASCADE (`occt-import-js`, WASM). STL is read directly.
2. **Place.** The part is turned by the user, centered on the parting plane, and sunk to the chosen depth.
3. **Jaws** (Rust core in `core/`, compiled to WASM):
   - Mesh → band-limited signed distance grid (exact point-triangle distance, sign by ray parity)
   - Grow by the clearance
   - Sweep straight up, so the part can lift out (one running min per column)
   - Subtract the pocket from each jaw box
   - Dual contouring → watertight mesh with sharp edges → binary STL

Frame: X along the jaw length, Y is the clamp direction (parting plane at Y = 0), Z up (jaw tops at Z = 0). Jaw A is +Y, jaw B is −Y.

## Project layout

```
core/                 Rust geometry core
  src/sdf.rs          mesh -> SDF grid, offset, sweep
  src/dc.rs           dual contouring
  src/jaw.rs          the pipeline and its parameters
  src/wasm.rs         browser bindings (wasm-bindgen)
  tests/jaws.rs       closed-mesh and volume tests
  examples/           bench.rs (timing), from_raw.rs (dev helper)
src/                  Svelte app
  App.svelte          UI: load, orient, jaws, make
  lib/Viewer.svelte   Three.js view (Z up)
  lib/worker.ts       STEP import + jaw generation off the main thread
test-parts/           a sample STEP to try
.github/workflows/    test, build, deploy to Pages
```

## First-time setup on GitHub

1. Create a new repo and push this folder to `main`.
2. In the repo, open **Settings → Pages** and set **Source** to **GitHub Actions**.
3. Push again (or run the workflow from the Actions tab). The site appears at `https://<user>.github.io/<repo>/`.

## Local development

Needs Rust (via rustup), wasm-pack, and Node 22.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack        # or: curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
npm install
npm run wasm                   # builds core/pkg
npm run dev                    # http://localhost:5173
```

Run `npm run wasm` again after changing Rust code.

Core tests and timing:

```sh
cd core
cargo test --release
cargo run --release --example bench
```

## Current limits

- Output is mesh only (STL). No B-rep STEP yet.
- Flat faces are meshed at full grid resolution, so files are large (about 270k triangles per jaw at 0.4 mm).
- Single-threaded WASM. GitHub Pages can't set the headers needed for threads.
- The part must be a closed mesh for the sign test. STEP solids are.
