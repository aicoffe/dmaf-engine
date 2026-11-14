---
name: data-standard
level: L1
domain: data
description: Load when data modeling, database design, data migration, data governance, or the data lifecycle is involved. Governs all data-related work.
owner: DBA Team
based_on: [organization-principles, security-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Data Domain Standard

> This standard governs "what to do and to what degree", and is the single source of truth for data classification. Development-level details are carried by the corresponding domain skills.

---

## 1. Data Classification and Grading

### 1.1 Data Classification Levels

| Level | Definition | Examples | Storage requirements | Access requirements |
| :--- | :--- | :--- | :--- | :--- |
| **L1 public** | Publicly accessible | Product introductions, announcements | No special requirements | No authentication needed |
| **L2 internal** | Internal visibility only | Internal reference numbers, employee information | Backup required | Authentication required |
| **L3 sensitive** | Leakage has severe impact | PII, financial data | Encrypted storage + backup | Authentication + role authorization + auditing |
| **L4 confidential** | Leakage has catastrophic impact | Secrets, credentials, core algorithms | Encrypted storage + off-site backup + access isolation | Least privilege + approval + auditing |

### 1.2 Classification Labeling Requirements

- **Tables**: must carry a `classification` label (L1-L4)
- **Fields**: sensitive fields must carry a `classification` label
- **API responses**: must label the classification level of the response data
- **Data export**: must label the classification level of the exported data

**Labeling method** (in DDL comments or the data dictionary):

```sql
COMMENT ON TABLE claims IS 'classification: L3; purpose: claims business processing; retention: 5y';
COMMENT ON COLUMN claims.id_card_number IS 'classification: L3; pii: true; encrypted: true';
```

### 1.3 Classification and Storage

| Level | Encryption | Backup | Archive | Destruction |
| :--- | :--- | :--- | :--- | :--- |
| **L1** | Not required | Optional | Optional | No requirements |
| **L2** | Not required | Required | Optional | Soft delete |
| **L3** | Field-level encryption | Required | Required | Soft delete + hard delete after 30 days |
| **L4** | Field-level encryption + key isolation | Required + off-site | Required | Soft delete + immediate hard delete (approval required) |

---

## 2. Data Modeling

### 2.1 Modeling Principles

| Principle | Description |
| :--- | :--- |
| **Normalization first** | Default to third normal form (3NF) to reduce redundancy |
| **Moderate denormalization** | Denormalize when query performance requires it, but it must be documented |
| **Domain-driven** | Model per bounded context; tables are not shared across contexts |
| **Explicit constraints** | Primary keys, foreign keys, unique constraints, and NOT NULL constraints must be declared explicitly |
| **Audit fields** | Core business tables must have audit fields |

**Modeling practice** (modeling steps, inheritance mapping choice, denormalization approval checklist): carried by the corresponding domain skills.

### 2.2 Field Naming, Types, and Mandatory Fields

Details of naming rules, field type mapping, and mandatory audit fields are carried by the corresponding domain skills. Hard invariants at this standard's level:
  - **Money must not use floating-point types** (`FLOAT` / `DOUBLE`); use `DECIMAL` or integers (cents)
  - **Time must carry a time zone** (`TIMESTAMPTZ`), stored in UTC uniformly
  - **Primary keys** use `BIGINT` (snowflake IDs) or `UUID`; auto-increment `INT` as a business primary key is forbidden

### 2.3 Relationship Modeling

| Relationship type | Implementation | Example |
| :--- | :--- | :--- |
| **One-to-one** | Foreign key + unique constraint | `users` ↔ `user_profiles` |
| **One-to-many** | Foreign key on the "many" side | `users` → `claims` (`claims.user_id`) |
| **Many-to-many** | Junction table | `users` ↔ `roles` (`user_roles`) |
| **Inheritance** | Single-table / class-table / concrete-table inheritance | Choose per scenario |

**Forbidden**:
- ❌ Sharing tables across bounded contexts is forbidden
- ❌ Storing multiple values as comma-separated strings is forbidden (e.g., `tags: "a,b,c"`)
- ❌ Storing relational data in JSON is forbidden (relational data must use relational tables)

---

## 3. Database Design

### 3.1 Database Selection

| Scenario | Recommended database | Description |
| :--- | :--- | :--- |
| **Transactional business** | PostgreSQL 15+ | Strong consistency, transaction support (the organization's sole OLTP standard selection) |
| **Analytical queries** | ClickHouse / Doris | Columnar storage, aggregate queries |
| **Document data** | MongoDB | Flexible schema |
| **Cache** | Redis 7+ | Caching strategy: see `rules/standards/architecture-standard` |
| **Search** | Elasticsearch | Full-text retrieval |
| **Time-series data** | TimescaleDB / InfluxDB | Monitoring metrics, logs |
| **Graph data** | Neo4j | Relationship networks |

**Selection principles**:
- Prefer organization-approved databases
- Introducing a new database requires Architecture Group approval
- The selection must be recorded in an ADR

### 3.2 Table Design Conventions

> The thresholds in the table below are organization defaults and may be adjusted for data scale; adjustments must be recorded.

| Convention | Requirement |
| :--- | :--- |
| **Columns per table** | ≤ 50 (beyond that, split the table) |
| **Rows per table** | Recommended ≤ 10 million (beyond that, split or archive) |
| **Primary key** | Required and immutable |
| **Foreign keys** | Must be declared explicitly (or enforced at the application layer + documented) |
| **NOT NULL constraints** | Business-required fields must be `NOT NULL` |
| **Default values** | Fields with default values must declare them explicitly |
| **Comments** | Tables and columns must have `COMMENT`s |

### 3.3 Index Design

- **Principles**: frequently queried fields must be indexed; foreign keys must be indexed (avoid full table scans); business-unique fields must have unique indexes; composite indexes follow the leftmost prefix principle
- Index details (column order, type choice, EXPLAIN verification, review checklist) are carried by the corresponding domain skills

### 3.4 Partitioning and Table Splitting

| Scenario | Strategy |
| :--- | :--- |
| **Table > 10 million rows** | Partition by time (e.g., monthly partitions) |
| **Table > 100 million rows** | Split across databases and tables (e.g., hash by user ID) |
| **Hot/cold data separation** | Archive historical data to cold storage |
| **Partition key choice** | Frequently queried fields (e.g., `created_at`, `tenant_id`) |

**Database/table splitting principles**:
- Shard keys must be high-cardinality and evenly distributed
- Cross-shard queries must be avoided (or solved via redundant tables)
- Sharding rules must be documented

---

## 4. Data Migration

### 4.1 Migration Tooling

- Migrations must be managed with a versioned migration tool (tool selection, script conventions, and practice: see )

**Requirements**:
- All migrations must be version-controlled
- Every migration must have a corresponding rollback script
- Migration scripts must be verified in a test environment

### 4.2 Migration Safety Principles

| Principle | Description |
| :--- | :--- |
| **Expand-Contract** | Large table changes must go through three phases: expand → migrate → contract |
| **Zero downtime** | Production migrations must not lock tables for more than 1 second |
| **Rollback-able** | Every migration must have a rollback plan |
| **Canary** | High-risk migrations must roll out as canaries |
| **Auditing** | All migrations must be recorded in audit logs |

### 4.3 The Expand-Contract Pattern

```
Phase 1: Expand
- Add columns/tables without affecting existing code
- Dual write: write both old and new fields
- Deploy code and start using the new field

Phase 2: Migrate
- Backfill historical data with a background job
- Verify data consistency

Phase 3: Contract
- Switch code fully to the new field
- Drop the old field (after confirming no callers remain)
```

### 4.4 Prohibitions

- ❌ Running `ALTER TABLE` during business peak hours is forbidden
- ❌ Direct `DROP COLUMN` is forbidden (confirm no callers first)
- ❌ Changing the type of an existing column is forbidden (add a new column + migrate instead)
- ❌ Business logic inside migration scripts is forbidden
- ❌ Migrations without rollback scripts are forbidden

---

## 5. Read/Write Splitting and Database/Table Splitting

### 5.1 Read/Write Splitting

| Scenario | Strategy |
| :--- | :--- |
| **Read-heavy, write-light** | Writes to the primary, reads from replicas |
| **Strongly consistent queries** | Go to the primary |
| **Eventually consistent queries** | Go to replicas |
| **Replica lag** | Must be monitored; alert when lag > 1 second |

**Requirements**:
- Read/write splitting must be declared explicitly (e.g., a `@ReadOnly` annotation)
- Strongly consistent scenarios must go to the primary
- Replica lag must be monitored

### 5.2 Database/Table Splitting

| Scenario | Strategy |
| :--- | :--- |
| **Table > 100 million rows** | Split across databases and tables |
| **Shard key** | High-cardinality, evenly distributed (e.g., `user_id`, `tenant_id`) |
| **Sharding algorithm** | Hash modulo / range sharding / consistent hashing |
| **Cross-shard queries** | Avoid, or solve via redundant / aggregate tables |
| **Distributed transactions** | Avoid cross-shard transactions; use Saga / eventual consistency |

**Forbidden**:
- ❌ Frequently changing the shard key is forbidden
- ❌ Cross-shard JOINs are forbidden
- ❌ Cross-shard transactions are forbidden (unless a distributed transaction framework is used)

---

## 6. Data Quality

### 6.1 Data Quality Dimensions

| Dimension | Definition | Check method |
| :--- | :--- | :--- |
| **Completeness** | Required fields are non-null | `NOT NULL` constraints + periodic checks |
| **Uniqueness** | Unique fields have no duplicates | `UNIQUE` constraints + periodic checks |
| **Consistency** | Related data is consistent | Foreign key constraints + periodic checks |
| **Accuracy** | Data conforms to business rules | Validation rules + periodic checks |
| **Timeliness** | Data is updated in time | Update-time monitoring |
| **Validity** | Data formats are legal | Format validation + periodic checks |

### 6.2 Data Quality Monitoring

| Monitoring item | Frequency | Alert threshold |
| :--- | :--- | :--- |
| **Null rate** | Daily | Key-field null rate > 1% |
| **Duplicate rate** | Daily | Duplicate rate on unique fields > 0 |
| **Consistency** | Daily | Related-data inconsistencies > 0 |
| **Lag** | Real time | Replica lag > 1 second |
| **Abnormal growth** | Daily | Single-day growth > historical mean + 3σ |

### 6.3 Data Repair

- **Before repair**: must back up
- **During repair**: must log the repair
- **After repair**: must verify
- **Repair scripts**: must be version-controlled

---

## 7. Data Lifecycle

### 7.1 Lifecycle Stages

```
Creation → Active → Cold data → Archive → Destruction
```

| Stage | Definition | Storage | Access frequency |
| :--- | :--- | :--- | :--- |
| **Active** | Last 90 days | Hot storage (SSD) | High |
| **Cold data** | 90 days - 1 year | Warm storage (HDD) | Low |
| **Archive** | 1 year - retention period | Cold storage (object storage) | Very low |
| **Destruction** | Past the retention period | Deleted | None |

### 7.2 Archiving Strategy

| Data type | Archive timing | Archive location | Access method |
| :--- | :--- | :--- | :--- |
| **Business data** | After 1 year | Object storage (Parquet) | Restore on demand |
| **Logs** | After 90 days | Object storage | Query on demand |
| **Audit logs** | After 1 year | Cold storage | Query on demand |
| **Backups** | Per backup policy | Off-site storage | Disaster recovery restore |

### 7.3 Destruction Strategy

| Data type | Retention period | Destruction method | Approval |
| :--- | :--- | :--- | :--- |
| **User personal information** | 3 years | Anonymization or deletion | Compliance officer |
| **Transaction records** | 5 years | Archive then delete | Finance + Compliance |
| **Audit logs** | 5 years | Archive then delete | Compliance officer |
| **Logs** | 90 days | Automatic cleanup | No approval needed |
| **Temporary data** | 7 days | Automatic cleanup | No approval needed |

**Destruction requirements**:
- **Routine destruction at retention expiry**: archive before destruction (see 7.2), then hard delete + overwrite to ensure irrecoverability
- **User-exercised deletion rights or compliance-mandated deletion**: the "archive before destruction" requirement does not apply; all copies (including backups, archives, logs, caches) must be deleted or anonymized in sync, completed within 30 days
- The destruction action itself must be recorded in audit logs (the log content excludes the destroyed data)

---

## 8. Data Backup and Recovery

### 8.1 Backup Strategy

- **Core requirement**: core business databases get a daily full backup + hourly incremental backups, retained 30 days, local + off-site; configuration data goes into Git with each change and is kept permanently

### 8.2 Recovery Drills

- **Frequency**: once a quarter
- **Scope**: core business databases
- **Targets**: RTO ≤ 1 hour, RPO ≤ 15 minutes
- **Records**: drill reports must be archived

### 8.3 Disaster Recovery

- **RTO** (recovery time objective): core business ≤ 1 hour
- **RPO** (recovery point objective): core business ≤ 15 minutes
- **Off-site backup**: core data must be backed up off-site
- **DR failover**: drilled once a year

---

## 9. Data Security (Reference: `rules/standards/security-standard`)

### 9.1 Encryption

- **Transport encryption**: TLS 1.3 end to end
- **Storage encryption**: L3/L4 data must be field-level encrypted
- **Key management**: managed by a KMS, never hardcoded

### 9.2 Masking

- **Display masking**: PII fields are masked on display
- **Export masking**: masked by role on export
- **Log masking**: raw PII in logs must be masked (masked values are not PII)

### 9.3 Access Control

- **Least privilege**: each service is granted only the database permissions it needs
- **Role separation**: read and write accounts are separated
- **Auditing**: all sensitive data access must be audited

---

## 10. Data Governance

### 10.1 Data Dictionary

- **Requirement**: every database must have a data dictionary
- **Content**: table names, column names, types, constraints, comments, classification, retention periods
- **Maintenance**: updated in sync with DDL changes
- **Location**: `docs/data-dictionary.md` or a data governance platform

### 10.2 Data Lineage

- **Requirement**: core data must have lineage
- **Content**: data sources, flows, transformation logic
- **Tools**: Apache Atlas / DataHub / in-house

### 10.3 Data Owners

| Data type | Owner | Responsibilities |
| :--- | :--- | :--- |
| **User data** | Product Manager | Definition, usage, retention |
| **Transaction data** | Finance team | Accuracy, compliance |
| **Audit logs** | Compliance officer | Integrity, retention |
| **Configuration data** | Architecture Group | Change management |

---

## 11. Conflict Adjudication with Other Domains

### 11.1 With Security

- **Adjudication basis**: P-001 (security first)
- **Adjudication process**: Security Owner + DBA adjudicate jointly
- **Example**: encrypted fields cannot be indexed vs query performance → security first; use hash indexes or application-layer encryption

### 11.2 With Compliance

- **Adjudication basis**: P-002 (compliance first)
- **Adjudication process**: Compliance officer + DBA adjudicate jointly
- **Example**: 5-year data retention vs storage cost → compliance first; archive to cold storage

### 11.3 With Performance

- **Adjudication basis**: P-009 (user experience is part of the feature)
- **Adjudication process**: Architect + DBA adjudicate jointly
- **Example**: normalization vs query performance → moderate denormalization, but it must be documented

### 11.4 With Backward Compatibility

- **Adjudication basis**: P-005 (backward compatibility first)
- **Adjudication process**: the Architecture Group approves breaking changes
- **Example**: deleting old fields vs keeping deprecated fields → Expand-Contract, three phases

---

## 12. Change History

| Version | Date | Change | Approver |
| :--- | :--- | :--- | :--- |
| v1.0.0 | 2026-10-17 | Initial version | DBA Team |
