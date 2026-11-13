# Complete walkthrough (document mode): from a document excerpt to generated files

Using the concept + requirements phases of the AI-native SDLC process document (engine repo `idea/00-工作流定义.md`) as input, walk the full flow once. Handling notes for the design and development phases are appended at the end.

## 1. Document excerpt (input)

> ### 2.1 Concept phase (Stages 1.1–1.2)
>
> | Stage | Name | Brief description | Inputs | Outputs |
> |---|---|---|---|---|
> | 1.1 | Intent Capture | Capture the problem, goals, and constraints in plain language, producing an `intent.md` draft | Ideas, tickets, alerts, incidents; existing system context | `intent.md` draft |
> | 1.2 | Intent Review | A human reviews the intent and accepts or rejects; on acceptance a tag is applied | `intent.md` draft | Accepted `intent.md` (tagged); review record |
>
> **Gate**: intent acceptance (GATE-1), reviewed by the Product Owner
>
> ### 2.2 Requirements phase (Stages 2.1–2.2)
>
> | Stage | Name | Brief description | Inputs | Outputs |
> |---|---|---|---|---|
> | 2.1 | Spec Generation | Generate a `spec.md` draft from the intent, doing requirement transformation only | Accepted `intent.md` | `spec.md` draft |
> | 2.2 | Spec Review | A human reviews the requirement specification and accepts or rejects | `spec.md` draft | Accepted `spec.md`; review record |
>
> **Gate**: spec review (GATE-2), Product Owner + Tech Lead

## 2. Extraction list (output of step 1)

- Phases: concept, requirements — two stages each
- intent-capture: inputs = external (ideas/tickets/alerts/incidents); outputs = the intent draft
- intent-review: humans only accept/reject → approval gate candidate
- spec-generation: inputs = the accepted intent; outputs = the spec draft
- spec-review: same as intent-review → approval gate candidate
- The review-record artifact: both reviews claim to produce a "review record" — but an approval gate produces no files → needs a decision
- "On acceptance a tag is applied": a git operation, outside the session artifact contract → body text

## 3. Mapping decisions and assumption list (output of steps 2–3)

**Mapping decisions:**

| Document item | Decision | Basis (mapping-guide §) |
|---|---|---|
| 1.2 Intent Review / 2.2 Spec Review | No stage; hang the approval gates `intent-review` / `spec-review` on the producing stages | §2: a review producing only accept/reject → approval gate |
| GATE-1 / GATE-2 | Gate names to English kebab-case | §2 gate naming convention |
| External inputs such as ideas and tickets | Not written into `inputs`; they go into `task_description` and the body | §4: hard inputs must have a producer |
| The review-record artifact | Omitted | §2: an approval gate produces no files; if the user wants it, the reviewed stage produces it alongside |
| The "per feature" dimension | scope `vars` (mapping: `feature:` required, `constraints: ""` optional default) | §6 variable design |

**Assumption list (submitted for user confirmation):**

1. Agent names derived from the document's roles: `sdlc-intent-capturer` (Originator), `sdlc-po` (Product Owner) — the project's `dmaf/agents/` has no such assets; these are **assets to be created** (deployment-time `dmaf check --harness` will flag them);
2. Skill names derived from the document's "intent capture category / requirement transformation category": `intent-capture-guide` (named in the document), `requirement-transform` (not named; coined) — also to be created;
3. The review-record artifact is omitted; to keep it, add `{feature}-intent-review.md` to the corresponding stage's `outputs`;
4. "On acceptance a tag is applied" is outside the session contract; it goes into the stage body as a human-executed step.

## 4. Generated files (output of step 4)

`dmaf/workflows/stages/concept/intent-capture.md`:

```markdown
---
id: intent-capture
phase: concept
collaboration: single
agents:
  - agent: sdlc-intent-capturer
    skills: [intent-capture-guide]
    inputs: []
    outputs: ["dmaf-output/{SESSION_ID}/intent/{feature}-intent.md"]
    task_description: "Capture in plain language the problem '{feature}' is meant to solve, the expected outcome, hard constraints, and open questions, producing an intent draft (no user stories, no design)"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: intent-review
execution: always
---

# 1.1 Intent Capture

Turn ideas, tickets, alerts, and incidents into a versioned, human-readable, machine-executable intent draft.
Capture intent only: no user stories, no design. Once the draft is done, halt at the human approval gate `intent-review` (GATE-1,
where the Product Owner reviews the problem, goals, constraints, openness, and business value); rejection enters a revision loop automatically.

## Steps

1. Capture the problem definition from the session variables and the raw material provided by the user (ideas, tickets, alerts, incidents).
2. Produce structured output per the intent-capture-guide skill: problem, expected outcome, hard constraints, open questions.
3. Write to the path given in outputs and wait for approval. The git tag after acceptance is executed by a human.
```

`dmaf/workflows/stages/requirement/spec-generation.md`:

```markdown
---
id: spec-generation
phase: requirement
collaboration: single
agents:
  - agent: sdlc-po
    skills: [requirement-transform]
    inputs: ["dmaf-output/{SESSION_ID}/intent/{feature}-intent.md"]
    outputs: ["dmaf-output/{SESSION_ID}/specs/{feature}-spec.md"]
    task_description: "From the accepted intent of '{feature}', do requirement transformation only: user stories, acceptance criteria, NFRs, the UE contract, scope (introduce no design decisions)"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: spec-review
execution: always
---

# 2.1 Spec Generation

Turn the accepted intent into a requirement specification. Read the intent only; do not load design skills;
NFRs are derived by the skill and need human confirmation; the UE contract describes interaction expectations only.
When done, halt at the human approval gate `spec-review` (GATE-2); rejection enters a revision loop automatically.

## Steps

1. Read the accepted intent produced by intent-capture.
2. Produce the spec per the requirement-transform skill: scope, user stories, acceptance criteria, NFRs, the UE contract.
3. Write to the path given in outputs and wait for approval.
```

`dmaf/workflows/scopes/sdlc-standard.md` (excerpted from the larger flow; the example carries only the first two phases):

```markdown
---
name: sdlc-standard
description: "AI-native SDLC: concept → requirements → design → development (TDD)"
vars:
  feature:                # required: must be provided via start --var
  constraints: ""         # optional: inline default, the empty string
completion_template: |
  Concept and requirements phases complete! Feature: {vars.feature}

  Deliverables:
  📌 Accepted intent: {outputs.intent-capture}
  📋 Accepted spec:   {outputs.spec-generation}
include_stages:
  - intent-capture
  - spec-generation
---

# sdlc-standard scope

The first half of the AI-native SDLC: intent capture (human approval gate intent-review) → spec generation (human approval gate spec-review).
This example carries only the concept and requirements phases; after the design/development phases are plugged in, add their stages into include_stages
in execution order (producers of hard inputs precede their consumers).
```

## 5. Handling notes for the design/development phases (the rest of the same document)

- **3.3 detailed design ∥ 3.4 high-fidelity UE**: no data dependency between the two — just place them adjacently in `include_stages` in document order (the engine runs one stage at a time; no stage-level parallelism); if they truly must be produced simultaneously, merge them into one `collaboration: parallel` stage (sharing the approval gate). Both come after the architecture stage (expressed by sequence position; no declaration needed).
- **3.1 the "is architecture design needed" internal judgment**: stays out of the structure; `outputs` promises only the impact statement, with the hld/ADR written on demand per the body text (mapping-guide §3 strategy 1); downstream soft-references the hld/ADR (`required: false`).
- **3.5 conflict adjudication and fixes, 3.6 design review finalization**: they produce new artifacts → independent stages, not folded into approval gates; if 3.6 also needs human finalization, hang a gate on it.
- **4.4 the TDD loop**: Red/Green/Refactor is intra-stage flow → body Steps; "retry 3 times" → `retry.max_attempts: 3`.
- **4.2 Plan Review**: produces "the accepted plan.md" — the same artifact as 4.1 → an approval gate; the "high-risk approval record" is handled per the assumption list.
