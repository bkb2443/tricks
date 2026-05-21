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

**You decide** — not the user. Read the approved spec and ask yourself: "Could any requirement or acceptance criterion be interpreted two different ways? Is there any decision an implementer would need to make that isn't answered by the spec?" If any answer is yes, the issue is not ready.

**If YES (spec is complete and unambiguous)** — create with `ready` label:
```bash
gh issue create \
  --title "[feature title]" \
  --body "[full spec body]" \
  --label "ready"
```

**If NO (spec has gaps)** — create without `ready`, add a comment:
```bash
gh issue create \
  --title "[feature title]" \
  --body "[full spec body]"
```
Then comment on the created issue listing each gap specifically:
```bash
gh issue comment [issue-number] \
  --body "Not labeled ready — needs clarification before implementation can begin:

- [Gap 1: e.g. 'Requirement 2 says \"show an error\" but doesn't specify what the error message should say']
- [Gap 2: e.g. 'Acceptance criteria 3 is ambiguous: does \"player cannot play\" mean the card is greyed out, or removed from hand?']"
```

## Rules

- Do NOT discuss implementation approach, file paths, or technology choices
- Do NOT mention the agent team or how automation works
- Requirements must be testable: "the UI shows X" not "the UI is improved"
- Acceptance criteria must be binary: either it passes or it doesn't
- Out of Scope section is mandatory — make it explicit
