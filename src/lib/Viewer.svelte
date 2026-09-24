<script lang="ts">
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
  import { STLLoader } from 'three/addons/loaders/STLLoader.js';
  import type { JawParams } from './types';

  let {
    part = null,
    indices = null,
    jawParams,
    jawA = null,
    jawB = null,
    showPart = true,
  }: {
    part: Float32Array | null;
    indices: Uint32Array | null;
    jawParams: JawParams;
    jawA: ArrayBuffer | null;
    jawB: ArrayBuffer | null;
    showPart: boolean;
  } = $props();

  let host: HTMLDivElement;
  let renderer: THREE.WebGLRenderer;
  let camera: THREE.PerspectiveCamera;
  let controls: OrbitControls;
  let ready = $state(false);
  let floor: THREE.GridHelper;
  const scene = new THREE.Scene();
  const partGroup = new THREE.Group();
  const ghostGroup = new THREE.Group();
  const jawGroup = new THREE.Group();
  scene.add(partGroup, ghostGroup, jawGroup);

  const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const partMat = new THREE.MeshStandardMaterial({ color: '#2440b3', roughness: 0.55, metalness: 0.05, side: THREE.DoubleSide });
  const jawMat = new THREE.MeshStandardMaterial({ color: '#b9c0c6', roughness: 0.42, metalness: 0.55 });
  const ghostMat = new THREE.LineBasicMaterial({ color: '#7d868e' });

  function clear(g: THREE.Group) {
    for (const c of [...g.children]) {
      g.remove(c);
      (c as THREE.Mesh).geometry?.dispose();
    }
  }

  function frame() {
    const box = new THREE.Box3();
    for (const g of [partGroup, ghostGroup, jawGroup]) if (g.children.length) box.expandByObject(g);
    if (box.isEmpty()) return;
    const c = box.getCenter(new THREE.Vector3());
    const r = box.getSize(new THREE.Vector3()).length() * 0.5;
    const dist = r / Math.sin((camera.fov * Math.PI) / 360);
    camera.position.copy(c).add(new THREE.Vector3(0.9, -1.4, 1.0).normalize().multiplyScalar(dist * 1.05));
    camera.near = dist / 100;
    camera.far = dist * 10;
    camera.updateProjectionMatrix();
    controls.target.copy(c);
    controls.update();
  }

  // Part geometry
  let lastPartLength = -1;
  $effect(() => {
    if (!ready) return;
    clear(partGroup);
    if (!part || !indices) return;
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(part, 3));
    geo.setIndex(new THREE.BufferAttribute(indices, 1));
    geo.computeVertexNormals();
    partGroup.add(new THREE.Mesh(geo, partMat));
    if (part.length !== lastPartLength) {
      lastPartLength = part.length;
      frame();
    }
  });

  // Outline of the jaw blanks, shown until jaws are generated.
  $effect(() => {
    if (!ready) return;
    clear(ghostGroup);
    const { width: w, height: h, thickness: t, gap } = jawParams;
    floor.position.z = -h - 0.01;
    if (jawA) return;
    for (const s of [1, -1]) {
      const geo = new THREE.EdgesGeometry(new THREE.BoxGeometry(w, t, h));
      const lines = new THREE.LineSegments(geo, ghostMat);
      lines.position.set(0, s * (gap / 2 + t / 2), -h / 2);
      ghostGroup.add(lines);
    }
    if (!part) frame();
  });

  // Generated jaws
  $effect(() => {
    if (!ready) return;
    clear(jawGroup);
    const loader = new STLLoader();
    for (const buf of [jawA, jawB]) {
      if (!buf) continue;
      const geo = loader.parse(buf);
      geo.computeVertexNormals();
      jawGroup.add(new THREE.Mesh(geo, jawMat));
    }
  });

  $effect(() => {
    partGroup.visible = showPart;
    partMat.transparent = !!jawA;
    partMat.opacity = jawA ? 0.85 : 1;
  });

  onMount(() => {
    renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    host.appendChild(renderer.domElement);

    camera = new THREE.PerspectiveCamera(35, 1, 0.1, 10000);
    camera.up.set(0, 0, 1); // Z up, like the machine
    camera.position.set(120, -200, 150);
    controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;

    scene.add(new THREE.HemisphereLight('#ffffff', '#8a939b', 1.6));
    const key = new THREE.DirectionalLight('#ffffff', 2.2);
    key.position.set(1, -2, 3);
    scene.add(key);
    // 10 mm floor grid under the jaws, for scale.
    floor = new THREE.GridHelper(400, 40, css('--line') || '#c5cbd0', css('--line') || '#c5cbd0');
    floor.rotation.x = Math.PI / 2;
    scene.add(floor);

    const resize = () => {
      const { clientWidth: w, clientHeight: h } = host;
      renderer.setSize(w, h, false);
      camera.aspect = w / Math.max(h, 1);
      camera.updateProjectionMatrix();
    };
    const ro = new ResizeObserver(resize);
    ro.observe(host);
    resize();

    let raf = 0;
    const loop = () => {
      controls.update();
      renderer.render(scene, camera);
      raf = requestAnimationFrame(loop);
    };
    loop();
    ready = true;

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      controls.dispose();
      renderer.dispose();
    };
  });
</script>

<div class="viewer" bind:this={host}></div>

<style>
  .viewer {
    position: absolute;
    inset: 0;
  }
  .viewer :global(canvas) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
