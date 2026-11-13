---
name: quality-standard
level: L1
domain: quality
description: Load when test strategy, code quality, quality gates, defect management, or quality metrics is involved. Governs all quality-related work.
owner: Testing Team
based_on: [organization-principles, architecture-standard, security-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Quality Domain Standard

> This standard governs quality standards and red lines; implementation details are carried by quality domain skills.

---

## 1. Quality Definition

Quality = functional correctness + security + maintainability + observability. Quality is the shared responsibility of all roles, not the exclusive domain of the testing team; the testing team is the measurer and gatekeeper of quality.

---

## 2. Test Layering

| Layer | Share (default) | Responsibility | Characteristics |
| :--- | :--- | :--- | :--- |
| **Unit tests** | ~70% | Domain logic, business rules, utilities | Millisecond-level, no external resource dependencies |
| **Integration tests** | ~20% | Module collaboration, database, messaging, cache | May depend on test containers (Testcontainers) |
| **End-to-end tests** | ~10% | Key user paths, cross-system flows | Restrained in number; cover only core flows |

**Principle**: the test pyramid, lower layers first; ❌ an inverted pyramid is forbidden (an E2E-dominated test structure is costly to maintain and slow to give feedback).

---

## 3. Test Type Requirements

| Type | Requirement |
| :--- | :--- |
| **Functional tests** | Must cover core use cases and boundary conditions |
| **Contract tests** | Interfaces between services must have contract tests |
| **Security tests** | Dependency vulnerability scanning must be in CI; key OWASP Top 10 items must have test cases (requirements: see `rules/standards/security-standard`) |
| **Performance tests** | Core interfaces must verify the performance budget (see `rules/standards/performance-standard`) |
| **Regression tests** | Defect fixes must add a regression case first (red before green) |

---

## 4. Code Quality

| Requirement | Description |
| :--- | :--- |
| **Static checks** | Lint, complexity, duplication must be in CI; blocking violations must not merge |
| **Cyclomatic complexity** | Default ≤ 10; over the limit requires refactoring or a documented exemption rationale |
| **Duplication rate** | Default ≤ 5% |
| **Comments** | Complex logic must have comments or documentation (P-008) |
| **Coding conventions** | Formatting and style are left to tools and CI; human review does not check style |

---

## 5. Quality Gates

The following automated checks must pass before merging (P-006):

- [ ] Static checks with zero blocking issues
- [ ] Unit + integration tests all passing
- [ ] Coverage targets met (default values: overall ≥ 70%, core modules ≥ 85%)
- [ ] Architecture boundary checks passing (see `rules/standards/architecture-standard` architecture checks)
- [ ] Dependency vulnerability scanning with no Critical findings (see `rules/standards/security-standard`)
- [ ] Observability check items passing (see `rules/standards/observability-standard` checklists)

**Requirements**:
- Gate rules must be version-controlled (automation first, rules as code)
- Exemptions must record the rationale, the responsible person, and an expiry date; they expire automatically when due

---

## 6. Defect Management

| Level | Definition | Response requirement |
| :--- | :--- | :--- |
| **P0** | Production incidents, data errors, security vulnerabilities | Fix immediately |
| **P1** | Core features unavailable, wrong results | Fix in the current iteration |
| **P2** | Non-core feature anomalies | Scheduled into an iteration |
| **P3** | Minor issues, experience-related | Scheduled for handling |

**Requirements**:
- Every defect must record: symptom, root cause, fix, preventive measures
- P0 defects must be reviewed; improvement items from the review must land as gate rules or test cases

---

## 7. Quality Metrics

| Metric | Target (default values) |
| :--- | :--- |
| **Coverage** | Overall ≥ 70%, core modules ≥ 85% |
| **Defect escape rate** | Share of defects escaping to production; trend downward |
| **Change failure rate** | ≤ 15% |
| **P0 defect count** | Zero tolerance (any occurrence triggers a review) |

---

## 8. Conflict Adjudication with Other Domains

### 8.1 With Delivery Schedule

- **Adjudication basis**: P-008 (maintainability over short-term delivery)
- **Adjudication process**: Tech Lead + Quality Owner adjudicate jointly
- **Example**: delivery pressure vs testing → testing cannot be skipped; scope may be reduced (keep core paths), with the risk explicitly recorded

### 8.2 With Security

- **Adjudication basis**: P-001 (security first)
- **Adjudication process**: Security Owner + Quality Owner adjudicate jointly
- **Forbidden**: omitting security testing and vulnerability scanning for any reason is forbidden

### 8.3 With Manual Efficiency

- **Adjudication basis**: P-006 (automation first)
- **Example**: manual regression vs automation → regression that can be automated must be automated; humans focus on exploratory testing
