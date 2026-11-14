---
id: atomic-execution
phase: development
collaboration: sequential
agents:
  - agent: backend-engineer
    skills: [automated-testing, backend-development]
    inputs:
      - "dmaf-output/{SESSION_ID}/development/{topic}-tasks.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-lld-backend.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/development/{topic}-implementation-backend.md"]
    task_description: "Execute the backend tasks in the task list (lld absent means not produced on demand; the task list and its verification commands are the baseline): per-task TDD (Red failing test → Green minimal implementation → Refactor); after each task run full verification (tests / build / lint); when verification fails, fix and retry up to 3 times, then report; record task → change → verification evidence"
    required_output_format: markdown
  - agent: frontend-engineer
    skills: [automated-testing, frontend-development]
    inputs:
      - "dmaf-output/{SESSION_ID}/development/{topic}-tasks.md"
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-lld-frontend.md", required: false}
      - {artifact: "dmaf-output/{SESSION_ID}/design/{topic}-high-fidelity-ue.md", required: false}
    outputs: ["dmaf-output/{SESSION_ID}/development/{topic}-implementation-frontend.md"]
    task_description: "Execute the frontend tasks in the task list (lld absent means not produced on demand; the task list and its verification commands are the baseline): implement components and interactions with per-task TDD, doing visual verification against the UE prototype and the click-through review export (if present; absent means not produced on demand); after each task run full verification (tests / build / lint); when verification fails, fix and retry up to 3 times, then report; record task → change → verification evidence"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
execution: always
---

# Atomic Execution (TDD)

Implement task by task with TDD per the task list: **Red (write the failing test first) → Green
(minimal code to pass) → Refactor (refactor while keeping green) → Task verification (run the full
tests / build / lint; the frontend includes screenshot comparison)**.
Tests come first and must not be tampered with (unless a human confirms the test itself is wrong);
when verification fails, fix and retry up to 3 times; if it still fails, mark blocked and report —
skipping verification or falsely claiming completion is forbidden. Each task's full verification
is the self-check.

## Steps

1. Backend: execute the backend tasks per the list and produce `{topic}-implementation-backend.md`
   (task → change → verification evidence)
2. Frontend: execute the frontend tasks per the list, verifying against the UE prototype and the
   click-through review export (if present), and produce `{topic}-implementation-frontend.md`
3. Any task failing verification 3 times → mark blocked and report truthfully; do not proceed to
   the next task
