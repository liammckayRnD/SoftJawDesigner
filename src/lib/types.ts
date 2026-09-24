export interface JawParams {
  voxel: number;     // grid spacing, mm
  clearance: number; // per side, mm
  gap: number;       // between jaws at the parting plane, mm
  width: number;     // jaw length along X, mm
  height: number;    // along Z, mm
  thickness: number; // each jaw, along Y, mm
}

export type ToWorker =
  | { type: 'load'; name: string; buffer: ArrayBuffer }
  | { type: 'generate'; positions: Float32Array; indices: Uint32Array; params: JawParams };

export type FromWorker =
  | { type: 'loaded'; positions: Float32Array; indices: Uint32Array }
  | { type: 'done'; a: ArrayBuffer; b: ArrayBuffer; trisA: number; trisB: number; grid: number[]; ms: number }
  | { type: 'error'; message: string };
