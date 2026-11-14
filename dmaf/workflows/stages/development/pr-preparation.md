---
id: pr-preparation
phase: development
collaboration: single
agents:
  - agent: backend-engineer
    skills: [git-workflow, code-review, documentation]
    inputs:
      - "dmaf-output/{SESSION_ID}/development/{topic}-plan.md"
      - "dmaf-output/{SESSION_ID}/development/{topic}-tasks.md"
      - "dmaf-output/{SESSION_ID}/development/{topic}-implementation-backend.md"
      - "dmaf-output/{SESSION_ID}/development/{topic}-implementation-frontend.md"
      - "dmaf-output/{SESSION_ID}/development/{topic}-system-test-report.md"
    outputs: ["dmaf-output/{SESSION_ID}/development/{topic}-pr.md"]
    task_description: "Prepare the PR: verify the diff matches the plan and llds and that tests cover all acceptance criteria (including the system test report); determine the documentation impact and update the affected documents (README / API contracts / CHANGELOG / ADR); organize task-level and system-level test evidence; attach a risk statement"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: pr-review
execution: always
---

# PR Preparation (approval gate `pr-review`)

Confirm the change diff is consistent with the plan/llds and that tests cover all spec acceptance
criteria (task-level verification plus the system test report as two layers of evidence); determine
and complete the documentation impact updates; produce the PR description (change summary, test
output, documentation update notes, risk statement). The Code Owner makes the final check at
approval gate `pr-review` (review points per the code review skill: correctness, security,
maintainability, test sufficiency, compatibility); once approved, the PR is submitted and merged.
(Release / deployment is out of scope for this workflow.)

## Steps

1. Read the plan, tasks, both implementation verification records, and the system test report;
   check item by item: diff ↔ plan,
   diff ↔ lld, acceptance criteria ↔ tests; blocking defects in the system test report must be
   presented explicitly to the gate
2. Documentation impact determination (per the `documentation` skill's document type matrix):
   interface changes → API contracts and
   CHANGELOG; schema changes → data dictionary; architecture decisions → ADR; onboarding
   information changes → the service
   README — update directly where needed
3. Produce `{topic}-pr.md`: change summary, task-level and system-level test evidence,
   documentation update notes,
   risks and rollback
4. Wait at approval gate `pr-review`: approved → merge; rejected → revise per review comments
