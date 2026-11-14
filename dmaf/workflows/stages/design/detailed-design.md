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
    task_description: "Author the backend detailed design based on the spec, the architecture conclusion, and the hld (if present): interfaces, classes, key algorithms, state machines, sequences, database physical design, exceptions and degradation, with an acceptance criteria mapping table attached"
    required_output_format: markdown
  - agent: frontend-engineer
    skills: [detailed-design-frontend]
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-architecture-impact.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-hld.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-lld-frontend.md"]
    task_description: "Author the frontend detailed design based on the spec, the architecture conclusion, the hld (if present), and the UE prototype (if present, proto/** addressed via the delivery index): page / component / service layer / state partitioning, API integration conventions, interaction implementation points (aligned with the prototype mocks and interaction annotations), with an acceptance criteria mapping table attached"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: lld-review
execution: always
---

# Detailed Design (backend and frontend lld in parallel, approval gate `lld-review`)

Based on the accepted spec, the approved architecture conclusion and hld (if produced, through the
`hld-review` gate), and the UE prototype (if present), produce the functional-level `lld`s.
Describe only this change; reference the hld without restating it; when conflicting with the hld
or adjudication conclusions, escalate rather than silently bypassing. This stage executes after
the architecture judgment, the architecture design (hld, on demand, through the `hld-review`
gate), and the UE prototype (on demand); when artifacts are absent, the architecture impact
statement serves as the architecture baseline.
A human reviews both llds at approval gate `lld-review` (conformance to the hld, complete
exceptions and degradation, no gaps in the acceptance criteria mapping, decomposable into atomic
tasks); on approval the workflow enters conflict resolution and design finalization, on rejection
it revises.

## Steps

1. Backend: interfaces (REST contracts), classes and domain model, key algorithms (complexity
   declarations and boundary lists), state machine transition tables, sequences (including failure
   paths), database physical design, exceptions and degradation
2. Frontend: directory and routing deltas, component and service layer design, state
   classification (server state goes through request caching), API integration conventions,
   implementation points for interaction details (aligned with the UE prototype mocks, if present)
3. Each attaches an "acceptance criteria mapping table": every spec acceptance criterion ↔
   implementation point ↔ planned test
4. Before producing the draft, run the "pre-draft self-check" item by item per each side's pinned
   detailed-design skill (detailed-design-backend / detailed-design-frontend; the source of truth
   for review criteria follows the in-skill pointers)
5. Produce `{topic}-lld-backend.md` and `{topic}-lld-frontend.md`
6. Wait at approval gate `lld-review`: approved → enter conflict resolution and design
   finalization; rejected → revise per feedback
