# Supplementary Architecture Techniques (Integration / Module Decomposition / Microservices)

> Trigger conditions: required reading when Phase 4 detailed solution design involves integration patterns, module decomposition, or microservice splitting.

## 1. Integration-pattern selection (decision tree)

```
Does the caller need the result to continue?
├── Yes → Synchronous (HTTP/gRPC; timeout/retry/circuit breaking — the four essentials)
│         └── Processing > 30s? → Convert to an async task
└── No → Reliable delivery + event replay needed?
          ├── Yes → Message queue
          └── Full audit/rebuild needed → Event sourcing (use cautiously: high complexity, ADR mandatory)
Heavily divergent read/write with read-mostly? → Evaluate CQRS (read/write separation, ADR mandatory)
```

Write the decision conclusion into an ADR or the spec's design section; mixing two semantics at the same integration point is forbidden.

## 2. External (third-party) system integration

| Concern | Requirement |
|---|---|
| Anti-corruption layer (ACL) | Third-party models do not penetrate the domain layer: a dedicated adapter module does model conversion |
| Timeout and degradation | Treat third parties as the least reliable dependency: timeout, circuit breaking, degradation fallback — the four essentials are mandatory |
| Credentials | Keys are managed centrally; hardcoding is forbidden |
| Replay and idempotency | Our callback interfaces must be replay-proof; our calls to third parties must carry a request number for idempotency |
| Sandbox verification | Verify in the sandbox before joint debugging; production credentials are isolated from the sandbox |

Webhook design (providing callbacks outward): signature verification (HMAC + time window against replay); idempotency via event-ID dedup, ordering not guaranteed — consumers tolerate via the state machine; failed deliveries retry with exponential backoff and provide a manual resend entry; separate metrics for callback receipt / verification failure / processing failure.

Data-sync integration: consuming data across contexts uses read-only snapshots + event sync; direct connections to the other side's database are forbidden; sync jobs must be reconcilable (source-target counts/checksum comparison, alert on divergence).

## 3. Module decomposition (landing the modular monolith)

Decomposition steps: event storming/domain analysis to identify bounded contexts and the ubiquitous language → a context map marking upstream/downstream and integration modes (ACL/events/shared kernel) → one module per context → layering within a module (interface/application/domain/infrastructure) → define external interfaces (API module) and dependency directions.

Physical organization: `{module}-api` (external DTOs + interface definitions, no implementation) / `{module}-domain` (domain model and rules) / `{module}-app` (use-case orchestration, transaction boundaries) / `{module}-infra` (database and external dependency implementations) / `{module}-boot` (assembly and startup). Modules depend only on other modules' `{module}-api`; depending on another module's `-domain`/`-infra` is forbidden; mixing DTOs and domain entities is strictly forbidden (convert in the application layer).

Dependency constraints are committed with the codebase and enforced by a CI gate (e.g., ArchUnit: no reaching into another module's internals, no circular dependencies).

Module evolution: new business adds use cases inside existing modules by default; a genuinely needed new module reviews its boundary first; inter-module calls must go through interfaces, preserving replacement points for future splitting; evaluate splitting when deployment-rhythm conflicts, resource-requirement conflicts, or team-ownership separation signals appear.

## 4. Microservices (split only when every prerequisite is met)

Prerequisite check: a genuine need for independent deployment/scaling/tech stack (not "reserving extensibility"); team size matches the service count; the target boundaries already have stable modular-monolith boundaries (logic first, physical later); observability infrastructure ready (trace end to end/centralized logs/metrics); an existing ADR stating the benefits and costs.

Strangler migration: converge independent modules within the monolith → build the new service to coexist with the monolith (traffic switch) → funnel calls through the new service's API (keep a local degradation switch) → separate the data (Expand-Contract) → canary traffic cutover → retire the old implementation (keep it for one version cycle).

Inter-service call wrapping: unified client wrapping (timeouts connect 3s/read 10s, retries only for idempotent operations + exponential backoff, circuit breaking); call-chain depth ≤ 3 levels — go event-driven beyond that; propagate W3C Trace Context; error handling distinguishes retryable/non-retryable.

Independent data: each service owns its database (instance-level by default); cross-service queries go through APIs or redundant read-only snapshots; cross-service consistency uses Saga/local message tables; cross-service JOINs and shared databases are forbidden; cross-service references store IDs only.

Governance: service contracts registered (OpenAPI/Proto) with contract tests; record Owner, Runbook entry, dependency list; review periodically (default quarterly) — retire uncalled services, merge nanoservices.
