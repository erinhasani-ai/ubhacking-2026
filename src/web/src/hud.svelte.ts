// HUD state: the only Svelte-reactive data. The Three.js scene never reads it; the stream writes it at about 10 Hz.

export type Provenance = 'LIVE' | 'SIM' | 'REPLAY';

export interface DroneRow {
  id: number;
  mode: string;
  armed: boolean;
  link_ms: number;
  provenance: Provenance;
}

export interface VerdictLine {
  seq: number;
  source: string;
  summary: string;
  /** Verdict words (approved, rejected) only for real Verdicts; pending until the engine answers; not_sent when the link was down. */
  state: 'pending' | 'approved' | 'rejected' | 'not_sent';
  reason: string | null;
}

export type Link =
  | { state: 'connecting' }
  | { state: 'open' }
  | { state: 'waiting'; retryAt: number; attempt: number };

export const hud = $state({
  url: '',
  link: { state: 'connecting' } as Link,
  /** Seconds until the next reconnect attempt, refreshed by the HUD timer. */
  retryIn: 0,
  room: null as null | { provenance: Provenance; size: [number, number, number]; points: number; drawn: number; cut: number; processing: boolean; enhanced: boolean },
  drones: [] as DroneRow[],
  selected: [] as number[],
  follow: false,
  verdict: null as null | VerdictLine,
  verdictVisible: false,
  /** The link dropped: Drones and link values are last-known until a fresh state frame arrives. */
  stale: false,
});
