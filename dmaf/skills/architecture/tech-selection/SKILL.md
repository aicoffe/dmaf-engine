---
name: tech-selection
description: Tech selection and tech-stack governance. Evaluating new technology choices, maintaining the organization's standard tech-stack registry, checking selection compliance. Use when the user asks for "tech selection", "introducing a new framework/middleware", "replacing a technology", or "updating the tech-stack registry". Includes pilot validation and ADR records.
---

# Tech Selection

Pick technology following the process: check the registry first, run the evaluation, leave a decision record.

## Workflow

### 1. Check the registry
- The supported tech-stack set = the `stacks/` skill registry (backend-rust / backend-go / backend-java /
  frontend-vue3-naiveui / frontend-react-mantine and project-created entries; see `rules/guides/tech-stack-guide`)
- A hit within the set → project binding (the three-mode flow is in the `architecture-analysis` skill's "tech-stack binding" section)
- Outside the set → run the full process below; consolidate the conclusion into a new stack skill (fixed questionnaire) or run the project in generalized mode

### 2. Candidate evaluation (≥ 2 candidates, including "build in-house / do not introduce")

- **Source of truth for evaluation dimensions** (six dimensions; write a conclusion per dimension — one-liners from impression are forbidden):

| Dimension | Evaluation questions |
|---|---|
| Maturity | Production-proven cases? Release cadence? LTS policy? |
| Team capability | Match with existing skills? Learning cost? |
| Community and maintenance | Commit activity, issue responsiveness, backing organization |
| Organization standards | Conflicts with the existing stack? License compatibility? |
| Operating cost | Resource footprint, operational complexity, monitoring support |
| Exit cost | Replacement difficulty, data migration, degree of lock-in (replaceability is a hard requirement) |

- **Candidate collection**: the supported set (the stacks/ registry) + what similar projects use + mainstream options in the problem domain, until ≥ 2 are gathered; when there is only one reasonable candidate (a de facto standard or the only mature implementation), record "why there is no second candidate" in the ADR — silently omitting it is forbidden
- **Integrated decision** (six-dimension conclusions → the choice, borrowing the hld-design evaluation discipline):
  1. **Hard gates first**: hitting a hard-rule veto item (unmaintained / license conflict / unfixed Critical vulnerability) eliminates the candidate — it does not enter the trade-off
  2. **Choose by priority**: order the six dimensions by nature — organization standards and exit cost are **baseline dimensions** (no concession), maturity and community maintenance are **risk-reduction dimensions**, team capability and operating cost are **fit dimensions**; satisfy the first priority before looking at the second; counting pros and cons or weighted scoring is forbidden
  3. **Handling drawbacks**: write out the chosen option's drawbacks explicitly plus why they are acceptable for now (which drawback is absorbed by which dimension-priority decision)

### 3. Pilot
- Pilot on non-core business or an isolated module; goals must be quantifiable (performance benchmarks, fault-injection behavior)
- Backfill the conclusion into the ADR's "rationale" section

### 4. Decision and adoption
- ADR: candidate comparison, conclusion, impact, exit plan (when to write one and the key points are in `rules/guides/adr-authoring-guide`; apply the hld-design skill's `assets/adr-template.md` skeleton)
- Pick one adoption form: create the `stacks/` skill via the fixed questionnaire (recommended, reusable); or run the project's tech-stack declaration in generalized mode
- After the review passes, update the stack-binding registry (`rules/guides/tech-stack-guide`)

## Boundaries (not in this skill)

- Producing and verifying the project's tech-stack declaration (the three modes: recommendation / identification / verification) belongs to `architecture-analysis` — identifying an existing stack does not go through this skill
- Landing the selection conclusion in the architecture documents (the hld tech-selection section and the ADR index) belongs to `hld-design`

## Hard rules

- Direct vetoes: unmaintained, license conflict, unfixed Critical vulnerability
- Replacing an existing choice: must present evidence of the current option's pain points (incident/bottleneck data, with timestamps) + switch only after parallel-run validation; keep the old option for one release cycle
- Exit cost is a hard evaluation dimension: critical dependencies must be replaceable

## Reference files

- `rules/guides/tech-stack-guide.md` — the stack-binding registry and the out-of-registry introduction process
- `rules/guides/adr-authoring-guide.md` — when to write an ADR, the key points, and status transitions (required reading when drafting an ADR)
- hld-design skill `assets/adr-template.md` — the ADR skeleton (apply it when drafting)
- `rules/standards/architecture-standard.md` — selection principles and prohibitions
