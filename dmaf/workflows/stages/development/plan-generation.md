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
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/development/{topic}-plan.md"]
    task_description: "Based on spec v2 and the design artifacts (finalized design index / both llds / UE delivery index; absent means not produced on demand; all absent = the lightweight-flow form, with the spec as the sole baseline and an explicit \"no design backing\" declaration in the plan), generate an executable implementation plan in the planning state: change scope, implementation steps and their order, test strategy (aligned with acceptance criteria), risks and rollback plans"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: plan-review
execution: always
---

# Plan (approval gate `plan-review`)

Read spec v2 and the design artifacts (if present), generating a `plan.md` draft: change scope,
implementation steps, test strategy, risks and rollback plans. When all design artifacts are
absent (the lightweight flow trimmed the design chain), the spec is the sole baseline and the plan
declares "no design backing" explicitly. A human reviews at approval gate `plan-review`: full
coverage of spec requirements, sensible steps, clear dependencies and prerequisites; high-risk
changes are explicitly approved at the gate. Implementation is allowed only after the plan is
accepted.

## Steps

1. Read spec v2 (spec v1 if absent), the design index, both llds, and the UE delivery index
   (if present); check the acceptance criteria mapping table
2. Produce `{topic}-plan.md`: change scope (files / modules), implementation steps and their
   order, test strategy (covering all acceptance criteria), risks and rollback plans
3. Wait at approval gate `plan-review`: approved → task breakdown; rejected → revise per feedback
