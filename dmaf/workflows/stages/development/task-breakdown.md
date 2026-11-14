---
id: task-breakdown
phase: development
collaboration: single
agents:
  - agent: backend-engineer
    skills: [test-design]
    inputs: ["dmaf-output/{SESSION_ID}/development/{topic}-plan.md"]
    outputs: ["dmaf-output/{SESSION_ID}/development/{topic}-tasks.md"]
    task_description: "Break the implementation plan down into atomic tasks: each task independently implementable, independently verifiable, independently rollback-able, with TDD steps (which failing test to write first) and verification commands, annotated with dependency order and backend/frontend assignment"
    required_output_format: markdown
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
execution: always
---

# Task Breakdown (TDD steps)

Break the implementation plan down into an atomic task list: each task independently implementable,
verifiable, and rollback-able, with verification commands, dependency order, and backend/frontend
assignment; **every task states its TDD steps**
(Red: which failing test to write first → Green: what to implement → Refactor: what to refactor).

## Steps

1. Read the implementation plan and split tasks by "test first, then implement": one behavior
   point per task, completable in minutes
2. Produce `{topic}-tasks.md`: the task list (number, description, TDD steps, verification
   commands, dependency order, backend/frontend assignment)
3. Large-grained tasks are forbidden; sharing intermediate state across tasks is forbidden
