---
id: hld-design
phase: design
collaboration: single
agents:
  - agent: architect
    skills: [hld-design]
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-architecture-impact.md"
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-hld.md"]
    task_description: "Per the impact statement's conclusion, expand the design points into a complete hld review draft: style and module decomposition, module boundaries and responsibilities, key integration patterns, ADR completion (2–3 alternatives with rationale where real trade-offs exist, cited evidence where best practices / de facto standards exist), a non-functional plan covering all spec NFRs, verified consistent with the tech-stack declaration; when the conclusion is that no architecture design is needed (brownfield project with no trigger conditions), produce no document and report skipped with a reason"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: hld-review
execution: conditional
---

# Architecture Design (hld authoring, on demand, approval gate `hld-review`)

After architecture-review is approved, expand the design points in the impact statement into a
**complete hld review draft** (a service-level long-lived asset: the review draft lands in the
session artifact bucket; merging it into the repository is performed manually). Greenfield
projects execute it by default; brownfield projects execute or skip per the impact statement's
conclusion (report `skipped` with a reason; an on-demand skip sets no gate).
A human reviews hld completeness and feasibility at approval gate `hld-review` (module boundaries
and responsibilities, ADRs, non-functional plan coverage, consistency with the stack declaration);
on approval the workflow enters detailed design, on rejection it revises.
Downstream stages (detailed design / conflict resolution and finalization) reference this artifact
as a soft input; when it is absent, they proceed as if there were no hld.

## Steps

1. Read the spec and the impact statement: conclusion is design needed → author the complete hld;
   conclusion is no design needed → report `skipped` (with a reason, no gate), producing no
   placeholder document
2. Expand the hld: style confirmation and module decomposition, module boundaries and
   responsibilities, key integration patterns, ADR completion
   (alternatives are not mandatory: 2–3 alternatives with rationale where real trade-offs exist;
   recorded evidence where best practices / de facto standards exist), non-functional plan
   covering all spec NFRs (observability / resilience / security / capacity)
3. Verify item-by-item consistency with the `dmaf-output/tech-stack.md` stack declaration; when
   conflicting with existing repository constraints, escalate rather than silently bypassing
4. Produce `{topic}-hld.md` (the hld review draft)
5. Wait at approval gate `hld-review`: the human reviews hld completeness and feasibility (module
   boundaries and responsibilities, ADR trade-offs and evidence, NFR plan coverage, consistency
   with the stack declaration); approved → enter detailed design;
   rejected → revise per feedback
