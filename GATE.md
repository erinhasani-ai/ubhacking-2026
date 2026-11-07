# Gates: UB Hacking 2026 acceptance ledger

Format: unlazy (`.claude/skills/unlazy/references/gates.md`). One `## Tn` section per ticket; gate ids are `Tn.Gm`.
Run from the repo root: `node .claude/skills/unlazy/scripts/gate-check.mjs GATE.md`. First time on a machine: `--status` to read the commands, then `--approve`.
A ticket is not done until every one of its gates shows passing evidence here.

## T0 — Project setup

- [x] T0.G1: Working tree contains no secrets
  CHECK: gitleaks dir . --redact --no-banner
  EXPECT: no leaks found
  EVIDENCE: automatic-evidence=v1; definition-sha256=ca369da599251f22ee2fbd05d4f4cf816a7b34b8d3bf27e970c7c1a0ba8a43d4; exit=0; EXPECT=matched; output-sha256=8e28c680b30ebddbe7a34eb5cfe526ca977bc492b68c3adbd4a0347da834f003; output-bytes=132; shell=/bin/sh; cwd=/Users/erinhasani/Documents/UBHacking2026; path=441ef2ff855b/30 entries

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
