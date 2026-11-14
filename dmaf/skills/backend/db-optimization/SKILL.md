---
name: db-optimization
description: Database performance optimization. Slow-query diagnosis, EXPLAIN analysis, fixing common degradation patterns, connection pool tuning. Use when the user reports "slow queries", "database performance", "optimize SQL", or "connection pool". Measure first, then optimize.
---

# Database Optimization

Measure first, then optimize: locate problems with data and prove the effect with a before/after comparison.

## Workflow

### 1. Locate
- Slow-query logs / pg_stat_statements to find the TOP SQL
- EXPLAIN (ANALYZE, BUFFERS) to read the execution plan:
  - `Seq Scan` on a large table → missing index or index not hit
  - Large gap between the `rows` estimate and actual rows → run ANALYZE first to refresh statistics
  - High `Buffers: shared read` → cache misses
  - `Sort` over large sets → consider an index to eliminate the sort
  - Nested loop × large table → missing index on the join condition

### 2. Common degradation patterns and fixes
| Pattern | Fix |
| :--- | :--- |
| N+1 queries | Batch IN queries or JOIN; turn on the ORM's batch loading |
| Function wrapped around a column (`WHERE LOWER(email)=`) | Expression index |
| Implicit type conversion | Make the parameter type explicitly match the column type |
| Large OFFSET | Cursor pagination |
| SELECT * | Select only the needed columns, enabling a covering index |
| COUNT(*) on large tables | Estimated count / counter table / async tallying |

### 3. Write optimization
- Batch inserts use multi-row INSERT / COPY; row-by-row inserts inside a loop are forbidden
- Split large bulk updates into batches (5000 rows per batch) to avoid long transactions and replica lag
- Use `ON CONFLICT` for unique-key conflicts; do not check-then-insert

### 4. Connection pool (configure explicitly; relying on defaults is forbidden)
- Max connections ≈ 2–4× core count; idle timeout 5 minutes; acquisition timeout 3 seconds with fast failure; leak detection on (15 minutes)

### 5. Verify and consolidate
- Before/after comparison on the same benchmark; prove the effect with data
- Index changes go through the index review checklist; backfill conclusions into the standards or an ADR

## Boundaries (not in this skill)

- Design-time decisions on indexes and table structure go through `detailed-design-backend` (the index-design section of its database reference file); this skill owns feedback-driven runtime optimization
- Preventive design against slow queries (batch writes, initial connection pool configuration) is executed with `backend-development`

## Hard rules

- Speculative optimization without supporting data is forbidden
- Micro-optimization that sacrifices readability is forbidden
- EXPLAIN validation on an empty table is invalid; the data volume must approach production levels

## Reference files

- For index design and validation details, see the index-design section of the `detailed-design-backend` skill's database reference file
- `rules/standards/performance-standard.md` / `rules/standards/data-standard.md` — higher-level constraints

## Default values

Connection pool ≈ 2–4× core count, idle timeout 5 minutes, acquisition timeout 3 seconds, leak detection 15 minutes, backfill batches of 5000 rows are organization defaults; adjustments based on measurements must be recorded. This skill uses PostgreSQL as its baseline; other databases bind per the project's tech-stack declaration.
