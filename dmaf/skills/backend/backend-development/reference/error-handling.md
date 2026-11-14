# Error Handling Implementation Techniques

> Deep-dive for implementing exception handling after the lld has landed. Trigger condition: must-read when implementing retry, degradation, compensation, or distributed-transaction compensation.

## 1. Retry policy techniques

**The three pre-retry questions** (must be answered before any retry):

1. Is it idempotent? — Make a non-idempotent operation idempotent first (Idempotency-Key / business request number); only then talk about retrying
2. Is it retryable? — Only transient failures (timeout, connection failure, 503) are retried; 400/404/business-rule rejections must never be retried
3. Is the budget enough? — Upper and lower layers retrying at the same time multiply the attempt counts (upper 3 × lower 3 = 9 real hits): only one layer in the whole chain may retry; every other layer passes failures through

**Backoff technique**: exponential backoff + jitter — `delay = base × 2^retry_count + random(0, jitter)`. Jitter spreads out synchronized retries (concurrent requests triggered by the same failure, if they retry at a fixed interval, form a retry storm that knocks the dependency down a second time). Take base and the maximum attempt count from the standard values in `rules/standards/reliability-standard`; do not set your own here.

**Retry visibility**: retries must leave a trace (logs carry the attempt number and the next delay); failures that exhaust retries must escalate (alert or enter the compensation flow); silent exhaustion is forbidden.

## 2. Degradation design techniques

**Which dependencies may degrade** (judge each one and write it into the design):

| Dependency type | Degradation method |
|---|---|
| Read-data type (recommendations, rankings, profiles) | Serve the stale cached value → predefined default → hide the module |
| Write-collaboration type (notifications, points) | Defer via an async message (enqueue while the dependency is unavailable) |
| Critical path (payment, risk control) | **Cannot degrade** — failures pass through; returning fake success is forbidden |

**Fallback data source tiers** (highest to lowest priority): stale value from cache (marked "data not real-time") → predefined defaults → static configuration data → feature switched off (stated explicitly). Returning a half-finished state or a null-pointer-style "degradation" is forbidden.

**Degradation switches and recovery**: degradation logic must have a configurable switch (manual forced degradation serves dependency maintenance windows); recovery relies by default on automatic dependency probing (circuit breaker half-open state); blindly switching back on a timer is forbidden.

## 3. Compensation implementation techniques

- **Compensation operations must be idempotent**: compensations themselves get retried; repeating the same compensation must not produce new side effects
- **Compensation order**: compensate in reverse business-step order (the step completed last is compensated first)
- **Compensation failure**: goes to dead letter / manual queue; silently dropping a failed compensation is forbidden — the business is then in a "partially completed" state and must be queryable (compensation status is traceable along with the business document)
- **Local message table consumer side**: consumption idempotency (dedup table / state precondition) + failure retry + dead letter on exhaustion; the key implementation point is that **the business operation and the message insert share one transaction**, with delivery handed to a background scanner; sending messages directly inside the business transaction is forbidden

## 4. Self-check

- [ ] Every retry point has answered the three pre-retry questions; confirmed only one layer in the whole chain retries
- [ ] Backoff carries jitter; standard values taken from reliability-standard
- [ ] Every external dependency answers "can it degrade, the fallback source, how it recovers"; critical paths explicitly declare themselves non-degradable
- [ ] Every compensation operation is idempotent, reverse-order, and traceable on failure
