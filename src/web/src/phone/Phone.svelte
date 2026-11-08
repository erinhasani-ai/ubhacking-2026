<script lang="ts">
  // Building sensor test: a phone running phyphox stands in for an accelerometer fixed to a building.
  import { onMount } from 'svelte';
  import Nav from '../Nav.svelte';
  import { classify, simulate, G } from './classify.js';

  type Sample = { t: number; m: number; x?: number; y?: number; z?: number };
  type Kind = 'waiting' | 'still' | 'handling' | 'jump' | 'quake';
  const TRIGGER_G = 0.05; // the campus building nodes report above this
  const CHART_S = 10;
  const VERDICT: Record<Kind, { title: string; note: string }> = {
    waiting: { title: 'Waiting for data', note: 'Connect the phone, or start a simulated feed below.' },
    still: { title: 'Still', note: 'Only sensor noise. A building node would report nothing.' },
    handling: { title: 'Normal handling', note: 'Irregular movement, like picking the phone up or tapping it. Not an earthquake.' },
    jump: { title: 'Jump or impact', note: 'One sharp spike or a free fall over a quiet background. Not an earthquake.' },
    quake: { title: 'Earthquake-like shaking', note: 'Sustained, oscillating shaking. A building node would report this; three nodes confirm a quake on the campus map.' },
  };

  let ip = $state(load());
  let source = $state<'off' | 'phone' | 'sim'>('off');
  let simKind = $state<'still' | 'handling' | 'jump' | 'quake'>('quake');
  let status = $state('Not connected');
  let verdict = $state<{ kind: Kind; rms: number; peak: number; freq: number; active: number }>({ kind: 'waiting', rms: 0, peak: 0, freq: 0, active: 0 });
  let lines = $state<{ id: number; text: string; over: boolean }[]>([]);
  let rate = $state(0);
  let canvas: HTMLCanvasElement;

  let samples: Sample[] = [];
  let quakeRun = 0;
  let lineId = 0;
  let received = 0;
  let session = 0;

  function load() {
    try {
      return localStorage.getItem('aether.phone.ip') ?? '172.20.10.1';
    } catch {
      return '172.20.10.1';
    }
  }
  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

  function push(batch: Sample[]) {
    if (!batch.length) return;
    received += batch.length;
    samples.push(...batch);
    const end = samples[samples.length - 1].t;
    if (samples[0].t < end - CHART_S - 1) samples = samples.filter((s) => s.t >= end - CHART_S - 1);
    // The stream shows every fifth sample, newest first, so it stays readable at 100 Hz.
    const shown = batch.filter((_, i) => i % 5 === 0).map((s) => {
      const dyn = Math.abs(s.m - G);
      const xyz = s.x === undefined ? '' : `  x ${s.x.toFixed(2)} y ${s.y!.toFixed(2)} z ${s.z!.toFixed(2)}`;
      return { id: ++lineId, text: `${s.t.toFixed(2).padStart(7)} s  |a| ${s.m.toFixed(2)} m/s²${xyz}`, over: dyn > TRIGGER_G * G };
    });
    lines = [...shown.reverse(), ...lines].slice(0, 80);
  }

  async function connect() {
    if (source === 'phone') return stop();
    try {
      localStorage.setItem('aether.phone.ip', ip.trim());
    } catch {}
    const mine = ++session;
    reset('phone');
    status = `Connecting to ${ip.trim()}…`;
    // session changes on Disconnect or a simulated feed, which ends this loop
    while (session === mine) {
      try {
        const r = await fetch(`/api/accel?ip=${encodeURIComponent(ip.trim())}`);
        if (!r.ok) {
          status = `${await r.text()}. Retrying…`;
          await sleep(1500);
          continue;
        }
        const body: { samples: number[][] } = await r.json();
        if (session !== mine) return;
        status = `Live from ${ip.trim()}`;
        push(body.samples.map(([t, m, x, y, z]) => ({ t, m, x, y, z })));
      } catch {
        status = 'Aether engine not reachable. Is it running?';
        await sleep(1500);
      }
      await sleep(120);
    }
  }

  function startSim(kind: typeof simKind) {
    simKind = kind;
    if (source === 'sim') return;
    session++;
    reset('sim');
    status = 'Simulated feed';
  }

  function stop() {
    session++;
    source = 'off';
    status = 'Not connected';
  }

  function reset(next: typeof source) {
    source = next;
    samples = [];
    lines = [];
    quakeRun = 0;
    verdict = { kind: 'waiting', rms: 0, peak: 0, freq: 0, active: 0 };
  }

  onMount(() => {
    const t0 = performance.now();
    let simT = 0;
    const simTimer = setInterval(() => {
      if (source !== 'sim') return;
      const now = (performance.now() - t0) / 1000;
      const batch: Sample[] = [];
      for (; simT < now; simT += 0.01) batch.push({ t: simT, m: simulate(simKind, simT) });
      push(batch);
    }, 50);
    // Verdict at 5 Hz; "earthquake" needs three readings in a row so a single burst cannot raise it.
    const verdictTimer = setInterval(() => {
      const v = classify(samples) as { kind: Kind; rms: number; peak: number; freq: number; active: number };
      quakeRun = v.kind === 'quake' ? quakeRun + 1 : 0;
      verdict = { ...v, kind: v.kind === 'quake' && quakeRun < 3 ? (verdict.kind === 'quake' ? 'quake' : 'handling') : v.kind };
    }, 200);
    const rateTimer = setInterval(() => {
      rate = received;
      received = 0;
    }, 1000);
    let raf = 0;
    const draw = () => {
      drawChart();
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => {
      clearInterval(simTimer);
      clearInterval(verdictTimer);
      clearInterval(rateTimer);
      cancelAnimationFrame(raf);
      session++;
    };
  });

  /** |a| − g over the last 10 s, with the ±0.05 g building-node trigger as dashed lines. */
  function drawChart() {
    if (!canvas) return;
    const dpr = Math.min(devicePixelRatio, 2);
    const w = canvas.clientWidth, h = canvas.clientHeight;
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
    }
    const g = canvas.getContext('2d')!;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, h);
    const css = getComputedStyle(canvas);
    const ink3 = css.getPropertyValue('--ink-3').trim() || '#9a9284';
    const accent = css.getPropertyValue('--accent').trim() || '#e3c07c';
    const left = 44, right = 8, top = 10, bottom = 22;
    const pw = w - left - right, ph = h - top - bottom;
    const end = samples.length ? samples[samples.length - 1].t : 0;
    const peak = samples.reduce((a, s) => Math.max(a, Math.abs(s.m - G)), 0);
    const range = Math.max(1.5, Math.ceil(peak * 1.15));
    const y = (v: number) => top + ph / 2 - (v / range) * (ph / 2);
    const x = (t: number) => left + pw - ((end - t) / CHART_S) * pw;

    g.font = '11px "Geist Mono Variable", ui-monospace, monospace';
    g.fillStyle = ink3;
    g.strokeStyle = 'rgba(243, 232, 210, 0.09)';
    g.lineWidth = 1;
    g.textAlign = 'right';
    g.textBaseline = 'middle';
    for (const v of [-range, 0, range]) {
      g.beginPath();
      g.moveTo(left, y(v));
      g.lineTo(left + pw, y(v));
      g.stroke();
      g.fillText(`${v > 0 ? '+' : v < 0 ? '−' : ''}${Math.abs(v).toFixed(1)}`, left - 6, y(v));
    }
    g.textAlign = 'center';
    g.textBaseline = 'alphabetic';
    for (let s = 0; s <= CHART_S; s += 2) g.fillText(s === 0 ? 'now' : `−${s} s`, left + pw - (s / CHART_S) * pw, h - 6);
    // Building-node trigger, ±0.05 g.
    g.setLineDash([4, 4]);
    g.strokeStyle = 'rgba(255, 122, 89, 0.7)';
    for (const v of [TRIGGER_G * G, -TRIGGER_G * G]) {
      g.beginPath();
      g.moveTo(left, y(v));
      g.lineTo(left + pw, y(v));
      g.stroke();
    }
    g.setLineDash([]);
    if (samples.length < 2) return;
    g.strokeStyle = accent;
    g.lineWidth = 1.5;
    g.lineJoin = 'round';
    g.beginPath();
    let started = false;
    for (const s of samples) {
      if (s.t < end - CHART_S) continue;
      const px = x(s.t), py = y(Math.max(-range, Math.min(range, s.m - G)));
      if (started) g.lineTo(px, py);
      else g.moveTo(px, py), (started = true);
    }
    g.stroke();
  }

  const fmt = (v: number, k = 2) => v.toFixed(k);
</script>

<Nav current="phone" />

<main class="page">
  <section class="col">
    <header class="panel intro">
      <div class="brand">
        <h1 class="wordmark">Aether</h1>
        <span class="sub">Building sensor test</span>
        <span class="tag" data-live={source === 'phone'}>{source === 'phone' ? 'LIVE' : source === 'sim' ? 'SIM' : 'OFF'}</span>
      </div>
      <p>Your phone stands in for an accelerometer fixed to a building. Lay it flat on a table, then shake the table, jump, or pick it up, and watch how Aether reads the motion.</p>
    </header>

    <form
      class="panel connect"
      onsubmit={(e) => {
        e.preventDefault();
        connect();
      }}
    >
      <label for="ip">phyphox address</label>
      <div class="row">
        <input id="ip" bind:value={ip} inputmode="decimal" autocomplete="off" spellcheck="false" placeholder="172.20.10.1" disabled={source === 'phone'} />
        <button class="btn primary" type="submit">{source === 'phone' ? 'Disconnect' : 'Connect'}</button>
      </div>
      <p class="status" data-state={source}>{status}</p>
      <p class="help">In phyphox, open <b>Acceleration (with g)</b>, tap the menu, choose <b>Allow remote access</b>, and type the address it shows (without <span class="data">http://</span>). The phone and this laptop must be on the same network.</p>
      <div class="sim">
        <span>No phone? Simulated feed</span>
        {#each ['still', 'handling', 'jump', 'quake'] as const as k (k)}
          <button type="button" class="btn chip" aria-pressed={source === 'sim' && simKind === k} onclick={() => startSim(k)}>
            {{ still: 'Still', handling: 'Handling', jump: 'Jump', quake: 'Earthquake' }[k]}
          </button>
        {/each}
        {#if source === 'sim'}<button type="button" class="btn chip" onclick={stop}>Stop</button>{/if}
      </div>
    </form>

    <section class="panel verdict" data-kind={verdict.kind} aria-live="polite">
      <p class="label">Interpretation</p>
      <h2>{VERDICT[verdict.kind].title}</h2>
      <p class="note">{VERDICT[verdict.kind].note}</p>
      <dl>
        <div><dt>Shaking (RMS)</dt><dd class="data">{fmt(verdict.rms)} m/s² · {fmt(verdict.rms / G, 3)} g</dd></div>
        <div><dt>Peak</dt><dd class="data">{fmt(verdict.peak)} m/s² · {fmt(verdict.peak / G, 3)} g</dd></div>
        <div><dt>Frequency</dt><dd class="data">{fmt(verdict.freq, 1)} Hz</dd></div>
        <div><dt>Sustained</dt><dd class="data">{Math.round(verdict.active * 100)}% of 3 s</dd></div>
      </dl>
    </section>

    <section class="panel chart">
      <div class="chart-head">
        <h3>Acceleration minus gravity, m/s²</h3>
        <span class="key"><span class="dash" aria-hidden="true"></span>building node trigger, ±0.05 g</span>
      </div>
      <canvas bind:this={canvas} aria-label="Acceleration over the last 10 seconds"></canvas>
    </section>
  </section>

  <aside class="panel stream" aria-label="Data stream">
    <div class="stream-head">
      <h3>Data stream</h3>
      <span class="data rate">{rate} samples/s</span>
    </div>
    <ol>
      {#each lines as l (l.id)}
        <li class="data" class:over={l.over}>{l.text}</li>
      {:else}
        <li class="empty">Samples appear here as they arrive.</li>
      {/each}
    </ol>
  </aside>
</main>

<style>
  .page {
    position: fixed;
    inset: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 24rem;
    gap: 1rem;
    padding: 5.75rem var(--gutter) var(--gutter);
    overflow: auto;
  }
  .col {
    display: grid;
    grid-template-rows: auto auto auto minmax(14rem, 1fr);
    gap: 1rem;
    min-height: 0;
  }
  .panel {
    background: var(--panel);
    border: 1px solid var(--hairline);
    border-radius: var(--radius);
    padding: 1rem 1.1rem;
  }
  .brand {
    display: flex;
    align-items: baseline;
    gap: 0.8rem;
  }
  .wordmark {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.75rem;
    font-weight: 400;
    line-height: 1;
  }
  .sub {
    color: var(--ink-2);
  }
  .tag {
    margin-left: auto;
    padding: 0.1rem 0.4rem;
    border: 1px solid var(--ink-2);
    border-radius: 4px;
    font-family: var(--font-data);
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--ink-2);
  }
  .tag[data-live='true'] {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--ground);
  }
  .intro p {
    margin: 0.6rem 0 0;
    max-width: 68ch;
    color: var(--ink-2);
  }
  label,
  .label {
    display: block;
    margin: 0 0 0.45rem;
    color: var(--ink-3);
    font-size: 0.8rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
  }
  input {
    flex: 1;
    min-height: var(--tap);
    padding: 0 0.9rem;
    border: 1px solid var(--hairline-strong);
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.25);
    font-family: var(--font-data);
    font-size: 1rem;
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .btn {
    min-height: var(--tap);
    padding: 0 1.1rem;
    border: 1px solid var(--hairline-strong);
    border-radius: 999px;
    background: transparent;
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 140ms var(--ease-out);
  }
  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
    min-width: 8.5rem;
  }
  .btn.chip[aria-pressed='true'] {
    border-color: var(--accent);
    color: var(--accent);
  }
  @media (hover: hover) and (pointer: fine) {
    .btn:hover {
      background: var(--wash);
    }
    .btn.primary:hover {
      background: #ecd09a;
    }
  }
  .status {
    margin: 0.55rem 0 0;
    color: var(--ink-2);
    font-size: 0.85rem;
  }
  .help {
    margin: 0.4rem 0 0;
    color: var(--ink-3);
    font-size: 0.8rem;
    max-width: 75ch;
  }
  .help b {
    color: var(--ink-2);
    font-weight: 600;
  }
  .sim {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.8rem;
    padding-top: 0.8rem;
    border-top: 1px solid var(--hairline);
    color: var(--ink-3);
    font-size: 0.8rem;
  }
  .sim span {
    margin-right: 0.3rem;
  }

  .verdict h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 2.6rem;
    font-weight: 400;
    line-height: 1.05;
    letter-spacing: -0.01em;
  }
  .verdict .note {
    margin: 0.4rem 0 0;
    color: var(--ink-2);
    max-width: 70ch;
  }
  .verdict[data-kind='quake'] {
    border-color: rgba(255, 122, 89, 0.55);
    background: linear-gradient(180deg, rgba(255, 106, 77, 0.16), rgba(255, 106, 77, 0.04)), var(--panel);
  }
  .verdict[data-kind='quake'] h2 {
    color: #ff8a6a;
  }
  .verdict[data-kind='jump'] h2 {
    color: var(--accent);
  }
  .verdict[data-kind='still'] h2,
  .verdict[data-kind='waiting'] h2 {
    color: var(--ink-2);
  }
  dl {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
    margin: 1rem 0 0;
  }
  dt {
    color: var(--ink-3);
    font-size: 0.75rem;
  }
  dd {
    margin: 0.2rem 0 0;
    font-size: 0.9rem;
  }

  .chart {
    display: flex;
    flex-direction: column;
    min-height: 14rem;
  }
  .chart-head,
  .stream-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.5rem;
  }
  h3 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--ink-2);
  }
  .key {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--ink-3);
    font-size: 0.75rem;
  }
  .dash {
    width: 1.2rem;
    border-top: 1.5px dashed rgba(255, 122, 89, 0.8);
  }
  canvas {
    flex: 1;
    width: 100%;
    min-height: 10rem;
  }

  .stream {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .rate {
    color: var(--ink-3);
    font-size: 0.75rem;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
    font-size: 0.75rem;
    line-height: 1.6;
    color: var(--ink-2);
  }
  li.over {
    color: #ff8a6a;
  }
  .empty {
    color: var(--ink-3);
  }

  @media (max-width: 900px) {
    .page {
      grid-template-columns: 1fr;
    }
    .stream {
      max-height: 22rem;
    }
    dl {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
