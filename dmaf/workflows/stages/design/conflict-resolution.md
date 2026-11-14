---
id: conflict-resolution
phase: design
collaboration: sequential
agents:
  - agent: tech-lead
    skills: []
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-lld-backend.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-lld-frontend.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-architecture-impact.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-hld.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-conflict-resolution.md"]
    task_description: "Mark and classify conflicts among spec / hld / lld / UE (security vs UX, performance vs cost, compliance vs business; absent on-demand artifacts simply do not participate in the check), assign an adjudicator per conflict type and render adjudications; then fix the affected design artifacts based on the adjudication results, folding the fixes into the corresponding sections of this file"
    required_output_format: markdown
  - agent: product-manager
    skills: []
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-lld-backend.md"
      - "dmaf-output/{SESSION_ID}/design/{topic}-lld-frontend.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-hld.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-conflict-resolution.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-design.md", "dmaf-output/{SESSION_ID}/design/{topic}-spec-v2.md"]
    task_description: "Finalize the design: verify spec ↔ architecture (hld if present, otherwise the architecture impact statement) ↔ lld ↔ UE consistency and how adjudications were resolved, produce the design.md index (linking the design artifacts actually produced; absent on-demand artifacts are not listed) and write back spec v2 — using the full text of spec v1 as the baseline, absorbing only the three categories of design-confirmed increments (the architecture-confirmed NFR plan, the UE contract, adjudication-revised acceptance criteria), rewriting nothing else and adding no revision-history section; v2 is the authoritative state, v1 the historical baseline"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: design-accept
execution: always
---

# Conflict Resolution and Design Finalization (approval gate `design-accept`)

Adjudicate and fix first, finalize second: the Tech Lead marks, classifies, and adjudicates
conflicts among spec / architecture / lld / UE, and **fixes the design based on the adjudication
results** (fixed content is folded into this file; if architecture conclusions need to be revised
back, escalate the review). Adjudication evidence is looked up upward: the jointly depended-on L1
domain standards → L0 organization principles → with no basis found, write it into Areas of
Concern and escalate to a human. AI only marks and suggests; adjudication conclusions are recorded
once confirmed by a human.
Then the PO finalizes: verifying spec ↔ architecture ↔ lld ↔ UE consistency and how adjudications
were resolved, producing the `design.md` index (linking the design artifacts actually produced:
architecture impact statement / architecture design / llds / UE delivery index /
adjudication records, without duplicating content) and writing back spec v2 — using the full text
of spec v1 as the baseline, absorbing only the design-confirmed increments (NFR plan, UE contract,
adjudication revisions); the design does not change the baseline, and the increments pass the gate
together with the finalization; v2 is the authoritative state, v1 the historical baseline.
A human approves the design finalization at approval gate `design-accept` (adjudication records,
fix results, the design index, and spec v2); on approval the workflow enters development, on
rejection it revises per feedback — adjudication fix authority covers hld / lld / UE, and the
revision loop closes within this stage.

## Steps

1. Read the spec, both llds, and the on-demand artifacts (UE delivery index / prototype, hld,
   architecture impact statement, if present); list the conflict inventory and classify it
2. Assign adjudicators per conflict type (business → PO, technical → Tech Lead/Architect, security
   and compliance → Policy Owner, experience → Designer); adjudicate item by item citing evidence
   (principle numbers / standard clauses)
3. Fix the affected design sections based on the adjudication results (hld / lld / UE /
   architecture points), folding the fixes into this file
4. Produce `{topic}-conflict-resolution.md`: conflict inventory, adjudication records (evidence +
   conclusion), fix notes; remaining Areas of Concern are listed explicitly and escalated
5. PO finalization: verify four-way consistency and adjudication resolution; produce the
   `{topic}-design.md` index and
   `{topic}-spec-v2.md` (merge discipline: v1 baseline + absorb only the three increment
   categories, no revision history appended)
6. Wait at approval gate `design-accept`: accepted → design finalized, enter development;
   rejected → revise per feedback
