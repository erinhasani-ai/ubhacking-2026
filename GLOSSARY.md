# Aether

Aether is a rescue swarm commanded by voice, by pointing, and by hand signals, flying through a copy of a real room. These are the words used in the database, the code, and the UI.

## Language

### The space

**Room Twin**:
The 3D map of a real room, built from a LiDAR scan of that room.
_Avoid_: room copy, map, scene, model

**Place**:
A named spot in the Room Twin, such as the door or the table, that a Command can send Drones to. Each one is read off the scan and confirmed by hand.
_Avoid_: landmark (that word belongs to hand tracking), waypoint, location, POI

### The swarm

**Drone**:
One simulated aircraft in the Swarm.
_Avoid_: agent, unit, bot, UAV

**Swarm**:
All the Drones taking part in a Mission.
_Avoid_: fleet, squad

**Launch pad**:
The clear patch of floor where the Drones wait, take off, and land, each on its own slot.
_Avoid_: base, home, dock

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
One instruction the Swarm can carry out, taken from what someone said, a Gesture, or a click.
_Avoid_: intent, order, request

**Verdict**:
The policy check's result for one Command: approved, or rejected with a reason.
_Avoid_: decision, approval, ruling

**Abort**:
Stopping every Drone at once, without waiting for the AI. Anyone can call it, by voice with or without the name, or with a Halt.
_Avoid_: cancel, kill, e-stop

**Advance**:
Moving the selected Drones, or all of them, one step forward in the view. Called by voice or by a Push.
_Avoid_: push, go forward

**Recall**:
Flying the Drones back to the Launch pad and landing them. Called by voice or by a Pull.
_Avoid_: return home, come back

**Voice line**:
The strip on screen that shows Aether listening, being addressed, understanding a Command, giving its Verdict, and speaking.
_Avoid_: voice orb, assistant bubble, chat

### Hand signals

**Gesture**:
One hand signal to the laptop's webcam that Aether acts on: Halt, Point, Pinch, Push, or Pull. The palm toward the webcam can only stop the Swarm; the back of the hand moves it.
_Avoid_: sign, pose, hand command

**Halt**:
An open palm held still toward the webcam. It calls Abort.
_Avoid_: stop sign, palm stop

**Point**:
The index finger held out. It moves a cursor over the Room Twin and aims like the phone in Pointer mode.
_Avoid_: aim, finger mode

**Pinch**:
The thumb and index fingertip touching while pointing. It clicks at the cursor: a Drone is selected, a spot becomes a target.
_Avoid_: tap, grab

**Push**:
The back of the hand moved toward the webcam. It calls Advance.
_Avoid_: shove, forward gesture

**Pull**:
The back of the hand moved away from the webcam, toward the body. It calls Recall.
_Avoid_: beckon, back gesture

### Honesty

**Provenance**:
Where a piece of data came from: LIVE (measured now), SIM (simulated), or REPLAY (a recorded LIVE run played back).
_Avoid_: source, data mode
