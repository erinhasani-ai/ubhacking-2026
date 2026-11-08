<script lang="ts">
  import { ui } from './hud.svelte';

  interface Actions {
    run(text: string): void;
    toggleListening(): void;
    togglePhone(): void;
    toggleStream(): void;
    igniteHere(): void;
    focusFire(): void;
  }
  let { actions }: { actions: Actions } = $props();

  let text = $state('');
  const EXAMPLES = ['All teams, patrol the campus', 'Simulate a fire at Capen Hall', 'Bravo, go to Lockwood Library', 'Simulate an earthquake', 'Status report', 'All teams, return to base'];
  const STATUS = { burning: 'Burning', contained: 'Contained', out: 'Out' } as const;
  const micWords = $derived({ off: 'Start listening', listening: 'Listening', hearing: 'Hearing you…', transcribing: 'Transcribing…' }[ui.mic]);

  function submit(e: SubmitEvent) {
    e.preventDefault();
    actions.run(text);
    text = '';
  }
</script>

<header class="panel identity">
  <div class="brand">
    <h1 class="wordmark">Aether</h1>
    <span class="sub">Campus safety</span>
    <a class="switch" href="./">Room Twin</a>
  </div>
  <button class="key small stream-toggle" aria-pressed={ui.streamOpen} onclick={actions.toggleStream}>
    Data stream <span class="data">{ui.rate.msgs}</span> msg/s
  </button>
  <p class="room">
    <span class="tag">SIM</span>
    <span>UB North Campus</span>
    <span class="sep" aria-hidden="true">·</span>
    <span><span class="data">30</span> Drones, 5 teams, 5 pads</span>
    <span class="sep" aria-hidden="true">·</span>
    <span>sim time <span class="data">×5</span></span>
  </p>
  <p class="note">
    Buildings from OpenStreetMap: <span class="data">{ui.heights.tagged}</span> with tagged heights, <span class="data">{ui.heights.estimated}</span> estimated at 12 m (faint edges)
  </p>
</header>

<div class="column">
  <section class="panel teams" aria-labelledby="teams-title">
    <h2 id="teams-title">Teams</h2>
    <p class="roles">Each team of 6 scouts; <span class="swatch suppress" aria-hidden="true"></span>4 suppression, <span class="swatch alert" aria-hidden="true"></span>2 alert</p>
    <ul>
      {#each ui.teams as t (t.id)}
        <li class="team">
          <span class="name">{t.name}</span>
          <span class="role">Pad · {t.pad}</span>
          <span class="num"><span class="data">{t.airborne}/{t.total}</span> up</span>
          <span class="words">{t.words}</span>
        </li>
      {/each}
    </ul>
  </section>

  {#if ui.fire}
    {@const f = ui.fire}
    <section class="panel incident" aria-labelledby="fire-title">
      <div class="head">
        <h2 id="fire-title">Fire · {f.name}</h2>
        <span class="status" data-status={f.detected ? f.status : 'undetected'}>{f.detected ? STATUS[f.status] : 'Undetected'}</span>
        <span class="tag">SIM</span>
      </div>
      <dl>
        <div>
          <dt>Detected</dt>
          <dd>
            {#if f.detected}by {f.detectedBy} after <span class="data">{f.detectedAfter?.toFixed(0)}</span> s sim time{:else}not yet: no Drone within 250 m{/if}
          </dd>
        </div>
        {#if f.status === 'out'}
          <div><dt>Out</dt><dd><span class="data">{f.outAfter?.toFixed(0)}</span> s after ignition (sim time)</dd></div>
          <div><dt>Alerted</dt><dd>{f.alertedNames.join(', ') || 'none'}</dd></div>
        {:else}
        <div><dt>Burning area</dt><dd><span class="data">{f.across}</span> m across</dd></div>
        <div>
          <dt>Intensity</dt>
          <dd class="meter-row">
            <span class="meter" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(f.intensity * 100)} aria-label="Fire intensity">
              <span style="width: {Math.round(f.intensity * 100)}%"></span>
            </span>
            <span class="data">{Math.round(f.intensity * 100)}%</span>
          </dd>
        </div>
        {#if f.detected}
          <div><dt>Alert radius</dt><dd><span class="data">{f.alertRadius}</span> m · <span class="data">{f.zone}</span> buildings inside</dd></div>
          {#if f.responders.length}
            <div><dt>Responding</dt><dd>{f.responders[0]} suppresses; {f.responders.join(' and ')} alert</dd></div>
          {/if}
          <div><dt>Suppression</dt><dd><span class="data">{f.onStation}</span> of 4 Drones on station</dd></div>
          <div><dt>Alerted</dt><dd>{f.alertedNames.length ? f.alertedNames.join(', ') : 'Charlie en route'}</dd></div>
        {/if}
        {/if}
      </dl>
      <button class="key small" onclick={actions.focusFire}>Show fire</button>
    </section>

    {#if f.detected && f.status !== 'out'}
      <aside class="panel phone" aria-label="Simulated campus alert">
        <p class="app"><span>Campus alert</span><span class="tag">SIM</span></p>
        <p class="msg">Fire reported at {f.name}. Avoid the area within {f.alertRadius} m and follow directions from responders.</p>
      </aside>
    {/if}
  {/if}

  {#if ui.quake}
    {@const q = ui.quake}
    <section class="panel incident quake" aria-labelledby="quake-title">
      <div class="head">
        <h2 id="quake-title">Earthquake</h2>
        <span class="status" data-status={q.detected ? (q.status === 'clear' ? 'out' : 'contained') : 'undetected'}>{q.detected ? (q.status === 'clear' ? 'Survey complete' : 'Surveying') : 'Shaking'}</span>
        <span class="tag">{q.live ? 'LIVE trigger' : 'SIM'}</span>
      </div>
      <dl>
        <div><dt>Trigger</dt><dd>{q.live ? 'Phone accelerometer (measured)' : 'Simulated shaking'}</dd></div>
        <div><dt>Detected</dt><dd>{q.detected ? `${q.sensors} campus accelerometers over 0.05 g` : 'waiting for 3 nodes over 0.05 g'}</dd></div>
        <div><dt>Peak</dt><dd><span class="data">{q.peakG.toFixed(2)}</span> g (simulated campus effect)</dd></div>
        {#if q.detected}
          <div><dt>Inspected</dt><dd><span class="data">{q.inspected}</span> of <span class="data">{q.affected}</span> buildings over 0.1 g</dd></div>
        {/if}
      </dl>
    </section>
  {/if}

  {#if ui.picked}
    <section class="panel picked">
      <p>Selected <strong>{ui.picked}</strong></p>
      <button class="key small" onclick={actions.igniteHere}>Simulate fire here</button>
    </section>
  {/if}
</div>

<footer class="dock">
  <ol class="log" aria-live="polite">
    {#each ui.log.slice(-4) as l (l.seq)}
      <li data-who={l.who}><span class="who">{{ you: 'You', error: 'Not run', heard: 'Heard, no command', aether: 'Aether' }[l.who]}</span>{l.text}</li>
    {/each}
  </ol>
  <div class="panel bar">
    <button class="key mic" aria-pressed={ui.mic !== 'off'} data-state={ui.mic} onclick={actions.toggleListening}>
      <svg class="icon" viewBox="0 0 16 16" aria-hidden="true"><path d="M8 1.8a2 2 0 0 1 2 2v4a2 2 0 0 1-4 0v-4a2 2 0 0 1 2-2zM4 7.6a4 4 0 0 0 8 0M8 11.6v2.6" /></svg>
      <span class="mic-words">{micWords}</span><kbd>Space</kbd>
    </button>
    <form onsubmit={submit}>
      <input bind:value={text} placeholder="Or type: Bravo, go to Capen Hall" aria-label="Command" autocomplete="off" />
      <button class="key" type="submit">Send</button>
    </form>
    <button class="key abort" onclick={() => actions.run('Abort')}><kbd>Esc</kbd>Abort</button>
  </div>
  <div class="examples" aria-label="Example commands">
    {#each EXAMPLES as ex (ex)}
      <button class="example" onclick={() => actions.run(ex)}>{ex}</button>
    {/each}
  </div>
  <p class="meta">
    <span>Voice: {ui.voice}</span>
    <label><input type="checkbox" bind:checked={ui.autoResponse} /> Automatic fire response</label>
    <label><input type="checkbox" bind:checked={ui.muted} /> Mute</label>
    <label>
      <input type="checkbox" checked={ui.phone !== 'off'} onchange={actions.togglePhone} /> Phone accelerometer
      {#if ui.phone !== 'off'}<span class="phone-state" data-state={ui.phone}>{{ connecting: 'connecting…', live: 'LIVE', error: 'not reachable, retrying', off: '' }[ui.phone]}</span>{/if}
    </label>
    <span><kbd>R</kbd> reset</span>
  </p>
</footer>

{#if ui.streamOpen}
  <section class="panel stream" aria-label="Data stream">
    <p class="stream-head">
      <span>Data stream</span><span class="tag">SIM telemetry</span>{#if ui.phone === 'live'}<span class="tag live">LIVE phone</span>{/if}
      <span class="data rate">{ui.rate.msgs} msg/s · {ui.rate.kb.toFixed(1)} kB/s</span>
    </p>
    <ol>
      {#each ui.stream as l (l.seq)}
        <li class="data" data-kind={l.kind}>{l.text}</li>
      {/each}
    </ol>
  </section>
{/if}

<p class="attribution">Map data {ui.attribution}</p>

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
    gap: 0.75rem;
  }
  .wordmark {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .sub {
    color: var(--ink-2);
  }
  .switch {
    margin-left: auto;
    color: var(--accent);
    text-decoration: none;
    padding: 0.6rem 0 0.6rem 0.5rem;
  }
  .switch:hover {
    text-decoration: underline;
  }
  .room {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem 0.45rem;
    margin: 0.4rem 0 0;
  }
  .sep {
    color: var(--ink-3);
  }
  .note {
    margin: 0.3rem 0 0;
    color: var(--ink-3);
    font-size: 0.75rem;
  }
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

  .column {
    position: fixed;
    top: var(--gutter);
    right: var(--gutter);
    width: 22rem;
    max-height: calc(100vh - 2 * var(--gutter));
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    pointer-events: none;
  }
  h2 {
    margin: 0;
    font-size: 0.875rem;
    font-weight: 600;
  }
  .teams {
    padding: 0.75rem 0.9rem 0.5rem;
  }
  .teams ul {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
  }
  .team {
    display: grid;
    grid-template-columns: 4rem 1fr auto;
    gap: 0 0.5rem;
    padding: 0.32rem 0;
    border-top: 1px solid var(--hairline);
  }
  .name {
    font-weight: 600;
  }
  .role {
    color: var(--ink-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .roles {
    margin: 0.25rem 0 0;
    color: var(--ink-3);
    font-size: 0.78rem;
  }
  .swatch {
    display: inline-block;
    width: 0.55rem;
    height: 0.55rem;
    margin: 0 0.25rem 0 0.15rem;
    border-radius: 2px;
    transform: rotate(45deg);
  }
  .swatch.suppress {
    background: #e8f5ff;
  }
  .swatch.alert {
    background: #ffb35c;
  }
  .num {
    color: var(--ink-2);
    text-align: right;
  }
  .words {
    grid-column: 1 / -1;
    color: var(--ink-2);
  }

  .incident {
    padding: 0.75rem 0.9rem;
    border-color: rgba(255, 106, 44, 0.45);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .status {
    padding: 0.1rem 0.45rem;
    border-radius: 4px;
    font-weight: 600;
    font-size: 0.8rem;
    background: #ff6a2c;
    color: #1d0a02;
  }
  .status[data-status='contained'] {
    background: #ffb36b;
  }
  .status[data-status='out'] {
    background: var(--ink);
    color: var(--ground);
  }
  .status[data-status='undetected'] {
    background: transparent;
    color: var(--ink-2);
    border: 1px dashed var(--ink-3);
  }
  dl {
    margin: 0.6rem 0;
    display: grid;
    gap: 0.3rem;
  }
  dl div {
    display: grid;
    grid-template-columns: 6.2rem 1fr;
    gap: 0.5rem;
  }
  dt {
    color: var(--ink-3);
  }
  dd {
    margin: 0;
    color: var(--ink);
  }
  .meter-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .meter {
    flex: 1;
    height: 0.4rem;
    border-radius: 3px;
    background: var(--wash-press);
    overflow: hidden;
  }
  .meter span {
    display: block;
    height: 100%;
    background: #ff6a2c;
    transition: width 120ms linear;
  }

  .phone {
    padding: 0.7rem 0.9rem;
    border-radius: 16px;
    border-color: var(--hairline-strong);
  }
  .phone .app {
    display: flex;
    justify-content: space-between;
    margin: 0;
    color: var(--ink-3);
    font-size: 0.75rem;
  }
  .phone .msg {
    margin: 0.3rem 0 0;
    text-wrap: pretty;
  }
  .stream-toggle {
    margin-top: 0.55rem;
    min-height: 2.25rem;
    font-size: 0.8rem;
  }
  .stream-toggle[aria-pressed='true'] {
    border-color: var(--accent);
    color: var(--accent);
  }
  .stream {
    position: fixed;
    left: var(--gutter);
    top: 10.5rem;
    width: min(36rem, calc(100vw - 22rem - 3 * var(--gutter)));
    max-height: calc(100vh - 10.5rem - 22rem);
    overflow: hidden;
    padding: 0.55rem 0.8rem;
  }
  .stream-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.35rem;
    font-weight: 600;
  }
  .rate {
    margin-left: auto;
    color: var(--ink-2);
    font-weight: 400;
  }
  .tag.live {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--ground);
  }
  .stream ol {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .stream li {
    font-size: 11px;
    line-height: 1.5;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink-2);
  }
  .stream li[data-kind='1'] {
    color: #ffcf96;
  }
  .stream li[data-kind='2'] {
    color: #ff8a5c;
  }
  .phone-state {
    color: var(--ink-2);
  }
  .phone-state[data-state='live'] {
    color: var(--ink);
    font-weight: 600;
  }
  .picked {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.4rem 0.4rem 0.4rem 0.9rem;
  }
  .picked p {
    margin: 0;
  }

  .dock {
    position: fixed;
    left: var(--gutter);
    bottom: var(--gutter);
    max-width: min(46rem, calc(100vw - 22rem - 3 * var(--gutter)));
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    pointer-events: none;
  }
  .log {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .log li {
    align-self: flex-start;
    padding: 0.35rem 0.7rem;
    background: var(--panel);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-sm);
    text-wrap: pretty;
  }
  .log li:not(:last-child) {
    opacity: 0.6;
  }
  .who {
    font-weight: 600;
    margin-right: 0.5rem;
  }
  .log li[data-who='aether'] .who {
    color: var(--accent);
  }
  .log li[data-who='error'] .who {
    color: #ff9a6b;
  }
  .log li[data-who='heard'] {
    color: var(--ink-3);
  }
  .bar {
    display: flex;
    gap: 0.4rem;
    padding: 0.4rem;
    align-items: center;
  }
  form {
    display: flex;
    flex: 1;
    gap: 0.4rem;
    min-width: 0;
  }
  input:not([type='checkbox']) {
    flex: 1;
    min-width: 0;
    min-height: var(--tap);
    padding: 0 0.75rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--hairline-strong);
    background: rgba(0, 0, 0, 0.25);
    color: var(--ink);
    font: inherit;
  }
  .examples {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }
  .example {
    pointer-events: auto;
    min-height: 2.2rem;
    padding: 0 0.7rem;
    border-radius: 999px;
    border: 1px solid var(--hairline-strong);
    background: var(--panel);
    color: var(--ink-2);
    cursor: pointer;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem 1rem;
    margin: 0;
    color: var(--ink-3);
    font-size: 0.78rem;
    pointer-events: auto;
  }
  .meta label {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    cursor: pointer;
  }
  .attribution {
    position: fixed;
    right: var(--gutter);
    bottom: 0.4rem;
    margin: 0;
    color: var(--ink-3);
    font-size: 11px;
  }

  .key {
    pointer-events: auto;
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
  .key.small {
    padding: 0 0.85rem;
  }
  .mic {
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }
  .mic[data-state='hearing'],
  .mic[data-state='transcribing'] {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .mic-words {
    min-width: 7.5rem;
  }
  .mic[data-state='listening'] {
    border-color: var(--accent);
    color: var(--accent);
  }
  .icon {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
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
  .key.abort {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
  }
  @media (hover: hover) and (pointer: fine) {
    .key:hover,
    .example:hover {
      background-color: var(--wash);
      color: var(--ink);
    }
    .key.abort:hover,
    .mic[aria-pressed='true']:hover {
      background-color: #7cc9ff;
    }
  }
  @media (prefers-reduced-motion: no-preference) {
    .key:active,
    .example:active {
      transform: scale(0.97);
    }
  }

  /* Map labels live outside this component (scene.ts), so they are global. */
  :global(.labels) {
    position: fixed;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  :global(.label) {
    position: absolute;
    left: 0;
    top: 0;
    white-space: nowrap;
    font: 600 11px var(--font-data);
    padding: 2px 6px;
    border-radius: 4px;
    will-change: transform;
  }
  :global(.label.building) {
    color: var(--ink-3);
    font-weight: 500;
  }
  :global(.label.base) {
    color: var(--accent);
  }
  :global(.label.team) {
    background: var(--panel);
    border: 1px solid var(--accent);
    color: var(--ink);
  }
  :global(.label.fire) {
    background: #ff6a2c;
    color: #1d0a02;
  }
  :global(.label.picked) {
    border: 1px solid var(--ink);
    background: var(--panel);
    color: var(--ink);
  }

  @media (max-width: 900px) {
    .column {
      width: 18rem;
    }
    .dock {
      max-width: calc(100vw - 18rem - 3 * var(--gutter));
    }
  }
</style>
