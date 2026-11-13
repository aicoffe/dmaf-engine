# Caching Design (lld design phase)

> Deep norms for the lld caching scheme. Trigger condition: must-read when lld design involves caching (read acceleration / key design / TTL / consistency).

## Workflow

### 1. Necessity criteria
- Caching is not a default: design a cache only when **read-heavy + a tolerable inconsistency window + a hit-rate observation loop** all hold; writing only "add a cache to improve performance" with no hit-rate estimate = incomplete design
- Strongly consistent data (balances, stock deduction results) never goes through the cache read path; this kind of data needs no evaluation — excluded outright

### 2. Design decision sequence
1. **Pattern**: cache-aside by default (on a read miss, load from the database and backfill; on write, update the database first then evict the cache); read-through/write-through and Write-Behind only when latency-sensitive with operational guarantees in place, and they require an ADR
2. **Keys and TTL**: key format `{service}:{domain}:{object}:{ID}`, colon-separated, one organization-wide convention; every key gets an explicit TTL + random jitter; never-expiring keys are forbidden; single-key values default ≤ 100KB; large objects cache only ID lists; one serialization scheme per team, mixing forbidden; every key's TTL value must state its **business freshness rationale** (in the style of "dictionary 1 day / transaction session 30 minutes"); numbers without a rationale are forbidden
3. **The three classic failure protections** (write "what measure is adopted"; "has been considered" is forbidden): penetration (querying nonexistent data) → cache empty values with a short TTL + upfront validation of input legality; breakdown (a hot key expiring at the wrong instant crushes the database) → mutex rebuild (singleflight) or logical non-expiry for hot keys + background refresh; avalanche (many keys expiring together) → TTL jitter + highly available cache cluster + multi-level caching
4. **Consistency**: database first, then cache eviction, by default; consistency-sensitive data uses delayed double deletion or binlog-subscription eviction; caching unmasked L3/L4 data is forbidden; making business decisions in the cache layer is forbidden; the design must **explicitly declare the tolerable window** (how far behind at most, and the blast radius of reading a stale value) — leaving it undeclared counts as a design defect

### 3. Design artifact checklist
- [ ] Pattern selected; exception paths carry an ADR number
- [ ] Key list (naming compliant) and each key's TTL + freshness rationale
- [ ] The three failure protections itemized and concrete
- [ ] Consistency strategy + tolerable-window declaration
- [ ] Observability: hit-rate metrics (`{service}_cache_hit_total` / `{service}_cache_miss_total`) written into the design; a hit rate below 70% by default triggers a review of key design and cache necessity

## Reference files

- `rules/standards/architecture-standard.md` / `rules/standards/performance-standard.md` — higher-level constraints
