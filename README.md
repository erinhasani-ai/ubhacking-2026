# Aether

An all-in-one campus safety console: teams of drones you command by voice, on a 3D map of UB North Campus, responding to fires and earthquakes. Built at UB Hacking 2026.

**Everything that flies is simulated.** The screen labels each source: **SIM** for simulated, **LIVE** for real inputs. No drone has been flown on hardware.

| Real (LIVE) | Simulated (SIM) |
|---|---|
| LiDAR scan of the Davis Hall 3rd floor (iPhone 13 Pro Max, Scaniverse) | All drones: 6 on the Davis floor, 30 on campus |
| Campus buildings, roads and water from OpenStreetMap | Fire detection (a stand-in for a thermal camera) and suppression |
| Your voice (ElevenLabs Scribe v2) and Aether's spoken replies (ElevenLabs) | Earthquake shaking at campus accelerometer nodes |
| The phone accelerometer through phyphox, when connected | Campus alerts (a preview card; nothing is sent to anyone) |

## What it does

**Campus view** (`/campus.html`)
- UB North Campus in 3D: 291 buildings from OpenStreetMap. 103 have tagged heights; the rest are estimated at 12 m and drawn with fainter edges.
- Five teams (Alpha, Bravo, Charlie, Delta, Echo) of six drones, each team on its own pad. Every drone scouts; each team has 4 suppression drones and 2 alert drones.
- Hands-free voice. The mic turns on when the page opens (tap the mic button or press Space to turn it off). Start each command with "Aether":
  - "Aether, all teams patrol the campus"
  - "Aether, Bravo, go to Lockwood Library"
  - "Aether, Delta, alert nearby buildings"
  - "Aether, status report"
  - "Aether, abort"
  
  Speech that does not address Aether is shown greyed out and never acted on. Aether answers aloud. You can also type commands (no "Aether" needed) or click the example chips.
- **Random events:** "Simulate a fire somewhere random" or "Simulate a random earthquake" starts one at a random building. Untick **Automatic response** and direct the teams yourself: "Delta, go to the earthquake", "Echo, put out the fire".
- **Fire:** "Simulate a fire at Capen Hall". A patrolling drone that comes within 250 m detects it. The team on the nearest pad suppresses it, alert drones from two teams warn the nearest buildings, and one drone circles to monitor. The map shows the burning area, the alert radius, the buildings inside it and the buildings already warned.
- **Earthquake:** "Simulate an earthquake", or shake the phone with **Phone accelerometer** switched on. Accelerometer nodes on 28 buildings stream readings. When three exceed 0.05 g, Aether colours buildings by shaking strength and sends drones to inspect the hardest-hit ones.
- **Data stream:** every drone telemetry message, pad heartbeat and sensor reading flashes as a ping on the map. The **Data stream** button opens the feed.

**Davis floor** (`/`)
- The LiDAR scan of Davis Hall's 3rd floor as a point cloud, with a hologram layer of flat walls and floor rebuilt from it (H). Six simulated drones speak real MAVLink v2 (as PX4 quadrotors) to a Rust ground station.
- Click a spot, or a Place button, to send drones there. Every command first gets a safety verdict: the target must be free space with clearance. Paths go around the scanned furniture (A\* over a distance field), and ORCA keeps drones apart.
- R resets the drones to the pad, F follows the drones with the camera, and Esc aborts. Abort never waits on anything.

**Sensor test** (`/phone.html`)
- A proof of concept for building sensors. Type the address phyphox shows on a phone (Acceleration (with g), Allow remote access) to see its accelerometer as a live stream and chart. A verdict panel says still, handling, jump or earthquake-like shaking, with the numbers behind it.
- No phone? The SIM buttons play a generated feed for each case.

## Run it

**Release (macOS):** download the zip from [Releases](../../releases), unzip it and double-click `Aether.command`. If macOS says it cannot be verified, open System Settings → Privacy & Security, click **Open Anyway**, and double-click again. The browser opens the campus view.
- For voice, fill in `ELEVENLABS_API_KEY` and `ELEVENLABS_VOICE_ID` in `settings.env` in the same folder.
- For the campus phone trigger, add `PHYPHOX_URL` to `settings.env`. The Sensor test page takes the phone's address on screen instead.
- Without keys, Aether speaks with the browser voice and you type the commands.

**From source:** Rust (stable) and Node 22.

```sh
cp .env.example .env          # fill in the keys you have; ROOM_SCAN_PATH points at a Scaniverse OBJ
cd src/web && npm ci && npm run build && cd ../..
cargo run --release -p aether-engine --bin aether
# open http://127.0.0.1:7700/campus.html  (room view: http://127.0.0.1:7700/)
```

Tests: `node tests/campus_sim.mjs` (campus grammar, wake word, random events, fire and earthquake scenarios), `node tests/phone_classify.mjs` (the sensor verdict on each simulated feed), and `cargo test --workspace` (core algorithms against known answers).

## How it's built

| Part | Built with |
|---|---|
| Engine (Rust: tokio, axum) | Floor scan → 20 cm occupancy grid → exact Euclidean distance field → A\* (0.35 m clearance) → ORCA → safety policy → mission control with priority Abort; MAVLink v2 ground station over UDP |
| Browser | Svelte 5, Three.js, Vite. Campus simulation in `src/web/src/campus/sim.js` |
| Voice | Engine routes `/api/stt` (ElevenLabs Scribe v2) and `/api/tts` (ElevenLabs), so API keys never reach the browser. Speech detection runs in the browser |
| Phone | Engine route `/api/accel` polls phyphox remote access |
| Prototypes, not wired into the demo | Gemini function calling for commands; Tiger Data (TimescaleDB) telemetry storage |

## AI disclosure

Built with AI coding assistants: Claude Code (Anthropic) and OpenAI Codex. We wrote the spec (`SPEC.md`), made the design and safety decisions, and wrote acceptance checks (`GATE.md`); the assistants wrote much of the code against those checks, under review.

## Credits

- Map data © OpenStreetMap contributors, ODbL.
- ORCA after RVO2-3D (Apache-2.0).
- Speech by ElevenLabs.
