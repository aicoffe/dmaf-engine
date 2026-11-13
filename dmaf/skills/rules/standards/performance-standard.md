---
name: performance-standard
level: L1
domain: performance
description: Load when performance budgets, performance testing, performance optimization, or capacity planning is involved. Governs all performance-related work.
owner: Architecture Group + SRE Team
based_on: [organization-principles, architecture-standard, observability-standard, data-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Performance Domain Standard

> This standard governs performance standards and red lines; implementation details are carried by the corresponding domain skills.

---

## 1. Performance Budgets

- Every externally exposed interface must have a performance budget defined in the Design phase (P95 latency, throughput)
- The budget must be recorded explicitly in the spec (P-007)
- Changes exceeding the budget must not merge unless there is an exemption approved by the performance Owner with an expiry date
- Frontend page performance budgets: see `rules/standards/frontend-standard`

---

## 2. Performance Optimization Principles

- **Measure before optimizing**: speculative optimization without data support is forbidden; optimization must be backed by evidence (traces, metrics)
- **Optimization order**: architecture layer (caching, asynchrony, batching) > query layer (indexes, SQL) > code layer (algorithms, data structures)
- **Before/after comparison**: every optimization must be verified with a before/after comparison on the same baseline, proving the effect with data
- ❌ Micro-optimizations that sacrifice readability are forbidden (skipping one or two object creations inside loops and the like — never do them)

---

## 3. Performance Testing

| Requirement | Description |
| :--- | :--- |
| **Load testing** | Required for core interfaces; default load is 1.5x the expected peak |
| **Performance regression** | Must run before major releases, compared against the performance budget |
| **Environment equivalence** | Must run on a configuration equivalent to production; differences must be recorded |
| **Data scale** | Test data volume must approach production scale (or be extrapolated from the capacity model) |

---

## 4. Capacity Planning

- Core services must establish a capacity model (QPS × per-request resource footprint → instance count)
- Resource watermarks (default values): 60% warning, 80% scale out
- Capacity assessments must be updated before major business events

---

## 5. Performance Monitoring

- Performance metrics must be hooked into monitoring, with alerts tied to SLOs (for the definition, see `rules/standards/observability-standard`)
- Slow queries must have monitoring and threshold alerts, together with index requirements (see `rules/standards/data-standard`)

---

## 6. Conflict Adjudication with Other Domains

### 6.1 With Maintainability

- **Adjudication basis**: P-008 (maintainability over short-term delivery)
- **Adjudication process**: Architect + SRE adjudicate jointly
- **Example**: speculative micro-optimization vs code readability → readability first; architecture-level optimization (caching/asynchrony) over code-level

### 6.2 With Cost

- **Adjudication process**: Performance Owner + management adjudicate jointly
- **Example**: unlimited scaling vs resource cost → optimize within the capacity model first; scale out per the watermark policy

### 6.3 With Delivery Schedule

- **Adjudication process**: Tech Lead + Performance Owner adjudicate jointly
- **Example**: within budget → delivery is not blocked; over budget → must optimize or obtain a time-limited exemption
