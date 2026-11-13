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
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: intent-accept
execution: always
---

# Intent Capture (approval gate `intent-accept`)

Turn ideas, tickets, alerts, and incidents into a structured `intent.md` draft; a human reviews and
accepts or rejects at approval gate `intent-accept` — once accepted, the intent is frozen and the
workflow enters the requirements phase; once rejected, it enters a revision loop (automatic
redispatch carrying `revision_feedback`). The concept phase produces only the accepted problem
definition; it writes no user stories and does no design.

## Steps

1. Extract from user input: problems, expected outcomes, hard constraints, open questions, sources
2. Capture red lines: no user stories or UI descriptions, no technical solutions; anything undecided
   must go into "Open Questions"
3. Produce `{topic}-intent.md` and wait at approval gate `intent-accept`: accepted → finalize;
   rejected → revise per feedback
