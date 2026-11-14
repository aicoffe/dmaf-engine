---
name: engineering-standard
level: L1
domain: engineering
description: Load when Git workflow, branching strategy, code review, documentation conventions, dependency management, or release versioning is involved. Governs all engineering collaboration work.
owner: Tech Lead
based_on: [organization-principles, quality-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Engineering Practices Domain Standard

> This standard governs collaboration processes and red lines; implementation details are carried by the corresponding domain skills.

---

## 1. Branching Strategy

- **Trunk-based by default**: short-lived branches + PR merges into the mainline; default branch lifetime ≤ 3 days
- Branch naming: `{type}/{ticket}-{summary}`, where type is feat / fix / chore / docs / refactor / test
- The main branch is protected: direct push is forbidden; changes must go through PRs
- The main branch must remain releasable at all times

---

## 2. Commit Conventions

- **Format**: Conventional Commits, `{type}({scope}): {summary}`
- One commit, one intent; mixing unrelated changes into the same commit is forbidden
- Local gates (lint + unit tests) must pass before committing

---

## 3. Code Review

| Requirement | Description |
| :--- | :--- |
| **Review required** | All changes must go through PR + at least 1 approval; core modules require 2 |
| **PR size** | Default ≤ 400 changed lines; over the limit the PR must be split |
| **Review focus** | Correctness, security, maintainability, test sufficiency; style issues are handled by automated tools (P-006) |
| **Response time** | First response within 1 business day on business days (default value) |
| **AI-generated code** | Reviewed by the same standard as human-written code; human reviewers must check it (AI usage red lines: see `rules/standards/ai-usage-standard`) |

---

## 4. Documentation Conventions

- Documentation is version-controlled in the same repository as code and updated in sync; stale documentation is treated as a defect
- README must include: purpose, how to run locally, how to test
- Architecture decisions are recorded as ADRs

---

## 5. Dependency Management

- Dependency versions must be locked (lockfiles committed to the repository)
- New dependencies must be reviewed: license compatibility, active maintenance, no known Critical vulnerabilities
- Dependency vulnerabilities must be scanned continuously (in CI); Critical vulnerabilities are fixed within a time limit
- Periodic upgrades: patch version upgrades once a month by default; minor version upgrades evaluated once a quarter

---

## 6. Version Releases

- **Semantic versioning** (SemVer): `major.minor.patch`
- Breaking changes must bump the major version and provide migration notes
- A CHANGELOG must be maintained; every release records additions, changes, deprecations, and removals

---

## 7. Conflict Adjudication with Other Domains

### 7.1 With Delivery Schedule

- **Adjudication basis**: P-008 (maintainability over short-term delivery)
- **Adjudication process**: Tech Lead + Quality Owner adjudicate jointly
- **Example**: skipping review to hit a deadline → not skippable; speed things up by splitting into smaller PRs and adding parallel reviewers instead

### 7.2 With Automation

- **Adjudication basis**: P-006 (automation first)
- **Example**: manually checking dependency licenses vs automated scanning → automated scanning; humans only handle alerts
