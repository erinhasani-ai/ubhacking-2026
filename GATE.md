# Gates: UB Hacking 2026 acceptance ledger

Format: unlazy (`.claude/skills/unlazy/references/gates.md`). One `## Tn` section per ticket; gate ids are `Tn.Gm`.
Run from the repo root: `node .claude/skills/unlazy/scripts/gate-check.mjs GATE.md`. First time on a machine: `--status` to read the commands, then `--approve`.
A ticket is not done until every one of its gates shows passing evidence here.

## T0 — Project setup

- [x] T0.G1: Committed history contains no secrets
  CHECK: gitleaks git --redact --no-banner
  EXPECT: no leaks found
  EVIDENCE: automatic-evidence=v1; definition-sha256=b760c9b7f158dcbdcbe880501e53259ae2b3d3a0b6be781a6db12ca612c01743; exit=0; EXPECT=matched; output-sha256=3bf41a8bd9494c9e941f6d310bfc01f772deab38fc8cf046cac9ce7f8a71b369; output-bytes=188; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

- [x] T0.G2: Commit hook scans staged changes with gitleaks
  CHECK: node -e "const fs=require('fs');const p='.git/hooks/pre-commit';fs.accessSync(p,fs.constants.X_OK);if(!/gitleaks git --pre-commit --staged/.test(fs.readFileSync(p,'utf8')))process.exit(1);console.log('hook'+'_ready')"
  EXPECT: hook_ready
  EVIDENCE: automatic-evidence=v1; definition-sha256=5cede94c9843731e5e14a040f7b14dbc8394deadf37075e70191aab946475a06; exit=0; EXPECT=matched; output-sha256=9290e0c4a3d6907a49e4859bdc6adeeacfa58ece9aa297e0a9c30298fb2089ce; output-bytes=11; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

- [x] T0.G3: Scaffold files and folders exist
  CHECK: node -e "const fs=require('fs');const need=['AGENTS.md','CLAUDE.md','SPEC.md','GATE.md','PROGRESS.md','.env.example','.gitignore','docs/WORKFLOW.md','docs/HANDOFF.md','docs/SETUP.md','.claude/agents/ticket-executor.md','.claude/agents/ticket-checker.md','.claude/skills/grill-me/SKILL.md','.claude/skills/unlazy/SKILL.md','src','tests'];const miss=need.filter(p=>!fs.existsSync(p));if(miss.length){console.log('missing: '+miss.join(', '));process.exit(1)}console.log('scaffold'+'_complete')"
  EXPECT: scaffold_complete
  EVIDENCE: automatic-evidence=v1; definition-sha256=ada0d275dd88c3009562508c87d9f9b074a0ed6cdb5d6808d9b21465231f30ef; exit=0; EXPECT=matched; output-sha256=400d20ba7a026a8877efae32a48ef6f0fd5645dbe35fbd77c0a0954bdc9d3145; output-bytes=18; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

- [x] T0.G4: Auto-compaction is 60% of a 600k window
  CHECK: node -e "const s=JSON.parse(require('fs').readFileSync('.claude/settings.json','utf8'));if(s.autoCompactWindow!==600000||s.env.CLAUDE_AUTOCOMPACT_PCT_OVERRIDE!=='60')process.exit(1);console.log('compaction'+'_ok')"
  EXPECT: compaction_ok
  EVIDENCE: automatic-evidence=v1; definition-sha256=8eb46f2c908ce1d144f6e6208cae563de403ef769f5ff425f0fd492410d2759c; exit=0; EXPECT=matched; output-sha256=c056e8d456d69863a8f51fdceaa454fb1a851d43b14cff09eeca52bf39f7b1b7; output-bytes=14; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

- [x] T0.G5: SkillSpector CLI is installed and on PATH
  CHECK: skillspector --version
  EXPECT: /SkillSpector v\d+\.\d+/
  EVIDENCE: automatic-evidence=v1; definition-sha256=4617305691f2a56cace6a2c0591fec439a635725e2edef63eb44bb1283ba81b4; exit=0; EXPECT=matched; output-sha256=fe89d4dc1f62ec31b6efcafa7979f3b251147d25af0bc8886a3d65b948e610c2; output-bytes=21; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

## T1 — Rust toolchain and workspace

- [x] T1.G1: Stable Rust with the WebAssembly target is installed
  CHECK: PATH="$HOME/.cargo/bin:$PATH" rustup target list --installed
  EXPECT: wasm32-unknown-unknown
  EVIDENCE: automatic-evidence=v1; definition-sha256=f2e0b6e4f8b4003b5b3b27f5af1a3310d97a28fafdaba1e3bc92b2aec0a01ef5; exit=0; EXPECT=matched; output-sha256=43ed07ffa58225043951db799899ce3c06f30b075021acc669f543b1626e263c; output-bytes=43; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=d082171f4bb6/24 entries

- [x] T1.G2: The workspace builds and its unit tests pass
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
  EXPECT: /test result: ok\. [1-9]\d* passed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=7811e910e44dfc50df73d347976340332db4c6f26b08102d374c3619b5cb70d2; exit=0; EXPECT=matched; output-sha256=bdddb72adf1ab42f204c1d4ad0a023d3c6fca9c1a9699d677576ee8952a048c0; output-bytes=1000; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=d082171f4bb6/24 entries

- [x] T1.G3: aether-wasm builds with wasm-pack and runs in Node, reporting the crate version from Cargo.toml
  CHECK: PATH="$HOME/.cargo/bin:$PATH" node tests/wasm-smoke.mjs
  EXPECT: /wasm_smoke_ok version=\d+\.\d+\.\d+/
  EVIDENCE: automatic-evidence=v1; definition-sha256=6ef11e202c19a57d3730d4ac72e8028f54d583091da93ae9ca007eb601066b8d; exit=0; EXPECT=matched; output-sha256=7f00f8b58f4f42bec638de70569e95a3a92f52f1cce666d755a6118cfea31055; output-bytes=495; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=d082171f4bb6/24 entries

- [x] T1.G4: .env.example lists the 7 variables with no values, every variable the code reads is listed, and the checker catches a planted unlisted variable
  CHECK: node tests/env-example-check.mjs
  EXPECT: /env_example_ok vars=7 self_test=pass/
  EVIDENCE: automatic-evidence=v1; definition-sha256=ac8482ff63b16dbe5aab2594fbd570b3ceb18bc4d1cf3630d5431c417534295a; exit=0; EXPECT=matched; output-sha256=c41789c0a17ec777aa0797caf60918d7d599dab48cad503a746beaff5db3cc32; output-bytes=37; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=d082171f4bb6/24 entries

- [x] T1.G5: The spike modules are declared, so T2–T7 each write only inside their own files
  CHECK: node -e "const fs=require('fs');const m={'src/engine/src/lib.rs':['tiger','gemini','voice','phyphox','mavlink_link'],'src/core/src/lib.rs':['command','room_twin','vehicle']};const miss=[];for(const[f,ms]of Object.entries(m)){const s=fs.existsSync(f)?fs.readFileSync(f,'utf8'):'';for(const x of ms)if(!s.includes('pub mod '+x+';'))miss.push(f+':'+x)}if(miss.length){console.log('missing: '+miss.join(', '));process.exit(1)}console.log('stubs'+'_ready')"
  EXPECT: stubs_ready
  EVIDENCE: automatic-evidence=v1; definition-sha256=fa46dc735ca5c9fbd18bedb71bb26cd28a12c7ecd15cf744d611ac6e442ad5a4; exit=0; EXPECT=matched; output-sha256=1aa1f2668321465d836aa43851928e536c54ab04298597b23ee704d251a7213c; output-bytes=12; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=d082171f4bb6/24 entries

## T2 — Tiger spike

- [x] T2.G1: The probe connects to Tiger over TLS, creates a scratch hypertable, COPYs 10,000 rows, counts all 10,000 back, reports rows per second, and drops the table
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo run -q -p aether-engine --bin probe_tiger
  EXPECT: /tiger_ok tls=1 hypertable=1 counted=10000 rows_per_s=[1-9]\d* dropped=1/
  EVIDENCE: automatic-evidence=v1; definition-sha256=e96b4cb66f415de8cd76a8d8111724c21958a7576c99f76a06078ad43de439c4; exit=0; EXPECT=matched; output-sha256=7247624cbd452f9b7746af7e48646c098cf583c298707844699349851673be1a; output-bytes=130; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

## T6 — Room Twin

- [x] T6.G1: The occupancy grid matches known answers on a synthetic box room (exact occupied and free counts, no holes in a two-triangle wall) and the scan header parses
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo test -p aether-core room_twin
  EXPECT: /test result: ok\. [1-9]\d* passed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=69c780722e177f4f5246e3afb38aca08b082df95f1c9e9b9dc16de10a2a22116; exit=0; EXPECT=matched; output-sha256=83dccce0c9a48f957ac8addc16e15f993c5d50646e79ff29f55b9468cabb02c2; output-bytes=1030; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026/.claude/worktrees/agent-a244c697308a7197d; path=d082171f4bb6/24 entries

- [x] T6.G2: The real scan loads as a plausible Room Twin with its location
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo run -q -p aether-engine --bin probe_room
  EXPECT: /room_ok points=\d+ triangles=\d+ voxels=\d+ free_pct=\d+ size_m=\d+\.\dx\d+\.\dx\d+\.\d geo=-?\d+\.\d+,-?\d+\.\d+ heading_deg=\d+\.\d+/
  EVIDENCE: automatic-evidence=v1; definition-sha256=0c07693b1e620a55fdb450315c82e5aebcaea442f9b7d3023e4e57249ace5bda; exit=0; EXPECT=matched; output-sha256=e36608b228a80c5dc75fbfc2cea33a7c4cff46bc7799791f6f1fde576a41a71e; output-bytes=167; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026/.claude/worktrees/agent-a244c697308a7197d; path=d082171f4bb6/24 entries

## T7 — MAVLink Drone

- [x] T7.G1: The kinematic Drone model matches known answers (reaches a step setpoint at the analytic time, tilt equals atan(a/g), stays above the floor)
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo test -p aether-core vehicle
  EXPECT: /test result: ok\. [1-9]\d* passed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=9ef344429af54b7ddd93ed4c7407f421b02036bc17e49cb2a0ba09a0fa0b1408; exit=0; EXPECT=matched; output-sha256=5b121faab2f99b7d7516b74e67f75efb7eea2cacd901b1773792d9b609cb0608; output-bytes=610; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

- [x] T7.G2: pymavlink flies the simulated Drone through the PX4 offboard sequence (offboard refused without setpoints, then arm, 1 m climb, second setpoint reached, land, auto-disarm) and Telemetry arrives at real rates
  CHECK: PATH="$HOME/.cargo/bin:$PATH" .venv/bin/python tests/mavlink_flight.py
  EXPECT: /mavlink_ok refused=1 armed=1 reached=1 landed=1 att_hz=(4[5-9]|[5-9]\d) pos_hz=(2[5-9]|[3-9]\d)/
  EVIDENCE: automatic-evidence=v1; definition-sha256=bc55814e9f0cf0638925e40c5b6f5cf6fb4ef57b181b54769f8df0c39032e083; exit=0; EXPECT=matched; output-sha256=3e405f957deb85cdbc585e381743e5f4e45b1b6481ce2a29a84c18f99d2d43ee; output-bytes=68; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

## T8a — Dependencies and module stubs (T8 and T16 dependencies, landed first)

- [x] T8a.G1: The engine has axum (ws), pathfinding, cpal and rodio (playback), and every later module is declared, so no later ticket touches Cargo.toml or a lib.rs
  CHECK: node -e "const fs=require('fs');const r=f=>fs.readFileSync(f,'utf8');const c=r('src/engine/Cargo.toml');const miss=['axum','pathfinding','cpal','rodio'].filter(d=>!new RegExp('^'+d+' =','m').test(c));if(!/^axum = .*\"ws\"/m.test(c))miss.push('axum ws');if(!/^rodio = .*\"playback\"/m.test(c))miss.push('rodio playback');const m={'src/core/src/lib.rs':['frame','planner','orca','policy','quake','pointer','places','hand','ray_history'],'src/engine/src/lib.rs':['gcs','mission','server','voice_loop','telemetry_store','hand_input','replay','report']};for(const[f,ms]of Object.entries(m))for(const x of ms)if(!r(f).includes('pub mod '+x+';'))miss.push(f+':'+x);if(miss.length){console.log('missing: '+miss.join(', '));process.exit(1)}console.log('t8a'+'_ready')"
  EXPECT: t8a_ready
  EVIDENCE: automatic-evidence=v1; definition-sha256=81e634aec76410395b3adfff2331e676c0bd7d4a71048a344a8c434f2d7d3891; exit=0; EXPECT=matched; output-sha256=f5ef7958aba36c824e05f1da41fe146c9f27c82e91d8ab724c7f415a5308d375; output-bytes=10; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

- [x] T8a.G2: The workspace and the WebAssembly crate build with the new dependencies
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo build --workspace && PATH="$HOME/.cargo/bin:$PATH" cargo build -p aether-wasm --target wasm32-unknown-unknown && echo "build"_ok
  EXPECT: build_ok
  EVIDENCE: automatic-evidence=v1; definition-sha256=265ddc570007970cd30af7a31f591985c85c3c469b7cf46e67a4a783d735b3c5; exit=0; EXPECT=matched; output-sha256=f79d7c9d2655adda1b2e7aeb2ff4e62ce74e50bdd9b02ee0e22d5682278b05dc; output-bytes=154; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

## T4 — Voice spike

- [x] T4.G1: On recorded replies, the ElevenLabs character timings and the Scribe partial and committed transcripts with word timestamps parse to known answers
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo test -p aether-engine voice::
  EXPECT: /test result: ok\. [1-9]\d* passed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=59958c4e9fc5a46ec952ebd6ca00621c32d80d0bac8e58cdffae17eb5502e6fb; exit=0; EXPECT=matched; output-sha256=8cc5ee20d405e01dddfddca1125cd671c65bffa3547848ee0cdfe084e8046a23; output-bytes=1992; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026/.claude/worktrees/t4-voice; path=441ef2ff855b/30 entries

- [x] T4.G2: Live, ElevenLabs speaks a fixed sentence with character timestamps, Scribe v2 Realtime hears that audio after a manual commit with "drones" and word timestamps in the transcript, and both latencies are reported
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo run -q -p aether-engine --bin probe_voice
  EXPECT: /voice_ok tts_ms=[1-9]\d* chars=[1-9]\d* stt_ms=[1-9]\d* words=[1-9]\d* drones_at_ms=\d+/
  EVIDENCE: automatic-evidence=v1; definition-sha256=7bcc056699222ac4a2dd14255f25b4ed952987219f4b63c1c8edc6e16df02219; exit=0; EXPECT=matched; output-sha256=83a8cad56d60fbf3da6e17169bbb9a5f7ab5739e1410636d8b3aaf01feddd4d4; output-bytes=66; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026/.claude/worktrees/t4-voice; path=441ef2ff855b/30 entries
## T5 — phyphox spike

- [x] T5.G1: The phyphox reply parser, on fixture replies, keeps only new samples and notices a changed session
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo test -p aether-engine phyphox
  EXPECT: /test result: ok\. [1-9]\d* passed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=fc2bc678fff2454fe573878d91142f2df0b50bf2b98a11c551db0d8e4198a92b; exit=0; EXPECT=matched; output-sha256=f0bd4ca360c10cd017e429159665399adcb669de56a25a474e3d87bf7c470c5e; output-bytes=1244; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026/.claude/worktrees/t5-phyphox; path=441ef2ff855b/30 entries

- [x] T5.G2: The probe streams at least 100 samples from the live phone over 5 s with one session, and reports sample rate and median round-trip time
  CHECK: PATH="$HOME/.cargo/bin:$PATH" cargo run -q -p aether-engine --bin probe_phyphox
  EXPECT: /phyphox_ok samples=([1-9]\d{2,}) rate_hz=\d+ rtt_ms=\d+/
  EVIDENCE: automatic-evidence=v1; definition-sha256=7f246074f1cdce59dc270d0678565143569403c156bd4638e93dfdba6a77cfa2; exit=0; EXPECT=matched; output-sha256=e6f9751a099086f26f2634b3a89f9fdcc501e34d8d1c33a2e1b9f89e40eda451; output-bytes=44; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026/.claude/worktrees/t5-phyphox; path=441ef2ff855b/30 entries
