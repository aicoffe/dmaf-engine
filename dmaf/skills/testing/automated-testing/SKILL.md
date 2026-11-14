---
name: automated-testing
description: Automated testing and TDD. Red → Green → Refactor loop, unit testing, integration testing (Testcontainers). Use when the user asks to "write tests", "do TDD", "write unit tests", "write integration tests", or "add tests". Tests come first; writing code first and backfilling tests afterwards is forbidden.
---

# Automated Testing (TDD)

Tests first: write a failing test, then the minimum code to pass it, refactoring while keeping the bar green.

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and act according to its state — one of four cases (when no declaration is provided, treat it as declaration absent):

- **Bound**: load the stack skill directory the "Binding" section points to — `SKILL.md` is the entry point and required reading, then route by the trigger conditions in its "Reference files" table to load the `reference/` documents for the domains the current task touches (one task reads only its own domain; dumping the whole directory is forbidden); when a stack rule conflicts with this skill, the more specific stack rule wins — escalate to a human if it cannot be adjudicated
- **Binding is "none" (generalized mode)**: execute with the neutral standard + model generalization based on the stack names declared in `backend_stack` / `frontend_stack`; the verification loop follows the declaration's "Verification commands" section; label artifacts "Generalized mode (<stack name>, no binding skill)"
- **Declaration absent**: execute per this skill's neutral standard; label artifacts "No tech stack declared for this project"
- **Declaration unparseable**: report as-is; do not guess

## Workflow

### 1. TDD loop (per behavior point, minute-scale)
- **Red**: write one failing test from an acceptance criterion; run it and confirm the failure reason is "feature not implemented" (not a syntax/wiring error)
- **Green**: write the minimum code to make the test pass; modifying the test file is forbidden
- **Refactor**: refactor for readability; keep the bar green after every refactor
- **Verify**: unit + integration tests, build, and lint all pass before committing; on failure, fix and retry at most 3 times; if it still fails, mark blocked and report

Bug-fix variant: write a reproduction test first (failing because of the bug, not because the test is wrong) → fix → refactor. The reproduction test is a regression test and is kept permanently.

### 2. Unit tests
- Scope: domain logic, business rules, boundaries; single test ≤ 100ms, suite ≤ 5 min; I/O forbidden
- Parameterize first: the same logic with multiple input sets uses parameterized tests, with data rows commented for their business meaning
- Naming and structure: test names describe behavior (given_when_then semantics), AAA (Arrange-Act-Assert) three-part structure; no ordering dependency between tests
- Test-double selection: **Stub** (feeds fixed return values, never asserts interactions) / **Mock** (asserts "the call happened") / **Fake** (lightweight real implementation, e.g. an in-memory database) — decision rule: use a Stub when a given input state is all you need; a Mock when you must verify the collaboration was triggered; a Fake when stateful logic is too complex for a Stub; mocking value objects and domain-cohesive units is forbidden (test the real behavior)
- Mock at the boundary, not the inside; prefer real or in-memory implementations
- Anti-patterns: if/for inside tests, asserting implementation details, sleep-based waiting, tautological assertions (copying the implementation into the assertion)
- TDD metrics: compliance rate (share of tasks that test before code), test-first rate (test commit earlier than the production code), number of times tests were modified (must be 0)

### 3. Integration tests
- For databases, start a real PostgreSQL via Testcontainers (same version as production); H2 and similar stand-ins producing false greens are forbidden
- For third-party HTTP, simulate per contract with WireMock, injecting timeout/failure scenarios
- Containers start once per suite; data is independent per case (transaction rollback, or unique prefix + teardown)
- Single test ≤ 10s, suite ≤ 15 min (default values); included in the CI gate

### 4. Assertion quality
- No assertions at all, or asserting only "no exception thrown", is forbidden; floating-point assertions take a tolerance
- Coverage met but assertions thin is treated as invalid

## Boundaries (not in this skill)

- Test-case design methods (equivalence partitioning / six-point boundary analysis / scenario method / decision table) belong to `test-design`; this skill covers TDD loop execution and landing the tests
- Inter-service contract verification belongs to `contract-testing`

## Reference files

- `rules/standards/quality-standard.md` — layering and gate requirements

## Default values

Unit single test ≤ 100ms / suite ≤ 5 min, integration single test ≤ 10s / suite ≤ 15 min, retry 3 times are organization defaults (adjustable per project).
