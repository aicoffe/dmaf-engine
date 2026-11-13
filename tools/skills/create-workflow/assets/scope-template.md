---
name: <scope-name>                # = file-name stem, English kebab-case
description: <one-sentence summary of this flow>
vars:                             # mapping: empty value = required (must be provided at start); literal value = optional inline default; a task-driven flow may declare none
  topic:                          # the per-feature/per-topic dimension word from the document; {topic} in templates matches this name
constants: {}                     # optional: fixed literal values (constant, rejected by --var), e.g. product_line: "课件"
completion_template: |            # optional; placeholders are only {vars.*} (constants included) and {outputs.<member stage>}; conditionals/loops unsupported
  Flow complete! Topic: {vars.topic}
  Deliverables: {outputs.<stage-id>}
include_stages:                   # order is the execution order: producers of hard inputs must be members and precede their consumers (the compiler never reorders)
  - <stage-id-1>
  - <stage-id-2>
---

# <scope-name> scope

<Flow description: how the members chain along the include_stages sequence and which approval gates exist (the revision loop is built into the approval gate; there is no stage-level parallelism —
"can run in parallel" relations are ordered by sequence position or merged into a collaboration: parallel stage).>

## Trimming notes (if any)

<Which stages were skipped relative to the full flow and why; how the trimmed chain is absorbed by downstream soft references.>
