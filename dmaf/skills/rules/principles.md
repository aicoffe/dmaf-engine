---
name: organization-principles
level: L0
owner: CTO
version: v1.0.0
effective_date: 2026-10-17
review_cycle: every year
---

# Organization Principles

This file is the highest-level specification. All domain standards (L1), work specifications (L2), and project specifications (L3) must conform to this file. When specifications conflict, adjudicate by the priority defined here.

---

## P-001: Security over Experience

**Principle**: When security requirements conflict with user experience, security wins, but a solution that accommodates both must be sought.

**Implications**:
- Security is the floor; it must not be compromised for experience
- But "security first" does not mean "sacrificing experience" — a solution that accommodates both must be actively sought
- When no such solution can be found, the security Owner and the product owner adjudicate together

**Conflict adjudication example**:
- Security requires logout in 5 minutes vs UX requires seamless refresh → seek a solution that accommodates both (e.g., extend to 30 minutes + secondary authentication for sensitive operations)

**Applies to**: architecture, security, ux, observability, ai-usage

---

## P-002: Compliance over Business Flexibility

**Principle**: When compliance requirements conflict with business needs, compliance wins and the business must adapt.

**Implications**:
- Compliance is a hard constraint and is not negotiable
- Business needs must adapt to compliance requirements, not the other way around
- If a compliance requirement severely impacts the business, escalate to the compliance officer and the business owner for joint adjudication

**Conflict adjudication example**:
- Compliance requires 3 years of data retention vs the business wants to keep it forever → follow compliance; delete after 3 years

**Applies to**: security, data

---

## P-003: Data Minimization

**Principle**: Collect only data the business requires; do not collect data that "might be useful later".

**Implications**:
- The purpose must be clear before collection
- Data with no clear purpose is not collected
- Existing data is reviewed periodically; data with no purpose is deleted

**Conflict adjudication example**:
- The product team wants to collect user behavior data "for future analysis" → do not collect unless the analysis goal is explicit

**Applies to**: security, data, ux, observability

---

## P-004: Observability over Implementation Convenience

**Principle**: Every critical path must have logs, metrics, and distributed tracing.

**Implications**:
- Each domain standard refines the definition of a critical path
- Observability is not "added later" but "there from the start"
- Changes lacking observability must not merge; enforced through automated gates (gate rules are defined by the work specifications)

**Conflict adjudication example**:
- Implementation convenience wants to skip logging vs observability requires recording → record the logs

**Applies to**: observability, performance, reliability

---

## P-005: Backward Compatibility over Code Cleanliness

**Principle**: Interface changes must be backward compatible; keeping deprecated code is preferable to breaking callers.

**Implications**:
- Changes to external interfaces (APIs, events, data models) must be backward compatible
- Deprecate rather than delete; the retention period of a deprecated item must be declared explicitly (defined by the domain standards)
- Breaking changes require approval from the Architecture Group

**Conflict adjudication example**:
- A refactor wants to delete an old field vs callers still using it → keep the old field and mark it deprecated

**Applies to**: architecture, data

---

## P-006: Automation over Manual Work

**Principle**: Checks that can be automated do not rely on manual review.

**Implications**:
- Repetitive checks must be automated (CI, hooks, Eval)
- Manual review focuses on judgment work, not mechanical checking
- Automated check rules must be version-controlled

**Conflict adjudication example**:
- Manual review of code style vs automated lint → automate the lint; manual review focuses on logic

**Applies to**: quality, engineering, observability, ai-usage

---

## P-007: Explicit over Implicit

**Principle**: Configuration, dependencies, and constraints must be declared explicitly; do not rely on default behavior.

**Implications**:
- Configuration items must be written out explicitly; do not rely on framework default values
- Dependencies must be declared explicitly; do not rely on transitive dependencies
- Constraints must be documented explicitly; do not rely on word of mouth

**Conflict adjudication example**:
- The framework's default timeout is 30 seconds vs leaving it unconfigured → the timeout must be configured explicitly

**Applies to**: architecture, data, observability, quality, engineering, reliability, frontend

---

## P-008: Maintainability over Short-Term Delivery

**Principle**: Code is written for future readers; maintainability must not be sacrificed for short-term delivery.

**Implications**:
- Do not skip tests, documentation, or refactoring to hit a deadline
- Technical debt must be recorded explicitly; it must not accumulate silently
- Complex logic must have comments or documentation

**Conflict adjudication example**:
- Delivery pressure wants to skip tests vs maintainability requires tests → tests cannot be skipped; scope can be reduced

**Applies to**: architecture, quality, engineering, performance, reliability, frontend

---

## P-009: User Experience Is Part of the Feature

**Principle**: A usable feature is not a finished feature; user experience is part of the feature.

**Implications**:
- Loading, empty, error, and edge states must be handled
- Accessibility is a basic requirement, not an option
- Copy and interactions must conform to the brand guidelines

**Conflict adjudication example**:
- The backend API works vs the frontend has no error handling → error handling is part of the feature and must be implemented

**Applies to**: ux, data, performance, frontend

---

## P-010: Humans Are Accountable for High-Risk Decisions

**Principle**: AI may generate, recommend, and detect, but high-risk decisions must be made by humans.

**Implications**:
- Architecture decisions, security policies, and compliance judgments require a human signature
- AI provides options and recommendations only; it does not decide in their place
- Every high-risk decision must record the decision maker and the time

**Conflict adjudication example**:
- AI recommends an option vs the human chooses an alternative → the human decision prevails; record the rationale

**Applies to**: all domains

---

## Conflict Adjudication Priority

When specifications conflict, adjudicate by the following priority:

| Priority | Level | Description |
| :--- | :--- | :--- |
| **1** | L0 principles | Highest priority; this file |
| **2** | L1 domain standards | Divided by domain |
| **3** | L2 work specifications | Divided by work type |
| **4** | L3 project specifications | Divided by feature |

**Adjudication process**:
1. On conflict, look for the upper-level specification both sides commonly depend on
2. If that upper-level specification provides a basis → adjudicate by it
3. If it provides none → consult L0
4. If L0 provides none → record it in Areas of Concern; humans adjudicate

**Priority within L0**:
- P-001 (security) and P-002 (compliance) are red lines with the highest priority
- P-010 (human decisions) is a procedural principle binding every decision process
- When the remaining principles conflict, the relevant domain Owners adjudicate together

---

## Changing the Principles

| Change type | Process | Approver |
| :--- | :--- | :--- |
| **Adding a principle** | proposal → review → approval | CTO + the relevant domain Owner |
| **Modifying a principle** | proposal → review → approval | CTO + the relevant domain Owner |
| **Removing a principle** | proposal → review → approval | CTO + the Board |
| **Modifying the adjudication priority** | proposal → review → approval | CTO |

**Review cycle**: reviewed every year, or triggered by a major event.

---

## Appendix: Principle-to-Domain-Standard Mapping

| Principle | Domain standards bound |
| :--- | :--- |
| P-001 security first | `rules/standards/architecture-standard`, `rules/standards/security-standard`, `rules/standards/ux-standard`, `rules/standards/observability-standard`, `rules/standards/ai-usage-standard` |
| P-002 compliance first | `rules/standards/data-standard`, `rules/standards/security-standard` |
| P-003 data minimization | `rules/standards/security-standard`, `rules/standards/data-standard`, `rules/standards/ux-standard`, `rules/standards/observability-standard` |
| P-004 observability | `rules/standards/observability-standard`, `rules/standards/performance-standard`, `rules/standards/reliability-standard` |
| P-005 backward compatibility | `rules/standards/architecture-standard`, `rules/standards/data-standard` |
| P-006 automation first | `rules/standards/quality-standard`, `rules/standards/engineering-standard`, `rules/standards/observability-standard`, `rules/standards/ai-usage-standard` |
| P-007 explicit over implicit | `rules/standards/architecture-standard`, `rules/standards/data-standard`, `rules/standards/observability-standard`, `rules/standards/quality-standard`, `rules/standards/engineering-standard`, `rules/standards/reliability-standard`, `rules/standards/frontend-standard` |
| P-008 maintainability | `rules/standards/architecture-standard`, `rules/standards/quality-standard`, `rules/standards/engineering-standard`, `rules/standards/performance-standard`, `rules/standards/reliability-standard`, `rules/standards/frontend-standard` |
| P-009 user experience | `rules/standards/ux-standard`, `rules/standards/data-standard`, `rules/standards/performance-standard`, `rules/standards/frontend-standard` |
| P-010 human decisions | all domains |

> Note: `compliance-standard` and `deployment-standard` are deferred (see the specification list in this directory's README). Once established, compliance joins the scope of P-001 / P-002 / P-003 and deployment joins the scope of P-006 / P-007, and both are added to this table. The L2 work specifications (`guides/`) are bound by the principles above together with their domains and are not listed separately in this table.
