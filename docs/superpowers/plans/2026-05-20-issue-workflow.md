# Issue Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a `/plan-issue` skill that turns a feature idea into a GitHub issue with a full spec, and an `/implement-issues` routine that autonomously drains the `ready`-labeled backlog using the agent team.

**Architecture:** Two project-local skill files in `.claude/skills/`. `plan-issue` guides a brainstorming dialogue and calls `gh issue create --label ready`. `implement-issues` is the autonomous loop — fetches ready issues, dispatches rust-server/vue-client/game-rules/qa/reviewer agents per issue, opens PRs, stops on count or time limit. A nightly schedule invokes `/implement-issues` automatically.

**Tech Stack:** Claude Code project-local skills (markdown), `gh` CLI, existing `.claude/agents/` team (rust-server, vue-client, game-rules, qa, reviewer)

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `.claude/skills/plan-issue.md` | Create | Brainstorming dialogue → GitHub issue |
| `.claude/skills/implement-issues.md` | Create | Autonomous issue implementation loop |

GitHub labels `ready` and `in-progress` are created in Task 1. Schedule is registered in Task 4.

---

### Task 1: Create GitHub Labels

**Files:** none (GitHub API via `gh`)

- [ ] **Step 1: Create `ready` label**

```bash
gh label create ready \
  --repo bkb2443/tricks \
  --description "Full spec present, cleared for automation" \
  --color "0E8A16"
```

Expected output: `✓ Label "ready" created`

- [ ] **Step 2: Create `in-progress` label**

```bash
gh label create in-progress \
  --repo bkb2443/tricks \
  --description "Implement-issues routine is actively working on this" \
  --color "E4B429"
```

Expected output: `✓ Label "in-progress" created`

- [ ] **Step 3: Verify both labels exist**

```bash
gh label list --repo bkb2443/tricks | grep -E "ready|in-progress"
```

Expected output (2 lines):
```
in-progress   Implement-issues routine is actively working on this   #E4B429
ready         Full spec present, cleared for automation               #0E8A16
```

---

### Task 2: `plan-issue` Skill

**Files:**
- Create: `.claude/skills/plan-issue.md`

- [ ] **Step 1: Create the skills directory and skill file**

```bash
mkdir -p /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design/.claude/skills
```

Create `.claude/skills/plan-issue.md`:

```markdown
---
description: Plan a new feature and create a GitHub issue with a full spec. Use when you have a feature idea to add to the backlog. The issue will be labeled `ready` so the implement-issues routine can pick it up automatically.
---

Guide the user through specifying a feature, then create a GitHub issue with a full spec. No implementation details — only requirements and acceptance criteria.

## Process

Ask one question at a time. Wait for each answer before continuing.

**Step 1 — Understand the feature:**
Ask: "What feature would you like to add? Describe it in a sentence or two."

**Step 2 — Clarify with follow-up questions (one at a time):**
- Who benefits from this feature and how?
- What specific behaviors must it have? Get concrete — "the player sees X when Y happens"
- What edge cases or error conditions matter?
- What is explicitly out of scope?

Ask as many questions as needed to make requirements testable and unambiguous.

**Step 3 — Draft the issue body:**

```
## Context
[Why this feature is needed; how it fits the platform]

## Requirements
[Specific, testable statements — each one either passes or fails]

## Acceptance Criteria
- [ ] [Concrete, verifiable condition]
- [ ] [Concrete, verifiable condition]

## Out of Scope
[Explicitly what this does NOT include]
```

**Step 4 — Show draft, get approval:**

Show the draft. Ask: "Does this capture what you want? Any changes?"

Revise until approved.

**Step 5 — Assess implementability:**

Before creating the issue, ask yourself: "Can this spec be fully implemented without asking a human any clarifying questions? Are requirements specific? Are acceptance criteria binary? Is scope unambiguous?"

**If YES** — create with `ready` label:
```bash
gh issue create \
  --title "[feature title]" \
  --body "[full spec body]" \
  --label "ready"
```

**If NO** — create without `ready`, add a comment:
```bash
gh issue create \
  --title "[feature title]" \
  --body "[full spec body]"
```
Then comment on the created issue:
```bash
gh issue comment [issue-number] \
  --body "Not labeled ready — needs clarification before implementation: [list exactly what's ambiguous or missing]"
```

## Rules

- Do NOT discuss implementation approach, file paths, or technology choices
- Do NOT mention the agent team or how automation works
- Requirements must be testable: "the UI shows X" not "the UI is improved"
- Acceptance criteria must be binary: either it passes or it doesn't
- Out of Scope section is mandatory — make it explicit
```

- [ ] **Step 2: Verify the skill file has a description in frontmatter**

```bash
grep "^description:" \
  /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design/.claude/skills/plan-issue.md
```

Expected output:
```
description: Plan a new feature and create a GitHub issue with a full spec. Use when you have a feature idea to add to the backlog. The issue will be labeled `ready` so the implement-issues routine can pick it up automatically.
```

- [ ] **Step 3: Commit**

```bash
git -C /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design \
  add .claude/skills/plan-issue.md && \
git -C /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design \
  commit -m "feat(skills): add plan-issue skill"
```

---

### Task 3: `implement-issues` Skill

**Files:**
- Create: `.claude/skills/implement-issues.md`

- [ ] **Step 1: Create the skill file**

Create `.claude/skills/implement-issues.md`:

```markdown
---
description: Fetch open GitHub issues labeled `ready` and implement them using the agent team. Runs until no implementable issues remain, MAX_ISSUES_PER_RUN is hit, or MAX_RUN_MINUTES elapses. Invoked manually or nightly via schedule.
---

Autonomously implement GitHub issues labeled `ready` using the agent team.

## Configuration

Edit these values to tune behavior:

```
MAX_ISSUES_PER_RUN=2
MAX_RUN_MINUTES=90
```

## Process

### Setup

Record start time and fetch issues:

```bash
START_EPOCH=$(date +%s)
gh issue list \
  --label ready \
  --state open \
  --json number,title,body \
  --limit 20
```

Set `issues_completed=0`.

### Per-Issue Loop

**Before starting each issue, check all stopping conditions:**

1. Compute elapsed: `elapsed=$(( ($(date +%s) - START_EPOCH) / 60 ))`
2. Stop if `elapsed >= MAX_RUN_MINUTES`
3. Stop if `issues_completed >= MAX_ISSUES_PER_RUN`
4. Stop if no more issues in the list

---

**Step 1 — Assess implementability**

Read the issue title and body carefully. Ask yourself:

> "Can this issue be fully implemented — including all edge cases and acceptance criteria — without asking a human any clarifying questions? Are the requirements specific and testable? Is scope clear? Are acceptance criteria binary?"

If **NO**:
```bash
gh issue comment [number] \
  --body "Skipped by implement-issues: not enough information to implement without human clarification. Missing: [list exactly what's ambiguous or missing]"
```
Move to the next issue. Do NOT add `in-progress` label.

If **YES**: proceed to Step 2.

---

**Step 2 — Add `in-progress` label**

```bash
gh issue edit [number] --add-label in-progress
```

---

**Step 3 — Create worktree**

Use the `superpowers:using-git-worktrees` skill to create an isolated worktree.
Name the branch: `issue-[number]-[kebab-case-title-slug]`

Example: issue #12 "Add trick history panel" → branch `issue-12-trick-history-panel`

---

**Step 4 — Derive plan in context**

Read the issue spec. Determine:
- Which files need to change
- Which agents own those files (see `.claude/agents/` for boundaries)
- Whether changes are independent (parallel dispatch) or dependent (sequential)

Do NOT write a plan file to disk. Hold the plan in context for this issue only.

Protocol changes (wire types touching both `server/src/engine/state.rs` and `client/src/engine/types.ts`) are always sequential: rust-server defines Rust types first, vue-client mirrors to TypeScript second.

---

**Step 5 — Dispatch domain agents**

| Files changed | Agent |
|---|---|
| `server/src/engine/`, `lobby/`, `ws/`, `main.rs`, `config.rs` | rust-server |
| `server/src/games/<name>/`, `bot.rs` | game-rules |
| `client/src/` | vue-client |
| Both `server/src/engine/state.rs` + `client/src/engine/types.ts` | rust-server first → vue-client second |

Pass each agent:
- The full issue spec (title + body)
- Exact list of files to touch
- Any interface contracts already decided (e.g. wire message shape for protocol changes)
- Link to relevant specs in `docs/superpowers/specs/` if applicable

---

**Step 6 — Dispatch qa agent**

Prompt: "Run all test suites and report results for the changes made to implement issue #[number]: [title]. Report per the qa agent output contract."

---

**Step 7 — Dispatch reviewer agent**

Prompt: "Review the changes made to implement issue #[number]: [title]. Run `git diff main...HEAD` and report findings per the reviewer agent output contract."

---

**Step 8a — On pass (QA + review both approve)**

```bash
git push -u origin issue-[number]-[slug]

gh pr create \
  --title "[issue title]" \
  --body "$(cat <<'EOF'
Fixes #[number]

Implemented via implement-issues routine.
EOF
)"

gh issue edit [number] \
  --remove-label in-progress \
  --remove-label ready
```

Increment `issues_completed`.

---

**Step 8b — On fail (QA or review found blocking issues)**

```bash
git push -u origin issue-[number]-[slug]

gh pr create \
  --title "[issue title]" \
  --draft \
  --body "$(cat <<'EOF'
Fixes #[number]

Attempted by implement-issues routine. Draft — see failures below.

## Failures

[document exact QA failures and/or reviewer findings]
EOF
)"

gh issue edit [number] --remove-label in-progress
```

Leave `ready` label. Issue is eligible for next run after failures are investigated.

Increment `issues_completed`.

---

### End-of-Run Report

Always output this summary when the loop ends:

```
IMPLEMENT-ISSUES COMPLETE
=========================
Duration:  X min
Attempted: N issues

Completed (PR opened):
  #[number] [title] → [PR URL]

Skipped (not implementable):
  #[number] [title] — [reason]

Failed (draft PR):
  #[number] [title] → [draft PR URL]

Stopped because: [no more ready issues | MAX_ISSUES_PER_RUN reached | MAX_RUN_MINUTES elapsed]
```
```

- [ ] **Step 2: Verify the skill file has a description in frontmatter**

```bash
grep "^description:" \
  /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design/.claude/skills/implement-issues.md
```

Expected output:
```
description: Fetch open GitHub issues labeled `ready` and implement them using the agent team. Runs until no implementable issues remain, MAX_ISSUES_PER_RUN is hit, or MAX_RUN_MINUTES elapses. Invoked manually or nightly via schedule.
```

- [ ] **Step 3: Commit**

```bash
git -C /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design \
  add .claude/skills/implement-issues.md && \
git -C /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design \
  commit -m "feat(skills): add implement-issues skill"
```

---

### Task 4: Register Nightly Schedule

**Files:** none (schedule registered via Claude Code `schedule` skill)

This task cannot be automated — it requires the user to invoke the `schedule` skill interactively. The schedule runs `/implement-issues` nightly.

- [ ] **Step 1: Invoke the schedule skill**

In a Claude Code session in the `tricks` repo, run:

```
/schedule
```

When prompted, configure:
- **Name:** `implement-issues-nightly`
- **Schedule:** `0 2 * * *` (daily at 2am)
- **Prompt:** `Run /implement-issues in the tricks repository`
- **Working directory:** `/Users/bkb2443/Git/tricks`

- [ ] **Step 2: Verify schedule is registered**

```
/schedule list
```

Expected: `implement-issues-nightly` appears with cron `0 2 * * *`.

- [ ] **Step 3: Smoke test manual invocation**

Confirm `/implement-issues` runs without errors when no `ready` issues exist (should output the end-of-run report with "Attempted: 0 issues").

- [ ] **Step 4: Commit plan notes**

```bash
git -C /Users/bkb2443/Git/tricks/.claude/worktrees/agent-team-design \
  commit --allow-empty -m "chore: register implement-issues nightly schedule"
```
