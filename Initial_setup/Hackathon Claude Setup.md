# Hackathon Claude Setup — Complete Environment Setup for Claude

> **For Claude:** Read this whole file before writing any code. Treat Sections 1–7 as operating instructions for the entire hackathon. Install only what a ticket needs. Never invent an install command, repo URL, prompt text, or statistic that this file marks as gated or unverified.

> **For the user:** This is the setup document to paste or hand to Claude at the start of the session. Work through Section 2 in order, ticking boxes as you go.

Source: a sweep of saved Instagram creator posts (reels included), 2026-11-07.

## Coverage and honesty notes

- Instagram reported 3,161 saved posts. The sweep returned **2,683 unique posts** across pages 1–54, after which pagination was exhausted (`has_next_page: false`). About 478 reported saves were not reachable through that pagination. Do not claim all 3,161 were scanned.
- Recent sweep: 995 posts. Older sweep: 1,688 additional unique posts, post dates 2021-06-15 to 2026-01-31 in that slice (saved order is not post-date order).
- Almost every creator gates the real download link behind a comment word such as "HACK" or "REPO". Those DM links were not available. Where a post gave an install command, it is quoted exactly. Where it did not, this file says **[GATED — comment "X"]** and gives the creator and post link. No link has been invented.
- Star counts, prices, "40x cheaper", accuracy gains, and agent/skill counts in the videos are **creator claims**. They conflict between saves. Treat them as hype, not verified facts.

## Recency rule — newest wins

Where an older save conflicts with a newer save on models, prices, install commands, plugin names, counts, or settings, use the newest version. Older versions are flagged `[SUPERSEDED]` below and kept only where the underlying lesson still applies.

| Topic | Use this — newest | Superseded |
|---|---|---|
| Models | Opus 5.5 plans / checks, Haiku 5.5 executes — @murphmaxxing, 2026-11-06 | Opus 4.6 (2026-02-15), Opus 5 (Aug 2026), Opus 4.7 (2026-04-17), Claude 4.5 Sonnet comparisons (Dec 2025), Claude 3 / Claude 2 |
| Auto-compact | Both: `/autocompact` at 600,000 tokens (2026-11-03) **and** auto-compact at 40%, 60% for larger tasks (2026-11-06) | fast-jev-compaction as the default (2026-10-01) |
| ECC counts | 63 agents, 249 skills — @kayvon.ai, 2026-09-07 | 181 skills / 47 agents (2026-07-10, conflicting on-screen numbers) |
| Security audit | 20-check guide — @murphmaxxing, 2026-10-27, comment "PATCH" | 5 checks (2026-10-11) and 19 checks (2026-10-03) — smaller scope, still usable as fallback |
| UX prompt | 20 UX Laws, tested — @run.curl.exe, 2026-10-29 | 20 UX details list — @murphmaxxing, 2026-10-25, untested, different list |
| UI stack | Impeccable + shadcn MCP + Chrome DevTools MCP + Apple Design — @markpyvo, 2026-09-29 | 4-repo list — @angelotrifanoff.ai, 2026-09-06 |
| unlazy install | `npx skills add Leonxlnx/unlazy` — @real_bruno_souza, 2026-10-21 | Earlier post with no install line, gated "WORKTREE" (2026-08-21) |
| Jev cost | 408x cheaper, 4x faster, measured on 216 emails — @baroobi.inc, 2026-10-02 | "About 200x less", launch claims (2026-09-23) |
| OmniRoute | 300+ providers — @thetechniko, 2026-09-28 | 350+ providers (2026-09-16) |

Unresolved same-date conflicts — do not pick one: VibeWise stars 888 vs 1,061 (both 2026-10-04); Apple Design stars 35,000+ vs "passed 40,000" (same 2026-09-22 post).

---

## 1. Setup prompt for Claude — paste this first

```text
You are my pair engineer for a one-day hackathon. Follow these rules all day:

1. Read CLAUDE.md, SPEC.md, and GATE.md at the start of every session and after every compaction.
2. Do not write code before we have done a grill interview and written SPEC.md.
3. Work one ticket at a time. Each ticket gets a fresh agent / fresh context.
4. If the code and the spec disagree, change SPEC.md first, then the code. Never silently change the spec by changing code.
5. Model routing: Opus 5.5 plans and checks. Haiku 5.5 executes well-defined tickets. Use a CHECK step after every ticket: PASS continues, FAIL redoes the ticket.
6. Never mark a ticket done without proof: run its gate command and paste the exit code and output into GATE.md.
7. Never make silent assumptions, never invent an API, never delete code, comments, or tests I did not ask you to delete, and never over-engineer. Make surgical changes only.
8. Before installing any skill or plugin, scan it with SkillSpector and show me the score and findings. Do not install anything that reads ~/.env or sends data out.
9. Design: use the Apple Design skill and the Impeccable / shadcn / Chrome DevTools MCP stack in Section 9. No purple gradients, no generic templates, no invented testimonials, stats, or customer counts.
10. Security: never commit a real API key. Run gitleaks and the security audit in Section 10 before the demo.
11. After every mistake I correct, append the correction as a new rule under "Learned rules" in CLAUDE.md.
12. When you finish a piece of work, report: what changed, why, and what tests ran — in that order.
```

## 2. Setup sequence — first 30 to 60 minutes

Do these in order. Do not code before step 9.

- [ ] Install Claude Code: `npm install -g @anthropic-ai/claude-code` — from the Boris Cherny talk save, @aititans0, https://www.instagram.com/reel/DZAjG3kAqOT/
- [ ] **Scan before you install anything.** Set up SkillSpector first (Section 5). Every later install gets scanned.
- [ ] Install **one** main pack — Matt Pocock skills **or** ECC, not both:
  - Matt Pocock: `/plugin install mattpocock-skills@claude-plugins-official`
  - ECC: `/plugin marketplace add` (ECC repo, affaan-m/ECC) then `/plugin install ECC` — exact marketplace slug was not spelled out in the post [GATED — comment "SETUP" / "ECC"]
- [ ] Install Apple Design: `npx skills add emilkowalski/skills --skill apple-design`
- [ ] If the project is mobile, install Expo: `/plugin install expo@claude-plugins-official`
- [ ] Drop the CLAUDE.md template from Section 12.1 into the project root and fill in the project-specific lines
- [ ] Install unlazy: `npx skills add Leonxlnx/unlazy` — newest install line, 2026-10-21
- [ ] Optional, for token savings: `/plugin marketplace add DietrichGebert/ponytail` then `/plugin install ponytail@ponytail`
- [ ] Run the grill: use `/grill-me` (Matt Pocock) or ask Claude the three grill questions in Section 7.1, then write SPEC.md from the template in Section 12.2
- [ ] Split SPEC.md into small tickets in GATE.md, each with a gate command (Section 12.3)
- [ ] Set compaction: `/autocompact` at 600,000 tokens, and auto-compact at 40% context (60% for larger tasks)
- [ ] Create the folder structure in Section 3 and, for a team, the agent board in Section 12.4
- [ ] Run the seven-pillars check (Section 7.9): frontend, backend, database, auth, payments (only if needed), security, infrastructure — each pillar explicitly covered or explicitly skipped

---

## 3. Folder organization

### 3.1 Project structure Claude should create

The structure actually named in the saves (@murphmaxxing, 2026-11-06) is `CLAUDE.md`, `.claude/agents/`, and `skills/`. The rest below is a synthesis applying the saves' rules — one owner per directory, spec and gates at the root where every agent reads them.

```text
project-root/
  CLAUDE.md          # project rules — read at every session start
  SPEC.md            # what we are building, in plain English
  GATE.md            # acceptance ledger — no ticket is done without its gate passing
  PROGRESS.md        # one line per finished ticket: what, when, gate result
  .claude/
    agents/          # project-specific agent definitions, one .md file per role
  skills/            # project-specific skills you add today (scan each one first)
  src/               # application code — split by owner, see 3.2
  tests/             # at minimum, one end-to-end test of the core flow
  .env.example       # variable names only, never real values
  .gitignore         # must include .env
```

**Instructions for Claude:**

- Put reusable, global agent definitions in `~/.claude/agents/` (one Markdown file per agent, e.g. `engineering-frontend-developer.md`). That is where the 147-agent pack installs. Keep project-specific rules in the project, reusable roles globally.
- Keep `CLAUDE.md`, `SPEC.md`, `GATE.md`, and `PROGRESS.md` at the root. Never bury the spec inside `src/`.
- Never put a real key in any file that git tracks. Real values go in `.env` only, and `.env` is gitignored from the first commit.

### 3.2 Split folders so agents never collide — the agent board rule

Source: @azizbuildss, 2026-11-01, https://www.instagram.com/reel/DfOLEP6v3AR/ — the team reports winning 4 sponsor tracks at one hackathon and a track prize at Cal Hacks (creator claim).

1. Split the project into folders so **each person owns one directory**. Agents never edit the same files as each other.
2. Pin **one GitHub issue as the message board**. Agents post updates, status, and blockers there instead of relying on humans relaying messages.
3. **Tag teammates' GitHub usernames** in board posts so agents can talk to each other without a human in the middle.
4. Assign one human as demo/judges person, not as a coder. In the source post the creator ran much of the coordination from his phone and spent his time on sponsors and judges.

> Check today's hackathon rules on AI assistance and multi-agent use before using the agent board. Comments on the source video were polarised about legitimacy.

### 3.3 Worktrees and branches

- Rule: **one ticket = one fresh agent = one branch or worktree.** Never run two agents on the same branch.
- unlazy is described as allowing worktree depth/branches [GATED — comment "WORKTREE", @cooper.simson]. No worktree commands were given in the post, so use standard git worktrees and do not claim the post specified them.
- Borrowed code rule (@cindie.zhu): a repository is reference code, not a magic asset. Check its licence, maintenance, and framework first, then ask Claude to isolate the smallest effect you need and adapt it **inside a disposable branch** before merging. Never merge borrowed code straight to main.

### 3.4 Editor folder visibility (older save, still useful)

If editing in VS Code (@jackk_flick, 2025-11-17): install ES7 Snippets, Tailwind CSS IntelliSense, Material Icon Theme (visual folder icons — helpful as folders multiply, but this is *not* the Claude Code folder structure above), Thunder Client (test API endpoints inside the editor), and GitLens (see who changed what — essential when several agents are editing).

---

## 4. Persistent memory — stopping Claude forgetting

### 4.1 CLAUDE.md is the memory

This is the single most repeated tip in the saves. The Karpathy behavioural CLAUDE.md (@pranavinno, https://www.instagram.com/reel/DZIYih_hExu/; @aifornontechies, https://www.instagram.com/reel/DdoV5mik0DU/, repo `mulca-ai/andrej-karpathy-skills`) is a 65-line file dropped in the project root that Claude reads at every session start [GATED — comment "KARPATHY" for the verbatim file].

What it fixes, per the posts: silent assumptions, bloated code, deleted comments, over-engineering, ignoring instructions, marking work complete when it is not, and hallucinated APIs. The creator claim of 65% to 94% accuracy is unverified — the rules themselves are the value.

**Instructions for Claude:**

- Read CLAUDE.md at every session start and after every compaction, before touching code.
- Follow the starter template in Section 12.1 until the user has the verbatim Karpathy file. The template is a paraphrase, not the original.
- **Continuously update CLAUDE.md after mistakes** (Boris Cherny, via @mindwired.ai): every correction the user makes becomes a reusable guardrail under "Learned rules" for the next run. Do this immediately, not at the end of the day.

### 4.2 The three-file memory routine

Keep three files current, and memory survives compaction, new sessions, and new agents:

| File | What goes in it | When Claude updates it |
|---|---|---|
| `CLAUDE.md` | Rules, conventions, learned corrections | After every corrected mistake |
| `SPEC.md` | What we are building, naming rules, decisions log | Before any code change that alters behaviour |
| `PROGRESS.md` | One line per finished ticket, with gate result | When a ticket's gate passes |

After any compaction, Claude re-reads all three before continuing. This is the practical antidote to the "re-explain everything every session" failure mode.

### 4.3 Compaction settings

- `/autocompact` at **600,000 tokens** — @j.t.rivard, 2026-11-03, from a claimed 690-hour run.
- Auto-compaction at **40%** of context, **60% for larger tasks** — @murphmaxxing, 2026-11-06.
- These two are complementary, not conflicting: the first is a hard ceiling, the second is the working threshold.
- [SUPERSEDED as default] fast-jev-compaction (2026-10-01), which replaces compaction summaries with Jev decisions. Do not make it the default over the November settings.
- Do **not** use the "memory hack" shouting prompt (@artificialintelligenceupdater, 2026-08-13, Opus 5 era, "10x better" unattributed). Superseded and unverifiable.

### 4.4 Knowledge graphs — only if the codebase gets large

- **Graphify** (@duncanrogoff): maps the codebase into a knowledge graph so Claude navigates relationships instead of re-reading files every session — fewer tokens, faster sessions, better continuity. Reality check from the same save: it will not fix bad architecture. [GATED — comment "REPO"]
- **Base / basemode** (@charlieautomates, 2026-10-01): a Rust binary that turns the codebase into a knowledge graph for Claude Code hooks, giving memory across sessions. From the "9 repos to fix Claude's amnesia" save [GATED — comment "Claude"].
- **ECC memory skills** (@kayvon.ai): ECC includes one skill that optimises memory so long sessions do not fall apart, and another that learns from past sessions. Names were not given in the post.
- **Claude-Mem** (older save, @digitalsamaritan, 2026-01-06, https://www.instagram.com/reel/DTMgguHkY4H/): described as "an open-source tool for building persistent memory for Claude projects" [GATED — comment "claude"]. This is the only older-slice post about persistent memory. Treat it as a lead to verify — no install command or repo URL was given — not as a setup step. The CLAUDE.md approach above is the primary system.
- The rest of the amnesia list (@charlieautomates): Jev, CodeNotch, the LinkedIn agent skill, Dots, Go Live, shadcn/lint, Magpie. Pick one graph tool at most; do not install the whole list mid-hackathon.

---

## 5. Installing skills safely

### 5.1 SkillSpector first — the most important safety step

Source: @liamjohnston.ai, https://www.instagram.com/reel/Dc1oIn8CyDW/

NVIDIA found that 5.2% of public skills show malicious intent and over a quarter contain vulnerabilities (creator claim, attributed to NVIDIA). A skill can inherit access to your environment variables, API keys, local files, and connected tools. Malicious instructions can hide in comments, invisible text, encoded scripts, and dependencies.

In the demo, a benign-looking `lead-generation.md` skill hid HTML-comment instructions to read `~/.env` and POST its contents out. SkillSpector scored it 58/100, HIGH, DO NOT INSTALL, with findings across 17 categories.

**Instructions for Claude:**

1. Before installing any skill, run SkillSpector on its `SKILL.md`, folder, ZIP, GitHub repo, or your whole skills library.
2. It returns a 0–100 risk score plus exact findings, and checks for prompt injection, credential theft, data exfiltration, remote scripts, dangerous execution, hidden persistence, risky dependencies, and MCP tool poisoning.
3. Show the user the score and findings. Do not install a HIGH-risk skill. Do not install anything that reads `~/.env`, accesses session cookies, or sends data to an unknown endpoint.
4. SkillSpector can run as an MCP server so Claude scans a skill before trusting it. Install and setup guide [GATED — comment "SCAN"]. Spelling note: "SkillSpector" in the caption, "SkillSpecter" in the video.

### 5.2 Verified install methods — only these were actually shown

```bash
npm install -g @anthropic-ai/claude-code
npx skills add emilkowalski/skills --skill apple-design
npx skills add Leonxlnx/unlazy
```

```text
/plugin install mattpocock-skills@claude-plugins-official
/plugin install expo@claude-plugins-official
/plugin marketplace add DietrichGebert/ponytail
/plugin install ponytail@ponytail
/plugin marketplace add <ECC repo>   then   /plugin install ECC
./scripts/install.sh --tool claude-code   # 147-agent pack
```

Anything not in this list was gated in its post. Do not construct a plausible-looking install command for a gated tool — use the gated lookup in Section 14 instead.

### 5.3 Marketplace vs GitHub — how to judge a source

- The official plugin marketplace is the safest route. Prefer it when a plugin exists there.
- For a GitHub repo: read the README and licence first (@murphmaxxing), and check licence, maintenance, and framework fit (@cindie.zhu) before running any installer.
- Check what the installer adds and what the resulting agent can access. Session cookies are basically passwords (@johannships, on Agent Reach) — never hand them to an unverified tool.
- BrowserSkill's install URL was truncated in its caption. Do not guess the missing part.

### 5.4 Skill, sub-agent, prompt, or plugin — how Claude should choose

| Need | Use | Why |
|---|---|---|
| A repeatable procedure | Skill | Same steps, every time, e.g. a design audit |
| A role with its own context | Sub-agent | Keeps the main context clean, e.g. a security reviewer |
| A one-off instruction | Prompt | No reason to persist it |
| A bundle of skills, agents, and commands | Plugin | Install once, e.g. Matt Pocock or ECC |
| An action invoked in the moment | Command | e.g. `/plan`, `/tdd`, `/security-scan` |

### 5.5 Finding skills without drowning

- **Skill-finder skill** (@benkimball.ai, https://www.instagram.com/reel/DduqYBaPFFr/): built by a Vercel engineer, searches 700,000+ Claude skills and auto-downloads the best for what you are working on (3M+ installs is a creator claim). Name not given [GATED — comment "skills"]. Scan anything it downloads before use.
- **The LEAN counterpoint** (@cooper.simson, 2026-10-06, https://www.instagram.com/reel/DeLgn1itpX6/): "delete most of your skills… somebody using a blank version of Claude Code might actually be outperforming your setup" [GATED — comment "LEAN"].
- **Rule for today:** install the Section 2 set only. Add at most one new skill per ticket, scanned first, and only to solve a problem you actually have.
- Roundups (40 repos, 9 repos, 20 repos, 5 repos) are menus, not shopping lists. See Section 8.4 for their contents.

## 6. Best practices for using skills — the rules that keep reappearing

**Instructions for Claude — apply these every time a skill is used:**

- [ ] Scan first (Section 5.1). No exceptions for "it looks harmless".
- [ ] Read the README and licence before installing from GitHub.
- [ ] Use one main pack, not five overlapping packs.
- [ ] After installing a design or quality skill, explicitly tell Claude to use it. Working phrasing from @angelotrifanoff.ai: "find three things to fix, explain each before changing anything, and check the results yourself."
- [ ] Change the spec, not the code, when behaviour is wrong.
- [ ] Never accept "done" without proof — a gate command, its exit code, and its output.
- [ ] Do not install a skill to fix a problem the current model no longer has. Older saves solve older models' problems.
- [ ] Check what access an installer grants before running it.
- [ ] Keep project rules in the project (`CLAUDE.md`), reusable roles in `~/.claude/agents/`, and nothing secret in either.
- [ ] Prefer named, dated, installable tools over roundup hype. If a count or star figure matters to a decision, note that it is a creator claim.
- [ ] Stop and ask when a fix breaks working code, rather than stacking another fix on top.

---

## 7. Workflows — the core methods

### 7.1 Grill → SPEC.md → tickets → fresh agent per ticket

The main workflow. Sources: @lukegarvai, https://www.instagram.com/reel/DfDxAq4oIYM/; @smartthingsexplained, https://www.instagram.com/reel/DenUlUKoQHt/; @cristiancruz on Matt Pocock skills.

**Instructions for Claude:**

1. **Grill before coding.** Ask, and do not proceed until answered: "Who is this for?" / "What breaks if it's wrong?" / "What does done look like?"
2. Use **Grill with Docs**: one shared reference file so the database, backend, and UI use the exact same words for the same things. Put the naming rules in SPEC.md. Pair this with automated tests.
3. **Write SPEC.md in plain English** from the template in Section 12.2.
4. **Cut the spec into small tickets.** Each ticket is small enough for one agent to finish and one human to check quickly.
5. **One fresh agent per ticket.** Fresh context per ticket is the point — it stops errors compounding.
6. Work Claude on **medium effort for build tickets**; reserve maximum reasoning for planning and hard debugging (see 7.4).
7. **Check the code against the spec.** If it is wrong, change SPEC.md first, then the code.
8. Avoid the doom loop — every fix breaking three other things — which the posts trace to agents silently guessing instead of asking. When unclear, ask. That is what the grill is for.

Matt Pocock extras: `/wait-what` re-explains Claude's last answer simply; `/teach` builds lessons from your codebase (e.g. "Lesson 1: How a booking moves") — use it before the demo; `/to-questionnaire` turns a stuck decision into a shareable questionnaire.

### 7.2 ECC commands

From Everything Claude Code: `/plan`, `/tdd`, `/security-scan`. Working pattern: say "Plan this feature", "Build it", "Test it", "Secure it". ECC runs tests toward 80% coverage and red-teams its own code before release (creator description). Pick ECC **or** Matt Pocock as the main pack in Section 2, not both.

### 7.3 unlazy — stopping "done" when it is not done

Sources: @cooper.simson; @real_bruno_souza, 2026-10-21.

The problem, named "partial compliance": the AI says done after 30 minutes but skipped steps, then agrees "yeah, great point" when called out. In the cited benchmark the best agent completed under 15% of checkpoints (creator claim).

**Instructions for Claude:**

1. Before starting, write an **acceptance ledger** in GATE.md (template in Section 12.3): every task gets a gate, and every gate has a command to run plus the exact expected output.
2. A ticket is done only when its gate command has run, passed, and the **exit code is pasted into GATE.md**.
3. Run independent tickets in parallel where the folder split allows it; a purely sequential run "goes one at a time forever".

### 7.4 Parallel agents and model routing — Opus plans, Haiku executes

Newest setup: @murphmaxxing, 2026-11-06, https://www.instagram.com/reel/DfaLyn7Rorf/

```text
Opus 5.5  →  plan the ticket, write the spec for it, define the check
Haiku 5.5 →  execute the ticket (research, build, edit)
CHECK     →  Opus (or a checker agent) verifies against SPEC.md and GATE.md
FAIL      →  redo the ticket, do not patch forward blindly
PASS      →  update PROGRESS.md, move to the next ticket
```

- Cost basis: $4 vs $0.10 per million tokens, "40x cheaper" — creator claim, the routing pattern is the takeaway.
- Proof the pattern scales: 25 Haiku agents pulled 1,000 job postings in under 11 minutes for under $1.50 (@remoteree, 2026-11-04), using an answers-to-form-fields map; a first pass got most items, a second pass fixed the rest; items the platform blocked and items needing a human were set aside, **nothing was auto-submitted, and human approval came before anything irreversible**. Copy that approval rule.
- **Ruflo / FLOW** (@kayvon.ai, 2026-05-30, older): 60 agents in parallel — one plans, one codes, one tests, one checks security — sharing memory, with cost routing (basic work to cheaper tiers, hard work to the best model), claimed 75% cost cut [GATED — comment "FLOW"]. The pattern matches 7.4; the model details predate 5.5.
- Five long-run commands (@j.t.rivard, 2026-11-03) [GATED — comment "LIST"]: #5 the "You should know" plugin (a second agent watches your agent); #4 `/loop` with no fixed interval; #3 `/autocompact` at 600,000; #2 Ultracode (coordinate dozens or hundreds of agents with custom JavaScript); #1 `ultrathink` — maximum reasoning **for one turn only**. Use `ultrathink` for planning and the hardest bug, never as a default.
- Boris Cherny's 10–15 parallel sessions (2026-02-25 / 05-31, older, pre-5.5): the method — parallel terminal and web sessions, CLAUDE.md continuously updated, data pulled via CLI rather than hand-written queries — still works. Session counts and models are dated.

### 7.5 Verification loops — never trust "done"

- "Letting Claude assume it fixed an issue without automated verification is how you end up debugging ghost code." — @murphmaxxing, 2026-10-03.
- Anthropic's own QA, per Boris Cherny (@tracebackqa, 2026-10-19): lots of lint rules, lots of tests, Claude-powered fuzzers daily, automated code and security reviews, Claude-driven end-to-end (E2E) tests. Instruction from the post: "Start with one: your checkout" — for a hackathon, substitute **your core flow** and write one E2E test for it before the demo.
- The older mega-prompt save (@agentic.james, 2025-12-23) had one durable rule worth keeping: **test and build after each step**, repeating until complete. Its one-shot approach itself is superseded — see Section 13.

### 7.6 VibeWise — so you can explain what Claude built

Repo: `github.com/nykoo11/vibe-wise` (@remoteree, https://www.instagram.com/reel/DeM2KoJFDhf/). Run `learn`: explain your own approach first, Claude walks the tradeoffs with a design checkpoint and an implementation checkpoint **before** writing code, then reports what changed, why, and what tests ran. Notes stay in the project folder. Marketplace handle was rendered as `nykooil/vibe-wise` in one save — verify the handle before using a marketplace install; the GitHub repo above is the grounded reference. The older "learning mode" post (@oliver_codes, 2026-01-22, building a blockchain in Go) is the same idea with no command given — prefer VibeWise.

### 7.7 Spec over prompt

- "The prompt matters way less… The real secret is a tight spec and letting the model iterate in a loop with real test feedback. Spec over prompt. Every time." — @dubibubiii. [SUPERSEDED model — Opus 4.6, 2026-02-15; the lesson is retained, the model is not current.]
- Current version of the same point (@thearseniodev, 2026-10-01): do not ask Claude to "make your website look premium". Bring the mood, the references, and the hierarchy yourself, ideally with a real-asset MCP library. "I decide the mood. I choose the references. I control the hierarchy. Claude does the heavy lifting."
- Boris Cherny's 28-minute prompting talk (@aititans0, link in Section 2) is the only primary-source prompting advice in the sweep: CLAUDE.md, memory shortcuts, parallel sessions, prompting patterns. Claude Code is terminal-based, general-purpose, and built to fit existing workflows.

### 7.8 harness2 — tune the setup in hour one

Google's open-source recursive agent harness (@keeran.ai, https://www.instagram.com/reel/Det7c9Ggnys/, repo `google-research/harness2`) [GATED — comment "tool"]: it tests your agent against a task, rewrites the prompts and tool instructions, and saves the lessons so the setup improves without retraining. Works with Claude Code and Codex. Hackathon use: run it once in hour one on a representative ticket, then save its lessons as rules in CLAUDE.md.

### 7.9 Seven pillars architecture check (older save, still valid)

@chase.h.ai, 2026-01-05, https://www.instagram.com/reel/DTJemnhDRfu/ — before calling the app "full-stack", confirm each pillar is covered or consciously skipped:

1. Frontend 2. Backend 3. Databases 4. Authentication 5. Payments 6. Security 7. Infrastructure

The post's "100+ agent templates" are gated (comment "agent") and are superseded as a shopping list by the named packs in Section 8.

### 7.10 MCP — the mental model and which servers to add

MCP (Model Context Protocol) connects AI to the real world (@theaiagents, 2025-03-12): without it, Claude cannot fetch live data or act in other tools and you copy-paste everything; with it, Claude can pull live data and operate connected software.

Add only the servers the project needs:

| Server | Use | Source / note |
|---|---|---|
| Chrome DevTools MCP | Claude opens the site in a real browser, sees bugs, fixes them | Newest preference, Section 9 |
| shadcn MCP | Real component library access for UI builds | Section 9 |
| Supabase MCP | Database and auth | @agentic.james, 2025-12-23 — ask Claude Code to install it; keep the MCP, drop his one-shot prompt |
| Playwright MCP | Browser tests simulating a real user | @nick_saraev, 2025-09-30 — valid alternative; Chrome DevTools MCP is preferred for Claude Code |
| Expo MCP | Mobile builds, with the Expo skills | Section 9.3 |
| RevenueCat / App Store Connect MCPs | Mobile payments / store submission | @agentic.james, 2025-12-31 — only if the mobile app needs them; verify current server names |

---

## 8. Full catalogue — every skill, plugin, and tool in the sweeps

Install status key: **Verified** = install shown in the post. **Gated** = comment word given in Section 14, no link available. Counts are creator claims.

### 8.1 Main packs — pick one

- **Everything Claude Code / ECC** (`affaan-m/ECC`) — auto-detects your stack and installs skills, sub-agents, and commands for planning, building, testing, and security scanning. Commands: `/plan`, `/tdd`, `/security-scan`. Newest counts: 63 agents, 249 skills (2026-09-07). **Gated** — "SETUP" / "ECC"; marketplace slug not spelled out.
- **Matt Pocock skills** (`mattpocock/skills`) — 27 skills, official plugin: `/grill-me`, `/teach`, `/to-questionnaire`, `/wait-what`, plus Grill with Docs. **Verified:** `/plugin install mattpocock-skills@claude-plugins-official`
- **Ruflo / FLOW** — 60 parallel agents with shared memory and cost routing. **Gated** — "FLOW" (2026-05-30, pre-5.5 models).
- **147 specialist agents (AGENCY)** — pre-built agents in `~/.claude/agents/` across business and engineering (Tax Strategist, Fractional CTO, AI Citation Strategist). Most useful for pitch and positioning. **Verified:** `./scripts/install.sh --tool claude-code` (2026-05-16).
- **Claude Code Game Studios** — 49 agents, 74 skills, 62 templates for Godot 4, Unity, Unreal 5. Games only. **Gated** — "STUDIO".
- **Superpowers** — a pack described as making Claude "plan before it codes" (@angelotrifanoff.ai). Repo slug not given. Do not guess it.

### 8.2 Single skills — small, high leverage

- **Untangle Codebase** (@voxaris_ai) — finds duplicated logic, unnecessary wrappers, and temporary workarounds and cleans them up. Run mid-hackathon when the codebase sprawls. No install in post.
- **VibeSec** (@baroobi.inc, 2026-08-14) — one Markdown file that makes Claude think like a bug-bounty hunter before writing code: IDOR, XSS, SSRF, hardcoded keys, weak passwords. Works in Claude Code, Cursor, Codex, Copilot. **Gated** — "BOUNTY".
- **Karpathy behavioural CLAUDE.md** — see Section 4.1. **Gated** — "KARPATHY".
- **unlazy** — acceptance ledgers and gate files, Section 7.3. **Verified:** `npx skills add Leonxlnx/unlazy`
- **Skill-finder skill** — searches 700,000+ skills. **Gated** — "skills"; name not given.
- **Jev** — "Jev sorts, Claude writes": a tiny decision model making one cheap decision at a time with a confidence score; items over ~70% confidence auto-sort, unsure items route to Claude. Newest figures: 408x cheaper, 4x faster (2026-10-02). Use it in front of repetitive classification and in front of Haiku-vs-Opus routing. **Gated** — "JEV".
- **VibeWise** — see Section 7.6. Repo `github.com/nykoo11/vibe-wise`.
- **harness2** — see Section 7.8. Repo `google-research/harness2`.

### 8.3 Plugins, MCP servers, and tools

- **Ponytail** — cuts token usage 50%+ (creator claim). **Verified:** marketplace add + install, Section 5.2.
- **OmniRoute** — connects Claude Code to 300+ free AI providers (newest figure, 2026-09-28) and auto-switches when limits run out. **Gated** — "CLAUDE" / "plugin".
- **Graphify** — codebase to knowledge graph. See Section 4.4. **Gated** — "REPO".
- **AgentSkills** — 24 skills for planning, coding, testing, publishing. **Gated** — "CLAUDE" / "plugin".
- **REA — Reverse Engineer Anything** (`morluto/rea`) — MCP server for reverse-engineering native binaries, Electron/JS apps, .NET, and websites. Needs Ghidra, IDA, or Hopper for real decompiling. Only inspect software you are allowed to inspect, and note the model provider still sees the results. **Gated** — "REA".
- **xtool** (`github.com/xtool-org/xtool`) — open-source Xcode replacement; Claude can build, sign, and deploy real iOS apps from Linux, Windows, or macOS. Needs Apple SDK and developer auth setup.
- **CLI-Anything** — gives desktop software a text CLI so agents can drive it: 79 ready-made CLIs including OBS, Zoom, Obsidian, Blender, GIMP, LibreOffice, Audacity. Apache 2.0, 50k+ stars (creator claim). **Gated** — "APPS".
- **Agent Reach** — one-click web access without a stack of API keys: Jina Reader, yt-dlp, gh CLI, plus X, Reddit, Facebook, Instagram, LinkedIn channels. Caption warns some sources still need a login and session cookies are basically passwords. **Gated** — "REPOS".
- **SkillSpector (NVIDIA)** — see Section 5.1. **Gated** — "SCAN".
- **BrowserSkill (Tencent)** — borrows your signed-in browser (`bsk install-skill`, `bsk doctor`); its own caption carries a prompt-injection warning and its install URL was truncated. **Do not use on real logged-in accounts.**
- **Scrapling MCP** — web scraping with a real-browser fingerprint and headers. **Gated** — "SCRAPE" / "DATA".
- **Dots, Go Live, Magpie, CodeNotch, shadcn/lint, LinkedIn agent skill** — from the amnesia list (Section 4.4). Go Live deploys with hosting, database, and auth. Magpie switches models across tools (binary under 15 MB). CodeNotch shows usage in the Mac notch. **Gated** — "Claude".
- **Pyrefly** (older save, @better.engineer, 2025-11-04) — Meta's Rust-based Python type checker. Use it if the backend is Python (e.g. FastAPI). No install command in the post — use the project's official install. Caption spelling "Pyrefly" is correct; the video transcript's "Pyrely" is not.
- **Claude-Mem** — persistent-memory lead only, Section 4.4. **Gated** — "claude".

### 8.4 Roundups — menus, not shopping lists

- **40 free Claude repos** (@lukegarvai) [GATED — "REPOS"]: Understand Anything, Prompt Master, Code Notch, Karpathy Skills, Open Design, Career Ops, Open Montage, Marketing Skills, Diagram Design, Website Cloner, Book to Skill, Hallmark, Claude Obsidian, Code Burn, Video Shotcraft, Dash PPT, HTML Anything, Canva Online, Autonomess, Open Executive, Lottie, Avoid AI Writing, Essayu Design, Feldex, Fast Jev Compaction, Maggie, Camera, Unlazy, LinkedIn Skills, Cost X-Ray, Claude of Duty, Commerce Agents, Social Media Research, Open SEO, Scroll Craft, Serpia, JS Stack, Anything to Explainer, Logo Design Skill, Universal Modder.
- **5 repos** (@lukegarvai, 2026-10-21) [GATED — "REPOS"]: yt-dlp, Whisper, Caveman (cuts Claude's tokens), HyperFrames, Last 30 Days.
- **9 repos to fix Claude's amnesia** (@charlieautomates, 2026-10-01) [GATED — "Claude"]: Jev, CodeNotch, Base/basemode, LinkedIn agent skill, Dots, Go Live, shadcn/lint, fast-jev-compaction, Magpie.
- **20 niche repos** — the shipping/security list, Section 10.1.
- **Hermes 17-skill list** — for a different agent system. Transferable ideas only: Agent Reach, Matt Pocock skills, codebase-memory-mcp, make-interfaces-feel-better, defuddle. Do not install Hermes-specific tooling (minions, oh-my-hermes, SkillClaw) into Claude Code.
- **60+ hidden commands cheat sheet** [GATED — "CLAUDE"] — contents not given in the post. Cannot verify.
- **342-hour AI course repo** [GATED — "SCRATCH"] — 523 lessons, 20 phases. After the hackathon, not during.
- **Dossier** (`https://github.com/AienTech/dossier`) — a job-application tool, Docker, single-user desktop. Not hackathon-relevant; listed for completeness.

### 8.5 Cost and token tools

- **Ponytail** — 50%+ token cut (creator claim), verified install above.
- **OmniRoute** — provider switching when limits hit, Section 8.3.
- **Graphify / Base** — stop re-reading the codebase, Section 4.4.
- **Jev** — cheap decisions in front of expensive ones, Section 8.2.
- **Caveman** — "cuts Claude's tokens, so your plan lasts longer" [GATED — "REPOS"]. No install line.
- **Magpie, CodeNotch, ccusage** — model switching and usage visibility. ccusage appears by name only in the 20-repo list.
- **Mid-day rescue rule [synthesis from the saves]:** if limits loom, switch executors to Haiku 5.5, compact at 40%, stop re-reading files (use the knowledge graph), and keep Opus for planning and checks only.
- **Flagged, not recommended:** "free Claude Code forever" via NVIDIA free APIs at 40 requests/minute (@rick.automates, 2026-04-26, repo unnamed) and a Gemma jailbreak of Claude Code (@wassimyounes_, 2026-04-15). Do not use either — unverified, and routing code and keys through third parties is a security risk.

---

## 9. Design and Impeccable / UI

Start the design pass early, not in the last hour. This stack is the newest UI source in the saves.

### 9.1 The Impeccable stack — install this for UI work

Source: @markpyvo, 2026-09-29 [GATED — comment "DESIGN" for the guide].

- **Impeccable** — spots and fixes AI-slop frontend: generic layouts, weak hierarchy, template look.
- **shadcn MCP** — gives Claude the real component library (Accordion, Alert Dialog, Avatar, Breadcrumb, Button, and the rest) instead of hand-rolled lookalikes.
- **Chrome DevTools MCP** — Claude opens your site in a real browser, sees the actual bugs, and fixes them itself, without you screenshotting everything.
- **Apple Design skill** — the taste layer, Section 9.2.

**Instructions for Claude — design audit sequence:**

1. Install the stack (Apple Design install is verified in Section 5.2; the rest are gated — see Section 14).
2. Build the screen first, then audit it. Do not audit an empty page.
3. Prompt: "Find three things to fix. Explain each before changing anything. Then fix them and check the results yourself in the browser."
4. Apply the 20 UX Laws check (Section 9.4).
5. Use Chrome DevTools MCP to verify the fix in a real browser.
6. Run Dimensional Drawing on the hero component (Section 9.5).
7. Final check: one accent colour, no purple gradient, no generic template layout, no invented stats or testimonials, all tap targets at least 44px.

### 9.2 Apple Design skill

Install: `npx skills add emilkowalski/skills --skill apple-design` (@liamjohnston.ai, 2026-09-22).

- Content: Apple's Human Interface Guidelines (described as 123 pages) plus WWDC talks, packaged as skills — spacing, typography, motion, hierarchy, materials, accessibility.
- Motion rules named in the post: respond on pointer-down, never lock input during an animation, anchor interactions to the element that started them.
- What Claude does with it: reviews hierarchy, restraint, clarity, feedback, spacing, motion, typography, materials, and accessibility, then fixes the highest-impact problems **without changing your product, copy, or information architecture**.
- This is a community project, not an official Apple product.
- Apple's own free resources (@keeran.ai, 2026-09-27) [GATED — "APP"]: iOS design templates, Figma/Sketch resources, SF Symbols, the HIG itself, and Icon Composer.

### 9.3 Expo skills — mobile only

Install: `/plugin install expo@claude-plugins-official` (@guhmp4, 2026-10-20) [guide GATED — "Expo"].

- **expo-design-system** — audits hard-coded colours and spacing into one theme.
- **expo-native-ui** — applies Apple's native UI rules.
- **expo-animation** and **expo-ui** — animation, and real SwiftUI components.
- Plus Expo MCP (Section 7.10).
- Working prompt from the post: "Make my grocery list screen feel more native."

### 9.4 The 20 UX Laws prompt

Newest and only tested version: @run.curl.exe, 2026-10-29 [GATED — comment "UX" for the prompt text].

The 20 laws: aesthetic-usability effect, Hick's law, Fitts's law, Jakob's law, Miller's law, Doherty threshold, law of proximity, law of similarity, common region, visual hierarchy, progressive disclosure, recognition over recall, Von Restorff effect, peak-end rule, goal-gradient effect, serial position effect, Tesler's law, Postel's law, Occam's razor, feedback loops.

Test reported in the post (one run each — a hint, not proof): the page got 30% shorter (6,866px to 4,803px), tap targets under 44px dropped from 7 to 2, and Claude stopped inventing fake copy such as "12,000+ happy customers".

[Superseded] @murphmaxxing, 2026-10-25 — a different list of 20 UX details drawn from Apple, Linear, Gmail, Stripe, and Amazon, untested. Use the 2026-10-29 version.

**Instructions for Claude:** after building each main screen, check it against the 20 laws above, list every violation with the law it breaks, fix tap targets under 44px first, and delete any statistic, testimonial, or customer count that is not in SPEC.md.

### 9.5 Dimensional Drawing — design-spec handoff

Repo given in caption: https://github.com/enesozturk/dimensional-drawing (@aiwithbibek, 2026-10-28, MIT).

Point it at a card or dialog and it produces a one-page Apple-style technical drawing: paddings, gaps, radii, and layers measured and labelled. Anything it cannot measure is marked TBD instead of invented — copy that behaviour. A second skill animates the drawing like a pen sketch; without Claude that animation needs Node and ffmpeg.

Use it on the one component judges will stare at — the hero or the core interaction — to show the design was deliberate.

### 9.6 Avoiding AI slop — the collected rules

- Purple gradients and generic templates are the signature of AI-built UI. Ban both in CLAUDE.md.
- Better rules beat longer prompts. Put design constraints in CLAUDE.md and the design prompt (Section 12.6), not in ever-longer one-off prompts.
- Use **one accent colour** (@niki.aroca).
- Never invent testimonials, statistics, logos, or customer counts. If it is not in SPEC.md, it does not go on the page.
- **shadcn/lint** — an agent-first Tailwind linter from the amnesia list; use it to kill generic AI design patterns.
- **21st.dev** (older save, @itsjamesmwild, 2026-01-03): a free library of premium UI components where every component comes with **the exact AI prompt that created it**. The trick: copy the prompt, not just the code, and paste it into Claude Code. [SUPERSEDED in part] the post names Cursor and Lovable as the builders — use Claude Code today. [GATED — "AI"]
- **Kitchen UI library** (@lukegarvai, 2026-10-29) [GATED — "KITS"]: distinctive components (cassette player, scrambling text, hold-to-delete, receipt checkout). Working phrasing: "hand it to Claude and say 'add this to my site'".
- **UNSlop** (@griffinwooldridge1) [GATED — "UNSLOP"]: no description in the post. Cannot verify — do not rely on it.
- Caption-only leads, cannot verify: "3 proven ways to vibecode a professional UI" (@aimn.dev, 2026-08-24), a motion-design prompt (@millee.md), and Claude animations (@buildwithbyron). An animations library of cursor effects, menus, carousels, and shape blur (@reach.webs, 2025-12-16) [GATED — "Website"] is inspiration only.
- Google Stitch (wireframe to UI design and front-end code, from the older "12 free Google tools" save, 2025-12-27) is a possible supplement only. Its "free, no credit limits" claim is from Dec 2025 — verify current pricing — and it does not replace this stack.

---

## 10. Security and shipping — the pre-demo gate

### 10.1 The 20-repo shipping checklist

@murphmaxxing, 2026-11-02 [GATED — comment "REPO"]. Read each repo's README and licence before using it.

gitleaks, claude-code-security-review, chrome-devtools-mcp, better-auth, tweakcn, claude-code-action, coolify, trivy, umami, ccusage, mem0, OpenSpec, semgrep, zaproxy, drizzle-orm, react-bits, claude-squad, marketingskills, CheatSheetSeries, claude-task-master — plus the design MCP already covered.

Hackathon picks from this list: **gitleaks + better-auth + claude-code-security-review + trivy or semgrep**. The rest are only if the project needs them.

### 10.2 The audits — newest first

- **Newest: 20-check guide** (@murphmaxxing, 2026-10-27) [GATED — comment "PATCH"]: paste the guide into Claude Code and it checks the app for all 20 flaws. The post reports AI-written code had flaws in 45% of Veracode's tests (creator-reported figure). Use this guide if you can get it.
- **VibeSec** (Section 8.2): build-time protection — IDOR, XSS, SSRF, hardcoded keys, weak passwords — caught while building, not after.
- [Superseded in scope] **19 checks** (@joshtheaiguy, 2026-10-03 / 09-29): permissions, exposed secrets, admin routes. Still useful if the 20-check guide stays gated.
- [Superseded in scope] **5 checks** (@baroobi.inc, 2026-10-11) [GATED — "AUDIT"] — the only check list actually printed in a caption, so it is reproduced as a ready prompt in Section 12.5: exposed secrets (`.env`, git history); auth (weak passwords, never-expiring sessions, open routes); database access (can one user read another's data?); input validation (SQL injection, XSS, file uploads); rate limiting (bots draining AI routes overnight).
- Topic-only saves, no detail to verify: @lincolndevine [GATED — "AUDIT"], @qasimbiz [GATED — "SECURITY"], @andixiongai ("how to not get hacked").

### 10.3 Older security saves that still hold

- **Tokens must expire** (@baroobi.inc, 2025-12-23, "Vulnerabilities in your vibe-coded app Pt.3"): login tokens that never expire give permanent access from one stolen token. Check expiry and refresh — better-auth handles this. Same post: code that works for 10 users can crash at 1,000–10,000 and be exploited, and code that works on localhost breaks in the cloud because the AI does not know the hosting environment. Add hosting context to prompts, review performance-critical paths, and **test the deployed build, not just localhost**.
- **GitHub "dorks"** (@ariacodez, 2025-10-29, educational, "DO NOT ABUSE THIS"): hackers search public GitHub code for exposed OpenAI keys, GitHub tokens, Slack tokens, and Google API keys — one search in the video returned 46,600+ matching files (Oct 2025 figure). Defensive use only: run gitleaks, search your own repos, never push `.env`, and rotate any key that was ever committed. Never use this technique against other people's repos.
- **PHP check** (@malikontechnology, 2026-10-21): only if the stack is PHP — ask the agent its PHP version; patch versions named were 8.5.12 / 8.4.27 / 8.3.36 / 8.2.35, and versions below 8.2 get no patch. Node and Python stacks are probably fine.

### 10.4 gitleaks and Strix

- **gitleaks** — ranked #1 in the 20-repo list. Finds leaked API keys. **Instructions for Claude:** run it before the demo and before every commit. Any hit means: remove the key, rotate it, and check git history — deleting the file alone does not remove it from history.
- **Strix** (@stevengrant_tech, 2026-08-20; @pranavinno, 2026-07-04) [GATED — "STRIX" / "hack"]: an open-source AI pentesting agent that acts like a real hacker against your app and validates findings with proof-of-concepts — "try to break it first". Creator claims: $3 per vulnerability vs $10k–$35k for a human pentest, 96% success, 25k+ stars. Treat the numbers as claims. Use Strix in the last hour only, as a gate, not as a build tool.

### 10.5 Deploying

- **Template stack** (Section 11.1): Next.js + Vercel for the frontend, Fly.io + FastAPI for the backend, Supabase for database and auth.
- **Go Live** (amnesia list) [GATED — "Claude"]: deploys an agent-built product with hosting, database, and auth wired up.
- **coolify** (20-repo list): self-hosting option if Vercel/Fly do not fit.
- Test the deployed URL end-to-end before the demo. Localhost success is not shipping (Section 10.3).
- SEO checklist of 20 places (@joshtheaiguy, 2026-10-03): the list itself was not given. Only if time remains after the gate below.

### 10.6 Pre-demo gate — last hour, in this order

- [ ] Run gitleaks. Rotate anything it finds.
- [ ] Run the 20-check audit (Section 10.2), or the 5-check prompt in Section 12.5 if the guide is gated.
- [ ] Run VibeSec's checks over auth and database access specifically: can one user read another user's data?
- [ ] Check tokens expire and refresh; check no route is unintentionally open.
- [ ] Run Strix if time allows.
- [ ] Run the one E2E test of the core flow, as a user, on the **deployed** build.
- [ ] Run the design audit sequence (Section 9.1, steps 3–7).
- [ ] Dry-run the demo out loud, including explaining the code (Section 11.4).

---

## 11. Hackathon playbook

### 11.1 Arrive with a template

@thad.codes, 2026-10-30: arrive with frontend, backend, auth, and database already wired together, with API keys and environment variables swappable. His stack: Next.js + Vercel, Fly.io + FastAPI, Supabase. He calls it a "slightly unethical tip" and claims wins or placements at every hackathon he entered including Harvard and SDSU (creator claim). No link was given — build your own template from this recipe before the event, and **check today's rules first**: a pre-built template may not be allowed.

For a mobile project, the equivalent is the Expo skills plus the mobile MCP four: Expo, Supabase, RevenueCat (payments), App Store Connect (submission) — @agentic.james, 2025-12-31.

### 11.2 Pick a track and tailor to the sponsor

Older save, principle still valid (@vardy.mov, 2025-06-07): pick a hackathon track, generate ideas tailored to that track's sponsor, and build what that track's judge values. Give the AI full context before asking it to code — in today's workflow that context step **is** the grill and SPEC.md.

[SUPERSEDED tools in that post] ChatGPT for ideas, Cursor as the editor, and an Express backend generated via Cursor. Use Claude Code with Opus 5.5 / Haiku 5.5 and the template stack instead. The post's "literally guarantees your idea is good" and $7,000 prize figures are creator hype.

[SUPERSEDED entirely] "Tools to win any hackathon in 2025" (@kaizencodes_, 2025-01-05): v0 for UI, Claude AI for debugging, Warp, Cursor. A January 2025 list — do not use it as the 2026 stack. Only its shape survives: a UI tool, an AI coder, a terminal, an editor — and Sections 2 and 9 fill those slots with current tools.

### 11.3 Team coordination

- Folder split and agent board: Section 3.2 and the template in Section 12.4.
- One human owns the demo and the judges. Everyone else owns a directory.
- Agents post to the board; humans read the board. Do not relay agent messages by hand.
- Keep the shared vocabulary in SPEC.md (Grill with Docs) so backend, database, and UI never diverge on naming.

### 11.4 Demo, pitch, and judging

- **Be able to explain the code.** Judges will ask. Use VibeWise during the build (Section 7.6) and `/teach` (Matt Pocock) before the demo to generate plain-English lessons from your own codebase, e.g. "How a booking moves through the system".
- **Design wins demos.** Do the Section 9 audit early enough to act on it.
- **E2E-test the core flow** on the deployed build (Sections 7.5, 10.6). A live failure costs more than a missing feature.
- The 147-agent pack's business agents (Fractional CTO, AI Citation Strategist, and similar) are for this half of the day — positioning and pitch — not for the build.
- For idea generation from a sponsor task: one save (@bashi_fuirkashi) [GATED — "BETTER"] suggests taking a Hugging Face task, screenshotting it, and asking Claude for project ideas. A lead, not a verified method.
- Past events in the saves — do not chase: a free Vibe Coding AI Hackathon on Mar 6–8, 2026 (@sabrina_ramonov, past), and summer 2025 contests from HP/NVIDIA, OpenAI, and Zindi (@mar_antaya, 2025-06-06, past).

### 11.5 Cost control for the day

Follow Section 8.5. The short version: Opus plans and checks, Haiku executes, Ponytail on, compact at 40%, knowledge graph instead of re-reading files, Jev in front of repetitive decisions. If you hit a limit mid-day, the rescue rule in Section 8.5 applies — do not switch to an unverified free-routing tool.

---

## 12. Copy-paste templates for Claude

Templates marked [SYNTHESIS] are built from the saves' rules. They are not verbatim files from any post.

### 12.1 CLAUDE.md [SYNTHESIS — paraphrased Karpathy rules, Section 4.1]

```markdown
# CLAUDE.md — project rules (read at every session start)

## Behaviour
- Never make a silent assumption. If something is unclear, ask before coding.
- Make surgical changes only. Touch only what the ticket requires.
- Never delete code, comments, or tests that were not part of the ticket.
- Never over-engineer. The simplest design that passes the gate wins.
- Never invent an API, library function, or config value. Check, or mark it TBD.
- Never mark a ticket complete without running its gate command and pasting the exit code into GATE.md.
- Weigh trade-offs out loud before a non-trivial decision.
- If a fix breaks working code, stop, say so, and revert before trying another approach.
- Change SPEC.md first, then the code. Never the other way round.
- Label anything unverified as unverified. Do not speculate in code or copy.

## Project
- Stack: [fill in — e.g. Next.js + Vercel / FastAPI on Fly.io / Supabase]
- Design: one accent colour, no purple gradients, no generic templates, no invented stats or testimonials. Tap targets at least 44px.
- Security: never commit a real key. Real values live in .env only, and .env is gitignored.

## Learned rules
- (Claude: append every correction the user makes here, dated, one line each.)
```

### 12.2 SPEC.md [SYNTHESIS — from the grill workflow, Section 7.1]

```markdown
# SPEC.md — [project name], hackathon 2026-11-07

## Who is this for?
## What breaks if it's wrong?
## What does done look like? (acceptance criteria per feature)
## Naming rules (one shared reference — DB, backend and UI use the exact same words)
## Tickets
- T1: ...
- T2: ...
## Decisions log (change this file first, then the code)
```

### 12.3 GATE.md [SYNTHESIS — the unlazy acceptance ledger, Section 7.3]

```markdown
# GATE.md — acceptance ledger (unlazy)

## T1 — [ticket name]
- Gate command: [exact command]
- Expected output: [exact expected result]
- Actual output / exit code: [paste after running]

## Rule: a ticket is NOT done until its gate command has run and its expected output is pasted here.
```

### 12.4 Agent board — pinned GitHub issue [SYNTHESIS — applying Section 3.2]

```markdown
# Agent board — [project name] — PINNED

## Who owns what
- [name / @username] — [directory] — [current ticket]

## Status
- [agent]: [ticket] — [state] — [gate result]

## Blockers
- [what is blocked, who can unblock it, tagged with their @username]
```

### 12.5 Five-check security prompt [from @baroobi.inc, 2026-10-11 — the only printed check list]

```text
Audit this app for these five things and report each as PASS or FAIL with the file and line:
1. Exposed secrets — check .env handling and the full git history for committed keys.
2. Auth — weak passwords, sessions or tokens that never expire, routes that should be protected but are open.
3. Database access — can one signed-in user read or change another user's data?
4. Input validation — SQL injection, XSS, and unsafe file uploads.
5. Rate limiting — can bots drain the AI/API routes overnight with no limit?
Do not fix anything yet. Report first.
```

### 12.6 Design prompt [SYNTHESIS — applying Sections 9.1, 9.2, 9.4]

```text
Audit this screen with the Apple Design skill and Impeccable.
1. Find three things to fix. Explain each before changing anything.
2. Check the screen against the 20 UX Laws. List every violation and the law it breaks.
3. Fix tap targets under 44px first.
4. Delete any statistic, testimonial, or customer count that is not in SPEC.md.
5. Enforce: one accent colour, no purple gradients, no generic template layout.
6. Open the result in Chrome DevTools MCP and check it yourself before reporting back.
Report: what changed, why, and what you verified in the browser.
```

---

## 13. What not to do

- Do not install everything. One main pack, plus one skill per real problem (Sections 5.5, 6).
- Do not trust "done" without a gate command, an exit code, and output (Sections 6, 7.3).
- Do not one-shot a whole app from a single mega prompt. [SUPERSEDED — @agentic.james, 2025-12-23] Keep his Supabase and Playwright MCPs and his test-after-each-step rule; the grill-to-tickets workflow replaces the prompt.
- Do not ask vague design prompts ("make it premium"). Bring mood, references, and hierarchy (Section 7.7).
- Do not let Claude invent testimonials, stats, or APIs (Sections 4.1, 9.6, 12.1).
- Do not install a skill that reads `~/.env`, harvests session cookies, or POSTs data out (Section 5.1).
- Do not use jailbreak or free-rerouting tricks for Claude Code (Section 8.5).
- Do not shout at the model or use "memory hack" prompts (Section 4.3).
- Do not re-explain the project every session — that is what CLAUDE.md, SPEC.md, and PROGRESS.md are for (Section 4.2).
- Do not merge borrowed code straight to main — disposable branch first (Section 3.3).
- Do not run an agent inside your real logged-in browser sessions without understanding the risk (BrowserSkill, Section 8.3).
- Do not copy leaked system prompts into your product, or present them as your own work. Three older saves (@nick_saraev, Jul/Sep/Dec 2025) point to a repo of leaked prompts for Cursor, Claude Code, v0, and others, with conflicting line counts (10,000 vs 6,500 lines — creator claims). Study prompt *structure* only, if at all; 2025 leaks are stale for current models. [GATED — "PROMPTS"]
- Do not switch your primary stack mid-day to an alternative from the older saves: Google Antigravity (Gemini 3.0 era, "free right now" was Dec 2025), Kimi K2 / OK Computer, MiniMax M2.1 ("beats Claude 4.5 Sonnet" — superseded comparison), Chef by Convex, a new LangChain/LangGraph/Mastra/Agno/AutoGen/CrewAI project, or a local LLaMA 3.3 70B setup. Each is dated in the older sweep and none replaces Claude Code with Opus 5.5 / Haiku 5.5 today. The *patterns* some of them show — plan, code, test as separate agent roles; wireframe-to-UI — are already covered by the current workflow.
- Do not start a course on hackathon day. Anthropic Skilljar, claude.com/resources/use-cases, Maven's agentic-AI and AI-engineer roadmaps, Google Skills, and the 342-hour course repo are learning resources for after today. The one piece to use now from the roadmaps (@mavenhq, Jan 2026) is **evals**: a fixed set of tasks the app must pass, re-checked after every change — which is exactly what GATE.md is.
- Do not chase past events (Section 11.4) or present Claude 2, Claude 3, or any pre-5.5 model claim as current.

## 14. Gated links lookup — comment words from the posts

No DM links were available and none are invented. To get a gated item: open the post, comment the word, and use the link the creator sends. Verify and scan anything you receive before installing it.

| Comment word | Item | Creator |
|---|---|---|
| HACK | Agent board script | @azizbuildss |
| PROMPT | Opus plans / Haiku executes setup | @murphmaxxing |
| PATCH | 20-check security guide | @murphmaxxing |
| REPO | 20-repo shipping list; also Graphify | @murphmaxxing; @duncanrogoff |
| REPOS | 40-repo and 5-repo roundups; Agent Reach; Caveman | @lukegarvai; @johannships |
| LIST | 5 long-run Claude Code commands | @j.t.rivard |
| SETUP / ECC | Everything Claude Code | @kayvon.ai |
| CLAUDE | 9-repo amnesia list; OmniRoute / AgentSkills; hidden commands | @charlieautomates; @petergriffin.ai |
| GRILL / AGENTIC | Matt Pocock skills guide | @cristiancruz |
| APPLE / REPO | Apple Design guide | @liamjohnston.ai |
| SCAN | SkillSpector install and setup | @liamjohnston.ai |
| DESIGN | Impeccable / shadcn / DevTools stack | @markpyvo |
| APP | Apple's free design resources | @keeran.ai |
| Expo | Expo skills guide | @guhmp4 |
| UX | 20 UX Laws prompt | @run.curl.exe |
| KITS / UNSLOP | Kitchen UI library / UNSlop | @lukegarvai / @griffinwooldridge1 |
| BOUNTY | VibeSec | @baroobi.inc |
| AUDIT | 5 security checks; also @lincolndevine's audit | @baroobi.inc |
| SECURITY | Security audit post | @qasimbiz |
| STRIX / hack | Strix pentesting agent | @stevengrant_tech / @pranavinno |
| KARPATHY | Karpathy CLAUDE.md file | @pranavinno |
| WORKTREE / GATES | unlazy guide and gate files | @cooper.simson |
| PLAYBOOK / LEAN | 4-prompt folder playbook / delete-most-skills post | @cooper.simson |
| JEV | Jev decision model | @baroobi.inc |
| FLOW | Ruflo, 60 agents | @kayvon.ai |
| AGENCY | 147 specialist agents | (pack post, 2026-05-16) |
| STUDIO | Claude Code Game Studios | (pack post) |
| REA | Reverse Engineer Anything | @j.t.rivard |
| APPS | CLI-Anything | @buildwithneej |
| tool | harness2 | @keeran.ai |
| skills | Skill-finder skill | @benkimball.ai |
| SCRAPE / DATA | Scrapling MCP | (scraping post) |
| HERMES | Hermes 17-skill list | (Hermes post) |
| SCRATCH | 342-hour AI course repo | (course post) |
| prompt | Technical co-founder prompt | @dubibubiii |
| claude | Claude-Mem and free Claude learning sites | @digitalsamaritan |
| agent | Seven pillars guides and agent templates | @chase.h.ai |
| AI | 21st.dev link; also Google's 12 free tools list | @itsjamesmwild |
| Website | Animations library | @reach.webs |
| PROMPTS | Leaked system prompts repo — study only, Section 13 | @nick_saraev |
| GOOGLE | Google Antigravity — not recommended, Section 13 | @nick_saraev |
| API | Free API key sites list (verify current tiers) | @nick_saraev |
| SKILLS | Google Skills courses — after the hackathon | @nick_saraev |
| build | Chef by Convex — alternative, Section 13 | @fullstackshrek |
| BETTER | Hugging Face task to project ideas | @bashi_fuirkashi |

## 15. What could not be verified

Be honest with the user about everything in this section. None of it should be presented as fact or as an available download.

- All gated DM links in Section 14, and therefore the exact contents of: the Karpathy CLAUDE.md file, the 20 UX Laws prompt text, the 20-check PATCH guide, the audit prompts beyond the printed 5-check list, the 19-check list, the agent-board script, the PLAYBOOK folder setup, SkillSpector's install steps, REA's setup command, the OmniRoute / Graphify / AgentSkills installs, the "superpowers" repo slug, the 60+ hidden commands list, the "BANANA" resource list, and the unnamed Microsoft file-to-markdown tool and unnamed "number 1 repo" for tokens.
- The exact ECC marketplace slug — posts show `/plugin marketplace add` and `/plugin install ECC` without spelling the slug out.
- BrowserSkill's full install URL — truncated in its caption.
- VibeWise's marketplace handle — rendered two ways in the saves. The GitHub repo `github.com/nykoo11/vibe-wise` is the grounded reference.
- Claude-Mem's repo and install — named only, behind a comment gate (Section 4.4).
- All star counts, prices, cost multipliers, accuracy figures, benchmark percentages, and user counts in the videos. Several conflict between saves; the unresolved ones are flagged in the Recency section.
- Whether the agent-board workflow and the pre-built template comply with today's hackathon rules. Check the rules before using either.
- UNSlop, the "3 proven ways" UI post, the motion-design prompt, "Build these 3 things", and the Andixiongai and Qasimbiz security posts — topic only in their captions, no verifiable detail.
- The roughly 478 reported saves that pagination never returned (see Coverage). Their contents are unknown, not merely unscanned.
- Two older posts returned empty media summaries (@meech_ward, 2025-11-11; @emrcodes, 2025-11-27). Only their captions were usable, and neither adds a setup step.

## 16. Key sources

Recent sweep — the posts this document leans on most:

- Opus plans / Haiku executes — @murphmaxxing, 2026-11-06, https://www.instagram.com/reel/DfaLyn7Rorf/
- Agent board — @azizbuildss, 2026-11-01, https://www.instagram.com/reel/DfOLEP6v3AR/
- Amnesia fixes — @charlieautomates, 2026-10-01, https://www.instagram.com/reel/Dd9XQ-QzIIf/
- 20 niche repos — @murphmaxxing, https://www.instagram.com/reel/DfQCcgByfcO/
- Untangle Codebase — @voxaris_ai, https://www.instagram.com/reel/DfYbglQxEbl/
- 20 UX Laws — @run.curl.exe, https://www.instagram.com/reel/DfF1YIYDRVu/
- Dimensional Drawing — @aiwithbibek, https://www.instagram.com/reel/DfB2YuGtVTJ/
- Karpathy CLAUDE.md — @pranavinno, https://www.instagram.com/reel/DZIYih_hExu/
- unlazy (partial compliance) — @cooper.simson, https://www.instagram.com/reel/DcVSyBvvthb/
- SkillSpector — @liamjohnston.ai, https://www.instagram.com/reel/Dc1oIn8CyDW/
- VibeWise — @remoteree, https://www.instagram.com/reel/DeM2KoJFDhf/
- REA — @j.t.rivard, https://www.instagram.com/reel/DeuyBI0xRCa/
- xtool — @voxaris_ai, https://www.instagram.com/reel/DecztN5xvxV/
- CLI-Anything — @buildwithneej, https://www.instagram.com/reel/Dd9NdbFxJLo/
- Agent Reach — @johannships, https://www.instagram.com/reel/Dd7IjLuRgI-/
- harness2 — @keeran.ai, https://www.instagram.com/reel/Det7c9Ggnys/
- Boris Cherny prompting — @aititans0, https://www.instagram.com/reel/DZAjG3kAqOT/

Older sweep — the posts that added something the recent sweep did not:

- Claude-Mem and free Claude learning sites — @digitalsamaritan, 2026-01-06, https://www.instagram.com/reel/DTMgguHkY4H/
- Seven pillars — @chase.h.ai, 2026-01-05, https://www.instagram.com/reel/DTJemnhDRfu/
- Mobile MCP four — @agentic.james, 2025-12-31, https://www.instagram.com/reel/DS8uHm-jROC/
- Supabase and Playwright MCPs — @agentic.james, 2025-12-23, https://www.instagram.com/reel/DSn6rrtAUMM/
- Playwright MCP — @nick_saraev, 2025-09-30, https://www.instagram.com/reel/DPQPRQej04Y/
- 21st.dev prompt-copying — @itsjamesmwild, 2026-01-03, https://www.instagram.com/reel/DTC-YZ5jC38/
- Vibe-coded app vulnerabilities Pt.3 — @baroobi.inc, 2025-12-23, https://www.instagram.com/reel/DSohA4iDGPJ/
- GitHub dorks, defensive use — @ariacodez, 2025-10-29, https://www.instagram.com/reel/DQajeFgiTxj/
- Track and sponsor tailoring — @vardy.mov, 2025-06-07, https://www.instagram.com/reel/DKnXexYxr1g/
