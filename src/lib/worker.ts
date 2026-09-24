/// <reference lib="webworker" />
// Heavy work runs here so the page stays responsive:
// STEP import (OpenCASCADE via occt-import-js) and jaw generation (Rust core).
import type { FromWorker, ToWorker } from './types';
import { parseStl } from './geometry';
import occtWasmUrl from 'occt-import-js/dist/occt-import-js.wasm?url';
import initCore, { generate_jaws } from '../../core/pkg/softjaw_core.js';

const post = (msg: FromWorker, transfer: Transferable[] = []) =>
  (self as unknown as DedicatedWorkerGlobalScope).postMessage(msg, transfer);

let occtPromise: Promise<any> | null = null;
function occt() {
  occtPromise ??= import('occt-import-js').then((m: any) =>
    (m.default ?? m)({ locateFile: () => occtWasmUrl }),
  );
  return occtPromise;
}

let corePromise: Promise<unknown> | null = null;
const core = () => (corePromise ??= initCore());

async function loadStep(buffer: ArrayBuffer) {
  const o = await occt();
  const r = o.ReadStepFile(new Uint8Array(buffer), {
    linearUnit: 'millimeter',
    linearDeflectionType: 'absolute_value',
    linearDeflection: 0.02,
    angularDeflection: 0.2,
  });
  if (!r.success || !r.meshes?.length) throw new Error('Could not read this STEP file. Check that it holds a solid body.');
  let nv = 0, ni = 0;
  for (const m of r.meshes) { nv += m.attributes.position.array.length; ni += m.index.array.length; }
  const positions = new Float32Array(nv);
  const indices = new Uint32Array(ni);
  let pv = 0, pi = 0;
  for (const m of r.meshes) {
    const base = pv / 3;
    positions.set(m.attributes.position.array, pv);
    for (const i of m.index.array) indices[pi++] = i + base;
    pv += m.attributes.position.array.length;
  }
  return { positions, indices };
}

self.onmessage = async (e: MessageEvent<ToWorker>) => {
  const msg = e.data;
  try {
    if (msg.type === 'load') {
      const lower = msg.name.toLowerCase();
      const part = lower.endsWith('.stl') ? parseStl(msg.buffer) : await loadStep(msg.buffer);
      post({ type: 'loaded', ...part }, [part.positions.buffer, part.indices.buffer]);
    } else if (msg.type === 'generate') {
      await core();
      const p = msg.params;
      const t0 = performance.now();
      const out = generate_jaws(msg.positions, msg.indices, p.voxel, p.clearance, p.gap, p.width, p.height, p.thickness);
      const a = out.jaw_a(), b = out.jaw_b();
      const res: FromWorker = {
        type: 'done',
        a: a.buffer as ArrayBuffer,
        b: b.buffer as ArrayBuffer,
        trisA: out.tris_a(),
        trisB: out.tris_b(),
        grid: Array.from(out.grid()),
        ms: performance.now() - t0,
      };
      out.free();
      post(res, [res.a, res.b]);
    }
  } catch (err) {
    post({ type: 'error', message: err instanceof Error ? err.message : String(err) });
  }
};
