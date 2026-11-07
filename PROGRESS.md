# PROGRESS.md — one line per finished ticket

Format: `YYYY-MM-DD | Tn | what shipped | gate result`

- 2026-11-07 | T0 | Project setup: AGENTS/CLAUDE rules, SPEC/GATE/PROGRESS, docs (WORKFLOW, HANDOFF, SETUP), ticket roles, grill skills, gitleaks commit hook, SkillSpector, compaction 60% | gate-check exit 0, T0.G1–G5 met
- 2026-11-07 | T1 | Rust 1.99.1 + wasm32 target + wasm-pack 0.15.1; Cargo workspace (aether-core, aether-engine, aether-wasm) with every spike dependency and module stub; .env.example with the 7 names; wasm smoke and env-example checks | gate-check exit 0, T1.G1–G5 met; ticket-checker PASS
- 2026-11-07 | T7 | MAVLink Drone spike: kinematic quadrotor in aether-core (WebAssembly-safe; speed and acceleration limits, tilt follows commanded acceleration, never below the floor); `sim_drone` speaks MAVLink v2 as a PX4 quadrotor to 127.0.0.1:14550 (HEARTBEAT 1 Hz, ATTITUDE 50 Hz, LOCAL_POSITION_NED 30 Hz, EXTENDED_SYS_STATE, COMMAND_ACK); pymavlink 2.4.50 flies offboard refused → arm → 1 m climb → second setpoint → land → auto-disarm. Not flown on hardware | T7.G1–G2 met (attitude 49 Hz, position 29 Hz); ticket-checker PASS
