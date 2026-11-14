---
name: adr-authoring-guide
level: L2
domain: adr
description: ADR (architecture decision record) authoring conventions covering when an ADR is mandatory, key authoring points, and status transitions. Load when making an architecture or technology-selection decision.
owner: Architecture Group
based_on: [architecture-standard]
enforcement: mandatory
version: v1.1.0
review_cycle: every six months
---

# ADR Authoring Work Specification

## 1. When an ADR Is Mandatory

- Technology selection (including replacing or major-version upgrades)
- Architecture pattern changes (style, decomposition, integration patterns)
- Cross-service interface design
- Major data model changes / partitioning strategy
- Non-functional designs (SLOs, caching strategy, degradation plans)
- Security policy changes
- Any decision in the hld whose "why" is not easily read from the code

❌ No ADR needed: implementation details within a module, small reversible changes.

## 2. Key Authoring Points

| Section | Requirement |
| :--- | :--- |
| **Context** | State with facts and constraints "why the decision must be made now"; link the spec / incident |
| **Alternatives** | Alternatives are not mandatory: when there is a real architectural trade-off, give **2–3 options** (including a "do nothing" option), each with pros and cons; where a best practice or de facto standard exists, cite it as the basis in place of alternatives |
| **Decision** | An actionable one-sentence conclusion |
| **Rationale** | Align with the organization principles (P-xxx) and the domain standard clauses; explain the trade-off |
| **Impact** | Which modules / teams / documents are affected (hld, specifications, callers) |
| **Follow-ups** | Follow-up items with an Owner and a deadline |

- **One ADR, one decision**: split multiple decisions into multiple ADRs to keep them individually referenceable
- **Write for others**: assume readers lack the context; avoid abbreviations only the current team understands

## 3. Status Transitions

```
proposed (draft) → decision review
  ├─ accepted (accepted, in force)
  │     └─ superseded (replaced by ADR-xxx, with a link noted; the original text is neither deleted nor modified)
  └─ rejected (rejected, kept — "why we did not do it this way" is knowledge too)
deprecated (the decision still holds but is no longer recommended for new cases)
```

- ❌ Modifying the content of an accepted ADR is forbidden; changes go through a new ADR + a superseded marker
- ADR numbers increase globally (ADR-001, ADR-002, …)

## 4. Quality Self-Check

- [ ] Six months later, a new colleague can understand the decision by reading only this ADR
- [ ] Where there is a real trade-off: the alternatives include "do nothing" and the cons are real (not a straw man); where a de facto standard was adopted: the basis is explicit and checkable
- [ ] The rationale cites checkable evidence (principle numbers, specification clauses, measurement data)
