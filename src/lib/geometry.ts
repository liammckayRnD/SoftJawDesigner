import { Matrix4, Vector3 } from 'three';

export interface Box3Like { min: [number, number, number]; max: [number, number, number] }

export function bounds(pos: Float32Array): Box3Like {
  const min: [number, number, number] = [Infinity, Infinity, Infinity];
  const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
  for (let i = 0; i < pos.length; i += 3) {
    for (let d = 0; d < 3; d++) {
      const v = pos[i + d];
      if (v < min[d]) min[d] = v;
      if (v > max[d]) max[d] = v;
    }
  }
  return { min, max };
}

/**
 * Rotate the part, then place it in the jaw frame:
 * XY bounding box centered on the origin (parting plane is Y = 0),
 * bottom `depth` mm below the jaw tops (Z = 0).
 */
export function placePart(raw: Float32Array, rotation: Matrix4, depth: number): Float32Array {
  const out = new Float32Array(raw.length);
  const v = new Vector3();
  for (let i = 0; i < raw.length; i += 3) {
    v.set(raw[i], raw[i + 1], raw[i + 2]).applyMatrix4(rotation);
    out[i] = v.x; out[i + 1] = v.y; out[i + 2] = v.z;
  }
  const b = bounds(out);
  const sx = -(b.min[0] + b.max[0]) / 2;
  const sy = -(b.min[1] + b.max[1]) / 2;
  const sz = -b.min[2] - depth;
  for (let i = 0; i < out.length; i += 3) {
    out[i] += sx; out[i + 1] += sy; out[i + 2] += sz;
  }
  return out;
}

/** Binary or ASCII STL -> unindexed triangles. */
export function parseStl(buf: ArrayBuffer): { positions: Float32Array; indices: Uint32Array } {
  const dv = new DataView(buf);
  const n = buf.byteLength >= 84 ? dv.getUint32(80, true) : 0;
  let positions: Float32Array;
  if (buf.byteLength === 84 + n * 50 && n > 0) {
    positions = new Float32Array(n * 9);
    for (let t = 0; t < n; t++) {
      const o = 84 + t * 50 + 12; // skip the facet normal
      for (let k = 0; k < 9; k++) positions[t * 9 + k] = dv.getFloat32(o + k * 4, true);
    }
  } else {
    const text = new TextDecoder().decode(buf);
    const nums: number[] = [];
    const re = /vertex\s+(\S+)\s+(\S+)\s+(\S+)/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text))) nums.push(+m[1], +m[2], +m[3]);
    if (nums.length < 9) throw new Error('This STL file has no triangles.');
    positions = new Float32Array(nums);
  }
  const indices = new Uint32Array(positions.length / 3);
  for (let i = 0; i < indices.length; i++) indices[i] = i;
  return { positions, indices };
}
