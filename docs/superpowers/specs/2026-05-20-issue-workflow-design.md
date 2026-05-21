# Issue Workflow Design

**Date:** 2026-05-20
**Status:** Approved

## Goal

Two-component workflow: a `/plan-issue` skill for adding well-specified features to the GitHub backlog, and an `/implement-issues` routine that autonomously drains that backlog using the agent team.

---

## Component 1: `plan-issue` Skill

**Location:** `.claude/skills/plan-issue/skill.md`
**Invocation:** `/plan-issue` (project-local slash command)

### Behavior

Runs a brainstorming-style dialogue — one question at a time — to understand a feature. Outputs a GitHub issue with a full spec. No file written to disk. No implementation plan. Terminus is `gh issue create --label ready`.

### Issue Body Structure

```markdown
## Context
[Why this feature is needed; how it fits the broader platform]

## Requirements
[What the feature must do — specific, testable statements]

## Acceptance Criteria
- [ ] ...
- [ ] ...

## Out of Scope
[Explicitly what this does NOT include]
```

### Scope Guards

- Does NOT ask about implementation approach, file paths, or tech choices — those belong to the implementation plan the routine derives
- If the dialogue reveals the request needs human input before it can be implemented, the skill creates the issue WITHOUT `ready` and adds a note explaining what's missing

---

## Component 2: `implement-issues` Routine

**Location:** `.claude/skills/implement-issues/skill.md`
**Invocation:** `/implement-issues` (manual) or nightly cron via `schedule` skill (`0 2 * * *`)

### Configuration

Editable at the top of the skill file:

```
MAX_ISSUES_PER_RUN: 2
MAX_RUN_MINUTES: 90
```

### Per-Run Flow

```
1. Fetch open issues with label `ready`
   gh issue list --label ready --state open --json number,title,body

2. For each issue (checked before starting each):
   a. Stop if elapsed >= MAX_RUN_MINUTES
   b. Stop if issues_completed >= MAX_ISSUES_PER_RUN
   c. Stop if no more ready issues

3. Per-issue execution:
   a. Assess implementability (LLM reads title + body):
      - Prompt: "Can this be fully implemented without asking a human any
        clarifying questions? Answer YES or NO, then explain briefly."
      - If NO: comment on issue with what's missing, skip, move to next
   b. Add label `in-progress`
   c. Create worktree for this issue
   d. Orchestrator reads spec, determines which files to touch and in what order — no plan file written to disk; the plan lives in the orchestrator's context for this issue only
   e. Dispatch domain agents based on what the issue touches:
      - Server infrastructure → rust-server agent
      - Game rules / bot AI → game-rules agent
      - Frontend → vue-client agent
      - Protocol changes (both sides) → rust-server first, then vue-client
   f. Dispatch qa agent
   g. Dispatch reviewer agent
   h. On pass:
      - Push branch
      - Open PR with body "Fixes #N" (auto-closes issue on merge)
      - Remove `in-progress` and `ready` labels
   i. On fail:
      - Push branch as draft PR with failure details documented
      - Remove `in-progress` label, leave `ready` (eligible for next run)

4. End-of-run report:
   - Issues attempted
   - PRs opened (links)
   - Issues skipped (with reasons)
```

### Stopping Conditions

Checked **before** starting each new issue — never mid-issue:

| Condition | Action |
|---|---|
| `elapsed >= MAX_RUN_MINUTES` | Stop, report |
| `issues_completed >= MAX_ISSUES_PER_RUN` | Stop, report |
| No more `ready` issues | Stop, report |

---

## Label Lifecycle

```
/plan-issue creates issue
    └─► ready

routine picks up
    └─► ready + in-progress

QA + review pass → PR opened
    └─► [both labels removed] → PR merges → issue auto-closes ("Fixes #N")

QA + review fail → draft PR opened
    └─► in-progress removed, ready kept → eligible next run

not implementable
    └─► ready unchanged + comment explaining what's missing
```

## GitHub Labels Required

Create once during setup:

| Label | Color | Purpose |
|---|---|---|
| `ready` | green | Full spec present, cleared for automation |
| `in-progress` | yellow | Routine actively working — prevents duplicate pickup |

---

## Schedule

Registered via `schedule` skill:
- Cron: `0 2 * * *` (nightly at 2am)
- Can also be invoked manually: `/implement-issues`
