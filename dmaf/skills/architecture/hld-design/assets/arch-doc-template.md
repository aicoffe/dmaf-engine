# Architecture Design Document Master Template (hld review draft)

> Purpose: assemble the hld review draft after Phase 5/6. The nine sections are the current-state structure: the body states conclusions only; process and derivations go to appendices; alternatives and the evaluation go into ADRs (`assets/adr-template.md`).
> Change maintenance: the module boundary diagram, the ADR index, and the technology-selection table are updated in sync.

---

# <System Name> Architecture Design (hld)

| Meta | Content |
|---|---|
| Version / status | v0.1 / draft · in review · finalized |
| Author / reviewers | |
| Date | |

## 1. Service context

<Service positioning, upstream/downstream dependencies, users; for the system context diagram see `reference/diagram-guide.md`>

## 2. Architecture style and principles

<Style-selection conclusion and rationale — the response to the 1–2 primary complexities; cite ADRs. The full screening derivation goes to Appendix A>

## 3. Module decomposition

<Module list, responsibilities, boundaries; state the decomposition rationale>

Express modules and dependency directions with the logical view: `assets/views/logical-view.md`; project/package structure belongs to the development view: `assets/views/development-view.md`

## 4. Technology selection

<Selection list + ADR links; must match the project stack declaration item by item; deviations require an ADR>

## 5. Service boundaries and integration

<External interface overview, integration patterns (sync/async), contract links; for the integration-pattern decision tree see `reference/architecture-guidance.md`>

Express process/service decomposition, concurrency synchronization points, and message flows with the runtime view: `assets/views/runtime-view.md` (the P0 scenario sequence diagram must appear)

## 6. Data architecture

<Database selection, core data model overview, partitioning/archiving strategy>

Express the data model and sharding/replication strategy with the data view: `assets/views/data-view.md`

## 7. Deployment topology

<Environments, instance shapes, dependent middleware topology>

Express nodes/specs/networking and capacity calculations with the physical view: `assets/views/physical-view.md`

## 8. Non-functional architecture

<Performance budget, availability targets (SLOs), capacity model, security boundaries — cover every spec NFR item by item; for an item with no design write "none + reason">

Attach the high-availability hazard screening FMEA table: `assets/fmea-checklist.md` (high-severity items without mitigation are not allowed to exist)

## 9. Architecture decision index

<ADR list: number, title, status; apply `assets/adr-template.md` when drafting>

Alternative cards and evaluation tables go into the corresponding ADRs (template `assets/option-evaluation.md`); they are not expanded in this document's body

## Appendix

- A. Complexity screening derivation (the full process from `assets/complexity-worksheet.md`)
- B. Estimation and load-test data
- C. Phased evolution planning (first phase minimally viable; scope and expected timeline per phase)
