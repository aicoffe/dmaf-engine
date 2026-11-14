---
name: code-review
description: Code review. PR review checklists, comment severity levels (block/suggest/nit), specialized review of AI-generated code, security-dimension review. Use when the user asks to "review code", "look at a PR", "review AI-generated code". Style issues go to tooling; human review focuses on correctness, security, and maintainability.
---

# Code Review

Approval = shared ownership. The three review questions: Is it correct? Is it secure? Is it maintainable?

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and act according to its state — one of four cases (when no declaration is provided, treat it as declaration absent):

- **Bound**: load the stack skill directory the "Binding" section points to — `SKILL.md` is the entry point and required reading, then route by the trigger conditions in its "Reference files" table to load the `reference/` documents for the domains the current task touches (one task reads only its own domain; dumping the whole directory is forbidden); when a stack rule conflicts with this skill, the more specific stack rule wins — escalate to a human if it cannot be adjudicated
- **Binding is "none" (generalized mode)**: execute with the neutral standard + model generalization based on the stack names declared in `backend_stack` / `frontend_stack`; the verification loop follows the declaration's "Verification commands" section; label artifacts "Generalized mode (<stack name>, no binding skill)"
- **Declaration absent**: execute per this skill's neutral standard; label artifacts "No tech stack declared for this project"
- **Declaration unparseable**: report as-is; do not guess

## Workflow

### 1. PR description completeness (reject and return if missing)
Background (linked spec/defect number), what changed, how it was verified, risks and blast radius, checklist (tests added / docs updated / AI-authored portions marked)

### 2. Review checklist
| Dimension | Checkpoints |
| :--- | :--- |
| Correctness | Logic correct, boundaries (empty/zero/extremes), concurrency safety |
| Security | Input validation, unauthorized access (horizontal/vertical), sensitive data, injection surface |
| Maintainability | Clear naming, no duplication, respects layering |
| Testing | New logic has tests, assertions are meaningful (not tautologies) |
| Compatibility | API/data changes are backward compatible |
| Observability | Critical paths have logs/metrics |

### 3. Comment severity
- `block:` must be fixed before merge (defects, security issues) — must point to concrete lines and offer an alternative implementation
- `suggest:` recommended to adopt; the author may decline with a written rationale
- `nit:` a minor style point, non-blocking

### 4. AI-generated code specialization
- Confirm the commit message marks the AI-generated scope; PRs with a high AI share (> 50%) add one more reviewer
- Hallucination detection: compilation must pass; verify each new dependency name against the official registry (guard against look-alike package names); check invented configuration keys against config
- Security: hardcoded secrets, license contamination, missing permission annotations, SQL/command concatenation
- Test validity: tautological assertions are forbidden; red→green evidence is required

### 5. Process discipline
- First response within 1 business day; when busy, leave a comment with the expected time
- block comments must be fixed or escalated for adjudication; after each revision round, re-request review and state what changed
- Confirm CI is finally green before merge

## Boundaries (not in this skill)

- PR branch operations, opening, and merging go through `git-workflow`; this skill covers only review content standards (what to look at, how to judge)

### 6. Security specialization review
- Trigger depth: authentication/authorization/permission changes, or processing involving PII/L3+ data → mandatory security specialization review (the security Owner participates); ordinary business logic goes through the regular security dimension
- Abbreviated STRIDE threat modeling (walk every new trust boundary item by item): spoofing — is authentication covered?; tampering — input validation, signature-based replay protection?; repudiation — audit logs on critical operations?; information disclosure — PII masked, error messages leak nothing, transport and storage encrypted?; denial of service — rate limiting, oversized-request protection?; elevation of privilege — data-level permissions, default deny?
- Pre-merge security self-check: zero new unauthenticated paths, unauthorized-access tests (horizontal ID swap / vertical privilege promotion), input validation complete, logs and responses free of raw PII and secrets, dependency and secret scans pass, no injection surface, all ten OWASP items confirmed one by one or marked not applicable

## Anti-patterns

- Rubber stamping (approving without reading the code), nit bombardment drowning real issues, sneaking in unrelated refactors, verdicts left verbal with no PR record

## Reference files

- `rules/standards/quality-standard.md` / `rules/standards/security-standard.md` / `rules/standards/engineering-standard.md` — higher-level constraints
