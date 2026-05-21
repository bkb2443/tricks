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
