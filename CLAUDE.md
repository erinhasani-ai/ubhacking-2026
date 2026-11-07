# CLAUDE.md

@AGENTS.md

## Claude Code specifics
Shared rules are in AGENTS.md (imported above). Edit rules there, not here, so Codex sees them too. Learned rules also go in AGENTS.md.

- Model routing: plan, decide, and check in the main session (Opus 5.5). Hand well-defined build tickets to the `ticket-executor` subagent (Haiku) and verification to `ticket-checker` (Opus). Both live in .claude/agents/.
- Project skills (.claude/skills/): `/grill-me` (interview), `/grill-with-docs` (interview plus GLOSSARY.md and ADRs), `unlazy` (gate ledger in GATE.md). ECC is installed globally: `/ecc:plan`, `/ecc:tdd`, `/ecc:code-review`, `/ecc:security-scan`. Order: grill first, then ECC to plan, build, test, and secure.
- `ultrathink` only for planning and the hardest bug, never by default.
- Compaction: automatic at 60% of a 600,000-token window (about 360k tokens), set in .claude/settings.json. Before a manual `/compact`, make PROGRESS.md and the SPEC.md decisions log current.
- ECC GateGuard hook: the first Bash call of a session and the first write to each new file are blocked until you state the facts it asks for; state them and retry. Long inline scripts trip ECC's shell analysis — save the script to a file and run the file.
