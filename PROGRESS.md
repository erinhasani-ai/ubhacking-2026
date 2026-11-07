# PROGRESS.md — one line per finished ticket

Format: `YYYY-MM-DD | Tn | what shipped | gate result`

- 2026-11-07 | T0 | Project setup: AGENTS/CLAUDE rules, SPEC/GATE/PROGRESS, docs (WORKFLOW, HANDOFF, SETUP), ticket roles, grill skills, gitleaks commit hook, SkillSpector, compaction 60% | gate-check exit 0, T0.G1–G5 met
- 2026-11-07 | T1 | Rust 1.99.1 + wasm32 target + wasm-pack 0.15.1; Cargo workspace (aether-core, aether-engine, aether-wasm) with every spike dependency and module stub; .env.example with the 7 names; wasm smoke and env-example checks | gate-check exit 0, T1.G1–G5 met; ticket-checker PASS
- 2026-11-07 | T6 | Room Twin: OBJ from bytes (WebAssembly-safe), Scaniverse header (location, heading), 10 cm occupancy grid with triangle sampling; probe on the real scan: 177,186 points, 324,071 triangles, 27,313 occupied voxels, 94% free, 12.3 × 3.4 × 9.9 m at Davis Hall | T6.G1–G2 met (run in the T6 worktree; T7 gates unmet there by design); ticket-checker PASS
