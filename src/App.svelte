<script lang="ts">
  import { untrack } from 'svelte';
  import { Matrix4 } from 'three';
  import Viewer from './lib/Viewer.svelte';
  import { bounds, placePart } from './lib/geometry';
  import type { FromWorker, JawParams, ToWorker } from './lib/types';
  import GeomWorker from './lib/worker.ts?worker';

  const MAX_POINTS = 60e6; // matches MAX_NODES in core/src/jaw.rs
  const QUALITY = { draft: 0.8, standard: 0.4, fine: 0.25 } as const;
  type Quality = keyof typeof QUALITY;

  // Part
  let fileName = $state('');
  let raw = $state.raw<Float32Array | null>(null);
  let indices = $state.raw<Uint32Array | null>(null);
  let rotation = $state.raw(new Matrix4());
  let depth = $state(10);

  // Jaws (defaults: 6 in vise, 1.5 in tall, 1 in thick)
  let jaw = $state({ width: 152.4, height: 38.1, thickness: 25.4, gap: 0.5, clearance: 0.05 });
  let quality = $state<Quality>('standard');
  const params: JawParams = $derived({ ...jaw, voxel: QUALITY[quality] });

  // Run state
  let status = $state<'idle' | 'loading' | 'working' | 'done' | 'error'>('idle');
  let message = $state('');
  let jawA = $state.raw<ArrayBuffer | null>(null);
  let jawB = $state.raw<ArrayBuffer | null>(null);
  let stats = $state<{ trisA: number; trisB: number; ms: number } | null>(null);
  let showPart = $state(true);
  let dragging = $state(false);
  let fileInput: HTMLInputElement;

  const placed = $derived(raw ? placePart(raw, rotation, depth) : null);
  const size = $derived.by(() => {
    if (!placed) return null;
    const b = bounds(placed);
    return [0, 1, 2].map((d) => b.max[d] - b.min[d]);
  });

  const points = $derived.by(() => {
    const h = params.voxel, pad = 4 * h;
    const nx = (params.width + pad) / h + 1;
    const ny = (2 * (params.gap / 2 + params.thickness) + pad) / h + 1;
    const nz = (Math.max(params.height, depth + params.clearance) + pad) / h + 1;
    return nx * ny * nz;
  });

  const warnings = $derived.by(() => {
    const w: string[] = [];
    if (params.gap <= 2 * params.clearance)
      w.push('Gap should be more than twice the clearance, or the jaws will close on each other before they grip.');
    if (points > MAX_POINTS)
      w.push(`This needs about ${(points / 1e6).toFixed(0)}M grid points, over the ${MAX_POINTS / 1e6}M limit. Pick a coarser quality or smaller jaws.`);
    if (size) {
      if (depth > params.height) w.push('The part sits deeper than the jaws are tall. The pocket will cut through the bottom.');
      if (size[0] + 2 * params.clearance > params.width) w.push('The part is longer than the jaws. The pocket will run out the ends.');
      if (size[1] / 2 + params.clearance > params.gap / 2 + params.thickness)
        w.push('The part is deeper than the jaws front to back. The pocket will cut through the back.');
    }
    if (depth <= 0) w.push('Depth must be more than 0.');
    return w;
  });

  const blocking = $derived(!placed || depth <= 0 || points > MAX_POINTS || status === 'working' || status === 'loading');

  // Any change to the inputs makes old jaws stale.
  $effect(() => {
    placed; JSON.stringify(params);
    untrack(() => {
      jawA = null; jawB = null; stats = null;
      if (status === 'done') status = 'idle';
    });
  });

  const worker = new GeomWorker();
  const send = (msg: ToWorker, transfer: Transferable[] = []) => worker.postMessage(msg, transfer);
  worker.onmessage = (e: MessageEvent<FromWorker>) => {
    const m = e.data;
    if (m.type === 'loaded') {
      rotation = new Matrix4();
      raw = m.positions;
      indices = m.indices;
      status = 'idle';
      message = '';
    } else if (m.type === 'done') {
      jawA = m.a; jawB = m.b;
      stats = { trisA: m.trisA, trisB: m.trisB, ms: m.ms };
      status = 'done';
      message = '';
    } else {
      status = 'error';
      message = m.message;
    }
  };

  async function load(file: File) {
    const ok = /\.(step|stp|stl)$/i.test(file.name);
    if (!ok) { status = 'error'; message = 'Use a STEP (.step, .stp) or STL file.'; return; }
    fileName = file.name;
    status = 'loading';
    message = 'Reading the part…';
    const buffer = await file.arrayBuffer();
    send({ type: 'load', name: file.name, buffer }, [buffer]);
  }

  function turn(axis: 'x' | 'y' | 'z') {
    const r = new Matrix4();
    if (axis === 'x') r.makeRotationX(Math.PI / 2);
    if (axis === 'y') r.makeRotationY(Math.PI / 2);
    if (axis === 'z') r.makeRotationZ(Math.PI / 2);
    rotation = r.multiply(rotation);
  }

  function makeJaws() {
    if (!placed || !indices) return;
    status = 'working';
    message = 'Making jaws…';
    const pos = placed.slice();
    const idx = indices.slice();
    send({ type: 'generate', positions: pos, indices: idx, params: { ...params } }, [pos.buffer, idx.buffer]);
  }

  function download(buf: ArrayBuffer, suffix: string) {
    const base = fileName.replace(/\.[^.]+$/, '') || 'part';
    const url = URL.createObjectURL(new Blob([buf], { type: 'model/stl' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${base}_${suffix}.stl`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  const fmt = (n: number) => n.toFixed(1);
  const k = (n: number) => (n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : `${Math.round(n / 1e3)}k`);
</script>

<svelte:window
  ondragover={(e) => { e.preventDefault(); dragging = true; }}
  ondragleave={(e) => { if (!e.relatedTarget) dragging = false; }}
  ondrop={(e) => {
    e.preventDefault();
    dragging = false;
    const f = e.dataTransfer?.files?.[0];
    if (f) load(f);
  }}
/>

<main class="app">
  <aside class="panel">
    <header>
      <h1>SoftJaw Designer</h1>
      <p class="lede">Load a part. Get a pair of vise jaws that fit it.</p>
    </header>

    <section>
      <h2><span class="n">1</span>Load part</h2>
      <button class="drop" onclick={() => fileInput.click()} disabled={status === 'loading'}>
        {#if fileName}
          <strong>{fileName}</strong>
          {#if size}<span>{fmt(size[0])} × {fmt(size[1])} × {fmt(size[2])} mm</span>{/if}
          <span class="muted">Choose another file</span>
        {:else}
          <strong>Choose a STEP or STL file</strong>
          <span class="muted">or drop it anywhere</span>
        {/if}
      </button>
      <input
        bind:this={fileInput}
        type="file"
        accept=".step,.stp,.stl"
        hidden
        onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) load(f); e.currentTarget.value = ''; }}
      />
    </section>

    <section class:off={!raw}>
      <h2><span class="n">2</span>Orient</h2>
      <p class="muted small">The jaws clamp front to back. The part lifts out straight up.</p>
      <div class="row">
        <button class="ghost" onclick={() => turn('x')} disabled={!raw}>Turn about X</button>
        <button class="ghost" onclick={() => turn('y')} disabled={!raw}>Turn about Y</button>
        <button class="ghost" onclick={() => turn('z')} disabled={!raw}>Turn about Z</button>
      </div>
      <label class="field">
        <span>Depth in jaws</span>
        <input type="number" min="0.5" step="0.5" bind:value={depth} disabled={!raw} />
        <em>mm</em>
      </label>
    </section>

    <section>
      <h2><span class="n">3</span>Jaws</h2>
      <div class="grid">
        <label class="field"><span>Length</span><input type="number" min="1" step="0.1" bind:value={jaw.width} /><em>mm</em></label>
        <label class="field"><span>Height</span><input type="number" min="1" step="0.1" bind:value={jaw.height} /><em>mm</em></label>
        <label class="field"><span>Thickness</span><input type="number" min="1" step="0.1" bind:value={jaw.thickness} /><em>mm</em></label>
        <label class="field"><span>Gap</span><input type="number" min="0" step="0.05" bind:value={jaw.gap} /><em>mm</em></label>
        <label class="field"><span>Clearance</span><input type="number" min="0" step="0.01" bind:value={jaw.clearance} /><em>mm</em></label>
        <label class="field">
          <span>Quality</span>
          <select bind:value={quality}>
            <option value="draft">Draft (0.8 mm)</option>
            <option value="standard">Standard (0.4 mm)</option>
            <option value="fine">Fine (0.25 mm)</option>
          </select>
        </label>
      </div>
    </section>

    <section>
      <h2><span class="n">4</span>Make jaws</h2>
      {#each warnings as w}
        <p class="warn">{w}</p>
      {/each}
      <button class="primary" onclick={makeJaws} disabled={blocking}>
        {status === 'working' ? 'Making jaws…' : 'Make jaws'}
      </button>
      {#if status === 'error'}<p class="error" role="alert">{message}</p>{/if}
      {#if status === 'loading'}<p class="muted small" aria-live="polite">{message}</p>{/if}

      {#if jawA && jawB && stats}
        <div class="result" aria-live="polite">
          <p class="small muted">Done in {(stats.ms / 1000).toFixed(1)} s</p>
          <div class="row">
            <button class="ghost" onclick={() => download(jawA!, 'jaw_a')}>Download jaw A <span class="muted">{k(stats.trisA)} tris</span></button>
            <button class="ghost" onclick={() => download(jawB!, 'jaw_b')}>Download jaw B <span class="muted">{k(stats.trisB)} tris</span></button>
          </div>
          <label class="check"><input type="checkbox" bind:checked={showPart} /> Show part</label>
        </div>
      {/if}
    </section>
  </aside>

  <section class="stage" aria-label="3D view">
    <Viewer part={placed} {indices} jawParams={params} {jawA} {jawB} {showPart} />
    {#if !raw}
      <p class="hint">The outline shows the jaw blanks. Load a part to fit them.</p>
    {/if}
    {#if dragging}<div class="dropping">Drop to load</div>{/if}
  </section>
</main>

<style>
  .app {
    display: grid;
    grid-template-columns: 340px 1fr;
    height: 100%;
  }
  .panel {
    background: var(--panel);
    border-right: 1px solid var(--line);
    overflow-y: auto;
    padding: 24px 22px 32px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  h1 {
    font-size: 1.6rem;
    font-stretch: 80%;
    font-weight: 700;
    letter-spacing: -0.01em;
    margin: 0;
  }
  .lede { margin: 4px 0 0; color: var(--muted); }
  h2 {
    font-size: 1rem;
    font-weight: 650;
    margin: 0 0 10px;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .n {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--dykem);
    color: #fff;
    font-size: 0.8rem;
    font-weight: 600;
  }
  section.off { opacity: 0.55; }
  .muted { color: var(--muted); }
  .small { font-size: 0.87rem; margin: 0 0 10px; }

  button {
    cursor: pointer;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    background: #fff;
    padding: 7px 11px;
  }
  button:disabled { cursor: default; opacity: 0.5; }
  .ghost:hover:not(:disabled) { border-color: var(--muted); }
  .primary {
    width: 100%;
    padding: 11px;
    background: var(--dykem);
    border-color: var(--dykem);
    color: #fff;
    font-weight: 600;
  }
  .primary:hover:not(:disabled) { filter: brightness(1.1); }

  .drop {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: flex-start;
    padding: 16px;
    border: 1.5px dashed var(--muted);
    background: transparent;
    text-align: left;
  }
  .drop:hover:not(:disabled) { border-color: var(--dykem); background: var(--dykem-soft); }

  .row { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 12px; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px 12px; }
  .field { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 2px 6px; }
  .field span { grid-column: 1 / -1; font-size: 0.85rem; color: var(--muted); }
  .field input, .field select {
    width: 100%;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 6px 8px;
    background: #fff;
  }
  .field select { grid-column: 1 / -1; }
  .field em { font-style: normal; font-size: 0.85rem; color: var(--muted); }

  .warn {
    background: var(--warn-soft);
    color: var(--warn);
    border-left: 3px solid var(--warn);
    padding: 7px 10px;
    margin: 0 0 8px;
    font-size: 0.87rem;
  }
  .error { color: #a3261d; font-size: 0.9rem; }
  .result { margin-top: 14px; }
  .result .muted { font-size: 0.8rem; margin-left: 4px; }
  .check { display: flex; gap: 8px; align-items: center; font-size: 0.9rem; }

  .stage { position: relative; min-height: 360px; }
  .hint {
    position: absolute;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    margin: 0;
    color: var(--muted);
    pointer-events: none;
  }
  .dropping {
    position: absolute;
    inset: 16px;
    border: 2px dashed var(--dykem);
    border-radius: 10px;
    background: color-mix(in srgb, var(--dykem-soft) 70%, transparent);
    display: grid;
    place-items: center;
    font-weight: 600;
    color: var(--dykem);
    pointer-events: none;
  }

  @media (max-width: 760px) {
    .app { grid-template-columns: 1fr; grid-template-rows: auto 60vh; height: auto; }
    .panel { border-right: 0; border-bottom: 1px solid var(--line); }
  }
</style>
