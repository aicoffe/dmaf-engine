# Real definition examples

The definitions below are taken from the official SDLC samples (`dmaf/workflows/stages/`, `dmaf/workflows/scopes/`; excerpted and tidied — copy, rename, and adapt freely). Each example demonstrates one typical shape.

The samples are **task-driven + required topic**: the original requirement text is injected via `start --task` into the reserved variable `TASK` and carried only by the first stage; the business short name `topic` (required in scope `vars`) serves only as the artifact file-name prefix; artifact paths use `dmaf-output/{SESSION_ID}/<type dir>/` (session buckets isolate re-runs from overwriting each other).

## Example 0: intent/admission stage (the first stage of a task-driven workflow)

```yaml
---
id: intent-capture
phase: concept
collaboration: single
agents:
  - agent: product-manager
    skills: [requirement-analysis]
    inputs: []
    outputs: ["dmaf-output/{SESSION_ID}/concept/{topic}-intent.md"]
    task_description: "Original requirement text: \"{TASK}\". Capture in plain language the problems, expected outcomes, hard constraints, and open questions within it, producing an intent draft; open questions must be recorded explicitly, with no solutions given"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: intent-accept
execution: always
---
```

Takeaways: `{TASK}` carries the user's original requirement text — **it appears only in the first stage**; downstream stages always consume upstream artifacts; when the admission judgment fails, the agent reports `skipped` + a reason, downstream stages propagate the skip, and the workflow terminates gracefully (with an audit trail) — "does this input fit this flow" is a decision inside the flow, not the caller's business.

## Example 1: single-agent stage + approval gate (reading an upstream artifact)

```yaml
---
id: spec-generation
phase: requirements
collaboration: single
agents:
  - agent: product-manager
    skills: [requirement-analysis]
    inputs: ["dmaf-output/{SESSION_ID}/concept/{topic}-intent.md"]
    outputs: ["dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"]
    task_description: "Transform requirements from the intent draft: scope, user stories and acceptance criteria, NFRs (derived values marked as pending confirmation), the UE contract; introduce no design decisions"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: spec-review
execution: always
---
```

Takeaways: the input is the **full path** of the intent document (matching intent-capture's rendered outputs is what makes them producer-consumer); in the scope sequence spec-generation comes after intent-capture (order is the execution order); `approval.required: true` + `gate` — after generation the stage halts awaiting approval, and a human rejection enters the revision loop (automatic re-dispatch carrying the rejection notes); artifact file names use the `{topic}-` prefix (business-variable naming).

## Example 2: parallel dual agent + shared approval gate

```yaml
---
id: detailed-design
phase: design
collaboration: parallel
agents:
  - agent: backend-engineer
    skills: [detailed-design-backend]
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-architecture-impact.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-hld.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-lld-backend.md"]
    task_description: "Author the backend detailed design based on the spec, the architecture conclusion, and the hld (if present): interfaces, classes, state machines, sequences, database physical design, with an acceptance criteria mapping table attached"
  - agent: frontend-engineer
    skills: [detailed-design-frontend]
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-architecture-impact.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-hld.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-lld-frontend.md"]
    task_description: "Author the frontend detailed design based on the spec, the architecture conclusion, the hld, and the UE prototype (if present): page/component/state partitioning and API integration conventions, with an acceptance criteria mapping table attached"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: lld-review
execution: always
---
```

Takeaways: `collaboration: parallel` (two actions dispatched at once, sharing the same approval gate and retry policy); **soft inputs** `{artifact: …, required: false}` — the hld/UE are "on-demand" upstreams (absent when the scope does not claim them or they were skipped); absence lands in the instruction's `missing_inputs` (advisory), and the agent handles it as "absent = not produced on demand".

## Example 3: multi-input confluence + hard baseline + soft references (the on-demand trimming form)

```yaml
---
id: plan-generation
phase: development
collaboration: single
agents:
  - agent: backend-engineer
    skills: [test-design, git-workflow]
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-spec-v2.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-design.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-lld-backend.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-lld-frontend.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/development/{topic}-plan.md"]
    task_description: "Based on spec v2 and the design artifacts (absent = not produced on demand; all absent = the lightweight-flow form, with the spec as the sole baseline and an explicit \"no design backing\" declaration in the plan), generate an executable implementation plan: change scope, implementation steps, test strategy, risks and rollback plans"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: plan-review
execution: always
---
```

Takeaways: multiple inputs in confluence (the spec as the hard baseline + design artifacts as soft references); the legitimate form of **the lightweight flow trimming the design chain** — when the design stages are not scope members, downstream expresses "read it if present" via soft references. A soft input whose producer is **not a member** = legal on-demand trimming; a producer who is a member but comes after the consumer = a dead-wiring compile error.

## Example 4: scope (task-driven + required vars + delivery text)

```yaml
---
name: sdlc-standard
description: "AI-native SDLC standard flow: concept (intent) → requirements (Spec) → design → development"
vars:
  topic:                  # required: serves only as the artifact file-name prefix (the original requirement text goes through --task)
completion_template: |
  The "{vars.topic}" SDLC flow is complete.

  Requirements: {outputs.spec-generation}
  Plan and tasks: {outputs.plan-generation}, {outputs.task-breakdown}
  Implementation and delivery: {outputs.atomic-execution}, {outputs.system-testing}, {outputs.pr-preparation}
include_stages:
  - intent-capture
  - spec-generation
  - hifi-ue
  - architecture-analysis
  - hld-design
  - detailed-design
  - conflict-resolution
  - plan-generation
  - task-breakdown
  - atomic-execution
  - system-testing
  - pr-preparation
---
```

Takeaways: `vars` is in **mapping form** (empty value = required — `start --var topic=X` missing refuses startup; literal value = optional inline default); declaring vars makes the flow a structured contract callable by machines; `include_stages` **order is the execution order** (the compiler never reorders), and producers of hard inputs must precede their consumers; `completion_template` placeholders use only declared `{vars.*}` and member stages' `{outputs.*}`, with conditionals and loops unsupported.

## Example 5: constants and variable picking (synthetic example)

scope:

```yaml
---
name: publisher
description: Release flow for finished products
vars:
  topic:
constants:
  product_line: "课件"
---
```

an agents entry of the stage:

```yaml
  - agent: uploader
    skills: []
    inputs: []
    outputs: []
    task_description: "Submit the finished product for \"{topic}\" to the {product_line} management system"
    vars: [product_line]
```

Takeaways: fixed literal values (product line names and the like) go into `constants` — referencable from templates, always taking the defined value, rejected by `--var`; an agents entry's `vars` explicitly picks a subset from the session variable set (constants included), flattened into the action's `vars` field for the agent to read. The legacy fields `defaults`/`derived_vars` are removed: optional values use vars inline defaults, and composite values are referenced directly by templates (e.g. `"{product_line}-{SESSION_ID}"`).

## Example 6: trimmed scope (same library, different composition; soft references carry the on-demand parts)

```markdown
---
name: sdlc-bugfix
description: "AI-native SDLC defect-fix flow: defect intake → fix requirements → plan → tasks → TDD execution → regression testing → PR"
vars:
  topic:
include_stages:
  - intent-capture
  - spec-generation
  - plan-generation
  - task-breakdown
  - atomic-execution
  - system-testing
  - pr-preparation
---

# sdlc-bugfix scope

Shares the same stage library with sdlc-standard (no existing stage definition is modified).

## Trimming notes

The design chain (hifi-ue / architecture-analysis / hld-design / detailed-design /
conflict-resolution) is not a member of this flow: the design-artifact inputs of plan-generation and downstream are
all soft references — an absent non-member producer means not produced on demand, with no stage needing modification.
```

Takeaways: the same stage library as sdlc-standard, differing only in members and order — this is exactly the value of the scope model; the trimmed chain is absorbed by downstream soft references, **without** changing any stage definition for the trim.
