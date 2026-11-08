// Campus page (T32/T33): UB North Campus from OpenStreetMap, three simulated Drone teams, simulated fire response.
// Commands come from hands-free voice (ElevenLabs Scribe via the engine), the command box or the example chips.
import '@fontsource-variable/geist-mono';
import '../app.css';
import { mount } from 'svelte';
import Campus from './Campus.svelte';
import { ui } from './hud.svelte';
import { createCampusScene } from './scene';
import * as S from './sim.js';
import { createListener, say, transcribe } from './voice';

const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)');

async function main() {
  const data = await (await fetch('campus/ub-north.json')).json();
  const world = S.buildWorld(data);
  const sim = S.createSim(world);
  ui.heights = { tagged: data.heights.tagged, estimated: data.heights.estimated };
  ui.attribution = data.attribution;

  const canvas = document.createElement('canvas');
  canvas.className = 'map';
  canvas.setAttribute('aria-label', 'UB North Campus map with simulated Drone teams. Click a building to select it.');
  Object.assign(canvas.style, { position: 'fixed', inset: '0', width: '100%', height: '100%', display: 'block', touchAction: 'none' });
  document.body.prepend(canvas);
  const overlay = document.createElement('div');
  overlay.className = 'labels';
  overlay.setAttribute('aria-hidden', 'true');
  canvas.after(overlay);

  let seq = 0;
  const log = (who: 'you' | 'aether' | 'error' | 'heard', text: string) => {
    ui.log = [...ui.log.slice(-5), { seq: ++seq, who, text }];
  };
  const speak = (text: string) => {
    if (!ui.muted) say(text, (s) => (ui.voice = s));
  };

  let picked: any = null;
  const scene = createCampusScene(
    canvas,
    overlay,
    world,
    data,
    sim,
    {
      pick(p) {
        picked = S.buildingAt(world, p);
        ui.picked = picked ? (picked.name ?? 'Unnamed building') : null;
        scene.showPick(picked);
      },
      frame(dt) {
        sim.autoResponse = ui.autoResponse;
        for (const e of S.step(sim, dt)) {
          log('aether', e.text);
          speak(e.text);
          if (e.kind === 'detected') scene.focus(sim.fire.at);
          if (e.kind === 'quake') scene.focus(world.centre);
          if (e.kind === 'detected' || e.kind === 'quake') transmit(`ALARM  ${e.kind === 'quake' ? 'seismic trigger' : 'thermal trigger'}  ${e.text.split('.')[0]}`, 2, ...(e.kind === 'quake' ? [...world.centre, 0] : [...sim.fire.at, sim.fire.roof + 30]) as [number, number, number]);
        }
      },
    },
    () => reduceMotion.matches,
  );

  // ---------- Data stream (SIM telemetry; the phone accelerometer is LIVE) ----------
  let streamSeq = 0;
  let sentMsgs = 0;
  let sentBytes = 0;
  const clock = () => new Date().toTimeString().slice(0, 8);
  function transmit(text: string, kind: 0 | 1 | 2, x: number, y: number, z: number) {
    scene.ping(x, y, z, kind);
    const line = `${clock()}  ${text}`;
    sentMsgs++;
    sentBytes += line.length + 24; // shortcut: payload size approximated by the printed line plus a header
    if (ui.streamOpen) ui.stream = [{ seq: ++streamSeq, text: line, kind }, ...ui.stream.slice(0, 39)];
  }
  const f0 = (v: number) => v.toFixed(0).replace('-', '−');
  let tick = 0;
  setInterval(() => {
    tick++;
    // Drones at 2 Hz, staggered; pads and sensor nodes at 1 Hz; sensor nodes at 5 Hz while shaking.
    sim.drones.forEach((d: any, i: number) => {
      if (d.pos[2] <= 2 || (i + tick) % 5) return;
      const r = S.telemetry(sim, d);
      const hot = r.thermal > 60;
      transmit(`${d.id.padEnd(3)} → ${S.teamTitle(d.team)} pad  TELEM  x ${f0(d.pos[0])} y ${f0(d.pos[1])} z ${f0(d.pos[2])}  ${r.speed.toFixed(1)} m/s  batt ${d.batt.toFixed(0)}%  ${r.thermal.toFixed(0)} °C  ${r.dbm.toFixed(0)} dBm`, hot ? 2 : 0, d.pos[0], d.pos[1], d.pos[2]);
    });
    world.sensors.forEach((b: any, i: number) => {
      const g = S.groundAccel(sim, b.c, i);
      const shaking = g > S.QUAKE_TRIGGER_G;
      if (!shaking && (i + tick) % 10) return;
      transmit(`NODE ${b.name}  ACCEL  ${g.toFixed(3)} g${shaking ? '  OVER TRIGGER' : ''}`, shaking ? 2 : 1, b.c[0], b.c[1], b.h);
    });
    if (tick % 10 === 0)
      world.pads.forEach((p: any) => {
        const ds = sim.drones.filter((d: any) => d.team === p.team);
        const up = ds.filter((d: any) => d.pos[2] > 2).length;
        transmit(`${S.teamTitle(p.team)} pad → Command  HEARTBEAT  ${up} up, ${ds.length - up} on pad`, 0, p.at[0], p.at[1], 1);
      });
    if (tick % 10 === 0) {
      ui.rate = { msgs: sentMsgs, kb: sentBytes / 1024 };
      sentMsgs = 0;
      sentBytes = 0;
    }
  }, 100);

  // ---------- Phone accelerometer (LIVE through the engine) ----------
  // A shake or knock on the phone (|a| far from its running baseline) starts the campus earthquake.
  // shortcut: a fixed 2.5 m/s² rule; the planned handling detector (a lifted phone is never a quake) is not built yet.
  const SHAKE_MS2 = 2.5;
  let baseline: number | null = null;
  let over = 0;
  let lastQuakeAt = -Infinity;
  async function pollPhone() {
    while (ui.phone !== 'off') {
      try {
        const r = await fetch('/api/accel');
        if (!r.ok) throw new Error(await r.text());
        const { samples } = await r.json();
        if (ui.phone === 'off') break;
        ui.phone = 'live';
        let peak = 0;
        for (const [, a] of samples as [number, number][]) {
          baseline = baseline == null ? a : baseline * 0.98 + a * 0.02;
          const dev = Math.abs(a - baseline);
          peak = Math.max(peak, dev);
          over = dev > SHAKE_MS2 ? over + 1 : 0;
          if (over >= 3 && performance.now() - lastQuakeAt > 20000 && (!sim.quake || sim.quake.status === 'clear')) {
            lastQuakeAt = performance.now();
            const g = Math.min(0.6, Math.max(0.15, dev / 9.81));
            const text = S.quake(sim, [world.centre[0] - 560, world.centre[1] - 560], g, { live: true });
            log('aether', text);
            speak(text);
          }
        }
        ui.phoneLevel = peak;
        const home = world.pads[0];
        if (samples.length) transmit(`PHONE → Command  ACCEL LIVE  ${samples.length} samples  peak Δ ${peak.toFixed(2)} m/s²`, peak > SHAKE_MS2 ? 2 : 1, home.at[0], home.at[1], 2);
      } catch (e) {
        if (ui.phone !== 'off') ui.phone = 'error';
        console.warn('phone accelerometer:', e);
        await new Promise((d) => setTimeout(d, 2000));
        if (ui.phone === 'error') ui.phone = 'connecting';
        continue;
      }
      await new Promise((d) => setTimeout(d, 120));
    }
  }
  function togglePhone() {
    if (ui.phone === 'off') {
      ui.phone = 'connecting';
      baseline = null;
      pollPhone();
    } else ui.phone = 'off';
  }

  /** Runs one command. Hands-free chatter with no team and no command word is shown but not answered. */
  function run(text: string, heard = false) {
    text = text.trim();
    if (!text) return;
    const r = S.parse(sim, text);
    if (heard && r.unheard) {
      log('heard', text);
      return;
    }
    log('you', text);
    if (r.error) {
      log('error', r.error);
      speak(r.error);
      return;
    }
    const reply = S.apply(sim, r.cmd);
    if (r.cmd.kind === 'reset') {
      picked = null;
      ui.picked = null;
      scene.showPick(null);
    }
    log('aether', reply);
    speak(reply);
  }

  let transcribing = 0;
  const listener = createListener({
    state(st) {
      ui.mic = st === 'off' ? 'off' : transcribing ? 'transcribing' : st;
    },
    async phrase(wav) {
      transcribing++;
      ui.mic = 'transcribing';
      try {
        run(await transcribe(wav), true);
      } catch (e) {
        log('error', 'Speech-to-text unavailable (is Aether running?). Type the command instead.');
        console.warn(e);
      } finally {
        if (--transcribing === 0 && ui.mic === 'transcribing') ui.mic = 'listening';
      }
    },
  });
  async function toggleListening() {
    if (ui.mic !== 'off') return listener.stop();
    try {
      await listener.start();
    } catch (e) {
      ui.mic = 'off';
      log('error', 'Microphone unavailable. Type the command instead.');
      console.warn(e);
    }
  }

  const actions = {
    run,
    toggleListening,
    togglePhone,
    toggleStream() {
      ui.streamOpen = !ui.streamOpen;
      if (!ui.streamOpen) ui.stream = [];
    },
    igniteHere() {
      if (picked) run(`Simulate a fire at ${picked.name ?? 'this building'}`);
    },
    focusFire() {
      if (sim.fire) scene.focus(sim.fire.at);
    },
  };

  // Keys: Space turns listening on or off, Esc aborts, R resets. Never while typing in the command box.
  const typing = (e: KeyboardEvent) => (e.target as HTMLElement)?.closest?.('input, textarea');
  addEventListener('keydown', (e) => {
    if (e.metaKey || e.ctrlKey || e.altKey || typing(e)) return;
    if (e.key === ' ') {
      e.preventDefault();
      if (!e.repeat) toggleListening();
    } else if (e.key === 'Escape') run('Abort');
    else if ((e.key === 'r' || e.key === 'R') && !e.repeat) run('Reset');
  });

  // HUD at about 10 Hz.
  setInterval(() => {
    ui.teams = S.TEAMS.map((t) => ({ ...t, ...S.teamStatus(sim, t.id) }));
    const q = sim.quake;
    ui.quake = q
      ? { live: q.live, detected: q.detected, status: q.status, peakG: q.peakG, sensors: q.detectedBy, affected: q.affected.length, inspected: q.inspected.size }
      : null;
    const f = sim.fire;
    ui.fire = f
      ? {
          name: f.name,
          status: f.status,
          detected: f.detected,
          detectedBy: f.detectedBy ? S.spoken(f.detectedBy) : null,
          responders: (f.responders ?? []).map(S.teamTitle),
          detectedAfter: f.detectedAfter,
          outAfter: f.outAfter ?? null,
          across: Math.round(f.radius * 2),
          intensity: f.intensity,
          alertRadius: Math.round(f.radius + S.ALERT_MARGIN_M),
          zone: f.zone.length,
          alertedNames: [...f.alerted].map((i: number) => world.buildings[i].name ?? 'Unnamed building'),
          onStation: f.onStation ?? 0,
        }
      : null;
  }, 100);

  mount(Campus, { target: document.getElementById('app')!, props: { actions } });
  log('aether', 'Campus ready. Press Space or Start listening, then say "All teams, patrol the campus".');

  // Test-only, read-only hook for screenshots and checks.
  Object.defineProperty(window, '__campus', { value: Object.freeze({ sim, run, stats: () => scene.stats() }) });
}

main().catch((e) => {
  document.body.textContent = `Campus map failed to load: ${e?.message ?? e}`;
  console.error(e);
});
