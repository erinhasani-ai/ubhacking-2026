// Campus voice, hands-free: the mic stays open, a simple voice-activity detector cuts each phrase, and the phrase
// goes to the engine's /api/stt (ElevenLabs Scribe). Replies are spoken through /api/tts (ElevenLabs); the key stays
// in the engine. Listening pauses while Aether speaks, so it never hears itself. If ElevenLabs speech fails, the
// browser voice speaks and the HUD says so.

let chain: Promise<void> = Promise.resolve();
let waiting = 0;
let talkingUntil = 0;

/** True while Aether is speaking, and briefly after, so the mic ignores the speakers. */
export const aetherTalking = () => waiting > 0 || performance.now() < talkingUntil;

/** Queues one spoken line; drops it if three are already waiting, so the voice never lags far behind the map. */
export function say(text: string, onSource: (s: 'ElevenLabs' | 'Browser voice') => void) {
  if (waiting >= 3) return;
  waiting++;
  // A clip that never reports its end must not keep the mic paused: give each line a generous time limit.
  const limit = new Promise<void>((done) => setTimeout(done, 3000 + text.length * 120));
  chain = chain
    .then(() => Promise.race([play(text, onSource), limit]))
    .finally(() => {
      waiting--;
      talkingUntil = performance.now() + 400;
    });
}

async function play(text: string, onSource: (s: 'ElevenLabs' | 'Browser voice') => void) {
  try {
    const r = await fetch('/api/tts', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ text }) });
    if (!r.ok) throw new Error(`TTS ${r.status}: ${await r.text()}`);
    const url = URL.createObjectURL(await r.blob());
    onSource('ElevenLabs');
    const audio = new Audio(url);
    await new Promise<void>((done) => {
      audio.onended = audio.onerror = () => done();
      audio.play().catch(() => done());
    });
    URL.revokeObjectURL(url);
  } catch (e) {
    console.warn('ElevenLabs speech unavailable; using the browser voice.', e);
    onSource('Browser voice');
    await new Promise<void>((done) => {
      const u = new SpeechSynthesisUtterance(text);
      u.onend = u.onerror = () => done();
      speechSynthesis.speak(u);
    });
  }
}

// Voice-activity thresholds (RMS of the raw mic signal). shortcut: fixed ratios over an adaptive noise floor, tuned
// by ear on the laptop mic; a loud venue may need the browser's noise suppression or a closer mic.
const START_MIN = 0.02;
const START_RATIO = 3.5;
const END_RATIO = 2;
const END_SILENCE_MS = 700;
const MAX_PHRASE_MS = 8000;
const PRE_ROLL_MS = 400;
const MIN_SPEECH_MS = 350;

export function createListener(on: { phrase(wav: Blob): void; state(s: 'off' | 'listening' | 'hearing'): void; level?(rms: number): void }) {
  let ctx: AudioContext | null = null;
  let stream: MediaStream | null = null;
  let node: ScriptProcessorNode | null = null;
  let src: MediaStreamAudioSourceNode | null = null;
  let noise = 0.01;
  let loud = 0;
  let quietMs = 0;
  let pre: Float32Array[] = [];
  let cur: Float32Array[] | null = null;

  function chunk(x: Float32Array, rate: number) {
    let sum = 0;
    for (let i = 0; i < x.length; i++) sum += x[i] * x[i];
    const rms = Math.sqrt(sum / x.length);
    const ms = (x.length / rate) * 1000;
    on.level?.(rms);
    if (aetherTalking()) {
      if (cur) on.state('listening');
      cur = null;
      pre = [];
      loud = 0;
      return;
    }
    if (!cur) {
      pre.push(x);
      if (pre.length * ms > PRE_ROLL_MS) pre.shift();
      if (rms > Math.max(START_MIN, noise * START_RATIO)) {
        if (++loud >= 2) {
          cur = pre;
          pre = [];
          quietMs = 0;
          on.state('hearing');
        }
      } else {
        loud = 0;
        noise = noise * 0.95 + rms * 0.05;
      }
      return;
    }
    cur.push(x);
    quietMs = rms < Math.max(START_MIN * 0.6, noise * END_RATIO) ? quietMs + ms : 0;
    const total = cur.length * ms;
    if (quietMs >= END_SILENCE_MS || total >= MAX_PHRASE_MS) {
      const clip = cur;
      cur = null;
      loud = 0;
      on.state('listening');
      if (total - quietMs - PRE_ROLL_MS >= MIN_SPEECH_MS) on.phrase(toWav(clip, rate));
    }
  }

  return {
    async start() {
      stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true } });
      ctx = new AudioContext();
      src = ctx.createMediaStreamSource(stream);
      // shortcut: ScriptProcessorNode is deprecated but needs no worklet file; move to an AudioWorklet after the event.
      node = ctx.createScriptProcessor(2048, 1, 1);
      const rate = ctx.sampleRate;
      node.onaudioprocess = (e) => chunk(new Float32Array(e.inputBuffer.getChannelData(0)), rate);
      src.connect(node);
      node.connect(ctx.destination); // Chrome only runs the processor when it is connected; its output stays silent
      on.state('listening');
    },
    stop() {
      node?.disconnect();
      src?.disconnect();
      stream?.getTracks().forEach((t) => t.stop());
      ctx?.close();
      ctx = null;
      stream = null;
      cur = null;
      pre = [];
      on.state('off');
    },
  };
}

/** Mono 16 kHz 16-bit WAV from float chunks at `rate`. */
export function toWav(chunks: Float32Array[], rate: number): Blob {
  const total = chunks.reduce((n, c) => n + c.length, 0);
  const all = new Float32Array(total);
  let o = 0;
  for (const c of chunks) {
    all.set(c, o);
    o += c.length;
  }
  const out = 16000;
  const n = Math.floor((total * out) / rate);
  const buf = new ArrayBuffer(44 + n * 2);
  const v = new DataView(buf);
  const str = (at: number, s: string) => [...s].forEach((ch, i) => v.setUint8(at + i, ch.charCodeAt(0)));
  str(0, 'RIFF');
  v.setUint32(4, 36 + n * 2, true);
  str(8, 'WAVE');
  str(12, 'fmt ');
  v.setUint32(16, 16, true);
  v.setUint16(20, 1, true);
  v.setUint16(22, 1, true);
  v.setUint32(24, out, true);
  v.setUint32(28, out * 2, true);
  v.setUint16(32, 2, true);
  v.setUint16(34, 16, true);
  str(36, 'data');
  v.setUint32(40, n * 2, true);
  for (let i = 0; i < n; i++) {
    const s = Math.max(-1, Math.min(1, all[Math.floor((i * rate) / out)]));
    v.setInt16(44 + i * 2, s * 0x7fff, true);
  }
  return new Blob([buf], { type: 'audio/wav' });
}

export async function transcribe(audio: Blob): Promise<string> {
  const r = await fetch('/api/stt', { method: 'POST', headers: { 'content-type': audio.type || 'audio/wav' }, body: audio });
  if (!r.ok) throw new Error(`speech-to-text ${r.status}: ${await r.text()}`);
  return (await r.json()).text ?? '';
}
