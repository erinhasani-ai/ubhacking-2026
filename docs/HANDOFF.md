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

### Handoff — 2026-11-07 17:10 — from Claude Code to Claude Code (new session)
- Ticket / branch: T5 on `t5-phyphox` — state: not started. Its entry and the two gates the user approved are in SPEC.md (T5); copy the gates into GATE.md when T5 starts.
- Done since last handoff: T1, T6, T7 landed on main (PROGRESS.md). Plan T8–T30 in SPEC.md: hand tracking T28–T30 joined Tier B, and the rigid-body model (T25) moved to Tier C. UI quality rule in AGENTS.md: load the design skills before any UI code.
- Next step: the user connects the iPhone (USB Personal Hotspot on; phyphox "Acceleration with g" open, Remote access on). Then `git switch -c t5-phyphox`, write the T5 gates into GATE.md, run ticket-executor, then ticket-checker.
- After T5: T2, T3, T4 (all seven .env names have values as of 17:04; the keys are not yet tried), then T8.
- Gates: met T0.G1–G5, T1.G1–G5, T6.G1–G2, T7.G1–G2, each when its ticket landed; nothing unmet in the ledger.
- Open decisions for the user: (1) the Gesture first values in the SPEC decisions log (1 m Advance, Recall lands on the pad, 0.5 s Halt hold) — confirm before T29; (2) the Tier B first cut if behind — decide at 01:00; (3) T0.G1 runs `gitleaks dir .`, which also scans the gitignored .env, so it fails now that real keys are in .env — change its CHECK (for example, scan only tracked files) or accept it, before the final gate run.
- Gotchas: remove the stale worktree first, from the main checkout: `git worktree remove .claude/worktrees/agent-a244c697308a7197d` (add `--force` if its build folder blocks it). Never share CARGO_TARGET_DIR between checkouts. T0.G2 fails inside worktrees (.git is a file there). The ECC GateGuard asks for facts on the first Bash call and the first edit of each file. T7.G2's 45 Hz attitude floor sits close to the measured 49 Hz; a busy laptop can fail it.
