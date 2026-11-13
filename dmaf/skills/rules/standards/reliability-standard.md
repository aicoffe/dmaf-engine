---
name: reliability-standard
level: L1
domain: reliability
description: Load when timeouts, retries, circuit breaking, degradation, rate limiting, disaster recovery, or fault isolation is involved. Governs all reliability-related work.
owner: SRE Team
based_on: [organization-principles, architecture-standard, observability-standard, data-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Reliability Domain Standard

> This standard governs reliability standards and red lines; implementation details are carried by reliability domain skills (deferred domains: see rules-backup).

---

## 1. Failure Design Principles

- **Design for Failure**: assume at design time that any dependency may fail
- Every external dependency must explicitly configure the four-piece set: **timeout, retry (idempotent operations only), circuit breaking, degradation** (default parameters: see `rules/standards/architecture-standard`)
- Failures must be observable: error rate, circuit breaker state, and degradation triggers must have metrics and alerts (P-004)

---

## 2. Redundancy and Isolation

| Requirement | Description |
| :--- | :--- |
| **Multiple instances** | Critical services must be deployed with multiple instances |
| **Resource isolation** | Core and non-core workloads are isolated (thread pools / connection pools / clusters); non-core failures must not take down core |
| **Eliminate single points** | Database (primary/replica), cache (cluster), messaging (cluster + persistence) must never be single points |
| **Fault isolation** | A single service's failure must be contained within its boundary and must not cascade |

---

## 3. Rate Limiting and Overload Protection

- All external entry points must have rate limiting (thresholds: see `rules/standards/security-standard`)
- When a service is overloaded it must fail fast (return an overload error); unbounded queuing that drags down itself and upstream is forbidden
- Rate limit thresholds must be declared explicitly and reviewed periodically

---

## 4. Degradation and Fallbacks

- Core flows must have degradation plans (return cached data, default values, turn off non-core features); plans must be validated through drills
- Degradation switches must be configured explicitly and support hot switching; hardcoding is forbidden
- Non-core features may yield to core: circuit-break non-core calls to protect the core flow
- Exceptions on the degradation path must be caught and recorded; fallbacks return predefined data

---

## 5. Disaster Recovery and Restoration

- **Targets**: core business RTO ≤ 1 hour, RPO ≤ 15 minutes
- **Drills**: recovery drills once a quarter, disaster-recovery failover once a year (backup and disaster recovery requirements: see `rules/standards/data-standard`)
- **Post-incident review**: P0/P1 incidents must be reviewed; improvement items land as gates, automation, or architecture changes and are tracked to closure

---

## 6. Chaos Engineering (As Needed)

- Core services are advised to inject faults periodically (dependency timeouts, network jitter, instance crashes) to verify that circuit breaking and degradation actually work
- Chaos experiments must run in a pre-production environment first; production injection requires blast-radius control and a one-click abort

---

## 7. Conflict Adjudication with Other Domains

### 7.1 With Cost

- **Adjudication process**: SRE + management adjudicate jointly by business tier
- **Example**: active-active disaster recovery vs cost → define the disaster recovery level by business tier; non-core business may use degraded disaster recovery

### 7.2 With Performance

- **Adjudication basis**: reliability parameters (circuit breaker thresholds, retry counts) must not be sacrificed for performance
- **Adjudication process**: SRE + Architect adjudicate jointly

### 7.3 With Delivery Schedule

- **Adjudication basis**: same logic as P-004 — timeouts / circuit breaking / degradation are part of the feature
- **Forbidden**: omitting reliability facilities on "schedule pressure" grounds is forbidden
