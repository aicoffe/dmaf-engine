# Database Physical Design (lld design phase)

> Deep norms for the lld physical design chapter. Trigger condition: must-read when the lld includes physical design (modeling / table structure / indexes / migration).
> The organization-standard database is PostgreSQL; thresholds and default values are organization recommendations, adjustable to data scale with the adjustment recorded.

## Workflow

### 1. Modeling (conceptual → logical → context → physical)
- Steps: identify business entities and relationships → determine attributes/cardinality/unique identity → assign each entity to exactly **one bounded context** → physical landing → review against the self-check list
- Cross-context consumption goes through service interfaces or read-only snapshots (with a synchronization mechanism); cross-context foreign keys and shared tables are forbidden; the same concept may be modeled differently in different contexts
- Common modeling decisions: inheritance mapping (few subclasses → single table / many shared fields → class tables / large divergence → concrete tables per class); many-to-many join tables carry business attribute slots and audit fields; entities with a state machine keep a status field + a transition record table (who, when, from what to what); trees default to `parent_id` adjacency + recursive CTE, shallow categorizations may use path enumeration; multi-valued attributes must not be comma-separated (a separate child table, or JSONB + GIN subject to review)
- 3NF by default; every denormalization passes review one by one: quantifiable read benefit, an explicit redundancy synchronization mechanism, a declared inconsistency tolerable window, no cross-context redundancy of business data, and an ADR or spec on record

### 2. Logical → physical mapping
| Logical concept | Physical landing |
|---|---|
| Entity | Table (naming/types/required fields in §3) |
| Value object | Inlined fields or JSONB (queryable attributes via generated columns/expression indexes) |
| Aggregate | One primary table for the aggregate root + child tables with foreign keys; cross-aggregate only by ID reference |
| Enum | VARCHAR + application-layer enum (database ENUM is forbidden) |
| Inheritance | One of the three mappings in §1 |

### 3. Naming and types
| Object | Rule |
|---|---|
| Table name | lowercase with underscores, plural nouns (`claims`); fields lowercase with underscores (`claim_id`) |
| Primary key | `id` or `{entity}_id`, BIGINT (Snowflake) or UUID; auto-increment INT as a business primary key is forbidden |
| Foreign key/index/unique/constraint | `{referenced_entity}_id` / `idx_{table}_{cols}` / `uk_` / `fk_` |
| Types | money DECIMAL(19,4) or integer cents (FLOAT/DOUBLE forbidden); time TIMESTAMPTZ UTC (timezone-less TIMESTAMP forbidden); boolean BOOLEAN; enum VARCHAR + application layer; JSON as JSONB; large files go to object storage |
| Required fields for core tables | id, created_at, updated_at, created_by, updated_by, version (optimistic lock), deleted (soft delete) — log/audit/temp tables may omit updated_at/version/deleted |
| Constraints | not-null, unique, and comments explicit; PII fields marked for encryption and masking (classification and retention tier labels) |

### 4. Index design
- Mandatory: primary key (automatic), foreign keys, high-frequency query fields, business uniqueness; consider indexes for high-frequency sort/group combinations
- Composite index column order: equality columns first, range columns after, sort following; leftmost prefix; higher-selectivity columns first
- Types (PostgreSQL): B-Tree default; GIN for JSONB/full-text/arrays; GiST for geo/range; partial indexes for fixed subsets; expression indexes for function-based queries (strictly matching the application's expression)
- Forbidden: B-Tree on TEXT/JSONB; standalone indexes on low-cardinality fields; too many indexes on frequently updated fields (write amplification); unindexed foreign keys; more than 5 indexes per table (default value; beyond that, justify each one)
- Before release, `EXPLAIN (ANALYZE, BUFFERS)` validated on data volumes close to production (validating on empty tables is meaningless); `Index Only Scan` requires a covering index

### 5. Capacity and partitioning
- Estimated rows/year = write QPS × 86400 × 365 × peak factor — **a partitioning decision exists only with an estimate**
- 10 million rows in a single table (default value) triggers a partitioning plan: a high-frequency query field as the partition key; fix the key first, implementation can be deferred
- Hot/cold tiering: mark each table's archiving timing at design time

### 6. Performance design
- For each core table, list the **top-3 high-frequency queries** and the indexes they hit (for empty tables, write out the expected access pattern for review)
- Write-amplification assessment: index counts held strict on high-write tables; large fields (TEXT/JSONB/BLOB) assessed separately; large files go to object storage
- Fields per table ≤ 50 (default value)

### 7. Migration (Flyway)
- Scripts: `V{major}.{sequence}__{description}.sql` under `db/migration/`; merged scripts are never modified; rollback scripts stored in pairs under `db/rollback/`; every DDL carries a comment header (what changes, the linked spec, affected tables)
- Expand-Contract in three phases (each phase released and rollbackable independently, with a default gap ≥ 1 iteration): add column (nullable + default) → dual-write → batched backfill (5000 rows per batch) → verify consistency → switch reads to the new column → drop the old column
- Large-table changes: add indexes with `CREATE INDEX CONCURRENTLY`; column type changes never in place (go through a new column); before dropping a column, stop writes and observe for one iteration; table renames/drops go through review
- Red lines: no ALTER during peak hours; no migration without rollback; pre-release verification in a test environment (including data scale) + a rollback drill

### 8. Review self-check
- [ ] Table list includes purpose, owning context, estimated rows/year
- [ ] Modeling decisions traceable (inheritance-mapping rationale, denormalization review records)
- [ ] Naming/types/required fields fully compliant; constraints and tier labels complete
- [ ] Top-3 high-frequency queries and their indexes in place; partition key reserved
- [ ] Migrations have rollbacks, large tables assessed for lock impact; PII marked for encryption and masking

## Hard rules

- FLOAT/DOUBLE for money forbidden; cross-context shared tables forbidden; comma-separated multi-values forbidden; auto-increment INT primary keys forbidden
- Growth estimates come first: no partitioning decision without an estimate
- No ALTER during peak hours; no direct DROP COLUMN (stop writes first); no in-place column type changes (go through a new column); no migration without rollback

## Reference files

- `rules/standards/data-standard.md` — data tiering, backup policy, migration safety red lines (L1)
- `rules/standards/security-standard.md` — PII tiering and masking (L1)
