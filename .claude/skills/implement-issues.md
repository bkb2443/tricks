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

If the issue list is empty, go directly to the End-of-Run Report.

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

**QA passes** if all 4 suites show PASS. **QA fails** if any suite shows FAIL.

---

**Step 7 — Dispatch reviewer agent**

Prompt: "Review the changes made to implement issue #[number]: [title]. Run `git diff main...HEAD` and report findings per the reviewer agent output contract."

**Review passes** if no `critical` or `major` findings. Minor findings do not block. If the reviewer reports only minor findings or none, treat as pass.

---

**Step 8a — On pass (QA passes AND review passes)**

```bash
git push -u origin issue-[number]-[slug]

gh pr create \
  --title "[issue title]" \
  --body "$(cat <<'EOF'
Fixes #[number]

Implemented via implement-issues routine.
EOF
)"

gh issue edit [number] --remove-label ready --remove-label in-progress
```

Increment `issues_completed`.

---

**Step 8b — On fail (QA fails OR review has critical/major findings)**

```bash
git push -u origin issue-[number]-[slug]

gh pr create \
  --title "[issue title]" \
  --draft \
  --body "$(cat <<'EOF'
Fixes #[number]

Attempted by implement-issues routine. Draft — see failures below.

## Failures

[Paste the full verbatim output from the QA agent and/or reviewer agent — do not paraphrase or summarize]
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
