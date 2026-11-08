// Reads the last few seconds of acceleration magnitude (|a|, m/s², gravity included) and says what kind of motion it
// is. A building sensor is fixed to the structure, so the phone stands in for one when it lies on a table:
//   still    - lying flat, only sensor noise
//   handling - picked up, carried, tapped: irregular movement
//   jump     - one sharp impact or a free fall: a short spike over a quiet background
//   quake    - sustained, oscillating shaking for most of the window
// shortcut: fixed thresholds tuned on simulated feeds and the phyphox `acc` buffer; a real sensor network would learn
// them per building and confirm across several nodes (as the campus page does with three).

export const G = 9.81;
export const WINDOW_S = 3;
const STILL_RMS = 0.12; // m/s², phone noise on a table
const QUAKE_RMS = 0.35; // m/s², about 0.035 g of sustained shaking
const QUAKE_ACTIVE = 0.75; // share of the window that must be shaking
const SPIKE_CREST = 3.6; // peak / RMS above this is a single impact, not shaking
const SPIKE_PEAK = 4; // m/s²
const FREE_FALL = 3; // m/s²: |a| this low means the phone is falling

/** samples: [{ t (s), m (|a| m/s²) }] sorted by t. Returns the verdict and the numbers behind it. */
export function classify(samples) {
  if (samples.length < 20) return { kind: 'waiting', rms: 0, peak: 0, freq: 0, active: 0 };
  const end = samples[samples.length - 1].t;
  const win = samples.filter((s) => s.t >= end - WINDOW_S);
  const mean = win.reduce((a, s) => a + s.m, 0) / win.length;
  const dyn = win.map((s) => s.m - mean);
  const rms = Math.sqrt(dyn.reduce((a, d) => a + d * d, 0) / dyn.length);
  const peak = Math.max(...dyn.map(Math.abs));
  const lowest = Math.min(...win.map((s) => s.m));
  const span = Math.max(0.5, win[win.length - 1].t - win[0].t);

  // Dominant frequency from sign changes, with hysteresis so noise does not count.
  let crossings = 0, sign = 0;
  for (const d of dyn) {
    const s = d > 0.15 ? 1 : d < -0.15 ? -1 : 0;
    if (s && sign && s !== sign) crossings++;
    if (s) sign = s;
  }
  const freq = crossings / 2 / span;

  // Share of quarter-second slices that are moving.
  const slices = Math.max(1, Math.round(span / 0.25));
  let moving = 0;
  for (let k = 0; k < slices; k++) {
    const t0 = win[0].t + (k * span) / slices, t1 = t0 + span / slices;
    const part = dyn.filter((_, i) => win[i].t >= t0 && win[i].t < t1);
    if (part.length && Math.sqrt(part.reduce((a, d) => a + d * d, 0) / part.length) > STILL_RMS * 2) moving++;
  }
  const active = moving / slices;
  const crest = peak / Math.max(rms, 1e-6);

  let kind = 'handling';
  if (rms < STILL_RMS) kind = 'still';
  else if (lowest < FREE_FALL || (peak > SPIKE_PEAK && crest > SPIKE_CREST)) kind = 'jump';
  else if (rms > QUAKE_RMS && active >= QUAKE_ACTIVE && freq >= 1 && freq <= 12) kind = 'quake';
  return { kind, rms, peak, freq, active };
}

/** Simulated phone feed for testing without a phone: |a| at time t (s) for a scenario. Labelled SIM wherever shown. */
export function simulate(kind, t, rand = Math.random) {
  const noise = (rand() - 0.5) * 0.08;
  if (kind === 'still') return G + noise;
  if (kind === 'handling') return G + 0.9 * Math.sin(1.1 * t) * Math.sin(0.37 * t) + 0.5 * Math.sin(4.3 * t) * (Math.sin(0.5 * t) > 0.3 ? 1 : 0) + noise * 3;
  if (kind === 'jump') {
    const p = t % 4; // one jump every 4 s: push-off, free fall, landing
    if (p > 1 && p < 1.15) return G + 6;
    if (p >= 1.15 && p < 1.5) return 0.6 + noise;
    if (p >= 1.5 && p < 1.6) return G + 16;
    return G + noise;
  }
  // quake: 3 Hz shaking with a slowly varying envelope
  return G + 1.6 * (0.7 + 0.3 * Math.sin(0.8 * t)) * Math.sin(2 * Math.PI * 3 * t) + noise * 4;
}
