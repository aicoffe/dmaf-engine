# Domain Logic Implementation Techniques

> Deep-dive for implementing domain logic after the lld has landed. Trigger condition: must-read when implementing domain logic (aggregates/entities/value objects/domain events/repositories/concurrency control).

## 1. Drawing aggregation boundaries

**Invariant identification** (the primary technique for designing aggregates):

1. List the rules that must **hold at all times** (invariants), e.g. "claim total = the sum of detail line amounts", "a claim can only be filed while the policy is in force"
2. An invariant maintained across entities → those entities belong to **one aggregate**; invariants that hold independently → separate aggregates
3. Aggregate root = the sole entry point from outside: external code may only modify objects inside the aggregate through aggregate-root methods, and invariants are checked inside those methods

**Strong vs eventual consistency** (how to choose for cross-aggregate operations):

- Two updates that **must live or die together** (one succeeds while the other fails = a business rule is broken) → one aggregate, strong consistency
- The two updates have a business-tolerable intermediate state (e.g. "ordered but stock not yet deducted" can be compensated) → across aggregates + domain events for eventual consistency
- When you cannot decide, ask the business side: "How long can this intermediate state be exposed to users/reconciliation?" — if no tolerable window can be named, treat it as strong consistency

**Aggregate design anti-patterns**:

- Big aggregates (stuffing entities with unrelated invariants into one aggregate enlarges the lock and transaction scope)
- Designing aggregates for queries (queries go through the read model / dedicated queries; do not distort the aggregate structure)
- Holding object references across aggregates (reference IDs only, see §3)

## 2. Tactical pattern selection

| Pattern | When to use | Criteria |
|---|---|---|
| Value object | No unique identity, immutable, replaceable as a whole | Money/address/time range; wherever a value object works, prefer it over an entity |
| Domain service | Business logic spans multiple aggregates and fits none of them naturally | Stateless; named in business language |
| Repository | The persistence entry point of an aggregate | **One repository per aggregate**; the interface is defined in the domain layer, the implementation in infrastructure (dependency inversion); repositories only do aggregate-level storage and retrieval, never cross-aggregate queries |
| Factory | Aggregate construction is complex (many required fields, many invariants, default-value derivation needed) | Simple construction uses the constructor directly; do not build a factory for simple cases |
| Specification | The same business predicate is reused in multiple places (validation/query/filtering) | A specification is a value object and can be composed (and/or/not) |

## 3. Cross-aggregate collaboration patterns

- **Reference by ID only**: aggregates store only the other aggregate root's ID; when data is needed, look it up by ID or use a read-only snapshot (direct object references are forbidden — they punch through the aggregate boundary)
- **Eventual-consistency path**: the aggregate-root method produces a domain event → the local message table persists it in the same transaction → delivery and consumption
- **Cross-aggregate queries**: never go through the aggregate or repository — build a dedicated query (read model / view query) that returns in the query's shape, separated from the write model
- **Saga compensation criteria**: when any step in a cross-aggregate write sequence fails and already-succeeded steps must be rolled back, design an **idempotent compensation operation** for each step; compensations that themselves fail go to dead letter / manual handling

## 4. Concurrency control selection

- **Optimistic locking by default** (version field / ETag): for scenarios with low write-conflict frequency. Criteria: low probability of concurrent writes to the same aggregate, small retry cost
- **Pessimistic locking** (row locks) only when: hot rows (the same row written concurrently at high frequency, optimistic-lock retry storms), short lock-holding time, and queuing waits are acceptable — minimize the lock scope; making external calls or running long transactions inside a lock is forbidden
- **Distributed locks**: cross-process resource mutual exclusion (e.g. keeping scheduled tasks from re-entering); for the red lines see resource cleanup in the error-handling techniques (timeout-based release + active release + holder identity)
- Optimistic-lock conflict handling: catch the conflict → re-read the latest state → re-apply the business intent (blindly replaying the old request is forbidden)

## 5. Domain event design techniques

- **Granularity**: an event is a **completed business fact** a business person can read (`ClaimSubmitted`), not a technical action (`TableUpdated`)
- **Payload**: the consumer needs the data immediately (to skip the lookup) → include the key fields; the data can be looked up and consumption is low-frequency → ID only. Default to ID + key display fields, adding more per the consumer's actual needs
- Events are immutable: published events are never modified; to correct an error, publish a new event (a compensation event) — history is not rewritten
- Event naming `{aggregate}.{past-tense event}` (e.g. `claim.approved`); the envelope is uniformly message_id / trace_id / type / version / occurred_at / producer / payload

## 6. Self-check

- [ ] You can state each aggregate's invariant list (if you cannot, the aggregate boundary was a guess)
- [ ] Cross-aggregate collaboration goes only through ID references + events; no object references, no cross-aggregate transactions
- [ ] Repository interfaces live in the domain layer; no cross-aggregate queries; nothing returned in a non-aggregate shape
- [ ] Every concurrency-control choice has a rationale (optimistic/pessimistic/distributed lock, each answers "why")
- [ ] Event payload and granularity can answer "who is the consumer, is a lookup needed"
