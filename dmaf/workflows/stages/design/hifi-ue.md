---
id: hifi-ue
phase: design
collaboration: single
agents:
  - agent: ux-designer
    skills: [ue-prototyper-react-mantine]
    inputs:
      - "dmaf-output/{SESSION_ID}/requirements/{topic}-spec.md"
    outputs: ["dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md"]
    task_description: "Generate a coded UE prototype within the project's real React + Mantine stack based on the spec's UE contract: spec-first (proto/ue-spec.yaml), fenced scope proto/**, HTTP-layer mock, click-through review loop (?uer=1), productionization ledger gaps.md; on completion write the UE delivery index at the path in the outputs list (form, actual path of proto/manifest.json, review entry, feedback location, open items in gaps). For pure-backend / no-UI tasks, report skipped with a reason"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: ue-review
execution: conditional
---

# High-Fidelity UE (coded prototype, on demand, approval gate `ue-review`)

Executed after the spec is finalized (spec-review approved) and before the architecture judgment:
generates a runnable, clickable, reviewable coded UE prototype within the project's real
React + Mantine stack, replacing static design mockups. Prototype pages follow the
`frontend-react-mantine` skill conventions; the data plane connects to the mock channel through
the project's request layer (zero rewrite when moving to production). A human reviews the
prototype at approval gate `ue-review` via the **click-through review loop** and the delivery
index (interaction feel, state coverage, open feedback items); on approval the workflow enters
the architecture judgment, on rejection the prototype is revised per feedback.
Downstream stages (architecture judgment / detailed design / conflict resolution and finalization)
reference the **UE delivery index** as a soft input; `proto/**` is addressed through the index.
Pure-backend / no-UI tasks report `skipped` (with a reason; an on-demand skip sets no gate).

## Stack Binding

The organization-led stack is react-mantine: this stage binds directly to
`ue-prototyper-react-mantine`. Stack consistency is guaranteed by the project itself (no runtime
verification). Extension path when the project's frontend stack differs (modifying package assets
triggers add/push hash-gate conflicts and install skipped — organization-level changes go through
redistribution from the package source; when deliberately abandoning local changes, add/push
`--force` on the target project side explicitly overrides):

- **vue3-naiveui projects**: swap this stage's skills to the in-package
  `ue-prototyper-vue3-naiveui` — a one-line change;
- **other stacks**: create a new `stacks/<stack>/` prototype skill modeled on the structure of the
  existing two prototype skills, and update this stage's skills reference.

## Steps

1. On-demand determination: the spec has no UE contract (pure backend / no UI) → report `skipped`
   (with a reason), producing no placeholder document
2. Execute per the `ue-prototyper-react-mantine` skill workflow: survey and fix the mode
   (brownfield anchoring / greenfield whole-shell fencing — the entire application shell lives
   inside `proto/`, zero touches at the repo root) → draft
   `proto/ue-spec.yaml` (the spec's UE contract is the contract — reference and
   implement it, do not invent it; new interaction expectations go through conflict resolution) →
   generate page by page → quality gates → inline review loop
3. After the prototype is complete, write the **UE delivery index** at the path in the outputs
   list: form, actual path of `proto/manifest.json`
   (the layout floats; the index is the single record point), review loop entry (`?uer=1`),
   feedback export location, summary of open items in `gaps.md`
4. Wait at approval gate `ue-review`: the human experiences the prototype via the click-through
   review loop and reviews it against the delivery index (state coverage,
   open feedback items); approved → enter the architecture judgment; rejected → revise the
   prototype per feedback (revision loop)
