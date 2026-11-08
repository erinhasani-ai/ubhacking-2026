// Campus HUD state: the only Svelte-reactive data on the campus page. main.ts copies the sim into it at about 10 Hz.

export interface TeamRow {
  id: string;
  name: string;
  pad: string;
  words: string;
  airborne: number;
  total: number;
}

export interface FireView {
  name: string;
  status: 'burning' | 'contained' | 'out';
  detected: boolean;
  detectedBy: string | null;
  detectedAfter: number | null;
  outAfter: number | null;
  across: number;
  intensity: number;
  alertRadius: number;
  zone: number;
  alertedNames: string[];
  onStation: number;
  responders: string[];
}

export interface LogLine {
  seq: number;
  who: 'you' | 'aether' | 'error' | 'heard';
  text: string;
}

export interface QuakeView {
  live: boolean;
  detected: boolean;
  status: 'shaking' | 'surveying' | 'clear';
  peakG: number;
  sensors: number;
  affected: number;
  inspected: number;
}

export interface StreamLine {
  seq: number;
  text: string;
  kind: 0 | 1 | 2;
}

export const ui = $state({
  quake: null as null | QuakeView,
  streamOpen: false,
  stream: [] as StreamLine[],
  rate: { msgs: 0, kb: 0 },
  phone: 'off' as 'off' | 'connecting' | 'live' | 'error',
  phoneLevel: 0,
  teams: [] as TeamRow[],
  fire: null as null | FireView,
  log: [] as LogLine[],
  mic: 'off' as 'off' | 'listening' | 'hearing' | 'transcribing',
  voice: 'ElevenLabs' as 'ElevenLabs' | 'Browser voice',
  muted: false,
  autoResponse: true,
  picked: null as null | string,
  heights: { tagged: 0, estimated: 0 },
  attribution: '',
});
