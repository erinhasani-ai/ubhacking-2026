# HANDOFF.md — passing work between Claude Code and Codex

Everything the next agent needs is in the repo. If it is only in a chat, the next agent does not know it.

## What each agent reads and uses

| Need | Claude Code | Codex |
|---|---|---|
| Rules | CLAUDE.md (imports AGENTS.md) | AGENTS.md |
| Spec, gates, progress | SPEC.md, GATE.md, PROGRESS.md | Same files |
| Procedure | docs/WORKFLOW.md | Same file |
| Project skills | `.claude/skills/*`, loaded automatically | Read `.claude/skills/<name>/SKILL.md` as instructions; run its scripts with `node` |
| Grill | `/grill-me`, `/grill-with-docs` | Follow `.claude/skills/grilling/SKILL.md` and `.claude/skills/domain-modeling/SKILL.md` |
| Build and check roles | `ticket-executor` (Haiku), `ticket-checker` (Opus) in `.claude/agents/` | Do both roles yourself, or hand the check back to Claude Code |
| Plan, TDD, security commands | ECC: `/ecc:plan`, `/ecc:tdd`, `/ecc:security-scan` | No equivalent; follow WORKFLOW.md step 2 and "Before the demo" |
| Gate runner | `node .claude/skills/unlazy/scripts/gate-check.mjs GATE.md` | Same command |
| Secret scan | git pre-commit hook (gitleaks) | Same hook. Never `--no-verify` |
| Skill scan | `skillspector scan <path> --no-llm` | Same command |

Gate-command approvals live per machine in `~/.unlazy/approved`. The receiving agent runs `--status`, reads the CHECK lines, then `--approve` once.

## Sender checklist
1. Stop at a clean point: gates run, nothing half-written. Mid-ticket? Commit WIP on the ticket branch (`Tn: WIP — <state>`), never on main.
2. PROGRESS.md and the SPEC.md decisions log are current.
3. Replace "Latest handoff" below using the template. Keep it under about 25 lines.
4. Commit.

## Receiver checklist
1. Session-start reads (WORKFLOW.md step 0), then "Latest handoff".
2. `git switch <branch from the handoff>`, then `node .claude/skills/unlazy/scripts/gate-check.mjs --status GATE.md`.
3. Ask the user before touching anything the handoff lists as an open decision.

## Template

```text
### Handoff — YYYY-MM-DD HH:MM — from <Claude Code | Codex> to <Codex | Claude Code | human>
- Ticket / branch: Tn on `tN-short-name` — state: not started | in progress | gates failing | ready for check
- Done since last handoff: …
- Next step (one concrete action): …
- Gates: met … / unmet … (from --status)
- Open decisions for the user: …
- Gotchas (environment, flaky checks, approvals still needed): …
```

## Latest handoff

None yet.
