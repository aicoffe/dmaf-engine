---
id: system-testing
phase: development
collaboration: single
agents:
  - agent: qa-engineer
    skills: [automated-testing, contract-testing]
    inputs:
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-spec-v2.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-lld-backend.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-lld-frontend.md", required: false}
      - "dmaf-output/{SESSION_ID}/development/{topic}-implementation-backend.md"
      - "dmaf-output/{SESSION_ID}/development/{topic}-implementation-frontend.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/development/{topic}-system-test-report.md"]
    task_description: "Perform system-level verification on the integrated system: run end-to-end scenario tests against the spec v2 acceptance criteria (absent means the lightweight-flow form, falling back to spec v1; real dependencies preferred, Testcontainers standing up equivalent environments); perform interface contract verification against the lld contracts (when absent, the implementation and task list prevail; OpenAPI/schema validation); with a UE delivery index, walk the key user journeys against the prototype; produce the system test report (scenario coverage matrix, contract verification results, defect list). Discovered defects are recorded truthfully in the report (blocking items flagged, handled at the pr-review gate), and implementation code is not modified; report failure truthfully only when test environment problems make verification impossible to complete"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
execution: always
---

# System Testing

Task-level green does not equal system-level correctness: perform cross-task verification on the
integrated system — run end-to-end scenarios against the spec v2 acceptance criteria (falling back
to spec v1 when absent), verify interface implementations against the lld contracts (when llds are
absent, the implementation and task list prevail), and consolidate into the system test report.
The qa-engineer only tests and does not fix: discovered defects go truthfully into the report;
skipping cases or falsely claiming passes is forbidden; defects found by testing are not execution
failures (the report moves on through the flow to the `pr-review`
gate for handling); only when environment problems make verification impossible is `failed`
reported.

## Steps

1. Read the spec v2 (spec v1 if absent) acceptance criteria and both llds (the interface contract
   source, if present), together with the two implementation verification records to understand the
   implementation status
2. End-to-end scenario testing: at least one system-level scenario per acceptance criterion
   (real dependencies preferred, Testcontainers
   standing up equivalent environments), covering normal paths and key failure paths
3. Contract verification: interface implementations checked against the lld contracts (OpenAPI/
   schema validation; inter-service interfaces verified by consumer contracts); with a UE delivery
   index, walk the key user journeys against the prototype
4. Produce `{topic}-system-test-report.md`: scenario coverage matrix (acceptance criterion ↔ scenario ↔
   result),
   contract verification results, defect list (severity-graded, blocking items flagged explicitly)
5. When the report is complete, the flow enters PR preparation (the defect list passes through the
   `pr-review` gate with the report for handling; fixes belong to the next
   revision loop — implementation code is not changed within this stage)
