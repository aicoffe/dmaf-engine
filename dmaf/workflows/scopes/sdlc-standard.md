---
name: sdlc-standard
description: "AI-native SDLC standard flow: concept (intent) → requirements (Spec) → design (high-fidelity UE / architecture judgment / hld / detailed design / conflict resolution / finalization) → development (Plan → tasks → TDD execution → system testing → PR)"
vars:
  topic:                  # required: used only as an artifact filename prefix (sample of a user-defined variable; the original requirement text goes through --task)
completion_template: |
  The "{vars.topic}" SDLC flow is complete.

  Requirements: {outputs.spec-generation}
  Design (adjudication and finalization): {outputs.conflict-resolution}
  Plan and tasks: {outputs.plan-generation}, {outputs.task-breakdown}
  Implementation and delivery: {outputs.atomic-execution}, {outputs.system-testing}, {outputs.pr-preparation}
include_stages:
  - intent-capture
  - spec-generation
  - hifi-ue
  - architecture-analysis
  - hld-design
  - detailed-design
  - conflict-resolution
  - plan-generation
  - task-breakdown
  - atomic-execution
  - system-testing
  - pr-preparation
---

# sdlc-standard scope

The AI-native SDLC standard flow:

- **Concept**: intent capture (approval gate `intent-accept`)
- **Requirements**: Spec generation (approval gate `spec-review`)
- **Design**: high-fidelity UE (on demand: React + Mantine coded prototype, executed only when
  there is a UI;
  approval gate `ue-review`: the human reviews the prototype via the click-through review loop) →
  architecture impact judgment (approval gate `architecture-review`, including the tech-stack
  declaration) → architecture design hld
  (on demand: greenfield executes by default, brownfield per the impact statement's conclusion;
  approval gate `hld-review`) → detailed design
  (approval gate `lld-review`) → conflict resolution and design finalization (approval gate
  `design-accept`: adjudication
  fixes → PO finalization, producing the design index and spec v2)
- **Development**: Plan (approval gate `plan-review`) → task breakdown (TDD steps) → atomic execution
  (TDD loop, full verification per task) → system testing (end-to-end scenarios + contract
  verification, the qa-engineer
  only tests and does not fix, the defect list passes the gate with the report) → PR preparation
  (approval gate `pr-review`)
