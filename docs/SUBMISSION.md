# Submission kit: video, pitch, judge Q&A

Devpost text is in the chat reply. This file covers the demo video (must be under 3:00), the live pitch (3 min, then 2 min of questions), and the fallbacks.

## Before recording or pitching
1. Start Aether: `cargo run --release -p aether-engine --bin aether`, or the release's `Aether.command`.
2. Open `http://127.0.0.1:7700/campus.html` in Chrome, full screen.
3. Click **Start listening** once; allow the microphone. A good place for the laptop mic is 30–50 cm from your mouth.
4. Phone: on the USB hotspot with phyphox **Remote access** running, tick **Phone accelerometer** and wait for **LIVE**.
5. Press **R** to reset. Have the Room Twin tab (`/`) open in a second tab.

## Demo video (target 2:45)

| Time | On screen | Say |
|---|---|---|
| 0:00–0:15 | Campus map, slow orbit | "When something goes wrong on campus, the first minutes are blind. Aether gives campus safety teams of drones they command by voice." |
| 0:15–0:40 | Say "All teams, patrol the campus". Thirty drones lift off from five pads; open **Data stream** | "Five teams of six, on pads across UB North. Every drone scouts; each team has four suppression and two alert drones. Every ping is live telemetry: position, battery, thermal reading, radio link." |
| 0:40–1:25 | Say "Simulate a fire at Capen Hall". It's detected within seconds; the camera flies to it; the incident panel and alert card appear; the fire is contained, then out | "A patrol drone's thermal stand-in picks it up. The nearest team suppresses, alert drones warn the buildings around it, one drone monitors. The map shows the burning area, the alert radius, and who has been warned." |
| 1:25–1:55 | Shake the phone (LIVE), or say "Simulate an earthquake". The shock ring sweeps, buildings turn amber and red, drones inspect | "The phone's real accelerometer triggers it. Sensor nodes on 28 buildings confirm, and drones inspect the hardest-hit buildings first." |
| 1:55–2:25 | Room Twin tab: click a spot, see the approved verdict and the route around furniture; click inside furniture, see it rejected; press Esc to abort | "Under the hood is a Rust engine. This is a real LiDAR scan of a room in Davis Hall. Every command gets a safety verdict before a drone moves; unsafe targets are rejected with a reason; Abort never waits." |
| 2:25–2:45 | Point at the SIM and LIVE labels; end on the campus map | "The drones are simulated but speak real MAVLink, the protocol real autopilots use. Next: fly the same commands on hardware. Aether: one console for campus safety." |

Recording: QuickTime → New Screen Recording, with the system microphone, so Aether's voice is captured too. One take, then trim the ends.

## Live pitch (3:00)
1. **Problem (0:20).** Campus emergencies such as fires, earthquakes or a collapse: responders need eyes before they send people in, and their hands are busy.
2. **Live demo (2:00).** Run the campus flow above by voice: patrol, fire, then a phone-shake quake. Wait for each spoken reply to finish before you talk again.
3. **Under the hood (0:25).** Real LiDAR room, real MAVLink, a safety verdict on every command, priority Abort. Point at the Room Twin tab if there is time.
4. **Close (0:15).** "Everything that flies is labelled SIM. The same commands go to a real autopilot next."

If the mic misbehaves, click the example chips or type; they run exactly the same commands. If ElevenLabs fails, the browser voice speaks and the footer says "Voice: Browser voice". If the phone is unreachable, say "Simulate an earthquake" (the panel then shows SIM, not LIVE).

## Likely judge questions (answer honestly)
- **Are the drones real?** No, all simulated, and labelled SIM. The room drones speak MAVLink v2 as PX4 quadrotors; pymavlink flies them in our tests. Not flown on hardware.
- **How is the fire detected?** In the sim, any drone within 250 m "sees" it, a stand-in for a thermal camera. Real detection would need a thermal camera and a classifier. The data stream shows the simulated thermal reading climbing near the fire.
- **What stops a misheard command?** A fixed grammar (team + verb + building) and a spoken readback. Chatter without a team or command word is ignored. In the room, the safety policy rejects any target that isn't free space with clearance. Abort skips everything.
- **Why not an LLM for voice?** Speed and predictability for a known vocabulary. Our Gemini function-calling prototype (about 0.6 s) is the path to free-form orders.
- **Where does the map come from?** OpenStreetMap: 291 buildings, 103 with tagged heights, the rest estimated at 12 m and drawn fainter.
- **Is the earthquake real?** The trigger can be: the phone's accelerometer through phyphox. The campus shaking and building effects are simulated, and the shock ring is slowed down so you can see it.
- **What about privacy?** Voice audio goes to ElevenLabs for transcription; the API key stays in the engine. Aether stores nothing.
- **Could this be deployed?** It would need beyond-visual-line-of-sight approval, real payloads and detection, and the university's emergency management. Our next step is PX4's simulator, then a real quad.
- **What did AI do?** Claude Code and Codex wrote much of the code from our spec and acceptance checks. We made the design and safety decisions and verified the results.
- **What was hardest?** Keeping the real/simulated line honest; macOS blocking the phone link; speech recognition hearing "Capen" as "Capon" (fixed with aliases); getting the campus map from 26 to 60 fps on this laptop.
