# Each Drone is a MAVLink endpoint; Aether is its ground station

Every Drone speaks MAVLink v2 (common dialect) over its own UDP endpoint, with PX4-style arming, offboard setpoints that must keep arriving faster than 2 Hz, and failsafes for link loss and low battery. Aether reaches a Drone only through that protocol, the way a ground station does. This costs message encoding and a protocol state machine inside one program, where direct function calls would be simpler and faster. We pay it so that the same Aether code could command PX4 or ArduPilot vehicles by changing addresses, and so that independent software (pymavlink, QGroundControl, optionally PX4's software-in-the-loop simulator) can check compatibility.

## Consequences

- Real hardware is out of scope: no physical drones and no tests on real drones. Compatibility claims rest only on those software checks and must say "not flown on hardware".
- Telemetry follows MAVLink message families and real rates, which drives the Tiger schema and the storage budget.
- Any extra 3D flight simulator is optional and must run fast on the demo laptop (an Intel Mac). That favours PX4's lightweight built-in simulator (SIH) over Gazebo.
