# Async Tasks Implementation Techniques

> Deep-dive for implementing async tasks after the lld has landed. Trigger condition: must-read when implementing scheduled tasks, delayed tasks, large-task sharding, or task recovery.

## 1. Scheduled task techniques

- **Re-entry prevention** (the first red line under multi-instance deployment): preemptive locking — a database claim marker (optimistic lock / `FOR UPDATE SKIP LOCKED`) or a distributed lock; a distributed lock must carry **timeout-based release + holder identity** (if the holder crashes, the lock expires automatically and others can take over); locks without a timeout are forbidden
- **Off-peak scheduling**: keep execution away from business peaks; on-the-hour tasks (large numbers of systems run at exactly the hour) stagger their offsets
- **Idempotency**: repeated triggering of a scheduled task must be safe (the previous run not finished when triggered again = the re-entry case; triggered again after finishing = the duplicate-processing case) — design for "at-least-once + idempotent"
- **Failure handling**: alert on failure + retry per the retryable classification (only transient failures are retried); a scheduled task failing unnoticed is forbidden
- **No long transactions**: large batch processing commits in batches inside the scheduled task; wrapping the entire volume in one transaction is forbidden

## 2. Delayed task selection

| Dimension | DB table polling (delay table + scan) | Delay queue / timing wheel (MQ capability) |
|---|---|---|
| Precision | seconds to minutes | milliseconds to seconds |
| Volume | small (scan pressure manageable) | large |
| Infrastructure | database only | depends on MQ delay capability |
| Operational visibility | the table is the state; easy to troubleshoot | needs MQ monitoring in place |

**Decision**: an MQ already present with strong delay capability, large volume, high precision → delay queue; otherwise DB table polling (scan + state machine; retry follows the async-task clauses). Introducing new middleware for a few low-frequency delayed needs is forbidden.

## 3. Large task sharding techniques

- **Shard key choice**: evenly distributed (by ID range / hash modulo) with no data overlap between shards — a wrong shard key causes skew (one shard drags down the whole job)
- **Shard state recording**: each shard's completion state is persisted (task-table child records or a shard table), supporting **resumable runs** — after a failure only the unfinished shards rerun; rerunning the entire task is forbidden
- **Failure isolation**: a single shard failure marks only that shard failed and retries it, without invalidating the whole job; final summary check (count reconciliation: succeeded + failed = total shard count)
- Shard size follows "the cost of rerunning one failed shard is acceptable + one shard's execution time is far below the task timeout" (set per task type; there is no universal number)

## 4. Zombie task detection techniques

- **Heartbeat**: the executor refreshes the task heartbeat field periodically (e.g. every 30 seconds; the default is adjusted per task type); a heartbeat not refreshed within its window = the executor is dead and the task has become a zombie
- **Takeover and rerun**: the scanner flips zombie tasks back to a runnable state and another instance takes over and reruns them — **confirm idempotency before rerunning** (repeated execution of the same task_id produces no duplicate side effects)
- **Alerting**: alert as soon as a zombie task appears (it means the executor is abnormal, not just that the task has a problem)

## 5. Progress reporting techniques

- **Progress updates are independent of the business transaction**: progress writes are high-frequency and non-critical; sharing a transaction with business data is forbidden (it slows the main logic); update the progress table separately or refresh asynchronously
- **Throttling**: throttle by percentage step or time interval (write only when the change is ≥ 1% or the interval is ≥ a few seconds; the default is adjusted per task duration) to avoid progress write amplification
- **Failure reason is user-readable**: error_message faces the user; technical details go to logs, not the progress API

## 6. Self-check

- [ ] Scheduled tasks have answered: re-entry prevention method, lock timeout and holder identity, idempotency rationale, off-peak arrangement
- [ ] Delayed tasks are decided against the selection table with the reason written down; no new middleware introduced for a new need
- [ ] Large tasks have a shard key, shard state, and resumable runs; summary checks complete
- [ ] Heartbeat, zombie takeover, and rerun idempotency — all three present
- [ ] Progress updates on an independent transaction + throttled
