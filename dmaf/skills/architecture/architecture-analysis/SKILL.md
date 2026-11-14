---
name: architecture-analysis
description: Architecture impact analysis. Determines whether a requirement triggers architecture design (greenfield needs it by default; brownfield is checked trigger condition by trigger condition), produces the architecture impact statement, and produces or verifies the project's tech-stack declaration in three modes (recommendation / identification / verification). Use when the user asks for "architecture impact analysis", "determine whether architecture design is needed", "tech stack selection", "identify the existing tech stack", or "verify the tech-stack declaration". Judgment and declaration only; full hld design is the hld-design skill's job.
---

# Architecture Impact Analysis

An analysis step at the process's upstream end: determine "whether architecture design is needed" and produce the project's tech-stack declaration; the full hld design is expanded in the next stage by the `hld-design` skill. Judgment and declaration only — no design.

## Workflow

### 1. Architecture impact determination
- Greenfield projects: architecture design is needed by default
- Brownfield projects, check the trigger conditions: adding/removing modules or changing boundaries, adding/replacing technology choices, changing service boundaries or downstream dependencies, changing integration patterns or deployment topology, changing the non-functional architecture — any hit means design is needed; otherwise produce the impact statement (item-by-item rationale)
- When design is deemed necessary, also provide hld revision points and an ADR draft in the impact statement (point-level; full expansion is `hld-design`'s job)

### 2. Tech-stack binding (this skill's exclusive responsibility)

Produce or verify the project's tech-stack declaration. Three modes converge on the same outcome:

| Mode | Trigger | Action |
| :--- | :--- | :--- |
| Recommendation | No declaration exists; the repo shows no existing stack traits | Produce candidates and a recommendation within the supported set (the `stacks/` registry: backend rust/go/java, frontend vue3-naiveui/react-mantine), with a comparative conclusion per the `tech-selection` evaluation framework |
| Identification | No declaration exists; the repo shows stack traits | Identify the existing stack from build files (Cargo.toml / go.mod / pom.xml / package.json + framework dependencies) and generate the declaration draft |
| Verification | A declaration exists | Read the declaration, cross-check it against the repo's current state, and submit for review; revise and re-report if anything changed |

- The backend/frontend dimensions bind independently; "none" is allowed; choices outside the candidates take the out-of-registry branch
- **Out-of-registry stack** (no matching stacks/ skill): state explicitly at the gate ① the identification/recommendation result ② a recommendation to create the `stacks/` skill via the fixed questionnaire (named `<language>-<framework>`, spelled out in full) ③ the option to continue generalized; the user's reply is the decision — **defaulting to generalized is forbidden**
- **Generalized mode**: the declaration's "Binding" section says "None (beyond the currently supported set)" and it must carry a "Verification commands" section (determine the test/build/lint commands from build files) — this is the sole authority for the verification loop in generalized execution; when the skill is added later, changing only the Binding section to the skill path upgrades to bound mode
- For the declaration's fixed format and field semantics see `reference/tech-stack-declaration.md` (the format source of truth, cohesively carried by this skill)

### 3. Self-check

Every impact determination has item-by-item rationale; when design is needed, revision points and the ADR draft are in place; backward compatibility has been assessed; the stack declaration matches the recommendation.

## Boundaries (not in this skill)

- The full hld design is `hld-design`'s job (this skill produces only the impact statement and the stack declaration)
- Full-process evaluation and pilot validation of out-of-registry technologies is `tech-selection`'s job (this skill's recommendation mode only produces candidate comparisons)

## Hard rules

- Technology-choice changes without an ADR are forbidden; defaulting out-of-registry stacks to generalized is forbidden
- High-risk architecture decisions (impact determination conclusions, stack bindings) take effect only after human review passes
- The impact statement is a current-state document: evolution and history stay out of the document body (carried by adjudication records)

## Reference files

- `reference/tech-stack-declaration.md` — **required reading when generating or verifying the stack declaration**: the tech-stack declaration's fixed format and field semantics (conventional file name tech-stack.md)
- `rules/guides/tech-stack-guide.md` — the stack-binding registry and the out-of-registry introduction process
- `rules/guides/adr-authoring-guide.md` — when to write an ADR and how (required reading when drafting an ADR)
- hld-design skill `assets/adr-template.md` — the ADR skeleton (apply it when drafting)
- `rules/standards/architecture-standard.md` — architecture red lines and governance
- `stacks/` — the registry of supported stack skills (the legal targets of the declaration's "Binding" section)
