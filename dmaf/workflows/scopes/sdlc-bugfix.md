---
name: sdlc-bugfix
description: "AI-native SDLC bugfix flow: defect reports (ticket/alert/incident) taken in as intent → fix requirements (Spec) → plan → tasks → TDD execution → regression system testing → PR"
vars:
  topic:                  # required: used only as an artifact filename prefix; the original requirement text (defect symptoms, reproduction steps, expected behavior) goes through --task
completion_template: |
  The "{vars.topic}" bugfix flow is complete.

  Defect intent: {outputs.intent-capture}
  Fix requirements: {outputs.spec-generation}
  Plan and tasks: {outputs.plan-generation}, {outputs.task-breakdown}
  Implementation and delivery: {outputs.atomic-execution}, {outputs.system-testing}, {outputs.pr-preparation}
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

The AI-native SDLC bugfix flow (light path), sharing the same stage library with sdlc-standard
(modifying no existing stage definition). **The include_stages order is the execution order**:

- **Concept**: intent capture takes in the defect symptoms, reproduction steps, and expected
  behavior (approval gate `intent-accept`)
- **Requirements**: fix-requirements Spec — scope converges to the minimal fix surface, and the
  acceptance criteria turn the defect reproduction case Green
  (approval gate `spec-review`)
- **Development**: Plan (design artifacts absent = the lightweight-flow form, the spec as the sole
  baseline with an explicit
  "no design backing" declaration; approval gate `plan-review`) → task breakdown (TDD steps: write
  the reproduction failing test first) →
  atomic execution → system testing (regression verification: the defect scenario turns Green with
  no regression in existing scenarios; acceptance criteria
  fall back to spec v1) → PR preparation (approval gate `pr-review`)

## Trimming notes

The design chain (hifi-ue / architecture-analysis / hld-design / detailed-design /
conflict-resolution) is not a member of this flow: the design artifact inputs of plan-generation
and downstream stages are all soft references; when a non-member producer is absent it simply was
not produced on demand, and no stage needs modification. When brownfield impact assessment is
needed, insert architecture-analysis between spec-generation and plan-generation (a human-reviewed
three-way junction; pure-backend fixes can be dissolved away through on-demand semantics such as
`ue-review`/`hld-review`); when a large defect genuinely needs full design review, go straight to
sdlc-standard.
