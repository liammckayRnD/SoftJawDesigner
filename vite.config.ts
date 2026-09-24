import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  // Relative base so the build works at https://<user>.github.io/<repo>/
  base: './',
  plugins: [svelte()],
  worker: { format: 'es' },
  build: { target: 'es2022' },
});
