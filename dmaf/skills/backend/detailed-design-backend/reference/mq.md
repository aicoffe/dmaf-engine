# Message Queue Design (lld design phase)

> Deep norms for the lld messaging scheme. Trigger condition: must-read when lld design involves async messaging (event modeling / load leveling / task queues).
> Design for "at-least-once delivery + idempotent consumption"; never rely on exactly-once.

## Workflow

### 1. Scenario classification (before any design)
| Scenario | Use messaging? |
|---|---|
| The caller does not need the result and tolerates delay (notifications, points, audit) | ✅ Event notification |
| Write traffic spikes far exceed what the database can absorb | ✅ Load-leveling queue |
| Long task moved to the background | ✅ Task queue |
| The result is needed synchronously | ❌ Use a synchronous call (timeout/retry/circuit breaker) |

Every async chain must be able to answer "why messaging" — a chain that cannot be classified is not designed.

### 2. Design decision sequence
1. **Event list**: list the events in this design (named `{aggregate}.{past-tense event}`, e.g. `claim.approved`), marking each event's consumers and its ordering-requirement scope (ordering is guaranteed only within an aggregate where necessary; global ordering is not promised)
2. **Uniform envelope**: `message_id` (globally unique, deduplicated on the consumer side) / `trace_id` / `type` (lowercase dot-separated) / `version` / `occurred_at` / `producer` / `payload`; structure changes only ever add, never modify or remove; breaking changes use a new type or a version bump; the payload carries only business fields and IDs, with large objects referenced via object storage
3. **Idempotency scheme**: each consumer picks one of two and writes it into the design — a dedup table (INSERT the message_id unique key before consuming; on conflict, skip; in the same transaction as the business operation) or a state-machine precondition update (version number; 0 rows updated means a duplicate); "decide at implementation time" is forbidden
4. **Delivery guarantee**: cross-aggregate eventual consistency goes through the local message table — the business operation + inserting the outgoing message share one local transaction → a background scanner delivers → mark as sent; the message table requires message_id, status, retry_count, next_retry_at, payload; the scan task must be concurrency-safe
5. **Dead-letter handling**: after a consumption failure exhausts retries (3 by default, exponential backoff) the message goes to the dead-letter queue; poison messages go straight to dead letter; dead letters must have monitoring, alerting, and a manual handling flow; "dump it in dead letter and move on" is forbidden

### 3. Design artifact checklist
- [ ] Scenario classification table (one classification rationale per chain)
- [ ] Event list: naming compliant, structure-change compatibility strategy stated
- [ ] Each consumer's idempotency scheme + dead-letter handling flow (who claims it, how it is remedied)
- [ ] Ordering-requirement scope declaration (which aggregates guarantee it, which do not)
- [ ] Payload carries only business fields and IDs; large objects referenced via object storage
- [ ] Observability: published count / consumed count / consumption latency (end-to-end lag) / dead-letter backlog have metrics and alerting; consumption logs carry trace_id and message_id

## Reference files

- `rules/standards/architecture-standard.md` / `rules/standards/reliability-standard.md` — higher-level constraints
