---
name: observability
description: Observability implementation. Landing structured logs, metric instrumentation, and distributed tracing. Use when the user asks for "adding logs", "instrumentation", "monitoring", "distributed tracing", or "trace". Critical-path instrumentation must be complete, regardless of the sampling rate.
---

# Observability Implementation

Unify the three pillars — logs, metrics, traces: trace_id threaded throughout, with a unified service name / environment name / version number.

## Workflow

### 1. Logs
- JSON structured (plain text forbidden), UTF-8, UTC ISO 8601
- Required fields: timestamp, level, service, version, environment, trace_id (when tracing is on), span_id, message, logger, context (as needed), exception (required for ERROR)
- Levels: DEBUG for development/testing only (off in production); INFO for key business nodes; WARN for potential problems; ERROR requires human intervention; FATAL for system crash
- Must log: request entry/exit (with trace_id and duration), key business nodes, exceptions (with stack traces), slow operations
- In production, output to stdout; write asynchronously; rate-limit per-service log volume

### 2. Metrics
- Metric design method: for the service and interface level, use the **RED** three questions to decide what must be collected (Rate — request volume / Errors — error rate / Duration — latency); for the resource and middleware level, use **USE** (Utilization / Saturation / Errors) — RED first, then USE; instrumenting by gut feeling is forbidden
- Naming `{namespace}_{subsystem}_{name}_{unit}`: lowercase with underscores, Counters end with `_total`, Histograms end with `_seconds/_bytes`
- Prefer Histogram (aggregatable); avoid Summary
- Four mandatory categories: service level (request count/latency/concurrency/errors), dependency level (DB/cache/MQ), resource level (CPU/memory/GC), business level (`{service}_business_{operation}_*`)
- Label cardinality ≤ 100; user_id/claim_id/trace_id as labels is forbidden; standard labels service/version/environment/method/path/status/error_type

### 3. Tracing
- OpenTelemetry + W3C Trace Context (traceparent/tracestate) propagated across services and messages
- Span types: Server/Client/Producer/Consumer/Internal
- Span names templated (`GET /api/v2/claims/{id}`; concrete IDs forbidden); attributes ≤ 20 and PII forbidden
- Failed operations must set status = ERROR plus an error_type attribute

### 4. Critical-path instrumentation check
- User request entry points, external service calls, message send/receive = critical path: the trio of logs + metrics + traces
- Database operations / cache / scheduled tasks = supporting path: logs + metrics
- Production sampling (10% base + 100% of errors/slow requests) controls storage cost, but instrumentation is never skipped

## Boundaries (not in this skill)

- Exception severity semantics (what counts as ERROR and what counts as WARN) is decided by `backend-development`'s exception classification; this skill owns landing the instrumentation
- Alert rules and Runbook authoring are in this skill's §5; critical-path instrumentation completeness is guaranteed by this skill

### 5. Alerts and Runbooks
- SLO-based error-budget burn-rate alerts take precedence over single-threshold alerts (at SLO 99.9%, burning 2% of the budget within 1 hour is P1); resource alerts must convert to utilization with rate() (Counters are cumulative values and must not be compared against thresholds directly)
- Every P0/P1 alert must have a Runbook: what the alert means, blast radius, troubleshooting steps (dashboards/logs/dependencies), remediation actions
- For alert severity P1/P2, noise reduction, and notification-channel policy, `rules/standards/observability-standard` is the authority; alert rule examples and the Runbook template are in this section

## Reference files

- `rules/standards/observability-standard.md` / `rules/standards/security-standard.md` — higher-level constraints

## Default values

Label cardinality ≤ 100, span attributes ≤ 20, and sampling (10% base + 100% of errors/slow requests) are default values; they may be adjusted per storage cost, with the adjustment recorded.
