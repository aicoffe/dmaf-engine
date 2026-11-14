---
name: ai-usage-standard
level: L1
domain: ai-usage
description: Load when AI-assisted coding, AI-generated code review, AI usage boundaries, or AI context security is involved. Governs all development work involving AI.
owner: Platform Team
based_on: [organization-principles, security-standard, quality-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# AI Usage Domain Standard

> This standard governs the boundaries and red lines of AI usage.

---

## 1. Usage Boundaries (Allowed / Not Allowed)

**AI may do** (outputs must go through human review):
- Code generation, refactoring suggestions, test case generation
- Documentation drafting, code explanation, defect localization assistance
- Standard retrieval and example generation

**AI must not do (human required)** (P-010):
- **Final decisions** on architecture decisions, technology selection, security policy, and compliance judgment
- Executing production deployments, changing security configurations, changing permissions
- Modifying and granting exemptions to quality gate rules
- Adjudication and approval of standards (L0–L3)

---

## 2. Quality Requirements for AI-Generated Code

- **Same standard**: AI-generated code is subject to the same quality gates (see `rules/standards/quality-standard`) and the same review requirements (see `rules/standards/engineering-standard`) as human-written code
- **Traceable**: AI-generated code must be flagged in the commit message (default requirement, for traceability and incident attribution)
- **Human review mandatory checks** (P-010):
  - Hallucinated APIs / nonexistent dependencies (must be verified via compilation and dependency locking)
  - Hardcoded secrets, credentials, sensitive information (secret red line: see `rules/standards/security-standard`)
  - Code snippets introducing incompatible licenses
  - Excessive complexity, deviation from existing architecture patterns
  - Whether tests genuinely cover behavior (tautological assertions are forbidden; carried by the corresponding domain skills)

---

## 3. Context and Data Security

- Content forbidden in AI prompts/context:
  - L3/L4 data and unmasked PII (data classification: see `rules/standards/data-standard`)
  - Secrets, credentials, tokens
  - Production customer data
- AI tool selection and configuration (models, plugins, data retention and training policy) must be approved by the security team
- AI session records are treated as a potential leakage surface: sessions involving sensitive projects are managed per the organization's data classification

---

## 4. AI Security Red Lines

- AI must not execute production deployment commands (unless there is a release approval)
- AI must not modify or bypass CI/CD gate configuration (changes to quality gates must go through human review)
- AI must not relax security configurations on its own (changes to authentication, encryption, and masking rules require human approval)
- When AI recommendations conflict with human decisions, the human decision prevails and the rationale is recorded (P-010)

---

## 5. AI Cost Control (As Needed)

- Set usage budgets per team/project with alerts on abnormal usage
- Prefer high-repetition, low-risk scenarios (boilerplate code, test generation, documentation drafts)
- In high-risk scenarios (security-related, core transaction logic), restrict free-form AI generation and adopt the "AI drafts + human line-by-line confirmation" mode

---

## 6. Conflict Adjudication with Other Domains

### 6.1 Efficiency vs Security

- **Adjudication basis**: P-001 (security first)
- **Example**: AI querying the production database directly is "faster" vs data security → security first; use the masked data channel

### 6.2 Efficiency vs Decision Accountability

- **Adjudication basis**: P-010 (humans are accountable for high-risk decisions)
- **Example**: adopting AI recommendations directly for speed vs high-risk decisions requiring human sign-off → human signs off; AI only provides options

### 6.3 Adjudication Process

- The platform team Owner and the security Owner adjudicate jointly; adjudication of standards escalates per the L0 conflict resolution process
