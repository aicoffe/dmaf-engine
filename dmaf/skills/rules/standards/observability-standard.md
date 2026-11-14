---
name: observability-standard
level: L1
domain: observability
description: Load when logging, metrics, tracing, alerting, dashboards, or observability design is involved. Governs all observability-related work.
owner: SRE Team
based_on: [organization-principles, security-standard, data-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Observability Domain Standard

> This standard governs "what to do and to what degree"; implementation-level details such as log structure, metric naming and the must-collect list, Span details, alert rule examples, and dashboard layouts are carried by the observability skill.

---

## 1. The Three Pillars of Observability

### 1.1 Pillar Definitions

| Pillar | Definition | Question answered | Typical tools |
| :--- | :--- | :--- | :--- |
| **Logs** | Records of discrete events | "What happened?" | Loki, ELK, CloudWatch |
| **Metrics** | Aggregatable numeric measurements | "What is the trend? Is anything abnormal?" | Prometheus, VictoriaMetrics |
| **Traces** | The complete path of a request through a distributed system | "Where is it slow? Which service is stuck?" | Jaeger, Tempo, Zipkin |

### 1.2 How the Three Work Together

```
User request
    ↓
Trace: records the complete call chain (trace_id threads through all services)
    ├── Span 1: API Gateway
    ├── Span 2: Claims Service
    │   ├── Log: INFO "query claim status claim_id=xxx"
    │   └── Metric: claims_query_duration_seconds
    └── Span 3: Database
        └── Log: WARN "slow query 800ms"
    ↓
Alert: P95 > 500ms → alert fires
    ↓
Dashboard: shows the trend
```

**Key requirements**:
- **`trace_id` threads through all pillars**: logs must contain `trace_id`; metrics are forbidden from carrying high-cardinality labels such as `trace_id` (see 3.3); metrics and traces are correlated through Exemplars or the chain "metrics → logs (containing `trace_id`) → traces"
- **Unified context**: logs, metrics, and traces use the same set of service names, environment names, and version numbers

---

## 2. Logging

### 2.1 Log Format and Required Fields

- **Format**: JSON structured logs (plain text is forbidden), UTF-8 encoded, UTC time (ISO 8601 format)
- The standard log structure and required fields are carried by the observability skill

### 2.2 Log Levels

| Level | Usage scenario | Example | Production |
| :--- | :--- | :--- | :--- |
| **DEBUG** | Debug information, detailed flow | "entered the validatePhone method, phone=138****8000" | ❌ Off |
| **INFO** | Key points of the normal business flow | "claim status query succeeded claim_id=xxx" | ✅ On |
| **WARN** | Recoverable anomalies, potential problems | "slow query 800ms, threshold 500ms" | ✅ On |
| **ERROR** | Business exceptions, human intervention needed | "call to claims-core timed out" | ✅ On |
| **FATAL** | System crash, unrecoverable | "database connection pool exhausted" | ✅ On |

**Level usage principles**:
- **DEBUG**: development/test environments only; off in production
- **INFO**: record key business points, not every detail
- **WARN**: record potential problems that do not affect the business
- **ERROR**: record exceptions requiring human intervention
- **FATAL**: record system-level crashes

### 2.3 Log Content and Masking

**Must log**:
- Request entry (method, path, user_id, trace_id)
- Request exit (status, duration_ms)
- Key business points (state changes, external calls)
- Exceptions (ERROR level, with stack traces)
- Slow operations (WARN level, beyond threshold)

**Must not log**:
- ❌ Passwords, tokens, secrets
- ❌ Unmasked PII (raw ID card numbers, phone numbers, bank card numbers, email addresses)
- ❌ Full request/response bodies (may contain sensitive data)
- ❌ SQL statements (may contain sensitive parameters)
- ❌ Large objects (e.g., complete JSON, binary data)

**Masking policy** (consistent with `rules/standards/security-standard`):
- Masked values are not PII and may enter logs (e.g., `138****8000`)
- Passwords, tokens, and secrets must never appear even partially masked; they must be replaced entirely with `[REDACTED]`

**Masking rules**:

| Data type | Masking method | Example |
| :--- | :--- | :--- |
| Phone number | Mask the middle 4 digits | 138****8000 |
| ID card number | Keep only the last 4 digits | ****1234 |
| Bank card | Keep only the last 4 digits | ****5678 |
| Email | Mask the username part | u***@example.com |
| Password/Token | Fully masked | [REDACTED] |

### 2.4 Log Output

| Environment | Output method | Description |
| :--- | :--- | :--- |
| **Development** | Console (colored) | Easy debugging |
| **Testing** | Console + file | Easy troubleshooting |
| **Production** | stdout (containers) | Collected by the log collector |

**Requirements**:
- Writing to local files is forbidden in production (container environments output to stdout)
- Logs are collected by a unified log collector (e.g., Fluent Bit, Filebeat)
- Log storage must be encrypted
- Log retention: 90 days (retention master table: see `rules/standards/data-standard`)

### 2.5 Log Performance

| Requirement | Description |
| :--- | :--- |
| **Async output** | Log output must be asynchronous and must not block the business |
| **Batch writing** | The log collector writes in batches to reduce I/O |
| **Sampling** | High-frequency logs may be sampled (e.g., 1% sampling for high-frequency INFO logs; DEBUG is off in production and not sampled) |
| **Rate limiting** | Per-service log volume limit (e.g., 100MB/min) |

---

## 3. Metrics

### 3.1 Metric Types

| Type | Definition | Example | Applicable scenario |
| :--- | :--- | :--- | :--- |
| **Counter** | Monotonically increasing counter | `http_requests_total` | Request counts, error counts |
| **Gauge** | Instantaneous value that can rise or fall | `active_connections` | Connection counts, queue lengths |
| **Histogram** | Bucketed statistics | `http_request_duration_seconds` | Latency distributions |
| **Summary** | Quantile statistics | `http_request_duration_seconds{quantile="0.95"}` | Latency quantiles |

**Recommendations**:
- Prefer **Histogram** (aggregatable, arbitrary quantiles computable)
- Avoid **Summary** (not aggregatable; cannot be merged across instances)

### 3.2 Metric Naming and Must-Collect Metrics

- **Naming format**: `{namespace}_{subsystem}_{name}_{unit}`, lowercase + underscores; Counters end with `_total`, Histograms end with `_seconds` / `_bytes`; units use international standard units
- Naming details and the must-collect metric list (service-level / dependency-level / resource-level / business-level) are carried by the observability skill

### 3.3 Metric Labels

**Conventions**:
- Label cardinality must be controlled (≤ 100)
- High-cardinality fields as labels are forbidden (e.g., `user_id`, `claim_id`, `trace_id`)
- Label naming: lowercase + underscores

**Standard labels**:

| Label | Description | Example |
| :--- | :--- | :--- |
| `service` | Service name | `claims-api` |
| `version` | Service version | `v2.5.0` |
| `environment` | Environment | `prod` |
| `method` | HTTP method | `GET` |
| `path` | Request path (templated) | `/api/v2/claims/{id}` |
| `status` | HTTP status code | `200` |
| `error_type` | Error type | `timeout` |

**Forbidden**:
- ❌ Using `user_id`, `claim_id`, or `trace_id` as labels is forbidden
- ❌ Using request parameters as labels is forbidden (unless cardinality is controlled)
- ❌ Dynamically concatenating label names is forbidden

### 3.4 Metric Collection

| Method | Applicable scenario | Tool |
| :--- | :--- | :--- |
| **Pull** | Service exposes a `/metrics` endpoint | Prometheus |
| **Push** | Short-lived jobs (e.g., CronJob) | Pushgateway |

**Recommendation**: prefer Pull mode (the Prometheus standard).

**Collection interval**: 15 seconds (default); critical metrics may go down to 5 seconds.

### 3.5 Metric Storage

| Requirement | Description |
| :--- | :--- |
| **Retention** | Raw data 30 days, downsampled data 1 year |
| **Downsampling** | 5-minute granularity kept 6 months, 1-hour granularity kept 1 year |
| **Storage encryption** | Metric data contains no PII; usually no encryption needed |
| **Capacity planning** | Estimated as service count × metric count × label cardinality |

---

## 4. Tracing

### 4.1 Tracing Standard

- **Standard**: OpenTelemetry (OTel)
- **Propagation protocol**: W3C Trace Context (`traceparent` / `tracestate` headers)
- **Sampling strategy**: head sampling + tail sampling

### 4.2 Span Definitions and Required Attributes

Span types (Server / Client / Producer / Consumer / Internal), required attributes, naming, and context propagation details are carried by the observability skill.

### 4.3 Tracing Scope

**Must trace**:
- All HTTP request entries (Server Span)
- All external service calls (Client Span)
- All database queries (Client Span)
- All message queue operations (Producer/Consumer Span)
- All cache operations (Internal Span)

**Optional tracing**:
- Internal steps of complex business logic
- Third-party SDK calls

### 4.4 Sampling Strategies

| Strategy | Description | Applicable scenario |
| :--- | :--- | :--- |
| **Head sampling** | Decide at the request entry whether to sample | Most scenarios |
| **Tail sampling** | Decide after the request ends, based on the result | Keeping all error traces |
| **Fixed-ratio sampling** | Sample at a fixed ratio (e.g., 10%) | Production default |
| **Adaptive sampling** | Sampling rate adjusts dynamically with traffic | Scenarios with large traffic swings |

**Recommended strategies**:
- **Development/test environments**: 100% sampling
- **Production**: 10% base sampling + 100% error sampling + 100% slow-request sampling (implemented via tail sampling)

**Instrumentation and sampling are two independent dimensions**: instrumentation coverage (whether tracing instrumentation is completed in code) must satisfy the critical path requirements in 8.2, regardless of the sampling rate; the sampling rate only decides how many traces are actually stored.

### 4.5 Trace Storage

| Requirement | Description |
| :--- | :--- |
| **Retention** | Raw traces 7 days, sampled traces 30 days |
| **Storage backends** | Jaeger / Tempo / Zipkin |
| **Indexing** | Indexed by `trace_id`, `service`, `status`, `duration` |
| **Capacity planning** | Estimated as QPS × sampling rate × Span count |

---

## 5. Alerting

### 5.1 Alerting Principles

| Principle | Description |
| :--- | :--- |
| **SLO-based** | Alerts are based on SLOs and error budgets, not raw metrics |
| **Actionable** | Every alert must have a clear handling action (Runbook) |
| **Deduplication** | The same problem does not alert repeatedly |
| **Tiering** | Tiered by severity |
| **Noise reduction** | Avoid alert storms |

### 5.2 Alert Tiers

| Level | Definition | Response deadline | Notification method |
| :--- | :--- | :--- | :--- |
| **P0 critical** | Service unavailable, data loss, security breach | 15 minutes | Phone + SMS + IM |
| **P1 severe** | Core features degraded, SLO nearly exhausted | 1 hour | SMS + IM |
| **P2 general** | Non-core feature anomalies, high resource usage | 4 hours | IM |
| **P3 notice** | Trend warnings, capacity planning | 1 day | Email |

### 5.3 Alert Rules and Runbooks

- Alerts must be based on SLOs / error budgets, and every alert must have a Runbook (alert meaning, impact scope, troubleshooting steps, handling actions, escalation path)

### 5.4 Alert Noise Reduction

| Measure | Description |
| :--- | :--- |
| **Grouping** | Alerts with the same labels are merged |
| **Inhibition** | When a P0 alert fires, P1/P2 alerts of the same service are inhibited |
| **Silencing** | Alerts are silenced during maintenance windows |
| **Deduplication** | The same alert is not re-sent within 5 minutes |
| **Delay** | Brief jitter does not alert (`for: 5m`) |

### 5.5 Alert Notifications

| Level | Notification channel | Recipients |
| :--- | :--- | :--- |
| **P0** | Phone + SMS + IM | On-call + Tech Lead + management |
| **P1** | SMS + IM | On-call + Tech Lead |
| **P2** | IM | On-call |
| **P3** | Email | Team |

---

## 6. Dashboards

### 6.1 Dashboard Tiers

| Tier | Audience | Content |
| :--- | :--- | :--- |
| **L1 business dashboard** | Management, product | Business metrics (order volume, user count, revenue) |
| **L2 service dashboard** | Tech Lead, SRE | Service health (SLOs, error rate, latency) |
| **L3 component dashboard** | Developers, SRE | Component details (DB, Cache, MQ) |
| **L4 debug dashboard** | Developers | Debugging details (specific requests, slow queries) |

### 6.2 Required Content of Service Dashboards

Required blocks: service health overview (availability, error rate, P95 latency, QPS), request volume trend, latency distribution (P50/P95/P99), error rate trend (by status code), dependency health (DB / Cache / MQ), resource usage (CPU / memory / network).

The standard layout is carried by the observability skill.

### 6.3 Dashboard Conventions

| Convention | Requirement |
| :--- | :--- |
| **Naming** | `{tier}-{service/domain}-{purpose}`, e.g., `L2-claims-api-health` |
| **Version control** | Dashboard configuration must be version-controlled (JSON/YAML) |
| **Templating** | Similar services use a unified template |
| **Variables** | Support environment, time range, and service instance filters |
| **Linking** | Dashboards link to each other (service → component → debug) |
| **Annotations** | Key metrics must have annotations explaining their meaning |

### 6.4 Dashboard Tools

| Tool | Applicable scenario |
| :--- | :--- |
| **Grafana** | General dashboards (recommended) |
| **Kibana** | Log analysis |
| **Jaeger UI** | Trace analysis |
| **In-house** | Special requirements |

---

## 7. Observability Data Governance

### 7.1 Data Classification

| Data type | Level | Retention | Encryption |
| :--- | :--- | :--- | :--- |
| **Logs (masked)** | L2 internal | 90 days | Transport encryption |
| **Logs (containing unmasked PII)** | L3 sensitive | Must not be produced or stored (raw values must be masked before output) | — |
| **Metrics** | L2 internal | 30 days + 1 year downsampled | Transport encryption |
| **Traces** | L2 internal | 7 days + 30 days sampled | Transport encryption |
| **Alert records** | L2 internal | 1 year | Transport encryption |
| **Dashboard configuration** | L1 public | Permanent (Git) | — |

### 7.2 Cost Control

| Measure | Description |
| :--- | :--- |
| **Sampling** | Log sampling, trace sampling |
| **Downsampling** | Metric downsampling (5 minutes → 1 hour) |
| **Retention** | Retention set per data type |
| **Archiving** | Cold data archived to object storage |
| **Capacity planning** | Periodically assess storage capacity |

### 7.3 Access Control

| Role | Permissions |
| :--- | :--- |
| **Developers** | Read logs, read metrics, read traces |
| **SRE** | Read/write alert rules, read/write dashboards |
| **Security team** | Read audit logs |
| **Management** | Read business dashboards |

**Requirements**:
- All access must be authenticated
- Sensitive data access must be audited
- Anonymous access is forbidden

---

## 8. Design for Observability

### 8.1 Design Principles

| Principle | Description |
| :--- | :--- |
| **Observability is part of the feature** | Not "add it later" but "there from the start" |
| **Critical paths must be observable** | All critical paths must have logs, metrics, and traces |
| **Observability as code** | Configuration must be version-controlled |
| **Unified standard** | All services use the same observability standard |

### 8.2 Critical Path Identification

**Critical path definition**: call chains that directly affect the correctness of business outcomes or the user-visible experience.

| Path type | Classification | Observability requirement |
| :--- | :--- | :--- |
| **User request entry** | Critical path | Logs + metrics + traces |
| **External service calls** | Critical path | Logs + metrics + traces |
| **Message queues** | Critical path | Logs + metrics + traces |
| **Database operations** | Supporting path | Logs + metrics (tracing recommended for write operations) |
| **Cache operations** | Supporting path | Metrics |
| **Scheduled jobs** | Supporting path | Logs + metrics |

**Instrumentation of critical paths must be 100% covered** (P-004); storage and collection costs in production are controlled through sampling (see 4.4); omitting instrumentation on sampling grounds is not allowed.

### 8.3 Observability Checklist

The Design-phase `spec.md` must contain:

- [ ] Critical paths identified
- [ ] Logs for critical paths planned
- [ ] Metrics for critical paths planned
- [ ] Traces for critical paths planned
- [ ] Alert rules planned
- [ ] Dashboards planned
- [ ] Runbooks planned (for P0/P1 alerts)
- [ ] Observability configuration version-controlled

---

## 9. Conflict Adjudication with Other Domains

### 9.1 With Security

- **Adjudication basis**: P-001 (security first)
- **Adjudication process**: Security Owner + SRE adjudicate jointly
- **Example**: logs contain PII vs troubleshooting needs → security first; log masked values
- **Forbidden**: logging unmasked PII for troubleshooting convenience is forbidden

### 9.2 With Performance

- **Adjudication basis**: P-004 (observability first)
- **Adjudication process**: Architect + SRE adjudicate jointly
- **Example**: tracing overhead vs performance → reduce storage and collection overhead through sampling; instrumentation of critical paths must still be 100% covered; omitting instrumentation for performance is not allowed

### 9.3 With Cost

- **Adjudication basis**: P-003 (data minimization)
- **Adjudication process**: SRE + management adjudicate jointly
- **Example**: full-volume logs vs storage cost → data minimization + sampling + downsampling

### 9.4 With Delivery Schedule

- **Adjudication basis**: P-004 (observability over implementation convenience)
- **Adjudication process**: Tech Lead + SRE adjudicate jointly
- **Forbidden**: omitting observability on "schedule pressure" grounds is forbidden

---

## 10. Observability Metrics

### 10.1 Measuring Observability Itself

| Metric | Description | Target |
| :--- | :--- | :--- |
| **Log coverage** | Share of services with critical path logs | 100% |
| **Metric coverage** | Share of services with standard metrics | 100% |
| **Trace instrumentation coverage** | Share of critical paths with tracing instrumented | 100% |
| **Alert coverage** | Share of services with SLO alerts | 100% |
| **Runbook coverage** | Share of P0/P1 alerts with a Runbook | 100% |
| **Dashboard coverage** | Share of services with a service dashboard | 100% |
| **MTTD** | Mean time to detect | ≤ 5 minutes |
| **MTTR** | Mean time to repair | ≤ 4 hours |

### 10.2 Observability Health Checks

- **Weekly**: check that alert rules are effective (no silences, no dead rules)
- **Monthly**: check that dashboards are up to date (no stale metrics)
- **Quarterly**: check that Runbooks are effective (validated through drills)
