---
id: architecture-analysis
phase: design
collaboration: single
agents:
  - agent: architect
    skills: [architecture-analysis]
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-architecture-impact.md", "dmaf-output/tech-stack.md"]
    task_description: "Make the architecture impact judgment: greenfield defaults to needing design; brownfield checks the trigger conditions item by item (module boundaries / technology selection / service boundaries / integration patterns / non-functional architecture). Produce the impact statement; if design is needed, also list the hld revision points and an ADR draft; and produce or verify the project tech-stack declaration dmaf-output/tech-stack.md in three modes (selection advice / identification confirmation / audit), persisted to disk after confirmation at the approval gate"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: architecture-review
execution: always
---

# Architecture Impact Analysis (approval gate `architecture-review`)

Unified entry point: first judge "whether architecture design is needed" (greenfield defaults to
yes; brownfield checks against trigger conditions), producing the **impact statement** (mandatory
artifact); simultaneously produce or verify the **project tech-stack declaration**
`dmaf-output/tech-stack.md` (mandatory artifact, persisted to disk after confirmation at the
approval gate). When design is judged necessary, the statement also gives the hld revision points
and an ADR draft (point-level; the full hld is expanded by the next stage `hld-design`).
A human reviews judgment accuracy and solution feasibility (including stack binding confirmation)
at approval gate `architecture-review`; on approval the workflow enters architecture design
(hld, on demand), on rejection it revises.

## Steps

1. Read the spec and the UE delivery index (if present; the prototype is produced on demand) to
   extract NFRs, constraints, and open questions; brownfield projects reference the existing hld
   as a soft input (it may be missing)
2. Produce `{topic}-architecture-impact.md`: evidence for each trigger condition checked + conclusion
3. Produce/verify `dmaf-output/tech-stack.md`: three modes (selection advice / identification
   confirmation / audit); stacks outside the catalog take the explicitly-flagged-at-the-gate
   branch, and the generalized mode must carry verification commands (see the "Stack Binding"
   section of the architecture-analysis skill)
4. If design is needed: style confirmation (modular monolith by default), module decomposition,
   ADR drafts for key decisions, non-functional plan (covering all spec NFRs)
5. Self-check: backward compatibility, observability, no circular dependencies, every decision has
   a trade-off record (alternatives and rationale where real trade-offs exist; cited evidence where
   best practices / de facto standards exist), stack declaration consistent with the selection
6. Wait at approval gate `architecture-review`: approved → enter architecture design (hld,
   on demand); rejected → revise per feedback
