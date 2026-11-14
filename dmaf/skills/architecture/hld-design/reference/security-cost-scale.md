# Security, Low Cost, and Scale

> When to read: requirements carry compliance/security demands or hard cost constraints, or fast business growth demands judging when the architecture qualitatively changes

Among the six sources of complexity, beyond high performance, high availability, and extensibility, there are three more: **low cost, security, scale**. Their triggers and responses differ; before designing, judge which is the current primary complexity.

## Identification signals

| Complexity source | Requirement/business trait signals | Typical phrasing |
|---|---|---|
| Security | System exposed to the public internet; involves user privacy/money/transactions; compliance audits required (MLPS, PCI-DSS, etc.); industry precedents exist (leaks, DDoS incidents) | "What happens if we are attacked / breached?" "Regulators require us to pass MLPS" |
| Low cost | Server fleet in the hundreds, thousands, even tens of thousands; startups / cash-burning / margin-sensitive businesses; the boss draws an explicit cost red line | "Can the scheme save 20% of the machines?" "The budget is only this much" |
| Scale | Users/data/features growing steadily toward known thresholds (e.g., a 50-million-row MySQL table); features accreting, logic branches multiplying; the old system is "unreadable, unmodifiable, untouchable, unfixable" | "Can today's architecture survive 10× more data?" |

**Priority judgment**: low cost is rarely the primary goal — usually an **additional constraint**; security gets attention only after incidents, at enormous price (a single leak can reach hundreds of millions of users); scale erupts when quantitative accumulation hits the threshold — the easiest to respond to passively.

## Security (functional security / architectural security)

By technical lens, two classes: **functional security** (against vulnerability exploitation, e.g., injection/XSS/privilege abuse) and **architectural security** (against systemic attacks, e.g., DDoS). A security failure damages the business and corporate image enormously.

| Dimension | Functional security | Architectural security |
|---|---|---|
| Root cause | Implementation vulnerabilities give attackers an opening | The exposed surface is directly hammered or destroyed by traffic |
| Typical threats | XSS, CSRF, SQL injection, password cracking, framework vulnerabilities | DDoS, network-layer attacks, egress bandwidth exhaustion |
| Relation to architecture | Tied to concrete coding; little to do with architecture | One of architecture design's core concerns |
| Resolution | Framework-builtin protections + coding standards + continuous attack/defense improvement | Boundary isolation + cloud provider bandwidth/traffic scrubbing |
| One-shot fix possible? | No — attack and defense are a running contradiction; countermeasures come only after problems appear | External capability as the backstop; self-built means are limited |
| The architect's role | Set standards, select frameworks, establish response mechanisms | Select the protection system, assess egress bandwidth and scrubbing options |

### Functional security

**Essence**: implementation vulnerabilities exploited by attackers. Tied to **concrete coding**, little to do with architecture; a running contradiction of "attack" and "defense", **never fully solved at architecture-design time**. Frameworks (Spring Security et al.) ship common protections and cut duplicated work, but only guard known vulnerabilities — and frameworks themselves have vulnerabilities (Struts2 repeatedly disclosed critical RCE vulnerabilities).

#### Common attacks and countermeasures table

| Attack | Principle | Main countermeasures | Notes |
|---|---|---|---|
| XSS | Malicious script injected into pages steals cookies/sessions | Output encoding/escaping, CSP, HttpOnly cookies, input filtering | Stored/reflected/DOM types handled separately |
| SQL injection | Concatenated SQL changes semantics | Parameterized queries/prepared statements, ORM, least-privilege database accounts, no SQL concatenation | Even with framework defaults, validate dynamic concatenation points |
| CSRF | Forged user requests abuse a logged-in session | CSRF tokens, SameSite cookies, Referer/Origin validation | Often chained with XSS — defend both |
| Session hijacking | Stealing/predicting Session IDs | Site-wide HTTPS, HttpOnly+Secure cookies, session timeouts and rotation, device-fingerprint binding | Reset the session on any login-state change |
| Password cracking | Credential stuffing, brute force, weak passwords | Salted password hashing (bcrypt/Argon2), rate limiting/captchas/lockout, multi-factor authentication (MFA) | Weak passwords/credential stuffing are the most common entry point |
| Unauthorized access | Horizontal/vertical privilege escalation — altering IDs to reach others' resources | Server-side per-request authorization, resource-ownership checks, RBAC | Hiding entry points on the frontend is not a defense |
| File upload / deserialization vulnerabilities | Uploadable executables, crafted malicious objects | Type/content validation, isolated storage, allowlist-based deserialization | Common RCE entries |
| Framework/component vulnerabilities | Known CVEs like Struts2 RCE | Component inventory (SBOM) + vulnerability subscriptions + timely upgrades | Legacy old-version components are common intrusion entries |

#### Common defense thinking

1. **All user input is untrusted**: validate and filter uniformly at every entry (URL, forms, headers, cookies, files, messages, third-party callbacks).
2. **Encryption**: transport encryption (HTTPS), storage encryption (sensitive fields), irreversible password hashing.
3. **Validation**: the server validates as the authority; frontend validation is UX only; authorization checks run per request.
4. **Filtering/escaping**: escape per output context (HTML/JS/SQL/Shell), not one global pass.
5. **Defense in depth**: any bypassed layer still has the next behind it; functional security is continuous — maintain a vulnerability response and patch mechanism.

### Architectural security

**Essence**: direct impact/destruction of the system's exposed surface; internet systems sit on the public internet — attacks can be launched from anywhere on Earth.

**The firewall's position and limits**: its basic function is **network isolation** — partitioning zones and setting inter-zone access-control policies, suitable for banks and traditional enterprises' layered defense; its limit is mediocre performance — unfit for internet-scale massive concurrency. **Firewalls cannot stop DDoS**: attacks reach tens of GB up to 665 Gbps (the 2016 Krebs blog attack), far beyond a single firewall's throughput (≈25 GB/s, ≈¥100k per unit — stacking nearly 30 units ≈¥3 million still fails), and DDoS's greatest impact is exhausting the datacenter's total egress bandwidth — with egress exhausted, normal requests cannot enter, so defending the inside equals not defending at all. **Conclusion**: internet systems' architectural security relies mainly on carriers'/cloud providers' strong bandwidth and traffic-scrubbing capacity, not self-built firewall stacks.

#### Architectural security means, by layer, with selection

| Layer | Means | What problem it solves | Selection essentials |
|---|---|---|---|
| Network isolation | Firewalls, security groups, VPC/subnet partitioning | Control inter-trust-domain access; against port scans and internal lateral movement | Mandatory for traditional/enterprise/banking systems; internet systems use it for internal isolation — not for anti-DDoS |
| Application-layer protection | WAF | Blocks SQL injection, XSS, and other application-layer attacks | Use when performance overhead is acceptable; rules need continuous updates |
| Transport security | HTTPS/TLS | Against eavesdropping, tampering, session hijacking | Site-wide HTTPS is the internet baseline; mind certificate management and handshake overhead |
| Volumetric attacks | Anti-DDoS: carrier/cloud traffic scrubbing, CDN offload, Anycast | Absorbs/scrubs mass attack traffic, protecting egress bandwidth | **The first choice for internet systems**: buy cloud Anti-DDoS (high-defense IP) services; do not build firewall stacks |
| Intrusion detection | IDS/IPS, host-security agents, log audit | Detects intrusions that bypassed boundary protection | Pair with an incident-response process; alerting without handling equals not doing it |

### Security design checklist

- [ ] Are the security boundary and trust domains marked out (public zone/DMZ/internal)?
- [ ] Is all user input validated uniformly? Is "all user input is untrusted" actually enforced?
- [ ] Are database accesses parameterized queries? Is dynamic SQL concatenation disabled?
- [ ] Is the whole site on HTTPS? Are cookies HttpOnly/Secure/SameSite?
- [ ] Do sensitive operations (login, payment) have rate limiting/captcha/MFA?
- [ ] Is there a unified authorization middleware checking permissions per request (against privilege escalation)?
- [ ] Do third-party frameworks/components have an inventory and a vulnerability-tracking mechanism?
- [ ] Does DDoS protection rely on cloud provider/carrier scrubbing capacity rather than self-built firewall stacks?
- [ ] Are there log audit and intrusion alerts, plus an incident-response plan for when things go wrong?
- [ ] Is sensitive data (passwords, ID numbers, phone numbers) encrypted/hashed at rest and masked in display?

## Low cost

### Position and identification signals

- **Scale threshold**: at a few to a dozen servers, cost usually is not the focus; at **hundreds, thousands, even tens of thousands** it becomes a major design consideration.
- **Order-of-magnitude conclusion**: at ten-thousand-server scale, one architecture decision can save tens of millions per year — option A at 10000 machines vs option B at 8000 saves 20% = 2000 machines × ≈¥20k/machine/year = **¥40 million/year**.
- **Identification signals**: startups / cash-burning / margin-sensitive businesses; total servers past a thousand; the boss draws an explicit cost red line; commoditized businesses competing on cost.

### The conflict with high performance/high availability: an additional constraint, not the primary goal

- High performance/high availability's generic means is **adding servers**; low cost demands exactly the opposite — **fewer servers** — the two are in essential conflict, so low cost is usually not the primary goal but an **additional constraint**.
- Recommended process: **set the cost target first → design for high performance/high availability → evaluate against the target → redesign on miss → only when design means are exhausted and the target still unmet, adjust the cost target (talk to the boss)**.
- Low-cost design **must not sacrifice the system's ability to evolve**: over-customized, paint-into-a-corner schemes pay far more after business growth; exhaust design means before adjusting the target.

### Cost-reduction means compared

Core thesis: **often only "innovation" reaches the low-cost target** — introducing new technology, or creating new technology (opening an entirely new technical field; beyond the vast majority of companies).

| Means | What problem it solves | Pros | Risks | Fit |
|---|---|---|---|---|
| Replace commercial software with open source | Eliminates commercial license fees | Immediate savings, mature communities | Requires building ops capability; some open source carries cloud vendors' "free-rider" risk | Universal across scales; the first choice for small/mid companies |
| Introduce new technology to replace old | Old technology's efficiency/cost fails at scale | Industry-validated, low pitfall cost | Requires learning the new and integrating it with existing; migration risk | Small/mid companies' main path to low cost |
| Replace mismatched technology with fitting technology | Wrong language/component choices wasting resources (Ruby on Rails → Java, Lua+Redis → Go) | Large performance/resource gains | High rewrite cost; team skill transition | When the bottleneck is clear and gains are quantifiable |
| In-house creation of new technology | No existing technology meets the cost target | A qualitative leap; largest cost benefit | Highest complexity: needs wholly new ideas and technology, with a qualitative improvement over the old | Large companies (with ample resources, technology, time) |
| Cache/storage tiering and hardware optimization | Memory caching is expensive, small, and pierces to the DB (Sina Weibo: Redis/MC + SSD Cache + MySQL, SSD Cache as L2) | Balances cost, capacity, and DB protection | Architecture complexity rises; consistency management gets harder | When cache costs dominate and reads far exceed writes |
| Down-speccing / virtualization / containerization / mixed deployment | Low machine utilization; exclusive allocation wastes | Lifts utilization; spreads cost | Resource contention hurts stability — needs isolation and scheduling | Large fleets with measurable utilization |

**Industry cases** (all innovations squeezed out by low-cost/high-concurrency pressure): NoSQL (Memcache/Redis) relieving relational databases' high-concurrency pressure; full-text search engines (Sphinx/Elasticsearch/Solr) fixing inefficient LIKE search; Hadoop solving massive data storage and computation; Facebook's HipHop PHP → HHVM fixing PHP inefficiency; Sina Weibo's Redis/MC + SSD Cache + MySQL fixing expensive/small/piercing caches; LinkedIn building Kafka for **500 billion** daily events; Google's three papers (GFS/Bigtable/MapReduce) are the wellspring of the "cheap commodity machines replacing high-end hardware" innovation (see the scale section).

**Company-scale paths**: small/mid companies reach low cost by **introducing new technology** (complexity = learning it + integrating it with existing technology); only large companies can **create new technology** (complexity = inventing wholly new ideas + a qualitative leap over the old — ample resources, technology, time required). **Design implication**: when routine means cannot meet the cost constraint, evaluate "introduce a more fitting new technology" as a formal alternative; the introduction complexity (learning + integration + migration) must enter the cost account.

### Cost assessment operating points

- Standardize accounting as "cost per machine per year" (≈¥20k/machine/year); compare schemes directly as annual money differences for decision communication.
- Assessment dimensions go beyond machine count: license fees, bandwidth/traffic fees, storage-media price gaps (memory vs SSD vs HDD), ops staffing.
- Gains must be quantifiable and merged across items (e.g., SSD Cache as L2 simultaneously saves memory cost and relieves DB pressure).

## Scale

### Identification signals

- No high-performance demands, no dual-datacenter high-availability demands, little extensibility need — yet the system is **especially complex**: many features, many logic branches.
- Long development history, features accreting; later maintainers do not even know the features' use contexts — facing a black-box system: **unreadable, unmodifiable, untouchable, unfixable**.
- Data growth curves approaching known thresholds: a MySQL single table is generally recommended at **≈50 million rows**; user/request counts multiplying yearly.

**Core thesis: scale breeds complexity mainly through "quantitative change triggering qualitative change"** — past a threshold, complexity changes in kind.

### Three manifestations of quantitative → qualitative change

**1. Function qualitative change: more functions, complexity rises explosively**

Complexity = function count + inter-function connection count (when pairwise related):

| Functions | Calculation | Complexity |
|---|---|---|
| 3 | 3 + 3 | 6 |
| 8 | 8 + 28 | 36 |

8 functions are not 5 more than 3 — they are 30 more: connection count grows **explosively** with function count. Response: split the system, servicization/microservices, turning "one system's internal connections" into "boundary connections between few systems".

**2. Data qualitative change: single-table splits and big data**

Past its limit (say 1 billion rows), a MySQL single table: adding an index takes hours with no inserts allowed during (effectively a business outage), DDL takes very long, oversized data makes indexes slow, backups take very long. Response: **sharding** — but split rules (by user id or registration time), once set, are hard to change; cross-shard queries (e.g., "users with a bachelor's degree" need multi-table aggregation) must be designed up front, or splitting lands on a performance cliff the day it ships.
One order of magnitude further, traditional collection/processing/storage/analysis fail wholesale — entering **big data**; the theoretical basis is Google's three papers, each opening a technical field: **GFS** (big-data file storage), **Bigtable** (column-oriented data storage), **MapReduce** (big-data computation).

**3. Structural qualitative change: deployment upgrades**

A single datacenter's capacity (bandwidth, power, disaster-recovery radius) has a threshold; beyond it come multi-datacenter and even cross-region active-active; one instance/cluster overwhelmed means clusters and active-active. This qualitative change costs the most — plan before the threshold, execute after it trips.

### Architecture roadmap by scale

| Scale stage | Typical signals | Typical architecture actions |
|---|---|---|
| Early (single/few machines) | Small data, few features | Monolith + one database; iterate fast |
| Growth (ten-thousand users; data near the single-table threshold) | Queries slowing; tens of millions of rows | Read/write splitting, add caching (Redis/MC), index tuning |
| Maturity (single table/database overwhelmed) | Index/DDL/backup measured in hours | Sharding (fix split rules and cross-shard queries first), introduce search engines/NoSQL |
| Massive (single-machine compute/storage failing wholesale) | Data processing beyond traditional means | The big-data technology stack (the Hadoop ecosystem: GFS/Bigtable/MapReduce ideas) |
| Feature bloat (complexity growing 6 → 36 style) | Black-box systems; untouchable, unmodifiable | Service splitting/microservices; cap inter-function connections |
| Capacity/disaster-recovery bottleneck | Datacenter bandwidth, power, DR radius at the ceiling | Multi-datacenter, same-city dual datacenters, cross-region active-active |

**Essentials**: architecture evolves with scale — do not over-design for imagined scale (violating the "fitness beats industry excellence" principle), but **monitor the qualitative-change thresholds** (a 50-million-row table, function connection counts, datacenter capacity) and reserve an evolution path during the quantitative phase.

### Scale qualitative-change threshold quick reference

| Dimension | Qualitative-change threshold (rule of thumb) | Quantitative warnings before it | Architecture actions after it |
|---|---|---|---|
| Data (single table) | ≈50 million rows | Queries slowing; index bloat | Sharding — fix split rules and cross-shard queries first |
| Data (system-wide) | Traditional storage/computation failing wholesale | Backup/analysis times intolerable | The big-data technology stack (GFS/Bigtable/MapReduce ideas) |
| Functions | Pairwise-related functions' connections grow explosively | One change drags many modules | System splitting/servicization; converge connections into boundary interfaces |
| Deployment | Datacenter bandwidth, power, DR radius at ceiling | Frequent bandwidth expansions; single-point risk rising | Multi-datacenter → same-city dual datacenters → cross-region active-active |
| Traffic | Single instance/cluster limits | Sustained peak expansions | Load balancing + horizontal scaling + caching/async peak shaving |

## How the three interact

- **Scale → low cost**: at scale, machine counts multiply every waste, and cost constraints force technology innovation (cache tiering, stack replacement, in-house systems).
- **Scale → security**: the larger the user and data volume, the bigger the blast radius of leaks/attacks; security upgrades from "framework default protections" to dedicated design (scrubbing capacity, encryption, audit).
- **Low cost ↔ security/high availability**: cost cutting must not cut the security floor — one incident's loss usually exceeds years of machine savings; security budget outranks performance optimization.
- **Recommended design order**: design for high performance/high availability first → test against cost constraints → overlay the security checklist → monitor scale thresholds for evolution timing.

## Selection decision table

| Condition/scenario | Judgment | Recommendation |
|---|---|---|
| <100 servers, no compliance requirements | Neither security nor cost is a primary complexity | Framework-builtin security capabilities + baseline HTTPS suffice; spend the effort on the business |
| Public-internet-reachable, touching user privacy/money/compliance | Security becomes the primary complexity | Functional security: framework protections + input validation + checklist implementation; architectural security: buy cloud anti-DDoS/traffic scrubbing — no self-built firewall stacks |
| Hit or expecting DDoS (GB–Tb scale) | An architectural security problem | Carrier/cloud provider bandwidth + traffic scrubbing + CDN offload; firewalls only for internal isolation |
| Hundreds to thousands of servers; the boss draws a cost red line | Cost constraints override | Set the cost target first → design high-performance/high-availability schemes → iterate on miss; prefer introducing new technology (open source for commercial, replacing mismatched stacks); do not build wheels in-house |
| New technology still cannot meet the cost target — and the company is a large one | New technology must be created | Assess whether in-house work (HHVM-, Kafka-class) can reach a qualitative leap; small/mid companies should turn back and adjust the cost target |
| A MySQL single table nearing 50 million rows | The eve of data qualitative change | Design sharding ahead: fix split rules (user id/time) and cross-shard query schemes before implementing |
| Data scale beyond traditional storage/computation | Entering big data | The open-source ecosystem behind GFS/Bigtable/MapReduce (HDFS/HBase/MapReduce and successors) |
| Features accreting; the system untouchable, unmodifiable | Function complexity qualitative change | Servicization/microservice splitting; cut pairwise inter-function connections |
| Datacenter bandwidth/disaster recovery at the ceiling | Structural qualitative change | Multi-datacenter → same-city dual datacenters → cross-region active-active, evolving stepwise |

## Common pitfalls

- **Treating low cost as the primary goal**: sacrificing high-performance/high-availability floors to save machines; the correct posture is an additional constraint — set the target, iterate design on miss, adjust the target only when truly stuck, never by crushing quality.
- **Stacking firewalls against DDoS**: expensive (665-Gbps scale needs ≈30 units, ≈¥3 million) and useless — with egress bandwidth exhausted the business is down anyway; use carrier/cloud traffic scrubbing.
- **Assuming framework-builtin security is a one-shot fix**: frameworks only guard common vulnerabilities and have vulnerabilities themselves (Struts2 RCE); functional security is continuous attack/defense, needing vulnerability tracking and response mechanisms.
- **One filter at the entry**: once the input-trust assumption is bypassed, everything falls; hold "all user input is untrusted", escape per output context, defend in depth.
- **Pre-sharding/microservicing for imagined scale**: scale qualitative change has thresholds — splitting early only adds complexity; the right rhythm is monitoring threshold signals, reserving the path during the quantitative phase, executing after it trips.
- **Setting split rules without query schemes**: defining split rules (by id or time) without designing cross-shard queries ("bachelor's degree" aggregation) lands on a performance cliff the day it ships.
- **Choosing technologies that cannot evolve to save money**: over-customized/locked-in schemes mean starting over after growth — total cost higher; low-cost design must preserve evolvability.
- **Ignoring the connection explosion from feature accretion**: assuming 3 → 8 functions is "5 more features" when complexity went 6 → 36; when adding features, assess coupling connections with existing ones and converge boundaries promptly.
- **Small/mid companies imitating large companies' in-house wheels**: creating new technology demands long investment in resources, technology, and time; small/mid companies' low-cost path is introducing validated new technology.
- **Security as remedial construction**: no peacetime investment, then losses (leaks/DDoS) dwarf construction costs (Yahoo's 500-million-user leak; Huidah's 20-million-record leak and its chain of consequences).
