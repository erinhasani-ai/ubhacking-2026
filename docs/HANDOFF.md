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

### Handoff — 2026-11-08 08:02 EST — from Codex to the next agent
- Ticket / branch: T14 on `t14-server-finish` — verified and ready to land; next is `t15-web-finish`.
- Landed this session: T9b scanned landing surfaces, T31 test-room Places, T13 mission controller. T14 adds the actual app runtime, browser stream and immediate SIM reset; all three scoped gates reverified exit 0. Existing flight/Abort integration also passes.
- Next step: recover T15 owned source from its worktree; update reset acknowledgment, saved-scan wording, label sizes and verify headed rendering performance before landing.
- Gates: 45 recorded, 44 met; T5.G2 remains unchecked, latest phone recheck still times out. Historical successful phone evidence is not current.
- User confirms gestures (Push 1 m, Pull returns/lands, palm 0.5 s Abort), test-room Places, features until 11:30 EST freeze, and R immediately resets the SIM to the pad. Actual demo location is unknown.
- Phone: user will set up USB Personal Hotspot and phyphox Remote Access shortly. Continue independent work meanwhile.
- Existing worktrees retain Claude's work. Recover owned source only: some branches copied stale dependencies. Do not merge entire histories blindly.
- T15 A/B reviews captured screenshots and browser reports; their final writeups were interrupted by usage limits. A found saved scan called measured now; B found tiny provenance labels and headed fps below 55 while builds were active. Root must verify fixes and rerun performance without compilation.
- T15 evidence: worktree src/web/test-results/design-a and temporary t15-assessment-b-20261108 directory. Neither is committed. No page JS errors; controls 44 px; keyboard and reduced-motion checks passed.
- T19 voice WIP needs partial Aboard alias and cancellation on Abort. T17 old detector fails handling cases; real recordings needed. T18 has gates but no implementation. Other Tier B tickets remain open.
- No hardware drone flight. No new skill/plugin installations. Required dataviz skill for T22 has not been found in installed directories; resolve before gauge code.
- Gate checker has no ticket selector: exact sections run in temporary ledgers with checkout --cwd, authentic evidence copied back with home paths sanitized; keep T5 honestly pending.
- Sandbox escalation is needed for local git metadata, UDP/browser checks and gate approval records. No active review rejection.
