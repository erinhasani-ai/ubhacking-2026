# WORKFLOW.md — the work loop

The procedure every agent (Claude Code or Codex) and the human follow for every ticket. Rules are in AGENTS.md; this file is the order of operations. Commands run from the repo root.

## 0. Start or resume a session
1. Read AGENTS.md → SPEC.md → GATE.md → PROGRESS.md → "Latest handoff" in docs/HANDOFF.md.
2. `git status` and `git log --oneline -5`. On a ticket branch? Continue that ticket.
3. Ledger status without running anything: `node .claude/skills/unlazy/scripts/gate-check.mjs --status GATE.md`

## 1. Grill — new idea or feature, no code yet
- Claude Code: `/grill-with-docs` (interview, plus GLOSSARY.md and ADRs). `/grill-me` for a quick interview without docs.
- Codex: follow `.claude/skills/grilling/SKILL.md`; for vocabulary, `.claude/skills/domain-modeling/SKILL.md`.
- Must answer: Who is this for? What breaks if it's wrong? What does done look like?
- Answers go into SPEC.md. Settled terms go into GLOSSARY.md.

## 2. Plan and cut tickets (main session, Opus)
- Optional: `/ecc:plan` to draft; the plan lands in SPEC.md, not in a side file.
- Each ticket in SPEC.md → Tickets: `Tn: <one outcome>`, the paths it owns, what it depends on.
- Size: one agent can finish it; a human can check it in about five minutes.
- Fill the seven-pillars table in SPEC.md: every pillar covered or consciously skipped.

## 3. Write gates before code
- In GATE.md add `## Tn — <name>` with gates `Tn.G1`, `Tn.G2`, … Each runnable gate has an indented `CHECK:` (command) and `EXPECT:` (plain text = substring the output must contain; `/pattern/flags` = regex). Format: `.claude/skills/unlazy/templates/gates-leaf.md`.
- One gate per acceptance criterion. Once the core flow exists, one gate runs it end to end.
- Prefer `node tests/<name>.mjs` scripts over shell one-liners; they work on every OS.
- Make the success marker something the command cannot print unless it succeeded (not a literal echoed by the command itself).
- Lint: `node .claude/skills/unlazy/scripts/gate-lint.mjs GATE.md` — fix every error, sharpen every warning.
- Show the user the new CHECK lines. Their go-ahead on the ticket is the approval: `node .claude/skills/unlazy/scripts/gate-check.mjs --approve GATE.md`. Approvals are stored per machine in `~/.unlazy/approved`, not in the repo.

## 4. Branch
- `git switch -c tN-short-name` from main.
- Parallel tickets that own disjoint paths: `git worktree add ../ubh-tN -b tN-short-name`.
- Never two agents on one branch.

## 5. Build
- Claude Code: delegate to the `ticket-executor` subagent ("Execute T3") — Haiku, fresh context. Keep design calls in the main session.
- Codex: implement the ticket directly under the same rules.
- Behaviour change needed? Edit SPEC.md first, then the code.
- Test and build after each step, not only at the end.

## 6. Gate
- `node .claude/skills/unlazy/scripts/gate-check.mjs GATE.md` — exit 0 means every gate is met; evidence is written into GATE.md automatically.
- Exit 1: fix and rerun. A gate genuinely impossible? Add `ABANDON: Tn.Gm <reason>` under it and tell the user. Never delete a gate.

## 7. Check — independently
- Claude Code: `ticket-checker` subagent ("Check T3") → PASS or FAIL.
- Codex: run `--reverify` and review `git diff main...HEAD` against SPEC.md yourself, or hand the check to Claude Code (docs/HANDOFF.md).
- FAIL → back to step 5 on the same branch. Do not patch forward on main.

## 8. Land
- Append to PROGRESS.md: `YYYY-MM-DD | Tn | what shipped | gates exit 0`.
- Commit on the branch with message `Tn: <outcome>`. The pre-commit hook scans staged changes with gitleaks (under a second).
- `git switch main && git merge --no-ff tN-short-name`.
- Any correction the user made during the ticket → AGENTS.md "Learned rules".

## 9. Context hygiene
- Claude Code auto-compacts at about 360k tokens (60% of a 600k window). Before a manual `/compact`, ending a session, or a handoff: PROGRESS.md current, SPEC.md decisions logged, handoff note written if someone else continues.
- If the codebase grows large, use graphify (installed globally) instead of re-reading files.

## Installing anything (skill, plugin, MCP server)
1. Clone or download into a scratch folder — never straight into the project.
2. `skillspector scan <path> --no-llm --format markdown --output <scratch>/report.md` (about a minute for a 40-file skill).
3. Read every HIGH finding at its source line. Keyword false positives are common: unlazy scored 100/100 CRITICAL on benign prose and sandboxed tests. Real red flags: reads `~/.env`, cookies, or SSH keys; network calls carrying your data; obfuscated code; install-time scripts.
4. Show the user the score, the findings, and your verdict. Install only on approval.
5. Install into the project as a copy: `npx skills add <owner/repo> -a claude-code -y --copy [-s <skill> ...]`. Then `diff -rq <scanned copy> .claude/skills/<name>` must print nothing.
6. Record it in docs/SETUP.md.

## Before the demo — last hour
- `gitleaks git --redact --no-banner` (full history) and `gitleaks dir . --redact --no-banner` (working tree).
- Security audit: `/ecc:security-scan`, or the five-check prompt in Initial_setup §12.5. Can one user read another's data? Do tokens expire? Any route open by accident?
- Run the end-to-end test of the core flow on the deployed build, not only localhost.
- Design pass: the prompt in Initial_setup §12.6 with Apple Design, Impeccable, and Chrome DevTools MCP.
- Dry-run the demo out loud, including explaining the code.
