---
name: backend-development
description: Backend implementation. Domain logic (domain modeling in code, transaction boundaries, state machines, concurrency control), exception handling (exception hierarchy, boundary translation, retryable classification, resource cleanup and degradation), async tasks (criteria for making long tasks async, 202 task API, task table, failure compensation). Use when the user asks to "implement business rules", "write domain logic", "handle transactions", "control concurrency", "handle exceptions", "define error codes", "do async tasks", "run background jobs", "do export/import/batch processing".
---

# Backend Development

Domain logic, exceptions, and async handled as one: behavior and data live together, the domain layer only throws and never catches, and anything over 30 seconds must go async.

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and act according to its state — one of four cases (when no declaration is provided, treat it as declaration absent):

- **Bound**: load the stack skill directory the "Binding" section points to — `SKILL.md` is the entry point and required reading, then route by the trigger conditions in its "Reference files" table to load the `reference/` documents for the domains the current task touches (one task reads only its own domain; dumping the whole directory is forbidden); when a stack rule conflicts with this skill, the more specific stack rule wins — escalate to a human if it cannot be adjudicated
- **Binding is "none" (generalized mode)**: execute with the neutral standard + model generalization based on the stack names declared in `backend_stack` / `frontend_stack`; the verification loop follows the declaration's "Verification commands" section; label artifacts "Generalized mode (<stack name>, no binding skill)"
- **Declaration absent**: execute per this skill's neutral standard; label artifacts "No tech stack declared for this project"
- **Declaration unparseable**: report as-is; do not guess

## Workflow

### A. Domain logic

> Techniques for drawing aggregation boundaries, selecting tactical patterns, cross-aggregate collaboration, concurrency control, and domain event design: `reference/domain-logic.md` (must-read when implementing domain logic)

#### 1. Domain modeling in code
- Tell entities (unique identity, life cycle) apart from value objects (no identity, immutable — money/address/time range; prefer value objects)
- The aggregate root is the consistency boundary: strongly consistent within an aggregate, eventually consistent across aggregates
- When logic spans multiple aggregates, use a domain service; cross-aggregate notification goes through domain events (`{aggregate}.{past-tense event}`)

#### 2. Layering discipline
- Domain rules live in the domain layer (no dependency on frameworks or infrastructure); use-case orchestration in the application layer; protocol adaptation in the interface layer
- Anemic models are forbidden: pure data classes operated on by big external functions

#### 3. Transactions and concurrency
- Transaction boundary = aggregate boundary: one transaction modifies one aggregate; across aggregates use eventual consistency (local message table)
- Concurrent modification uses optimistic locking (version field); on conflict return 409 or re-read and retry
- No external calls inside a transaction (direct HTTP/MQ sends); side effects are persisted through the local message table in the same transaction

#### 4. State machine implementation
- The transition table is the single authority: the implementation (enum + guard) is generated from the designed state/event/transition table
- Illegal transitions raise a business exception (mapped to 422 + business error code); silent ignoring is forbidden
- State change and side effects are atomic; every transition writes a transition record (who, when, from what to what, why)

### B. Exception handling

> Techniques for retry policy, degradation design, and compensation implementation: `reference/error-handling.md` (must-read when implementing retry/degradation/compensation)

#### 5. Exception hierarchy
| Level | Purpose |
| :--- | :--- |
| Business exception | A business rule is not satisfied; expected (extends a unified base class, carries an error code) |
| System exception | Technical failure (DB connection failure, downstream timeout) |
| Framework exception | Translated uniformly at the boundary (validation, serialization) |

#### 6. Handling red lines
- Swallowing exceptions is forbidden: an empty catch, or catching and only logging before continuing (unless an explicit comment explains why it is ignorable)
- Dropping the exception chain is forbidden: a wrapped exception must preserve the cause
- The domain layer only throws and never catches: translation happens in one place, at the interface layer / global exception handler
- Using exceptions to carry input-validation results is forbidden (validation failure returns 400)
- Using exceptions as flow control is forbidden; returning error-code strings in place of exceptions is forbidden

#### 7. Boundary translation (translate in one place)
- The global exception handler translates everything into API error responses: business exception → 4xx + error code; system exception → 500 + `SYS_INTERNAL_ERROR`, without leaking stack traces / SQL / internal paths
- Uncaught exceptions are logged at ERROR (with stack trace and trace_id)

#### 8. Retryable classification
- Retryable: transient failures such as timeouts, connection failures, 503 → handled upstream per the retry policy (idempotency is the precondition)
- Non-retryable: 400/404/409/business-rule rejection → fail directly; retrying is forbidden
- Custom exceptions carry a `retryable` flag or split into subclasses

#### 9. Resource cleanup and degradation
- Use try-with-resources / defer / finally; distributed lock release = timeout-based release + active release
- Exceptions on the degradation path must be caught and logged; fall back to returning predefined data (half-finished states are forbidden); a degradation failure must not break the main flow

### C. Async tasks

> Techniques for scheduled-task re-entry prevention, delayed-task selection, large-task sharding, zombie detection, and progress reporting: `reference/async-tasks.md` (must-read when implementing scheduled and batch tasks)

#### 10. Async criteria
- Processing takes over 30 seconds (export, bulk import, reports) → must go async
- Triggers a slow third-party dependency (reconciliation, external review) → must go async
- Deferrable non-core action (notification, points) → make it a message event, not a task

#### 11. Task API pattern
```
POST /api/v2/{resource}-tasks          → 202 Accepted + {"task_id": "..."}
GET  /api/v2/{resource}-tasks/{id}     → status + progress (0-100) + failure reason + result location
GET  /api/v2/{resource}-tasks/{id}/result → fetch the result once complete
```
- Return 202 immediately on submission; clients holding a long-lived connection to wait is forbidden
- State machine: `PENDING → RUNNING → SUCCEEDED / FAILED / CANCELED`
- Result files go to object storage with expiry cleanup; stuffing them into the database is forbidden

#### 12. Task table
Required fields: task_id (Snowflake/UUID), type, status, progress, payload (JSONB), result_uri, error_message (user-readable), owner_id/tenant_id, audit fields

#### 13. Execution and compensation
- Executor claiming is concurrency-safe (optimistic lock or FOR UPDATE SKIP LOCKED)
- Idempotency: repeated execution of the same task_id produces no duplicate side effects
- Retry: retryable failures back off exponentially, 3 attempts by default; non-retryable failures go straight to FAILED
- Timeout: configure an execution timeout per task type; on timeout set FAILED and release the execution slot
- Support cancellation (CANCELED, responding at checkpoints)

#### 14. Observability
- Submission count / success rate / duration / backlog have metrics and alerting; execution logs carry task_id + trace_id

## Boundaries (not in this skill)

- During implementation, follow the contracts and table structures already fixed by the lld; do not redesign them. Design norms for APIs/tables/caching/messaging live in the `detailed-design-backend` skill's reference/ (consumed during the lld phase); report needed design changes for adjudication instead of deviating unilaterally
- Input validation and injection defense go through `secure-coding`; landing logs/instrumentation/tracing goes through `observability`; slow-query diagnosis and tuning goes through `db-optimization`

## Hard rules

- The domain layer must not depend on application framework annotations (e.g. Spring annotations), the database, HTTP, or other infrastructure
- Modifying the status field directly, bypassing the transition table, is forbidden; complex rules must carry a comment or document explaining "why"
- Synchronous APIs must not carry operations over 30 seconds; tasks must not go without a timeout configuration; failure reasons must be user-readable and must not leak stack traces

## Reference files

- `reference/domain-logic.md` — **must-read when implementing domain logic**: aggregation boundary drawing / tactical pattern selection / cross-aggregate collaboration / concurrency control / domain event design techniques
- `reference/error-handling.md` — **must-read when implementing retry, degradation, and compensation**: the three pre-retry questions, backoff and jitter / degradation source tiers / compensation idempotency and reverse-order techniques
- `reference/async-tasks.md` — **must-read when implementing scheduled and batch tasks**: re-entry prevention / delayed-task selection / large-task sharding with resumable runs / zombie detection / progress throttling techniques
- `rules/guides/api-design-guide.md` — REST clauses and concurrency control (shared clause tables)
- `rules/standards/architecture-standard.md` / `rules/standards/quality-standard.md` / `rules/standards/reliability-standard.md` / `rules/standards/performance-standard.md` — higher-level constraints

## Default values

3 retry attempts and 0–100 progress are organization defaults (adjustable per project, with the adjustment recorded); the >30-second async threshold and the 202 task API convention follow the shared clause table `rules/guides/api-design-guide`; local message table = the eventual-consistency scheme where the business operation and its message are persisted in the same transaction.
