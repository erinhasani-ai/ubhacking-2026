# Aether

Aether is a rescue swarm commanded by voice and by pointing, flying through a copy of a real room. These are the words used in the database, the code, and the UI.

## Language

### The space

**Room Twin**:
The 3D map of a real room, built from a LiDAR scan of that room.
_Avoid_: room copy, map, scene, model

### The swarm

**Drone**:
One simulated aircraft in the Swarm.
_Avoid_: agent, unit, bot, UAV

**Swarm**:
All the Drones taking part in a Mission.
_Avoid_: fleet, squad

**Mission**:
One run, from the Quake that starts it to the final report. A reset begins a new Mission.
_Avoid_: session, run, demo

**Telemetry**:
The stream of reports a Drone sends about itself: where it believes it is, its attitude, battery, link, and flight mode. It is the Drone's estimate, not the simulation's ground truth.
_Avoid_: state, status, data feed

### The phone

**Quake**:
A shake of the surface under the phone, detected while the phone is in Sensor mode.
_Avoid_: seismic event, tremor, event

**Sensor mode**:
The phone lying still, used to detect a Quake.
_Avoid_: listening mode, idle mode

**Pointer mode**:
The phone held in the hand, used to aim at a spot in the Room Twin.
_Avoid_: wand mode, command mode

### Speaking to Aether

**Aether**:
The system, and the name it answers to. It responds only when what someone says starts with its name and contains a Command.
_Avoid_: the AI, the assistant, the bot

**Command**:
One instruction the Swarm can carry out, taken from what someone said.
_Avoid_: intent, order, request

**Verdict**:
The policy check's result for one Command: approved, or rejected with a reason.
_Avoid_: decision, approval, ruling

**Abort**:
Stopping every Drone at once, without waiting for the AI. Anyone can call it, with or without the name.
_Avoid_: cancel, kill, e-stop

**Voice line**:
The strip on screen that shows Aether listening, being addressed, understanding a Command, giving its Verdict, and speaking.
_Avoid_: voice orb, assistant bubble, chat

### Honesty

**Provenance**:
Where a piece of data came from: LIVE (measured now), SIM (simulated), or REPLAY (a recorded LIVE run played back).
_Avoid_: source, data mode
