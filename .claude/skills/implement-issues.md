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

Sync local with remote before doing anything:

```bash
git fetch origin
git rebase origin/main
```

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

**Step 2 — Check for existing open PR**

```bash
gh pr list --state open --search "fixes #[number]" --json number,title,isDraft,headRefName,url
```

Also check:
```bash
gh pr list --state open --head "issue-[number]-*" --json number,title,isDraft,headRefName,url
```

**If an open PR exists → go to Step 2a (resume existing PR). Skip Steps 2b–5.**

**If no open PR exists → go to Step 2b (fresh start).**

---

**Step 2a — Resume existing PR (open or draft)**

Check out the branch and sync it:

```bash
git fetch origin
git checkout [headRefName]
git rebase origin/main
```

If rebase has conflicts, resolve them:
- For each conflict, choose the version that best matches the issue spec
- Stage resolved files with `git add`
- Continue: `git rebase --continue`

Push the rebased branch:

```bash
git push --force-with-lease origin [headRefName]
```

Then proceed directly to **Step 6 (QA)**.

After QA + review pass (Step 8a), if the PR is a draft, mark it ready:

```bash
gh pr ready [pr-number]
```

---

**Step 2b — Add `in-progress` label**

```bash
gh issue edit [number] --add-label in-progress
```

---

**Step 3 — Create worktree** *(fresh start only — skip if resuming via Step 2a)*

Use the `superpowers:using-git-worktrees` skill to create an isolated worktree.
Name the branch: `issue-[number]-[kebab-case-title-slug]`

Example: issue #12 "Add trick history panel" → branch `issue-12-trick-history-panel`

---

**Step 4 — Derive plan and test map in context**

Read the issue spec. Determine:
- Which files need to change
- Which agents own those files (see `.claude/agents/` for boundaries)
- Whether changes are independent (parallel dispatch) or dependent (sequential)

**Derive a test map**: for each acceptance criterion in the issue, identify the test layer that verifies it:

| Acceptance criterion | Test layer | What to test |
|---|---|---|
| Server rejects illegal X | Rust unit test | call handler with illegal input, assert Err |
| Protocol message shape | Rust unit + TS type check | new variant serializes/deserializes correctly |
| UI shows Y when server sends Z | Playwright e2e | routeWebSocket → send Z → assert Y visible |
| Store updates when message arrives | Vitest unit | handleUpdate(msg) → assert store.field |
| Game rule: card X is legal/illegal | Rust unit test | legal_plays returns/excludes card |

Include this test map in every agent dispatch prompt. Agents must write the tests alongside implementation.

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
- The test map (from Step 4) — agents must write all tests assigned to their layer
- Any interface contracts already decided (e.g. wire message shape for protocol changes)
- Link to relevant specs in `docs/superpowers/specs/` if applicable

**TDD rule:** agents write tests alongside implementation — not after. A function with no test is incomplete.

---

**Step 6 — Dispatch qa agent**

Prompt: "Run all test suites and report results for the changes made to implement issue #[number]: [title]. Report per the qa agent output contract. Check for coverage gaps (new code with no tests) and write any missing tests before reporting."

**QA passes** if all 5 suites show PASS (cargo test, cargo clippy, npm test:unit, vue-tsc, playwright e2e) and COVERAGE GAPS reports "none". **QA fails** if any suite shows FAIL or if there are untested new code paths.

---

**Step 7 — Dispatch reviewer agent**

Prompt: "Review the changes made to implement issue #[number]: [title]. Run `git diff main...HEAD` and report findings per the reviewer agent output contract."

**Review passes** if no `critical` or `major` findings. Minor findings do not block. Missing tests for new behavior are **major** — treat them as blocking.

---

**Step 8a — On pass (QA passes AND review passes)**

If resuming an existing PR (came through Step 2a):
```bash
# branch already pushed in Step 2a; just mark ready if it was a draft
gh pr ready [pr-number]   # no-op if already ready
gh issue edit [number] --remove-label ready --remove-label in-progress
```

If fresh start (came through Step 2b):
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

If resuming an existing PR (came through Step 2a):
```bash
# branch already pushed; leave as draft (or convert to draft if it wasn't)
gh pr edit [pr-number] --draft --body "$(cat <<'EOF'
Fixes #[number]

Attempted by implement-issues routine. Draft — see failures below.

## Failures

[Paste the full verbatim output from the QA agent and/or reviewer agent — do not paraphrase or summarize]
EOF
)"
gh issue edit [number] --remove-label in-progress
```

If fresh start (came through Step 2b):
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
