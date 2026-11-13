---
id: spec-generation
phase: requirements
collaboration: single
agents:
  - agent: product-manager
    skills: [requirement-analysis]
    inputs: ["dmaf-output/{SESSION_ID}/concept/{topic}-intent.md"]
    outputs: ["dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"]
    task_description: "Transform requirements from the intent draft: scope, user stories and acceptance criteria, NFRs (derived values marked as pending confirmation), UE contract; introduce no design decisions"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: spec-review
execution: always
---

# Spec Generation (approval gate `spec-review`)

Turn the accepted intent into `spec.md` (problems and goals, scope, user stories, acceptance
criteria, NFRs, UE contract). A human reviews at approval gate `spec-review`: requirement
completeness, measurable acceptance criteria, reasonable NFRs, clear scope — **design is not
reviewed**. Once accepted, the spec (v1) is frozen and the workflow enters the design phase;
once rejected, it is revised.

## Steps

1. Read only intent.md; answer its open questions or move them into Out of Scope
2. Generate user stories (all three elements complete) with Given-When-Then acceptance criteria
   (3–7 per story)
3. Derive NFRs and mark them "derived value, pending confirmation"; scope In/Out must be explicit;
   "etc." and "and the like" are forbidden
4. Do not load design-related standards; the spec must not contain architecture, technology
   selection, table schemas, components, or concrete interaction implementations
5. Wait at approval gate `spec-review`: accepted → spec v1 frozen; rejected → revise per feedback
