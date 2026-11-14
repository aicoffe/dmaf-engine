---
name: rules
description: >
  Charter-layer index and loading entry for the organization's specification standards.
  L0 principles (principles.md), L1 domain standards (11 *-standard files under standards/,
  the single source of truth for numeric baselines and red lines), shared clause tables
  (5 files under guides/, parameter and format clauses consumed by multiple skills),
  L3 project specifications. Conflicts between specifications are adjudicated
  L0 > L1 > L2 > L3; unresolvable conflicts escalate to a human. Use this index when
  looking up red lines, numeric baselines, or shared clauses, or when adjudicating
  specification conflicts.
---

# Specification Charter Layer Index

This directory is the organization's specification charter layer, organized into levels L0 → L3. This file is the index and usage entry for all specification files.

```
L0 principles (principles.md) — adjudication order and organization red lines; load when adjudicating conflicts
    ↓ binds
L1 domain standards (standards/{domain}-standard.md) — single source of truth for numeric baselines and domain red lines; referenced across skills, never duplicated
    ↓ refines
L2 shared clause tables (guides/{work}-guide.md) — parameter and format clauses consumed by multiple skills
    ↓ layered on top
L3 project specifications (specs/{feature}/project-rules.md) — feature-specific constraints, stored with the project, location decided by the project
```

- **Conflict adjudication priority**: L0 > L1 > L2 > L3; look upward for the adjudication basis; unresolvable conflicts escalate to a human
- **Naming convention**: L1 files uniformly use the `-standard` suffix; L2 shared clause tables uniformly use the `-guide` suffix (not `-skill`, to avoid confusion with AI skills)

---

## 1. L0 Principles (1 file)

| File | Description | Owner | Approval |
| :--- | :--- | :--- | :--- |
| [principles.md](principles.md) | 10 organization principles (P-001–P-010): red lines, values, conflict adjudication priority; required reading when adjudicating spec conflicts | CTO | CTO + domain Owner |

---

## 2. L1 Domain Standards (11 files)

Load by the trigger conditions in each file's frontmatter `description`; the single source of truth for numeric baselines and red lines — lower layers reference, never duplicate.

| File | Domain | Owner | Core content |
| :--- | :--- | :--- | :--- |
| [architecture-standard](standards/architecture-standard.md) | architecture | Architecture Group | Architecture styles, layering, module/service boundaries, integration patterns, architecture evolution and governance |
| [security-standard](standards/security-standard.md) | security | Security Team | Authentication, authorization, interface security, data security, key management, OWASP baseline, AI red lines |
| [data-standard](standards/data-standard.md) | data | DBA Team | Data classification (the sole definition source), modeling, database design, migration, lifecycle, backup, governance |
| [observability-standard](standards/observability-standard.md) | observability | SRE Team | The three pillars of logging/metrics/tracing, alerting, dashboards, observability design and governance |
| [ux-standard](standards/ux-standard.md) | ux | Design Team | Brand tone, design system principles, interaction and state principles, component reuse, responsiveness |
| [quality-standard](standards/quality-standard.md) | quality | Testing Team | Test layering, code quality, quality gates, defect management, quality metrics |
| [engineering-standard](standards/engineering-standard.md) | engineering | Tech Lead | Branch strategy, commit conventions, code review, documentation, dependency management, version release |
| [performance-standard](standards/performance-standard.md) | performance | Architecture Group + SRE Team | Performance budgets, optimization principles, performance testing, capacity planning |
| [reliability-standard](standards/reliability-standard.md) | reliability | SRE Team | Failure design, redundancy and isolation, rate limiting and overload, degradation fallbacks, disaster recovery, chaos engineering |
| [frontend-standard](standards/frontend-standard.md) | frontend | Frontend Team | Frontend layering, state management, component conventions, compatibility, frontend performance |
| [ai-usage-standard](standards/ai-usage-standard.md) | ai-usage | Platform Team | AI usage boundaries, generated code quality, context data security, AI red lines, cost |

---

## 3. L2 Shared Clause Tables (5 files)

Parameter and format clauses consumed by multiple skills. Knowledge consumed by a single skill lives in that skill; document-filling templates travel with the producing skill — none of these belong at this level.

| File | Content | Primary consumers |
| :--- | :--- | :--- |
| [api-design-guide](guides/api-design-guide.md) | Full REST clause set: paths / methods / status codes / error responses / timeouts / idempotency / pagination | detailed-design-backend, backend-development, contract-testing |
| [interaction-design-guide](guides/interaction-design-guide.md) | Interaction modes and parameters: three-state thresholds, toasts, confirmation dialogs, form validation, pagination | frontend-development, detailed-design-frontend, UE review |
| [component-design-guide](guides/component-design-guide.md) | Component design clauses: Props conventions, reuse ladder, accessibility and testing | frontend-development, detailed-design-frontend, design-system |
| [adr-authoring-guide](guides/adr-authoring-guide.md) | When to write an ADR, key authoring points, and status transitions | hld-design, tech-selection, architecture-analysis, documentation |
| [tech-stack-guide](guides/tech-stack-guide.md) | The stack binding registry and the off-registry introduction process | architecture-analysis, tech-selection, each stack-binding preamble |

---

## 4. L3 Project Specifications

When working on a feature, read the project specification `project-rules.md` (feature-specific constraints, stored with the project, location decided by the project, not in this directory).

---

## 5. Loading Rules

1. **L0**: `principles.md` is required reading when adjudicating spec conflicts or looking up organization red lines
2. **L1**: when work touches a domain, load the corresponding `{domain}-standard.md` (the single source of truth for numbers and red lines)
3. **L2 shared clause tables**: loaded via pointers from the consuming skills
4. **L3**: when working on a feature, read the project specification

---

## 6. Change Management

- **L0**: adding or modifying a principle requires approval from the CTO + the relevant domain Owner
- **L1 / L2**: changes are reviewed by the domain Owner, `review_cycle: every six months`; adjustments to numeric default values must be recorded
- **Consistency requirement**: shared clause tables must not conflict with L1; L1 numbers have exactly one source — lower layers reference rather than copy; this index is updated in step with files added or removed
- **Reference form**: L1 does not reference L2/L3, L2 does not reference L3, and rules does not point at skill or artifact sections; a reference is kept only when removing it would cause something to be done wrongly or missed; cross-file references go to file granularity only, never §N; body text carries no justification or provenance descriptions
