---
name: detailed-design-backend
description: 'Backend feature-level detailed design. Produces the backend lld from spec and hld: API design, class design, algorithms, state machines, sequences, database physical design, caching and messaging schemes, exception degradation, acceptance-criteria mapping. Use when the user asks to "do backend detailed design", "write the backend lld", "design APIs/endpoints", "design table structures", "data modeling/table creation/migration", "caching strategy", "message/event modeling". Describes only this change; makes no architecture decisions.'
---

# Backend Detailed Design

Turn architecture and requirements into a backend implementation design ready for TDD. References the hld; describes only this change.

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and act according to its state — one of four cases (when no declaration is provided, treat it as declaration absent):

- **Bound**: load the stack skill directory the "Binding" section points to — `SKILL.md` is the entry point and required reading, then route by the trigger conditions in its "Reference files" table to load the `reference/` documents for the domains the current task touches (one task reads only its own domain; dumping the whole directory is forbidden); when a stack rule conflicts with this skill, the more specific stack rule wins — escalate to a human if it cannot be adjudicated
- **Binding is "none" (generalized mode)**: execute with the neutral standard + model generalization based on the stack names declared in `backend_stack` / `frontend_stack`; the verification loop follows the declaration's "Verification commands" section; label artifacts "Generalized mode (<stack name>, no binding skill)"
- **Declaration absent**: execute per this skill's neutral standard; label artifacts "No tech stack declared for this project"
- **Declaration unparseable**: report as-is; do not guess

## Workflow

1. **Read inputs**: spec acceptance criteria, hld, relevant ADRs
2. **API design**: paths/methods/inputs-outputs/error codes/idempotency — fields in snake_case, error responses carry code and trace_id, pagination via page/size, cursors for large offsets (deep norms: version compatibility / idempotency / secure access checklist → `reference/api.md`)
3. **Class design**: single responsibility (a one-sentence responsibility statement per class; if you cannot write it, the class carries too many responsibilities); behavior and data together, anemic models forbidden; value objects over entities; the aggregate root is the transaction boundary; composition over inheritance (hierarchy ≤ 2 and only for true is-a); dependency injection over internal new; everything crossing the boundary is a DTO; module boundaries and replaceable points define interfaces, method names speak intent (`approveClaim` not `doProcess`), objectify when parameters exceed 3
4. **Algorithms** (needed only when data volume ≥ 10⁴ or nested computation, correctness is non-obvious as in matching/allocation/scheduling, or money-compliance calculations are involved; not needed when ordinary CRUD composition and the standard library suffice): input-output contract (invalid input raises a business exception), complexity statement + scale assumptions, correctness argument (key invariants + counterexamples of alternatives), boundary list (empty/single element/extremes/duplicates/concurrency, each mapped to at least one test case), money precision (DECIMAL or integer cents, with remainder ownership explicitly defined); before designing, search the standard library and mature libraries first; inventing cryptography/random-number/serialization algorithms is forbidden; sacrificing readability for micro-optimization is forbidden
5. **State machine** (needed when there are 3 or more states, transitions carry business rules, or concurrent triggering exists): the state/event/transition table (with precondition Guards and side-effect Actions) is the single source of truth; implementation and tests are both generated from it; illegal transitions are explicitly rejected (422 + individually mapped business error codes; swallowing them all into a generic "state error" is forbidden); terminal states have no outgoing edges, "reopen"-style needs = a new business entity; concurrent transitions are rejected with optimistic locking (version field; after rejection, re-read and retry); state change and side effects share one transaction or go through the local message table; every transition writes a transition record; states in snake_case past tense, events as verb-derived nouns
6. **Sequence diagrams** (mandatory for cross-service / cross-aggregate transactions / money-compliance-sensitive / async chains / degradation paths; degradation always paired with the main flow; skip for ordinary single-module CRUD): annotate call type (synchronous solid line / async dashed line), timeout retries, transaction boundaries, failure paths (at least one per call, ownership explicit: upstream retries / degrades / reports the error to the user), idempotency points, trace_id entry; self-check: consistent with the hld module boundaries, call chain ≤ 3 levels, consistent with state-machine transition triggers and the error-code table, happy-path-only diagrams forbidden, granularity down to service calls — line-level operations are not drawn
7. **Physical design**: table list + growth estimate; types per the norms (money DECIMAL, time TIMESTAMPTZ); indexes designed around the top-3 high-frequency queries; partition key reserved up front; PII fields marked for encryption and masking (design sequence and artifact checklist → `reference/database.md`)
8. **Exceptions and degradation**: exception classification (retryable / non-retryable), a degradation path for every external dependency, translation unified at the boundary
9. **Acceptance-criteria mapping table**: every spec acceptance criterion ↔ implementation point ↔ planned test — no gaps
10. **Caching and messaging schemes** (when the design involves caching or async messaging): necessity criteria / decision sequence / artifact checklist → `reference/caching.md`, `reference/mq.md`

## Boundaries (not in this skill)

- Implementation following this design goes through `backend-development`; architecture-level decisions belong to `hld-design` (this skill describes only this change; report needed architecture changes for adjudication)
- Frontend feature-level design belongs to `detailed-design-frontend`

**lld.md structure (backend)**: 1 design overview (implementation approach; cites the current hld version and relevant ADRs) → 2 API design → 3 class/module design → 4 key algorithms → 5 state machine → 6 key flow sequences → 7 database physical design → 8 exceptions and degradation → 9 acceptance-criteria mapping

## Hard rules

- Implementation strategies that conflict with the hld are forbidden; report conflicts for adjudication, never silently route around them
- Vague design is forbidden ("use a suitable algorithm" is not a design); requirement content inside the lld is forbidden (point back to the spec instead)
- Exceptions and degradation must be in the design; "we will figure it out at implementation time" is not allowed
- API and data design follow the REST contract and the data norms
- The lld is a current-state design document: evolution and history stay out of the document body

## Pre-delivery self-check

After drafting and before delivery, check each item:

- [ ] Conforms to the hld (no bypassing reviewed architecture decisions) and to conflict-adjudication outcomes
- [ ] API and data design conform to the REST contract and the data norms
- [ ] Exception paths and degradation complete
- [ ] Acceptance-criteria mapping has no gaps
- [ ] Actionable: can be split directly into atomic tasks

## Reference files

- `reference/api.md` — **must-read when the lld includes API design**: REST contract / version compatibility / idempotency and concurrency / secure access checklist
- `reference/database.md` — **must-read when the lld includes physical design**: design sequence and artifact checklist
- `reference/caching.md` — **must-read when the design involves caching**: necessity criteria / decision sequence / checklist
- `reference/mq.md` — **must-read when the design involves async messaging**: scenario classification / decision sequence / checklist
- `rules/standards/architecture-standard.md` / `rules/standards/data-standard.md` / `rules/standards/quality-standard.md` — higher-level constraints
