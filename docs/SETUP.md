# SETUP.md — Claude Code hackathon setup, as built

> **For any agent:** This is how this environment was actually set up and verified on 2026-11-07, plus everything learned doing it. It supersedes `Initial_setup/Hackathon Claude Setup.md` wherever the two disagree. That file stays as research: creator tips, gated items, and the catalogue.
>
> **For the user:** Section 6 rebuilds this setup from scratch for the next hackathon.

## 1. Setup prompt — paste into a fresh Claude Code or Codex session

```text
You are my pair engineer for a hackathon. Read AGENTS.md, SPEC.md, GATE.md, PROGRESS.md, and the
"Latest handoff" in docs/HANDOFF.md before doing anything, and again after every compaction.
Follow docs/WORKFLOW.md for every ticket. No code before the grill and a written SPEC.md.
No ticket is done until its gates pass in GATE.md. Scan anything before installing it
(docs/WORKFLOW.md, "Installing anything"). After every correction I make, add a line to
"Learned rules" in AGENTS.md. Report finished work as: what changed, why, what tests ran.
```

## 2. What is installed, where, and why

| Item | Version / commit | Scope | Purpose |
|---|---|---|---|
| Claude Code | 2.1.319 | Machine | The agent |
| ECC (Everything Claude Code) plugin | marketplace `affaan-m/ECC` | Global | Main pack: `/ecc:plan`, `/ecc:tdd`, `/ecc:code-review`, `/ecc:security-scan`, many hooks |
| Ponytail plugin | marketplace `DietrichGebert/ponytail` | Global | Smallest-change discipline, token savings |
| apple-design, impeccable, emil-design-eng, and others | `~/.claude/skills/` | Global | Design and taste layer |
| Chrome DevTools MCP | bundled with ECC | Global | Agent opens the site in a real browser |
| graphify | `~/.claude/skills/graphify` | Global | Codebase knowledge graph when the code gets big |
| unlazy | `Leonxlnx/unlazy` @ `1667149` | Project, `.claude/skills/unlazy` | Gate ledger (GATE.md) and checker scripts |
| grill-me, grilling, grill-with-docs, domain-modeling | `mattpocock/skills` @ `4ae2317` | Project, `.claude/skills/` | The grill, without installing the whole Matt Pocock pack |
| SkillSpector | `NVIDIA/SkillSpector` @ `e48d771` | Machine: repo in `~/.local/share/SkillSpector`, CLI linked into `~/.local/bin` | Skill security scanner |
| gitleaks | 8.30.1 (Homebrew) | Machine, plus `.git/hooks/pre-commit` | Secret scanning on every commit |
| Auto-compaction | `.claude/settings.json` | Project | 60% of a 600,000-token window |
| Ticket roles | `.claude/agents/ticket-executor.md` (Haiku), `ticket-checker.md` (Opus) | Project | Opus plans and checks, Haiku builds |

`skills-lock.json` records the source and content hash of every project skill. Codex is not installed on this machine.

## 3. Verified commands — each one was run here

```bash
# Skill install: project scope, real copies (not symlinks) so the files travel with git
npx skills add <owner/repo> -l                                   # list skills, install nothing
DO_NOT_TRACK=1 npx skills add <owner/repo> -a claude-code -y --copy [-s <skill> ...]

# SkillSpector (official install from docs.nvidia.com/skills/scanning-agent-skills)
git clone https://github.com/NVIDIA/SkillSpector.git ~/.local/share/SkillSpector
cd ~/.local/share/SkillSpector && uv venv .venv && source .venv/bin/activate && make install
ln -sf "$PWD/.venv/bin/skillspector" ~/.local/bin/skillspector
skillspector scan <dir | SKILL.md | git URL | zip> --no-llm --format markdown --output report.md

# gitleaks
brew install gitleaks
gitleaks git --pre-commit --staged --redact --no-banner         # what the commit hook runs
gitleaks dir . --redact --no-banner                             # working tree
gitleaks git --redact --no-banner                               # full history, before the demo

# Gate ledger
node .claude/skills/unlazy/scripts/gate-lint.mjs GATE.md
node .claude/skills/unlazy/scripts/gate-check.mjs --status GATE.md    # read only, runs nothing
node .claude/skills/unlazy/scripts/gate-check.mjs --approve GATE.md   # approve pending commands, then run
node .claude/skills/unlazy/scripts/gate-check.mjs GATE.md             # run unmet gates
node .claude/skills/unlazy/scripts/gate-check.mjs --reverify GATE.md  # re-run everything
```

## 4. Lessons learned

### Compaction
- `/autocompact` is a real command. It saves `modelSettings.<model>.autoCompactWindow` (100,000 to 1,000,000 tokens). The project equivalent is top-level `"autoCompactWindow"` in `.claude/settings.json`.
- `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` is a percentage **of that window**, not of the model's full context. 600,000 × 60% ≈ 360,000 tokens (the default here); × 40% ≈ 240,000. The original doc assumed a 1M base: 40% = 400k, 60% = 600k. With a 1M window you would set `autoCompactWindow` to 1000000 to get those numbers.
- This session ran with a 1M context window (reported by ECC's StrategicCompact hook). Settings env vars apply from the next session start.

### Skill safety
- SkillSpector is **not** gated. It is open source at `github.com/NVIDIA/SkillSpector`, with install steps in NVIDIA's docs. The Instagram claim that it needs a comment-gated link is wrong.
- Static mode (`--no-llm`) needs no API key. LLM mode needs a provider key — never put one in this repo. The CLI also has an `mcp` subcommand (needs the `mcp` extra, not installed); the NVIDIA docs do not describe it.
- **The score is not a verdict.** unlazy scored 100/100 CRITICAL "DO NOT INSTALL". Every HIGH finding was a keyword match: prose saying "never… delete state", a doc describing the hook's file paths, a directory diagram containing the word "session", and sandboxed tests proving the hook installer refuses symlinked settings files. None read credentials or sent data. Always read each HIGH finding at its line before deciding.
- The grill skills scored 0–8/100 (SAFE).
- `npx skills add` cannot pin a commit. Clone and scan first, install, then `diff -rq <scanned copy> .claude/skills/<name>` to prove you installed what you scanned.
- The skills CLI sends install telemetry. `DO_NOT_TRACK=1` was set as a best effort; whether the CLI honours it was not verified.

### ECC plus the grill
- The original doc says pick Matt Pocock **or** ECC. Solution: keep ECC as the main pack and install only the four grill skills, at project scope. No overlapping packs, and both `/grill-me` and `/ecc:*` work.
- `grill-me` is a one-line stub that calls `grilling`. `grill-with-docs` calls `grilling` **and** `domain-modeling`. Install the dependencies together or the stubs do nothing.
- `domain-modeling` creates `GLOSSARY.md` and `docs/adr/` lazily. That is the "Grill with Docs" shared vocabulary, so SPEC.md points to GLOSSARY.md instead of duplicating naming rules.

### ECC hooks — friction to expect
- **GateGuard (fact-forcing):** the first Bash command of a session, and the first Write or Edit of each new file, are blocked until the agent states facts (who loads the file, that nothing duplicates it, the user's instruction). State them and retry. Siblings in the same folder are usually waved through after one denial. To exempt paths, set `GATEGUARD_EXEMPT_GLOBS`; `ECC_GATEGUARD=off` disables the gate entirely (not recommended).
- **Bash dispatcher:** long inline heredoc scripts are blocked with "Shell analysis work budget exceeded". Write the script to a file and run the file.
- **StrategicCompact:** prints context-usage hints. Treat it as a cue to update PROGRESS.md before compacting.

### gitleaks
- In 8.30, the staged-only scan is `gitleaks git --pre-commit --staged` (the old `protect` command is gone).
- Speed: the commit hook scans only the staged diff. Worst case, with every file staged (about 540 KB), took 0.9 s; a full working-tree scan took 0.5 s. A hook in `.git/hooks/` is per clone — a new clone or teammate re-creates it with the three-line script below.

```sh
#!/bin/sh
# Secret scan of STAGED changes only: cost scales with the diff, not the repo, so commits stay fast.
exec gitleaks git --pre-commit --staged --redact --no-banner
```

### unlazy
- Its default ledger is `GATES.md`; this project uses `GATE.md` (per the original doc) and always passes the path explicitly.
- `EXPECT:` is plain text matched as a substring of combined output; wrap it in `/pattern/flags` for a regex (a bare `\d+\.\d+` is a literal and failed here). The process must also exit 0.
- Gate ids can be any token before the colon, so one GATE.md holds every ticket as `Tn.Gm`.
- Approvals bind the exact command, working directory, shell, PATH, and platform. They are stored outside the repo in `~/.unlazy/approved`, so each machine and agent approves once.
- Its optional Claude Code Stop hook (`install-hooks.mjs`) was **not** installed.
- Automatic EVIDENCE lines record `cwd=<absolute path>`, which exposes the local username. Hand-editing evidence invalidates it. Options before a public push: accept it, or run gates from a checkout whose path holds no personal name.

### Sharing work between agents
- `AGENTS.md` holds the rules for every agent; `CLAUDE.md` imports it with `@AGENTS.md`, so the rules exist once.
- Project skills are plain Markdown and Node scripts. Codex can read `.claude/skills/<name>/SKILL.md` and run the scripts, so nothing is duplicated into a Codex-only folder.
- Handoff state lives in files (PROGRESS.md, SPEC.md, docs/HANDOFF.md), never only in a chat.

### Privacy
- Keep real names, social handles, emails, and home-directory paths out of committed files. The original research doc was scrubbed of its account handle and the user's name.

## 5. Not done, or still gated

| Item | Status |
|---|---|
| Expo plugin | Waits on the platform decision in the grill |
| shadcn MCP | Not installed; add only if the UI uses shadcn |
| SkillSpector as an MCP server | Not configured; the CLI is enough for the workflow |
| Karpathy CLAUDE.md, 20-check PATCH audit, 20 UX Laws prompt, VibeSec, Strix | Still comment-gated per the original doc; AGENTS.md carries a paraphrase of the Karpathy rules |
| GitHub repo | Private repo created 2026-11-07, remote `origin`, branch `main`. Repo-local `user.email` is the GitHub no-reply address |
| Agent board (pinned issue) | Not created; only needed once there is a team |

## 6. Rebuild from scratch — next hackathon
1. Install Claude Code; install ECC and Ponytail from their marketplaces; confirm the global design skills are present.
2. `brew install gitleaks`; install SkillSpector (section 3).
3. `git init -b main` in the new project; add the gitleaks pre-commit hook (section 4) and `chmod +x` it.
4. Copy AGENTS.md, CLAUDE.md, docs/, `.claude/agents/`, `.claude/settings.json`, `.gitignore`, and `.env.example` from this repo. Reset SPEC.md, GATE.md, and PROGRESS.md to their headers.
5. Scan, then install the project skills: unlazy and the four grill skills (section 3, then diff against the scanned copies).
6. Check the hackathon rules (AI, templates, multi-agent) and record them in AGENTS.md "Project facts".
7. Run `/grill-with-docs`, fill SPEC.md, cut tickets, write gates, and follow docs/WORKFLOW.md.
