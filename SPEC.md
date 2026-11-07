# SPEC.md — Aether, UB Hacking 2026

Status: grill complete (rounds 1–3 settled, 2026-11-07). Tickets T1–T7 cut; building starts once the user approves their GATE.md checks. Vocabulary: GLOSSARY.md. Decisions with trade-offs: docs/adr/.

## Idea and track
- Idea: Aether — a rescue swarm you command by voice and by pointing, flying through a LiDAR copy of the room the judges stand in. A real shake of the table, sensed by the phone, starts the mission. Picking the phone up turns it into a pointer. Spoken commands become typed commands; a policy check approves or rejects each one before the swarm acts. "Abort" never waits for the AI.
- Real vs simulated (must be shown in the UI and the pitch): the drones are simulated (SIM). The room geometry (LiDAR scan), the phone's motion, the voices, and optionally the webcam's view of people are real (LIVE). Recorded runs played back are REPLAY.
- Target: Overall (1st place). Also entering Radial Atoms, Best Use of Tiger Data, Best Use of ElevenLabs, Best Use of Gemini API, Best UI/UX Hack, Best Domain Name from GoDaddy Registry. Vultr decided in Tier C. Not entering Solana, Best Hardware, Most Over-Engineered.
- Platform: web app on the laptop. A native Rust engine plus a browser UI. Hardware is only an iPhone 13 Pro Max (phyphox for sensors, a LiDAR scanning app for the room) and the laptop (webcam, mic, speakers). Nothing physical gets built.
- Sponsor tech: Tiger Data is the database; ElevenLabs for speech in (Scribe v2 Realtime) and speech and sound out; Gemini turns speech into commands and writes the situation report; phyphox for phone sensors.
- Submission deadline: 2026-11-08 13:30 EST on Devpost (code freeze then; demo video under 3 minutes is part of the submission). Event rules are in AGENTS.md → Project facts.

## Who is this for?
Incident commanders of search-and-rescue teams after a building collapse. They must search a damaged space before sending people in, and they give orders while their hands and eyes are busy, so they speak and point.

## What breaks if it's wrong?
- A Drone flies into a wall or a person → the policy check rejects any target that is not free space in the Room Twin.
- A misheard Command searches the wrong place → every Command is shown and read back as it runs.
- A false Quake → a lifted or handled phone is never a Quake.
- Abort fails → Abort never depends on the network or the AI.
- Demo day: the phone link drops → a recorded LIVE run plays back, labelled REPLAY.

## What does done look like? (acceptance criteria per feature)
- Demo flow — what a judge sees end to end, in 3 minutes:
  1. The LiDAR copy of this room on screen, drones on a launch pad, LIVE/SIM labels visible.
  2. A judge shakes the table; the phone detects it; the alarm fires and the latency display shows the delay; the drones launch and start searching.
  3. Picking up the phone switches it from sensor to pointer; a target marker follows where it points.
  4. "Aether, send two drones there" (while pointing) sends two drones to that spot, routed around the scanned furniture. The transcript, the typed command, and the policy result are on screen.
  5. A judge gives a command; an unsafe one is rejected with a reason.
  6. "Abort" stops every drone without waiting for the AI.
  7. A spoken situation report, then the mission timeline from Tiger with every command and its policy result.
- Done means: the 7-step flow runs end to end 5 times in a row on the venue network, each run under 3 minutes, and every core algorithm passes a `cargo test` against a known answer.
- Per-feature criteria: written into GATE.md when tickets are cut.

## Build plan (tiers; each tier ends demo-able)
- Spikes T1–T7 (to about 17:30): Rust toolchain and workspace; Tiger insert; Gemini function call; ElevenLabs speech out and in, latency measured; phyphox on iPhone reaches Rust (Personal Hotspot over USB, else over Wi-Fi); the room scan loads as a Room Twin; one Drone as a MAVLink endpoint flown by pymavlink.
- Tier A "swarm in the room" (to 21:00): six MAVLink Drones on the kinematic model; occupancy grid, distance field, 3D path search, collision avoidance between Drones; point cloud and Drones rendered; keyboard Commands.
- Tier B "voice, wand, memory" (to 03:00): always-on voice loop (wake rule, Scribe, Gemini, policy check, spoken replies); Abort paths; Quake detection and Sensor/Pointer mode switch; pointing ray; full rigid-body Drone model with sensors, estimator, battery sag and radio link; pymavlink gate and QGroundControl view; Telemetry into Tiger with compression and continuous aggregates; Voice line and speech-to-map highlighting; telemetry panel; replay; latency display. First cut if behind: the full rigid-body model (the kinematic model stays, Telemetry keeps its real format and rates).
- Sleep 03:00–06:00.
- Tier C "wow" (to 08:00, only if ahead): one PX4 simulator Drone (SIH, only if it runs fast on this laptop) in the Swarm; exploration lighting up the map; drone-eye views; webcam people placed in the Room Twin; Tiger fork for what-if; Vultr plus domain live mirror.
- 08:00–11:30 design pass, rehearsals, fallbacks. 11:30 code freeze, repo public. 11:30–13:15 video and Devpost.

## Naming rules
One shared vocabulary for database, backend, and UI lives in GLOSSARY.md (created during the grill when the first term is settled). Code uses those exact words.

## Seven pillars — cover or consciously skip each
| Pillar | Covered / skipped | How |
|---|---|---|
| Frontend | Covered | Browser UI: Vite and Svelte 5 for panels, Three.js for the Room Twin (updated directly, outside Svelte state), Rust compiled to WebAssembly with wasm-pack for in-browser computation |
| Backend | Covered | Native Rust engine on the laptop (tokio, axum): sensor intake, simulation, voice-to-command pipeline, policy check, WebSocket stream, replay |
| Database | Covered | Tiger Data (Tiger Cloud Free plan): hypertables, continuous aggregates, compression |
| Authentication | Skipped | Single-operator demo on one laptop; no user accounts |
| Payments | Skipped | Not needed |
| Security | Covered | Secrets only in .env; API keys stay in the Rust engine (it relays or mints short-lived tokens); every AI command passes a policy check; abort bypasses the AI; voice transcripts are untrusted and only typed commands reach the swarm |
| Infrastructure | Covered | Runs on the laptop; iPhone linked by Personal Hotspot; Tiger Cloud; Vultr optional in Tier C |

## Tickets
Each ticket: one outcome, small enough for one agent to finish and one human to check in about five minutes. Its gates live in GATE.md under the same id.
- T0: Project setup — scaffold, rules, workflow, handoff and setup docs, secret scanning, skill scanning. Done.
- T1: Rust toolchain and workspace. Homebrew has no prebuilt rustup or wasm-pack for Intel Macs any more (checked 2026-11-07: no bottles, so it would compile them and Rust itself from source), so install rustup with the official installer from https://sh.rustup.rs (stable toolchain, default PATH setup through `~/.cargo/env`), add the `wasm32-unknown-unknown` target, and put the prebuilt `wasm-pack-v0.15.1-x86_64-apple-darwin` binary from the wasm-bindgen/wasm-pack GitHub release into `~/.cargo/bin`. Cargo workspace: `src/core` (aether-core, library), `src/engine` (aether-engine, library plus binaries), `src/wasm` (aether-wasm, cdylib). Dependencies, added with `cargo add` so versions come from crates.io and not from memory; TLS through native-tls wherever a crate offers it, feature names read from `cargo add` output: core — serde (derive), serde_json, glam, ply-rs-bw, tobj; engine — aether-core, tokio (full), anyhow, dotenvy, serde, serde_json, reqwest (json, stream), tokio-postgres, postgres-native-tls, native-tls, tokio-tungstenite, futures-util, base64, mavlink (common dialect, UDP); wasm — aether-core, wasm-bindgen. The audio crates (cpal, rodio) and the web server (axum) wait for Tier A. Stub the modules (engine: tiger, gemini, voice, phyphox, mavlink_link; core: command, room_twin, vehicle) so T2–T7 only write inside their own files. `target/`, `.venv/` and `src/wasm/pkg/` gitignored; .env.example lists every variable the code will read (ELEVENLABS_API_KEY, ELEVENLABS_VOICE_ID, GEMINI_API_KEY, GEMINI_MODEL, TIGER_DATABASE_URL, PHYPHOX_URL, ROOM_SCAN_PATH), without values. Owns: Cargo.toml, Cargo.lock, src/core/**, src/engine/**, src/wasm/**, tests/wasm-smoke.mjs, tests/env-example-check.mjs, .gitignore, .env.example. Depends on: nothing.
- T2: Tiger spike. Connect over TLS from `TIGER_DATABASE_URL`, create a scratch hypertable, batch-insert 10,000 rows, count them back, report rows per second, drop the table. Manual: try a fork on the Free plan; record whether it works and how long it takes. Owns: src/engine/src/tiger.rs, src/engine/src/bin/probe_tiger.rs. Depends on: T1, the Tiger connection string in .env.
- T3: Gemini spike. The Command type in aether-core, parsed and validated from JSON with unknown fields rejected; a Gemini function call turns "Aether, send two drones to the door" into a Command; latency reported. Owns: src/core/src/command.rs, src/engine/src/gemini.rs, src/engine/src/bin/probe_gemini.rs. Depends on: T1, `GEMINI_API_KEY`.
- T4: Voice spike. ElevenLabs text-to-speech with character timestamps for a fixed sentence; that audio streamed to Scribe v2 Realtime with a manual commit; the transcript must contain "drones" with word timestamps; both latencies reported. Owns: src/engine/src/voice.rs, src/engine/src/bin/probe_voice.rs. Depends on: T1, `ELEVENLABS_API_KEY`.
- T5: phyphox spike. Read buffer names from `/config`, poll `/get` incrementally for 5 seconds, report sample count and rate. Manual: record which link worked (USB Personal Hotspot or Wi-Fi). Owns: src/engine/src/phyphox.rs, src/engine/src/bin/probe_phyphox.rs. Depends on: T1, the iPhone, `PHYPHOX_URL`.
- T6: Room Twin spike. The first scan (2026-11-07) is a Scaniverse 5.2.9 "Area" OBJ with a JPG texture: meters, Y up, 12.3 × 9.9 m floor, 3.4 m high, 177,186 vertices, 324,071 triangles. Its header comments carry Latitude, Longitude, Elevation and Rotation (heading, degrees), at Davis Hall. aether-core parses OBJ from bytes, not from a file path, so the same code runs in WebAssembly later. It reads those header values when present, and fills a 10 cm occupancy grid over the scan's bounding box by sampling every triangle at no more than half a voxel spacing, so large flat triangles leave no holes. OBJ only for now; PLY only if a later scan needs it. Unit tests on a synthetic closed box room with walls on voxel centres check exact occupied and free counts, a wall made of two large triangles with no holes, and the header parse. The probe reads `ROOM_SCAN_PATH` and prints vertex, triangle, occupied-voxel counts, the free share, the size and the location. It exits 1 if the scan is implausible: height outside 2–6 m, a floor side outside 2–60 m, fewer than 1,000 occupied voxels, or a free share outside 50–99%. The scan files stay outside the repo; the texture can show people. Owns: src/core/src/room_twin.rs, src/engine/src/bin/probe_room.rs. Depends on: T1, the scan at `ROOM_SCAN_PATH`.
- T7: MAVLink Drone spike. One kinematic Drone (Tier A model: position follows the setpoint under a speed and acceleration limit, tilt follows commanded acceleration, never below the floor) as a MAVLink v2 endpoint on UDP. It identifies as a PX4 quadrotor (system id = Drone number, component 1) and sends from its own port to the ground station at 127.0.0.1:14550, the address QGroundControl listens on. It sends HEARTBEAT at 1 Hz with PX4 mode encoding, ATTITUDE at 50 Hz, LOCAL_POSITION_NED at 30 Hz, EXTENDED_SYS_STATE, and a COMMAND_ACK for every command. PX4 offboard flow: offboard mode is accepted only while position setpoints (SET_POSITION_TARGET_LOCAL_NED) have been arriving faster than 2 Hz; then arm, climb to a 1 m setpoint, move to a second setpoint, land on MAV_CMD_NAV_LAND, disarm on touchdown. pymavlink 2.4.50 (in .venv) flies that sequence, checks each step, and measures the Telemetry rates (at least 45 Hz attitude, 25 Hz position). Owns: src/core/src/vehicle.rs, src/core/src/vehicle/**, src/engine/src/mavlink_link.rs, src/engine/src/bin/sim_drone.rs, tests/mavlink_flight.py, tests/requirements.txt. Depends on: T1.
- Gates enter GATE.md when a ticket starts: the checker runs every unmet gate in the ledger, so only tickets in progress are listed. T2, T3, T4 and T7 can run in parallel after T1; T5 and T6 start when the phone link and the scan are ready. Tier A tickets are cut after the spikes report.

## Open questions
- Grill: none open.
- Unverified until the spikes: phyphox on iOS over Personal Hotspot; the scanning app's free export and scan quality; Scribe and Gemini latency; Gemini function calling on `gemini-3.6-flash` (fallback `gemini-3.5-flash-lite` if slow); forks on the Tiger Free plan; Rust compile times on this Intel Mac.
- Unverified until Tier B: QGroundControl accepting our Drones without a full parameter list; Tiger compression keeping Telemetry at real rates under 750 MB; whether PX4's SIH simulator runs fast on this laptop.

## Decisions log (change this file first, then the code)
- 2026-11-07 — Hackathon rules: AI assistants allowed; no pre-built template.
- 2026-11-07 — Solo for now; local git repo until a GitHub repo is created.
- 2026-11-07 — Auto-compaction default changed from 40% to 60% of a 600k window.
- 2026-11-07 — Organizers provide no repo. Code lives in a private GitHub repo (remote `origin`); commits use the GitHub no-reply email. Going public needs the personal-identifier check in AGENTS.md first.
- 2026-11-07 — Concept: Aether, the voice-and-point rescue swarm in a LiDAR copy of the judging room. Replaces the earlier crisis dashboard and the tabletop-tower idea: nothing physical can be built, and the only hardware is an iPhone 13 Pro Max and the laptop.
- 2026-11-07 — Dropped: SMS (a Twilio trial only texts verified numbers, with template text), LangGraph and the multi-voice swarm, real drones (no hardware; UB requires approval for drone flights).
- 2026-11-07 — Rust split: a shared `aether-core` crate (native and WebAssembly), a native `aether-engine`, and `aether-wasm` in the browser. TypeScript only draws. An all-Rust UI was rejected: slower design iteration and glue code for every JavaScript library.
- 2026-11-07 — Categories: Overall, Radial Atoms, Tiger Data, ElevenLabs, Gemini, Best UI/UX, GoDaddy domain. Vultr decided in Tier C.
- 2026-11-07 — Time plan: feature cut-off 08:00, sleep 03:00–06:00, code freeze 11:30, video and Devpost done by 13:15 on 2026-11-08.
- 2026-11-07 — Tiger: start on the Free plan; compress raw data after a few minutes; show the compression ratio in the UI; move to the 30-day trial if forks are missing on Free.
- 2026-11-07 — Going public at the 11:30 freeze: keep the GATE.md evidence paths; replace the third-party handle in the research doc with a plain credit.
- 2026-11-07 — Voice: no push-to-talk. The mic stays on while idle, and Aether responds only when addressed directly by name.
- 2026-11-07 — Wake rule: an utterance must start with something that sounds like "Aether" (aether, ether, either) and contain a valid Command; otherwise Aether stays silent and shows nothing.
- 2026-11-07 — Voice loop runs natively in the Rust engine: mic capture (cpal), local voice-activity check deciding utterance ends (Scribe manual commit), Scribe v2 Realtime with word timestamps, wake rule, Gemini, policy check, spoken replies (rodio). The word timestamp of "there" picks the pointing direction at that moment.
- 2026-11-07 — While Aether speaks, the mic ignores everything except "abort"; Aether never says its own name.
- 2026-11-07 — Privacy: listening only while the demo is armed, with a visible listening light; audio never stored; only accepted Command text goes to Tiger; disclosed on Devpost.
- 2026-11-07 — Abort paths, none waiting for the AI: the word "abort" with or without the name (caught in the partial transcript), the Esc key, a sharp flick of the phone in Pointer mode.
- 2026-11-07 — Reset between judge pairs: R returns Drones to the pad, clears exploration, starts a new Mission; putting the phone down returns it to Sensor mode. Target under 5 seconds.
- 2026-11-07 — Swarm of 6 Drones, each with a speed and acceleration limit, a 0.3 m safety radius, a simulated depth sensor (about 4 m range, 90° view), and a battery the policy check respects. Drone model fidelity reopened in round 3.
- 2026-11-07 — Frontend: Vite and Svelte 5; Three.js scene updated directly from the WebSocket, not through Svelte state.
- 2026-11-07 — Design direction: dark "field instrument". Near-black background, warm-grey point cloud, one accent colour (rescue orange) only for Drones, alarms and the target marker; no gradients; monospace numbers, clean sans-serif text; the Room Twin full screen with thin overlay panels (Command log left, Mission timeline bottom, latency top right, Drone list right); transitions under about 200 ms.
- 2026-11-07 — Vocabulary fixed in GLOSSARY.md. Rust split recorded in docs/adr/0001-rust-computes-typescript-draws.md.
- 2026-11-07 — Requirement: Drone telemetry must be realistic enough that real drones could replace the simulated ones and still work. Details in round 3.
- 2026-11-07 — Requirement: a polished, live, interactive interface with nothing that looks like generic AI design. It shows clearly when Aether is speaking and what is happening in the Room Twin at that moment. Details in round 3.
- 2026-11-07 — Vehicle boundary: MAVLink v2 (common dialect), one UDP endpoint per Drone, PX4-style arming, takeoff, land, return-to-launch, offboard setpoints faster than 2 Hz, failsafes for link loss and low battery. North-east-down local frame; the Room Twin is pinned to a fixed origin at Davis Hall with a heading set once. See docs/adr/0002-drones-are-mavlink-endpoints.md.
- 2026-11-07 — Drone model staged: Tier A kinematic quadrotor (tilt follows commanded acceleration); Tier B rigid body with four motors (spin-up lag, thrust with motor speed squared), drag, a PX4-style cascaded controller, noisy sensors with drift, an onboard estimator whose output is the Telemetry, a LiPo battery with voltage sag, and a radio link with delay and loss that worsen through walls. No GPS indoors: position comes from vision and drifts. Airframe parameters from a published small-indoor-quad spec, cited; unpublished values marked assumed.
- 2026-11-07 — Compatibility proof, software only: a pymavlink gate test (arm, take off, fly to a target, land); QGroundControl shows the Swarm live during the demo; stretch: one PX4 simulator instance in the Swarm. Devpost wording: "MAVLink-compatible, verified with pymavlink and QGroundControl; not flown on hardware."
- 2026-11-07 — Telemetry at real rates per Drone (attitude 50 Hz, position 30 Hz, battery and link 10 Hz, heartbeat 1 Hz) into typed Tiger hypertables, compressed after 5 minutes, continuous aggregates for trends; any Mission exports as a .tlog that opens in QGroundControl.
- 2026-11-07 — Telemetry panel per Drone: artificial horizon, altitude, speed, battery voltage, current and remaining with visible sag, link quality and loss, flight mode, estimator health, position source. Monospace values that update in place.
- 2026-11-07 — Voice line, not an orb or waveform: Listening (light plus real mic level), Being addressed (live words; locks and brightens on the wake rule), Understanding (the Command fills in field by field), Verdict, Speaking (words appear in sync with the audio from ElevenLabs character timestamps, with the real output level).
- 2026-11-07 — Speech-to-map linking: every Drone, place, or target mentioned highlights in the Room Twin at the instant its word is spoken, by the speaker or by Aether.
- 2026-11-07 — Interaction: orbit and zoom the Room Twin; click a Drone for its Telemetry; click a spot for a manual Command that gets a Verdict like a spoken one; scrub the Mission timeline (REPLAY); keys R reset, Esc Abort, F follow a Drone.
- 2026-11-07 — No physical drones and no tests on real drones (learned rule in AGENTS.md). Optional 3D flight simulators allowed only if fast on this Intel Mac; PX4's SIH over Gazebo.
- 2026-11-07 — Code layout: a Cargo workspace under src/ — src/core (aether-core), src/engine (aether-engine), src/wasm (aether-wasm); the web UI later in src/web.
- 2026-11-07 — T1 dependency features: tobj without default features (its `ahash` default breaks the wasm32 build); reqwest without defaults plus json, stream, native-tls (its default TLS is rustls, so one TLS stack stays; no HTTP/2 or system proxy, which the REST calls do not need); mavlink without defaults plus std, transport-udp, dialect-common (T7 adds `serde` if it needs it).
- 2026-11-07 — Code reads environment variables only as `std::env::var("NAME")` with a literal name, so tests/env-example-check.mjs can see every read.
