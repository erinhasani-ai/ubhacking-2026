<script lang="ts">
  import { hud } from './hud.svelte';

  interface Actions {
    select(id: number, shift: boolean): void;
    reset(): void;
    abort(source: 'click' | 'key'): void;
    toggleFollow(): void;
    retryNow(): void;
    toggleHolo(): void;
    resetView(): void;
    sendPlace(place: string): void;
  }
  let { actions }: { actions: Actions } = $props();

  const endpoint = $derived(hud.url.replace(/^wss?:\/\//, '').replace(/\/ws$/, ''));
  const linkText = $derived(
    hud.link.state === 'open'
      ? 'Connected'
      : hud.link.state === 'connecting'
        ? 'Connecting'
        : `Reconnecting in ${hud.retryIn} s`,
  );
  const fmt = (v: number, k = 1) => v.toFixed(k).replace('-', '−');
  const selection = $derived(
    hud.selected.length === 0
      ? 'No Drones selected, so a click sends all of them'
      : `${hud.selected.length === 1 ? 'Drone' : 'Drones'} ${[...hud.selected].sort((a, b) => a - b).join(', ')} selected`,
  );
</script>

<header class="panel identity">
  <div class="brand">
    <h1 class="wordmark">Aether</h1>
    <span class="link" data-state={hud.link.state}>
      <span class="dot" aria-hidden="true"></span>
      <span>{linkText}</span>
      <span class="endpoint data">{endpoint}</span>
    </span>
    <a class="campus" href="campus.html">Campus</a>
  </div>
  {#if hud.room}
    <p class="room">
      <span class="tag" data-provenance={hud.room.provenance}>{hud.room.provenance}</span>
      <span>Davis Hall, 3rd floor</span>
      <span class="data">{fmt(hud.room.size[0])} × {fmt(hud.room.size[1])} × {fmt(hud.room.size[2])} m</span>
      {#if hud.room.points}
        <span class="sep" aria-hidden="true">·</span>
        <span><span class="data">{hud.room.points.toLocaleString('en-US')}</span> LiDAR samples</span>
      {/if}
    </p>
    <p class="note">{hud.room.processing ? 'Enhancing scan…' : hud.room.enhanced ? 'Enhanced point cloud' : 'Original point cloud'} · ceiling cut {fmt(hud.room.cut)} m</p>
    {#if hud.room.enhanced}
      <p class="note"><span class="data">{hud.room.drawn.toLocaleString('en-US')}</span> visual samples · cleaned and resampled</p>
    {/if}
  {/if}
</header>

<div class="column">
  <aside class="panel legend" aria-label="Provenance">
    <span class="item"><span class="tag" data-provenance="LIVE">LIVE</span>real inputs</span>
    <span class="item"><span class="tag" data-provenance="SIM">SIM</span>simulated</span>
  </aside>

  <section class="panel swarm" aria-labelledby="swarm-title">
    <h2 id="swarm-title">Swarm</h2>
    <div class="cols" aria-hidden="true">
      <span>Drone</span><span>Mode</span><span>Armed</span><span class="num">Link</span><span></span>
    </div>
    {#if hud.drones.length}
      <ul>
        {#each hud.drones as d (d.id)}
          <li>
            <button
              class="row"
              aria-pressed={hud.selected.includes(d.id)}
              class:stale={hud.stale}
              aria-label="Drone {d.id}, {d.mode}, {d.armed ? 'armed' : 'disarmed'}, {hud.stale ? 'link unknown, last known values' : `link ${d.link_ms} milliseconds`}, {d.provenance}"
              onclick={(e) => actions.select(d.id, e.shiftKey)}
            >
              <span class="id data">{d.id}</span>
              <span class="data">{d.mode}</span>
              <span class:muted={!d.armed}>{d.armed ? 'Armed' : 'Disarmed'}</span>
              <span class="num"><span class="data">{hud.stale ? '—' : d.link_ms}</span> ms</span>
              <span class="tag" data-provenance={d.provenance}>{d.provenance}</span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">Waiting for Telemetry from the Swarm</p>
    {/if}
    <p class="selection">{selection}</p>
    {#if hud.follow}
      <p class="selection">Camera follows {hud.selected.length ? 'the selected Drones' : 'the Swarm centre'}</p>
    {/if}
  </section>

  {#if hud.places.length}
    <section class="panel places" aria-labelledby="places-title">
      <h2 id="places-title">Send to a Place</h2>
      <div class="chips">
        {#each hud.places as p (p)}
          <button class="key chip" onclick={() => actions.sendPlace(p)}>{p}</button>
        {/each}
      </div>
      <p class="selection">{hud.selected.length ? 'Sends the selected Drones' : 'Sends all Drones'}; every route is checked first.</p>
    </section>
  {/if}
</div>

<footer class="dock">
  <div class="verdict" role="status" data-visible={hud.verdictVisible}>
    {#if hud.verdict}
      {#key hud.verdict.seq}
        <span class="line" data-approved={hud.verdict.state === 'approved'} data-state={hud.verdict.state}>
          {#if hud.verdict.state === 'approved'}
            <svg class="icon ok" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>
            <span class="word">Approved</span>
          {:else if hud.verdict.state === 'pending'}
            <svg class="icon wait" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h.1M8 8h.1M13 8h.1" /></svg>
            <span class="word">Pending</span>
          {:else}
            <svg class="icon no" viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2.2l6.2 11H1.8zM8 6.4v3.2M8 11.6v.1" /></svg>
            <span class="word">{hud.verdict.state === 'not_sent' ? 'Not sent' : 'Rejected'}</span>
          {/if}
          <span class="data summary">{hud.verdict.summary}</span>
          {#if hud.verdict.reason}<span class="reason">{hud.verdict.reason}</span>{/if}
          <span class="source">{hud.verdict.source}</span>
        </span>
      {/key}
    {/if}
  </div>
  <div class="panel hint">
    <p>
      Click a Drone to select it, Shift-click to add one. Click the scan to send them there.
      <span class="camera">Drag to orbit · right-drag to pan · scroll to zoom · arrow keys to move</span>
    </p>
    <div class="keys">
      <button class="key" aria-pressed={hud.holo} onclick={actions.toggleHolo}><kbd>H</kbd>Hologram</button>
      <button class="key" onclick={actions.resetView}><kbd>V</kbd>Overview</button>
      <button class="key" onclick={actions.reset}><kbd>R</kbd>Reset</button>
      <button class="key" aria-pressed={hud.follow} onclick={actions.toggleFollow}><kbd>F</kbd>Follow</button>
      <button class="key abort" onclick={() => actions.abort('click')}><kbd>Esc</kbd>Abort</button>
    </div>
  </div>
</footer>

{#if !hud.room}
  <div class="waiting">
    <!-- Only the title is a live region: the retry countdown would be announced every second. -->
    {#if hud.link.state === 'waiting'}
      <p class="title" role="status">No stream at <span class="data">{hud.url}</span></p>
      <p>Start Aether on this laptop. Retrying in {hud.retryIn} s.</p>
      <button class="key" onclick={actions.retryNow}>Retry now</button>
    {:else}
      <p class="title" role="status">Waiting for the Room Twin from <span class="data">{hud.url}</span></p>
    {/if}
  </div>
{/if}

<style>
  .panel {
    pointer-events: auto;
    background: var(--panel);
    border: 1px solid var(--hairline);
    border-radius: var(--radius);
  }

  .identity {
    position: fixed;
    top: var(--gutter);
    left: var(--gutter);
    padding: 0.75rem 0.9rem 0.7rem;
    max-width: 30rem;
  }
  .brand {
    display: flex;
    align-items: baseline;
    gap: 0.9rem;
  }
  .wordmark {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--ink-2);
  }
  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    border: 1.5px solid var(--ink-2);
    align-self: center;
  }
  .link[data-state='open'] .dot {
    background: var(--ink);
    border-color: var(--ink);
  }
  .endpoint {
    color: var(--ink-3);
    font-size: 0.75rem;
  }
  .campus {
    margin-left: auto;
    padding: 0.6rem 0 0.6rem 0.5rem;
    color: var(--accent);
    text-decoration: none;
  }
  .campus:hover {
    text-decoration: underline;
  }
  .room {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem 0.45rem;
    margin: 0.55rem 0 0;
  }
  .sep {
    color: var(--ink-3);
  }
  .note {
    margin: 0.25rem 0 0;
    color: var(--ink-3);
    font-size: 0.75rem;
  }

  /* Provenance tags: LIVE is filled (measured), SIM is outlined (simulated), REPLAY dashed. Never colour alone. */
  .tag {
    display: inline-flex;
    align-items: center;
    height: 1.15rem;
    padding: 0 0.32rem;
    border-radius: 4px;
    font-family: var(--font-data);
    font-size: max(11px, 0.75rem);
    font-weight: 600;
    letter-spacing: 0.04em;
    line-height: 1;
    border: 1px solid var(--ink-2);
    color: var(--ink-2);
    white-space: nowrap;
  }
  .tag[data-provenance='LIVE'] {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--ground);
  }
  .tag[data-provenance='REPLAY'] {
    border-style: dashed;
  }

  .column {
    position: fixed;
    top: var(--gutter);
    right: var(--gutter);
    width: 21rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    pointer-events: none;
  }
  .legend {
    display: flex;
    gap: 1.1rem;
    padding: 0.6rem 0.9rem;
    color: var(--ink-2);
    font-size: 0.8rem;
  }
  .item {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }

  .swarm {
    padding: 0.75rem 0.5rem 0.6rem;
  }
  h2 {
    margin: 0 0.4rem 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
  }
  .cols,
  .row {
    display: grid;
    grid-template-columns: 2.8rem 4.9rem 1fr 3.2rem 2.3rem;
    align-items: center;
    column-gap: 0.4rem;
    padding: 0 0.4rem;
  }
  .cols {
    color: var(--ink-3);
    font-size: 0.75rem;
    padding-bottom: 0.3rem;
    border-bottom: 1px solid var(--hairline);
    margin: 0 0.1rem 0.25rem;
  }
  .num {
    text-align: right;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    width: 100%;
    min-height: var(--tap);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      transform 100ms var(--ease-out);
  }
  .row .id {
    display: inline-grid;
    place-items: center;
    width: 1.6rem;
    height: 1.35rem;
    border-radius: 5px;
    border: 1px solid var(--hairline-strong);
    font-weight: 600;
  }
  .row[aria-pressed='true'] {
    background: var(--wash);
  }
  .row[aria-pressed='true'] .id {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .muted {
    color: var(--ink-3);
  }
  .empty {
    margin: 0.75rem 0.4rem;
    color: var(--ink-2);
  }
  .selection {
    margin: 0.5rem 0.4rem 0;
    color: var(--ink-3);
    font-size: 0.75rem;
  }

  .dock {
    position: fixed;
    left: var(--gutter);
    bottom: var(--gutter);
    right: calc(var(--gutter) * 2 + 21rem);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
    pointer-events: none;
  }
  .verdict {
    min-height: 2.25rem;
    transition: opacity 200ms var(--ease-out);
  }
  .verdict[data-visible='false'] {
    opacity: 0;
  }
  .line {
    pointer-events: auto;
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem 0.6rem;
    padding: 0.5rem 0.8rem;
    background: var(--panel);
    border: 1px solid var(--hairline);
    border-radius: var(--radius);
  }
  .icon {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke-width: 1.9;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .icon.ok,
  .icon.wait {
    stroke: var(--ink);
  }
  .icon.no {
    stroke: var(--accent);
  }
  .row.stale {
    opacity: 0.55;
  }
  .word {
    font-weight: 600;
  }
  .summary {
    color: var(--ink);
  }
  .reason {
    color: var(--ink);
  }
  .reason::before {
    content: '\2014\00a0';
    color: var(--ink-3);
  }
  .source {
    color: var(--ink-3);
    font-size: 0.75rem;
  }

  .hint {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.35rem 0.4rem 0.35rem 0.9rem;
    max-width: 100%;
  }
  .hint p {
    margin: 0;
    color: var(--ink-2);
    text-wrap: pretty;
  }
  .hint .camera {
    display: block;
    color: var(--ink-3);
    font-size: 0.8rem;
  }
  .places h2 {
    margin: 0 0 0.6rem;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }
  .key.chip {
    padding: 0 0.8rem;
    text-transform: capitalize;
  }
  .keys {
    display: flex;
    gap: 0.35rem;
    flex: none;
  }
  .key {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-height: var(--tap);
    padding: 0 0.85rem 0 0.55rem;
    border: 1px solid var(--hairline-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      transform 100ms var(--ease-out);
  }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 1.45rem;
    height: 1.35rem;
    padding: 0 0.3rem;
    border-radius: 4px;
    background: var(--wash-press);
    font-family: var(--font-data);
    font-size: 0.72rem;
    font-weight: 600;
  }
  .key[aria-pressed='true'] {
    background: var(--wash-press);
    border-color: var(--ink-2);
  }
  .key[aria-pressed='true'] kbd {
    background: var(--ink);
    color: var(--ground);
  }
  .key.abort {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
  }
  .key.abort kbd {
    background: rgba(26, 12, 3, 0.16);
  }

  @media (hover: hover) and (pointer: fine) {
    .row:hover,
    .key:hover {
      background-color: var(--wash);
    }
    .row[aria-pressed='true']:hover {
      background-color: var(--wash-press);
    }
    .key.abort:hover {
      background-color: #ff7b33;
    }
  }
  .row:active,
  .key:active {
    background-color: var(--wash-press);
  }
  .key.abort:active {
    background-color: #f25f10;
  }
  @media (prefers-reduced-motion: no-preference) {
    .row:active,
    .key:active {
      transform: scale(0.97);
    }
  }

  .waiting {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    max-width: 34rem;
    text-align: center;
    color: var(--ink-2);
    pointer-events: auto;
  }
  .waiting .title {
    color: var(--ink);
    font-weight: 600;
  }
  .waiting .key {
    margin-top: 0.5rem;
    padding: 0 1rem;
  }
</style>
