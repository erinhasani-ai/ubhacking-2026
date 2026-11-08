<script lang="ts">
  // Zoom in, zoom out and back to the overview, for both maps. Keys: + and − zoom, V returns to the overview.
  let { zoomIn, zoomOut, overview }: { zoomIn(): void; zoomOut(): void; overview(): void } = $props();
</script>

<div class="map-controls" role="group" aria-label="Map view">
  <button onclick={zoomIn} aria-label="Zoom in" title="Zoom in (+)">
    <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 4.5v11M4.5 10h11" /></svg>
  </button>
  <button onclick={zoomOut} aria-label="Zoom out" title="Zoom out (−)">
    <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M4.5 10h11" /></svg>
  </button>
  <span class="rule" aria-hidden="true"></span>
  <button onclick={overview} aria-label="Overview" title="Overview (V)">
    <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M3.5 7.5v-4h4M12.5 3.5h4v4M16.5 12.5v4h-4M7.5 16.5h-4v-4" /></svg>
  </button>
</div>

<style>
  .map-controls {
    position: fixed;
    right: var(--gutter);
    bottom: 6.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 0.25rem;
    border: 1px solid var(--hairline);
    border-radius: 14px;
    background: var(--panel);
    backdrop-filter: blur(18px) saturate(140%);
    -webkit-backdrop-filter: blur(18px) saturate(140%);
    pointer-events: auto;
    z-index: 4;
  }
  button {
    display: grid;
    place-items: center;
    width: var(--tap);
    height: var(--tap);
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--ink);
    cursor: pointer;
    transition:
      background-color 140ms var(--ease-out),
      transform 100ms var(--ease-out);
  }
  svg {
    width: 1.15rem;
    height: 1.15rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .rule {
    width: 60%;
    height: 1px;
    margin: 0.15rem 0;
    background: var(--hairline);
  }
  @media (hover: hover) and (pointer: fine) {
    button:hover {
      background: var(--wash);
      color: var(--accent);
    }
  }
  button:active {
    background: var(--wash-press);
  }
  @media (prefers-reduced-motion: no-preference) {
    button:active {
      transform: scale(0.94);
    }
  }
</style>
