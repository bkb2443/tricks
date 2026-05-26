---
description: >
  Spec out a GitHub issue and mark it `ready` for implementation.
  Use when: the user wants to plan or spec an issue that is not yet ready,
  pulling non-ready issues and choosing the next most important one to spec,
  iterating through the backlog to plan issues one by one, or adding a brand-new
  feature idea to the backlog.
  Triggers on: "plan the issue", "plan issues", "plan an issue", "spec out issues",
  "plan the next issue", "issues not in a ready state", "make issue ready",
  "choose the next issue", "pick the next issue to plan", "lets start planning it".
---

Guide the user through specifying a feature, then create or update a GitHub issue with a full spec. No implementation details — only requirements and acceptance criteria.

## Process

Ask one question at a time. Wait for each answer before continuing.

**Step 1 — Identify the issue to spec:**

If the user has named a specific issue or feature idea: go to Step 2.

If the user wants to choose from existing non-ready issues:
1. Run: `gh issue list --state open --json number,title,labels,body` and filter to issues without the `ready` label.
2. Pick the most impactful one (user-facing features > dev infrastructure > new games), state your reasoning, and confirm with the user.
3. Pull the existing issue body as the starting point for the spec.

If the user has a brand-new idea with no existing issue: ask "What feature would you like to add? Describe it in a sentence or two."

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

**You decide** — not the user. Read the approved spec and ask yourself: "Could any requirement or acceptance criterion be interpreted two different ways? Is there any decision an implementer would need to make that isn't answered by the spec?" If any answer is yes, the issue is not ready.

**If YES (spec is complete and unambiguous):**

- **Existing issue** — edit the body and add the `ready` label:
```bash
gh issue edit [issue-number] --body "[full spec body]"
gh issue edit [issue-number] --add-label "ready"
```
- **New issue** — create with `ready` label:
```bash
gh issue create \
  --title "[feature title]" \
  --body "[full spec body]" \
  --label "ready"
```

**If NO (spec has gaps):**

- **Existing issue** — edit the body, leave unlabeled, add a comment:
```bash
gh issue edit [issue-number] --body "[full spec body]"
gh issue comment [issue-number] \
  --body "Not labeled ready — needs clarification before implementation can begin:

- [Gap 1: specific ambiguity]
- [Gap 2: specific ambiguity]"
```
- **New issue** — create without `ready`, add the same comment.

## Rules

- Do NOT discuss implementation approach, file paths, or technology choices
- Do NOT mention the agent team or how automation works
- Requirements must be testable: "the UI shows X" not "the UI is improved"
- Acceptance criteria must be binary: either it passes or it doesn't
- Out of Scope section is mandatory — make it explicit
