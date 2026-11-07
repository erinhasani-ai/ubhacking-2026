# AGENTS.md — shared rules for every coding agent

Single source of truth for project rules, for Claude Code, Codex, and any other agent. Codex reads this file directly; Claude Code loads it through the `@AGENTS.md` import in CLAUDE.md. Change rules here, never in a copy.

## Start of every session (and after every compaction or handoff)
1. Read, in order: AGENTS.md → SPEC.md → GATE.md → PROGRESS.md → "Latest handoff" in docs/HANDOFF.md.
2. Run `git status` and `git log --oneline -5`.
3. Follow docs/WORKFLOW.md for all work. Do not re-ask what these files already answer.

## Behaviour
- Never make a silent assumption. If something is unclear, ask before coding.
- Make surgical changes only. Touch only what the ticket requires.
- Never delete code, comments, or tests that were not part of the ticket.
- Never over-engineer. The simplest design that passes the gate wins.
- Never invent an API, library function, install command, or config value. Check it, or mark it TBD.
- Never mark a ticket done until its gates pass in GATE.md (exit code 0 from the gate checker).
- Weigh trade-offs out loud before a non-trivial decision.
- If a fix breaks working code, stop, say so, and revert before trying another approach.
- Change SPEC.md first, then the code. Never the other way round.
- Label anything unverified as unverified. Do not speculate in code or copy.
- Report finished work as: what changed, why, and what tests ran — in that order.

## Project facts
- Event: UB Hacking 2026, 2026-11-07. Rules: AI coding assistants allowed; no pre-built template — all project code is written during the event.
- Team: solo for now; may become a team. Then: one owner per directory under src/, and a pinned GitHub issue as the agent board.
- Platform and stack: TBD — decided in the grill, then recorded here and in SPEC.md.
- Design: one accent colour, no purple gradients, no generic template layouts, no invented stats, testimonials, logos, or customer counts. Tap targets at least 44px.

## Security (non-negotiable)
- Real secrets live only in `.env` (gitignored). `.env.example` lists variable names, never values.
- A git pre-commit hook runs gitleaks on staged changes. Never bypass it (`--no-verify`). If it fires: remove the secret, rotate it, and check history.
- No personal identifiers (real names, social handles, emails, home-directory paths) in committed files. Known exception: unlazy's automatic EVIDENCE lines in GATE.md record the absolute working directory, which contains the local username. Decide before pushing to a public repo (docs/SETUP.md, unlazy lessons).
- Before installing any skill, plugin, or MCP server: scan it with SkillSpector, read every HIGH finding at its source line, show the user, and install only after approval (docs/WORKFLOW.md, "Installing anything").

## Repository map
| Path | Purpose |
|---|---|
| AGENTS.md | Shared rules (this file) |
| CLAUDE.md | Claude Code additions; imports this file |
| SPEC.md | What we are building, tickets, decisions log — change before code |
| GLOSSARY.md | Shared vocabulary; created by the grill-with-docs flow when the first term is settled |
| GATE.md | Acceptance ledger in unlazy format; one section per ticket |
| PROGRESS.md | One line per finished ticket |
| docs/WORKFLOW.md | The work loop every agent follows |
| docs/HANDOFF.md | Claude ↔ Codex handoff protocol and the latest handoff note |
| docs/SETUP.md | How this environment was set up, as built, with lessons learned |
| docs/adr/ | Architecture decision records; created when the first one is needed |
| src/ | Application code; split by owner once there is a team |
| tests/ | Tests; at minimum one end-to-end test of the core flow |
| .claude/skills/ | Project skills (unlazy, grilling set). Plain Markdown plus Node scripts — any agent can read and run them |
| .claude/agents/ | Claude Code subagent roles (ticket-executor, ticket-checker) |
| skills-lock.json | Source and hash of each installed project skill |
| Initial_setup/ | Original research doc. Where it disagrees with docs/SETUP.md, docs/SETUP.md wins |

## Learned rules
- (Append every correction the user makes here: `YYYY-MM-DD — rule`, one line each.)
