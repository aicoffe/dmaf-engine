---
name: tech-stack-guide
level: L2
domain: tech-stack
description: The stack binding registry and the process for introducing technology beyond it. Load during technology selection, new project setup, dependency introduction, and stack binding.
owner: Architecture Group
based_on: [organization-principles, architecture-standard]
enforcement: mandatory
version: v2.0.0
review_cycle: every six months
---

# Tech Stack Work Specification (Stack Binding Registry)

> The organization maintains no single "standard selection table". The set of supported tech stacks is carried by the `stacks/` skill registry; the stack a project actually uses is bound explicitly by the project's tech-stack declaration (conventional file name tech-stack.md) — explicit over implicit (P-007).

## 1. Supported Tech Stack Set (Registry)

- The single source of truth for the registry is the `stacks/` skill directory itself; this specification maintains no duplicate list (avoiding dual-source drift): stack skills are named `<stack domain>-<stack name>`, where the stack domain is backend / frontend
- Extending the registry = adding a `stacks/<name>/` skill, authored against the unified fixed questionnaire (structure comparable across stacks)
- Project-local stack skills are named with the full `<language>-<framework>` spelling to avoid path collisions with the built-in set
- Each stack skill internally contains binding constraints for the toolchain, error handling, testing, concurrency, data access, layout, and the verification loop

## 2. Project Binding Mechanism (Summary)

- The binding declaration is produced with the architecture impact analysis: the agent writes the project's tech-stack declaration in one of three modes (recommendation / identification / verification), confirmed by the architecture review
- The backend and frontend dimensions are orthogonal and independent; each can be chosen or left empty (none)
- Downstream skills read the declaration via their own "stack binding" preamble: when bound, load the stack skill (`SKILL.md` entry point + `reference/` routing); when the binding is "none", execute in generalized mode, with the verification loop following the declaration's "Verification commands" section
- Declaration missing = unbound; downstream executes per the neutral specification and labels the output accordingly

## 3. Introducing Technology Beyond the Registry

1. Run the selection evaluation process (at least 2 candidates, including not introducing); write the conclusion as an ADR
2. Choose one of two landing forms: build the missing `stacks/` skill against the fixed questionnaire (recommended, reusable); or run the project's tech-stack declaration in generalized mode (verification commands mandatory)
3. A generalized-mode project that later builds the skill upgrades to bound mode by editing only the declaration's "Binding" section; the process does not need to rerun

## 4. Usage Requirements

- Stack knowledge converges to a single point: stack constraints live only in stack skills and the stack declaration; scattering stack-binding sentences across neutral specifications/skills is forbidden
- Generalized mode is a supported degraded form, not an exemption: outputs must be labeled generalized, and building the skill early to switch to bound mode is encouraged
- Review the registry every six months: when a stack skill's content is outdated, update the skill first, then notify projects to re-verify their stack declarations
