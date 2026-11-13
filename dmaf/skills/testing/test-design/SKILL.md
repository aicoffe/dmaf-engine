---
name: test-design
description: Test design. Test strategy, case design methods (equivalence partitioning / six-point boundary analysis / scenario method / decision table), test data construction. Use when the user asks to "design test cases", "test strategy", "write a test plan", or "prepare test data".
---

# Test Design

Cases are derived from acceptance criteria: every acceptance criterion maps to at least one case; a case with no source and an acceptance criterion with no case are both defects.

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and act according to its state — one of four cases (when no declaration is provided, treat it as declaration absent):

- **Bound**: load the stack skill directory the "Binding" section points to — `SKILL.md` is the entry point and required reading, then route by the trigger conditions in its "Reference files" table to load the `reference/` documents for the domains the current task touches (one task reads only its own domain; dumping the whole directory is forbidden); when a stack rule conflicts with this skill, the more specific stack rule wins — escalate to a human if it cannot be adjudicated
- **Binding is "none" (generalized mode)**: execute with the neutral standard + model generalization based on the stack names declared in `backend_stack` / `frontend_stack`; the verification loop follows the declaration's "Verification commands" section; label artifacts "Generalized mode (<stack name>, no binding skill)"
- **Declaration absent**: execute per this skill's neutral standard; label artifacts "No tech stack declared for this project"
- **Declaration unparseable**: report as-is; do not guess

## Workflow

### 1. Layering strategy
- Unit ~70% (domain logic, millisecond-scale, no I/O) / integration ~20% (module collaboration, databases, messaging) / E2E ~10% (core user paths)
- The inverted test pyramid (E2E-dominated) is forbidden
- Coverage baseline (industry convention): branch coverage leads for domain logic, line coverage only as a floor reference — coverage is a means of cross-checking the design, not a goal; tests written for the coverage number are invalid
- Risk-driven ordering: rank case priority by failure probability × blast radius — core paths such as money and state transitions come before edge presentation paths

### 2. Case sources
```
spec acceptance criteria (Given-When-Then)    → main-scenario cases
lld exception and degradation design          → exception-path cases
boundary and rule analysis                    → supplementary cases
historical defects                            → regression cases
```

### 3. Design methods
| Method | Usage |
| :--- | :--- |
| Equivalence partitioning | ≥ 1 valid class + 1 per invalid class |
| Six-point boundary analysis | min-1/min/min+1/max-1/max/max+1 |
| Scenario method | main flow + one alternate flow per branch point |
| Decision table | one per cell for multi-condition rules |
| Error guessing | historical defect patterns, concurrency, retries, null combinations |
- Combinatorial explosion control: for multi-parameter equivalence-class combinations, use pairwise (industry convention) to cover parameter interactions; expanding the full combination is forbidden; add dedicated cases for known strongly coupled specific combinations

### 4. Test data
- Build with Builder/Fixture; a case explicitly sets only the fields related to what it asserts
- Self-contained: depending on the execution order of other tests is forbidden
- Production data must not be used directly (de-identify + approve); fixtures must not contain real PII

### 5. Case quality self-check
- [ ] Acceptance criteria mapping complete
- [ ] Six-point boundary analysis complete
- [ ] Exception and degradation paths covered (every branch in the lld design)
- [ ] Cases mutually independent and runnable on their own
- [ ] Assertions specific (verify field values, not "no exception thrown")

## Boundaries (not in this skill)

- TDD loop execution and landing the tests belong to `automated-testing`; this skill produces only case design
- Inter-service contract verification belongs to `contract-testing`

## Reference files

- `rules/standards/quality-standard.md` — layering and gate requirements

## Default values

The layering ratio (70/20/10) and six-point boundary analysis are default values, adjustable per the project's risk profile and recorded.
