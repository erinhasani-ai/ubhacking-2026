import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [svelte()],
  // Three.js is most of the one bundle; the UI is served from the laptop, so one ~620 kB chunk is fine.
  // Two pages: the Room Twin (index.html) and the campus simulation (campus.html, T32).
  build: { chunkSizeWarningLimit: 800, rollupOptions: { input: { main: 'index.html', campus: 'campus.html' } } },
  // In `vite dev`, campus voice calls go to the running engine.
  server: { proxy: { '/api': 'http://127.0.0.1:7700' } },
});
