# High-Availability Architecture Patterns
> When to read: after the complexity identification stage confirms "high availability" as a primary complexity, or in detailed design when doing high-availability design, FMEA screening, and failure-response design

**Usage guide**: (1) identification — use the identification signals to confirm whether high availability is a primary complexity and anchor the target grade with availability metrics; (2) solution design — compute high availability consults the pattern comparison table, storage consults the two-machine/cluster/partition tables, cross-geo needs consult cross-region active-active; (3) solution validation — screen hazards function point by function point with FMEA; (4) failure-response design — consult the interface-level failure decision table; (5) final verification — check item by item against the selection decision table and common pitfalls.
## Identification signals and availability metrics
### Which requirements signal high availability as a primary complexity
- An explicit availability commitment (how many nines/SLA) or downtime meaning major money/reputation loss (Alipay, WeChat, Didi class); news sites, blogs, and internal IT systems have small disruption impact.
- Required to withstand **system-level failures** (crashes, datacenter power loss/fire, severed fiber) and even **geo-level disasters** (earthquakes, floods, major blackouts).
- Contains stateful data: storage high availability's difficulty is not backup but **reducing data inconsistency's impact on the business**; replication travels over links — a few ms in the same datacenter, tens to a hundred+ ms across datacenters (Guangzhou↔Beijing ≈50ms, spikes past 1s), and links break/congest/drop packets.
- Interface-level failures are frequent: the system is up but slow, timing out, erroring (typical: slow queries exhausting database resources); far more probable than system-level failures.
- Both compute and storage high availability rest on **state decisions**; a redundant system's state decision is essentially never fully correct — the design must accept and manage this contradiction.
### The essence of high availability
The essence of every high-availability solution is **redundancy**: in high performance, added machines "extend" capacity; in high availability, added machines "redunduplicate" units. Redundancy introduces three new complexities: task assignment (compute), data replication and inconsistency (storage), and state decision (judging who is primary and who is standby).
### Availability metrics
| Availability | Colloquial | Annual unavailability | Typical scenarios |
|---|---|---|---|
| 99.9% | three nines | ≈8.76 hours | ordinary online business |
| 99.99% | four nines | ≈50 minutes | core transactions, login |
| 99.999% | five nines | ≈5 minutes | finance-grade core |

Availability target → design action mapping:

| Commitment grade | Minimum design action |
|---|---|
| 2–3 nines (internal/back-office) | Single-machine redundancy: compute primary-standby (warm) + storage primary-standby replication; manual failover allowed |
| 3–4 nines (online business) | Automatic failover: load-balanced cluster + storage primary-secondary/failover; FMEA hazard screening |
| 4 nines+ (core transactions) | Cluster automation + mediator-based failover / centralized cluster (ZooKeeper) + the interface-level four essentials (degradation, circuit breaking, rate limiting, queuing) |
| Datacenter-level disaster recovery | Same-city cross-zone dual datacenters |
| City-level disaster survivable | Cross-city cross-region active-active (4 techniques + 4 steps) |
### Failure types and typical solutions overview
| Failure type | Typical scenario | Typical solution |
|---|---|---|
| Server hardware | Bad sectors (probability jumps past year 3), power loss | Compute redundancy clusters, storage replication |
| Program bugs | Infinite loops, memory exhaustion, slow queries | FMEA screening, failover, degradation and circuit breaking |
| Data corruption | Accidental deletion, dirty data | Multiple replicas, partitioned backups, log recovery |
| Infrastructure | Fiber cut (2015: Alipay outage over 4 hours; 2016: China-US submarine cable, 3 hours), datacenter power loss | Same-city cross-zone dual datacenters, multiple channels |
| Natural disasters | Earthquake, flood, major blackout | Data partitioning, cross-region active-active |
## CAP theory and practice
### Correctly understanding "choose two of three"
- Applies to: **interconnected** distributed systems that **share data**, concerned only with **data reads/writes**; a Memcache cluster (connectionless, no sharing) and ZooKeeper's election mechanism are not CAP subjects.
- Precise definitions of the three elements (second edition, the **client's** perspective):

| Element | Precise definition | Keyword analysis |
|---|---|---|
| Consistency | A read is guaranteed to return the **latest write's result** | Described from client+read; the system is inconsistent mid-transaction by nature — it does not claim "all nodes identical at every instant" |
| Availability | **Non-failing nodes** return a **reasonable** response in reasonable time (no error or timeout) | Reasonable ≠ correct: answering 90 when 100 is expected is a reasonable old value; "every request" is imprecise (failing nodes cannot guarantee it) |
| Partition Tolerance | The system continues to **function** after a **network partition** | States the phenomenon without limiting the cause: packet loss, connection breaks, congestion all count |

- **P is mandatory**: networks are never 100% reliable, partitions will happen, and a distributed system **cannot choose CA — only CP or AP**; without partitions it should **satisfy CA simultaneously** — the design must cover both states: choosing CP/AP under partition and keeping CA without one.
- CP: return Error during partition (sacrificing A); typical: ZooKeeper, user account data.
- AP: return this node's old data during partition — "a reasonable old value", not "the correct result" (sacrificing C).
### Practice guidance table
| Key detail | Practical guidance |
|---|---|
| CAP's granularity is **data**, not the system | Choose per data class: account data (ID/password) CP, profile data (nickname/hobbies) AP; one-size-fits-all for the whole system is forbidden |
| CAP ignores latency; perfect CP does not exist | Balances and inventory can only be **single-point writes** + replicas elsewhere; the system can still partition by user ID (0–100 → Node1, 101–200 → Node2) so one node's failure affects only some users |
| Sacrifice ≠ inaction; prepare for partition recovery | Log during partition; after recovery, sync/merge back to CA. CP: sync the newly added data after recovery; AP: merge conflicts by rule (last-modified-wins / longest-text-wins / human adjudication) |
| Guarantee CA when normal | Account data syncs via **message queues** (real-time and controllable, complex to implement); profile data via **database replication** (simple, higher latency) |
### CAP vs ACID vs BASE
| Theory | Domain | Essentials |
|---|---|---|
| ACID | Database transaction integrity | Atomicity/Consistency(integrity)/Isolation(four levels)/Durability; its A and C share names with CAP's but not meanings |
| BASE | **An extension of the AP approach** | Basically available (lose part to keep the core: login core, registration expendable); soft state (intermediate inconsistency allowed); eventual consistency |

Consistency window: account data should be consistent within **1 minute**; the latest post may tolerate **30 minutes**; CP is also eventually consistent (window of a few ms); AP sacrifices C during partition and must reach eventual consistency after recovery.
## FMEA hazard screening method
- **Definition**: Failure Mode and Effects Analysis; it does not guide design — used after design completes to **screen availability hazards comprehensively** — there are many exception scenarios, and missing one is a hazard.
- **Steps**: initial architecture diagram → assume a component fails → analyze the impact on function points → judge whether optimization is needed → compile "follow-up planning" ranked by risk.

**FMEA column meanings**:

| Column | Meaning and essentials |
|---|---|
| Function point | Divided from the **user's perspective** (login, registration); databases and caches are not function points |
| Failure mode | Failure point + failure form, quantified and precise: "MySQL response time reaches 3 seconds", not "slow responses"; causes are not listed — same phenomenon, same impact |
| Failure impact | Quantified: "80% of users cannot log in", not "most users"; order-of-magnitude estimates suffice |
| Severity | Fatal/high/medium/low/none; = importance × scope × damage. Fatal: >70% of users cannot log in; high: >30%; medium: all logins >5s or profiles uneditable; low: 10% of logins >5s, 20% cannot change avatars. Disputes are settled by discussion; unresolved, the architect decides |
| Failure cause | Same phenomenon, different causes → different probability, detection, and handling (bug → upgrade; missing index → add index; bad sectors → targeted inspection) |
| Failure probability | High/medium/low, three grades, for prioritization only. Rises with hardware age; mature open source low, newly released/first-use high; new in-house higher than mature in-house |
| Risk level | = severity × failure probability. Same impact, different causes, different risks: earthquakes are extremely rare (Guangzhou had exactly one magnitude-5+ quake in the 20th century) → low; AC units burn out about once in 2 years, rack power loss about once a year → high |
| Existing measures | Detection and alerting (manual intervention), fault tolerance (MySQL primary-standby automatic failover to the standby), self-healing (Hadoop reassigning replicas off a failed machine) |
| Avoidance measures | Lower failure **probability**: technically, keep a redundant copy of MongoDB data in MySQL; administratively, force replacement of disks older than 2 years |
| Solution measures | **Solve** the failure itself: password retry limits, sensitive-data encryption, allowlists; prefer solutions where possible; for self-unsolvable failures (bad sectors, open-source bugs) only avoidance applies |
| Follow-up planning | Compile missing/insufficient measures and invest resources ranked by risk level |

**Example** (user management: MySQL+Memcache+Server; function points: login/registration):

| Function point | Failure mode | Failure impact | Severity | Failure cause | Probability | Existing measures | Follow-up planning |
|---|---|---|---|---|---|---|---|
| Login | MySQL response time reaches 3 seconds | 80% of users cannot log in | Fatal | Bad sectors/slow queries/network failure | Medium | None | Add a MySQL standby (fault tolerance); dual-NIC connection (avoidance) |
| Login | Memcache crashes | All user logins >5s | Medium | MC crash | Medium | None | Scale MC from single machine to cluster |
## Compute high availability
- **Goal**: computing continues when some hardware is damaged; essence is redundancy, with complexity in **task management** (how failed tasks are reassigned). Two keys: (1) which servers may execute (all vs specific hosts, e.g., ZooKeeper writes only via the Leader); (2) how failed tasks re-execute.
- The "task assigner" is a **logical concept** and need not be a standalone module: Nginx, a ZooKeeper Follower (forwards writes to the Leader, serves reads itself), and standalone background task-assignment systems all qualify.
### Pattern comparison table
| Pattern | Mechanism | Pros | Cons/costs | Fit | Not fit |
|---|---|---|---|---|---|
| Primary-standby (cold/warm) | The primary runs all tasks; on failure a human promotes the standby and provisions a new standby. Cold = package ready but not started; warm = started but not serving (warm recommended) | Simplest: no interaction, no state-judgment failover, no data replication | Humans detect slowly (possibly an hour later), switch slowly (possibly half an hour), and err easily | Internal/back-office systems | Online business |
| Primary-secondary | The assigner classifies tasks: A to the primary, B to secondaries; no automatic reassignment on primary failure | Secondaries also execute tasks, using the hardware | Requires task classification; primary failure still needs human promotion + provisioning | Tasks can be classified and split | Tasks cannot be split |
| Symmetric cluster (load balancing) | Distribute by policy (random/round robin suffice); evict failures, readmit recovered; **heartbeats** carry status for detection | Automatic failover, no humans | Requires designing state-judgment conditions and tuning them per business | Online high-availability business (the vast majority) | Role/state-dependent systems |
| Asymmetric cluster | Different roles (Master/Slave) run different tasks; specified failure types trigger role reassignment (a Slave failure is only evicted) | Satisfies role semantics | Assignment strategy + role election are complex (ZAB/Raft) | Leader semantics (ZooKeeper: writes only via the Leader; on Leader failure, Followers pause reads/writes until election completes) | No role differences |

- **State-judgment condition examples** (customized per business): for online pages "80% of responses over 1s (including timeouts) within 1 minute = failure"; for background jobs "a single job running over 10 minutes without finishing = failure".
- **Selection**: low requirements/cost-sensitive → primary-standby; read-heavy → primary-secondary; automatic failover → symmetric cluster; leader/role semantics → asymmetric cluster. A compute cluster includes 2-machine setups (no two-machine distinction needed) — unlike storage high availability.
## Storage high availability (two-machine / cluster / partition)
- The essential difference: data replication must travel links (delay + interruption/congestion/packet loss), so at some moments data is necessarily inconsistent; "data + logic = business", so inconsistency is a business error (deposit 10000, query routes to an unsynced datacenter → user believes the money was stolen).
- Four questions for analyzing any scheme: (1) how does data replicate? (2) what is each node's role? (3) how is replication lag handled? (4) how is replication interruption handled?
### Two-machine architecture comparison
| Pattern | Mechanism | Pros | Cons/costs | Fit | Not fit |
|---|---|---|---|---|---|
| Primary-standby replication | The primary reads and writes; the standby only backs up and serves no business; failover is **manual**. Nearly everything supports it: MySQL/Redis/MongoDB etc. | Simplest: clients unaware, no state-judgment failover | The standby's cost is wasted; manual recovery is slow and error-prone (2–3 times a year, unpracticed, at night nobody knows) | Internal back-office systems (few changes, losses can be manually patched) | Online business |
| Primary-secondary replication | The primary reads and writes; secondaries read-only | Reads continue when the primary fails; secondary reads use the hardware | Clients must know primary vs secondary to route requests; large replication lag means stale reads; primary failure still needs humans | Read-heavy (forums/BBS/news — reads 10–100× writes or more) | Strongly consistent reads and writes |
| Primary-standby/primary-secondary failover | A replication scheme + automatic role decision and failover; replication is ≈1000 lines → failover ≈10000 lines, an order-of-magnitude complexity jump | Automatic failure recovery, no humans | No universal answer for state decision and conflict resolution — it varies by business | Highly available online core business | — |
| Primary-primary replication | Both machines are primary, replicating to each other; clients read/write either | No failover needed, no status passing or decision | Must support **bidirectional replication**: auto-increment IDs (both sides generate ID=100 — conflict), inventory (99 overwrites 98 → 97 in effect), balances — no | Loss- and overwrite-tolerant data: sessions, behavior logs, drafts | Unique-ID generation / deduction semantics |

Three design keys of two-machine failover (none has a universal answer):
1. **Primary-standby state judgment**: the passing channel (direct interconnect or third-party arbitration) + what is detected (power loss, process death, slow responses).
2. **Failover decision**: timing (power loss / process death / responses over 2s / 3 restarts within 3 minutes); policy (the recovered original primary takes back over, or is demoted to standby); automation level (fully automatic or semi-automatic with human confirmation).
3. **Data conflict resolution**: the old primary generated ID=100 during its failure window without replicating it; the new primary also generated ID=100 — how to resolve after recovery.
### State-decision implementations for primary-standby failover compared
| Implementation | Mechanism | Pros | Cons |
|---|---|---|---|
| Interconnected | Primary and standby build a direct state channel (network port/serial port; push or pull); clients use a **virtual IP** or record both addresses (the standby rejects requests directly) | Intuitive; rich status information (CPU/IO load, throughput, response time) | **Channel failure → the standby misjudges and promotes itself → dual primary**; multiple channels only lower the probability, and contradictory information has no solution |
| Mediator-based | Both connect to a third-party mediator to report/obtain status. Open source: **ZooKeeper, Keepalived**; MongoDB Replica Set (M/S hold data + A arbitrates, holds none) | Connection management and state decision are simpler; **ZooKeeper is the engineering recommendation** (a highly available cluster built in) | The mediator itself must be highly available, else dual standby and writes unavailable (the recursion trap); ZooKeeper has solved this |
| Simulated | Only the replication channel, no state channel; the standby simulates a client reading/writing the primary and judges by response (HTTP 404, timeout, response >3s) | Simplest: no state channel needed | Limited status information; decisions may err |

- **Mediator-based state-decision algorithm**: all start as standbys; losing connection to the mediator demotes one to standby (possibly dual standby).
- The primary losing its mediator connection → the mediator immediately tells the standby to promote.
- The primary cut off from the network demotes itself to standby and, after recovery, reports as the new standby.
- An old primary that lost power/restarted finds a primary already in place after recovery and stays standby.
- While both connections are healthy, failover follows actual state (e.g., the primary's responses over 3 seconds trigger failover).

Three state-decision styles (**no scheme is correct in every scenario — only per-business trade-offs**):
- **Dictatorial**: an independent decision maker collects all reporters' status; decision-maker failure means no decision; wrapping it in another decision maker recurses forever.
- **Negotiated**: the two exchange information and decide by rule; the connection-break dilemma — promote → dual primary, don't → true failure with no primary; extra connections only lower the probability.
- **Democratic**: many parties vote, majority wins (ZooKeeper's ZAB election); inherent defect **split brain** (a break splits into two subclusters each electing a primary), prevented by "**voting nodes must exceed half of all nodes**"; but when true failures exceed half, the remaining healthy nodes also cannot elect a primary — total outage.
### Storage clusters
- A single machine's storage capacity has limits (2013: Facebook held 250 billion photos, 250 PB, 350 million uploads/day); a cluster = at least 3 machines forming one system (primary-standby/primary-secondary are only 2).

| Type | Mechanism | Key complexities | Typical systems | Fit |
|---|---|---|---|---|
| Centralized-data cluster (1 primary + multiple standbies/secondaries) | Data is written only to the primary; reads are flexible (see primary-standby/primary-secondary) | (1) multiple replication channels → primary pressure and inter-standby inconsistency needing checks and fixes; (2) multiple standbies each judging the primary's state — resolving disagreement is complex; (3) electing among multiple standbies after primary failure is complex to coordinate | ZooKeeper (ZAB solves all three problems; high complexity) | Modest data, few machines: ≈5 machines, each able to carry the load |
| Distributed-data cluster | Each machine holds part and backs up part; each can read and write; some role must run the assignment algorithm (a dedicated server or an elected one) | The assignment algorithm's three points: balance, fault tolerance (reassign after failure), scalability (migrate and rebalance on expansion) | Hadoop HDFS (a dedicated **Namenode** manages block mapping); Elasticsearch (elects a **master node** to manage shard allocation) | Huge data, many machines: Hadoop/HBase at hundreds to thousands |

Core difference: in a centralized-data cluster clients write only to the primary; in a distributed-data cluster clients read/write any server — this decides their different application scenarios.
### Data partitioning
- Hardware-level high availability cannot handle geo-level disasters (an entire city/region's infrastructure paralyzed); partitioning = distributing data across geographic locations by rule, each partition holding part, so a disaster loses only part of the data and can recover quickly from other regions' backups. Three design points:
1. **Data volume** decides rule complexity: one MySQL machine holds 500 GB; 2 TB needs 4 machines managed in parallel; 200 TB needs 800 (with 1–2 machine failures per week, expansion config changes affecting everything, and a same-city concentration losing all in a disaster) — complexity changes qualitatively.
2. **Partitioning rules**: intercontinental (cross-continental latency unfit for online; not interconnected or backup only), country (language/law/business differ; usually backup only), city (low latency, similar business; can serve simultaneously = cross-region active-active).
3. **Replication rules** (against single-partition data corruption/loss):

| Replication rule | Mechanism | Pros | Cons |
|---|---|---|---|
| Centralized | A master backup center receives all partitions' backups | Simple design, no mutual interference; easy to extend | Higher cost (a dedicated backup center) |
| Mutual | Each partition backs up another (chained: Guangzhou → Beijing → Shanghai → Guangzhou) | Cheap (uses existing equipment) | Complex design, mutual interference; extension means rewriting the replication chain and hard history-data migration |
| Independent | Each partition has its own backup center, **backups must be in another city** (Beijing → Tianjin; a Chaoyang backup placed in Tongzhou means a city-wide blackout destroys both) | Simple design, no mutual interference; easy to extend | Most expensive (site costs dominate) |
## Cross-region active-active
### Purpose and criteria
- Purpose: business unaffected or recovered in minutes under geo-level disasters; cross-region cold standby recovery needs heavy human intervention and takes half an hour to 12 hours. Two criteria for "active-active": (1) normally, any site serves correctly; (2) when one site fails, the others serve normally. "Standby" = not serving normally; turning "standby" into "active" takes heavy human effort.
- High cost (a qualitative complexity jump + rising cost): bike-sharing/Didi/Alipay/WeChat must do it; news/internal IT/games/blogs can do cross-region backup only; with large business scale (including ad-derived revenue), do it as much as possible.
### Architecture pattern comparison
| Pattern | Mechanism | Pros | Cons/costs | Fit |
|---|---|---|---|---|
| Same-city cross-zone | Multiple datacenters in one city, different zones (Haidian + Tongzhou, tens of km) + dedicated high-speed network; treat as one datacenter in design | Near-same-datacenter speed; complexity and cost far lower; **the optimal architecture for datacenter-level failures** | Powerless against city-level disasters (probability: once in years to a decade+) | Datacenter-level disaster recovery; the compromise for finance-grade strong consistency |
| Cross-city | Datacenters in different cities and far apart (Beijing + Guangzhou, not Guangzhou + Shenzhen) | Truly survives city-level extreme disasters | Latency changes qualitatively: fiber ≈200,000 km/s; Guangzhou↔Beijing RTT ≈50ms, spikes 500ms–1s, packet loss adds seconds to tens of seconds; cross-city dedicated networks cost enormously | Core business geo-level disaster recovery |
| Cross-border | Datacenters in different countries, sync latency normally seconds | Serves users in different regions; read-only business active-active | Cannot satisfy "any site serves correctly" (a post made in Shanghai is invisible in New York for seconds) | Cross-border services (Amazon China/US are not interconnected); read-only (Google Search) |
| Cross-region cold standby | Off-site backup only, serves nothing | Cheap, low complexity | Failover needs heavy human effort, recovery half an hour to 12 hours; hidden problems stay hidden | Small disruption impact, cost-sensitive |

Strongly consistent data (bank deposits, Alipay balances) **cannot do cross-city active-active**: two datacenters each deducting means double spending (a 10000 balance transferring 10000 at both sites = 20000 out) — only same-city cross-zone works. Retrofit: async "transfer request" (submit → record → the background truly transfers → executes after Beijing recovers; fails if the balance is insufficient) — quasi-active-active at an experience cost (one operation becomes two).
### The 4 design techniques
1. **Guarantee cross-region active-active for core business only**: the misconception "all business active-active". A 10-million-DAU system: logins 10 million / registrations tens of thousands / profile edits <10000 → protect login (every center holds all account passwords). Registration active-active has no solution: if center A registers without syncing and goes down, B cannot tell phone-number duplicates; allowing registration means unsolvable conflicts when A recovers; changing to "one number, many accounts" = changing a core rule — the cure costs more than it's worth. Merging profile edits by timestamp has a trap: if centers' clocks differ by over 1 second, the earlier edit wins; globally unique incrementing IDs would themselves need active-active — extremely costly.
2. **Guarantee eventual consistency for core data only**: the misconception "all data syncs in real time" — physically impossible. Means: (1) shorten distance + high-speed network (cross-city cost only giants can carry); (2) sync only core data (token/session not synced; lost means re-login); (3) per BASE, guarantee eventual consistency: normally synced within 5 minutes; under failure 1 hour or even 1 day allowed, with B routing requests back to A.
3. **Sync data with multiple means**: the misconception "storage systems do the syncing". Counterexamples: MySQL 5.1 replication was single-threaded with lag of ten-plus seconds to ten-plus minutes; before Redis 2.8, a reconnect triggered **full** replication. Combination: message queues (account data is append-only), secondary reads (on local failure, read the peer per routing), storage-system sync (passwords change rarely), origin read-back (sessions read from the home center), regeneration (if origin read-back fails, re-login).
4. **Guarantee active-active only for the vast majority of users**: the misconception "100% available" — impossible (light speed, disks, extreme anomalies are uncontrollable); building a perfect scheme for 0.01% means failing the 99.99%. Strongly consistent business may affect up to 1/3 of users (transfers only at the account's home center). Mitigations: notices, after-the-fact compensation (vouchers/gift packs), experience supplements (SMS with the transfer result).

**Core idea: use multiple means to guarantee cross-region active-active of core business for the vast majority of users!**
### The 4-step design
| Step | Essentials |
|---|---|
| 1. Grade the business | Active-active for core business only. Criteria: (1) high traffic (login); (2) core business (QQ chat > Qzone); (3) generates major revenue (Qzone ads may overtake chat on revenue). When product says everything matters, adjudicate by these criteria |
| 2. Classify the data | Characteristics decide the scheme: **volume** (the larger, the likelier lag); **uniqueness** (must be unique → generated at one center, e.g., user ID); **real-time-ness** (the higher, the more complex); **loss tolerance** (session may be lost — re-login; lose user IDs and every association is gone); **recoverability** (posts can be reposted, passwords reset; accounts cannot be recovered) |
| 3. Sync the data | Choose per traits: **storage-system sync** (most common, simplest: MySQL primary-secondary/primary-primary; generic, no customization; severe lag at volume or high latency); **message queues** (Kafka etc.; fit for no-transaction, no-ordering data: accounts sync fine out of order, but a password changed m-then-n out of order is wrong → passwords cannot use MQ); **regeneration** (no sync; each datacenter generates its own: cookies, sessions, caches) |
| 4. Handle exceptions | Goals: (1) small data anomalies must not make the whole unavailable; (2) fix anomalous data after recovery; (3) placate and compensate users. Four measure classes in the table below |

Login business "data classification → sync scheme" example:

| Data | Key traits | Sync/handling scheme |
|---|---|---|
| User accounts | Append-only, unique, minute-level inconsistency tolerable | Message queue + MySQL replication dual channel; on remote read failure, secondary read per routing |
| User passwords | Rarely changed, **ordering-sensitive** | Storage-system sync (database replication); message queues unusable |
| User profiles | Rarely changed, conflicts mergeable | Storage-system sync; conflicts merged by agreed rule (last-modified-wins) |
| Login session/token | High volume, loss-tolerant, regenerable | Not synced; origin read-back; on failure re-login regenerates |

**The 4 exception-handling measure classes**:

| Measure | Mechanism | Key points |
|---|---|---|
| Multi-channel sync | Message queue + MySQL sync as mutually backing channels | Two channels generally suffice (more cost spikes); **they must run on different networks** (one public, one private); data must be overwrite-tolerant (new accounts yes, passwords no) |
| Sync + access combined | Datacenter B reads A's data via interfaces | The interface channel and the sync channel use different networks; data has routing rules making its home inferable; prefer local, fall back to the interface — for extremely real-time data |
| Log recording | Log before/after key operations; compare and repair after recovery | Storage location escalates with failure grade: on the server → a separate local system → logs off-site; choose by cost-benefit |
| User compensation | The system protects 99.99% of users; humans make up the 0.01% | Vouchers, gift packs, red packets; Blizzard's Hearthstone 2017 rollback compensated ≈¥200 per player (1000 gold + 15 card packs) and won goodwill |
## Interface-level failure response (degradation / circuit breaking / rate limiting / queuing)
- **Definition**: the system is not down, the network is not cut, but responses are slow with mass timeouts and errors; the root cause is excessive pressure/load. Internal causes: bug infinite loops, slow queries exhausting database resources, memory exhaustion; external: hacker attacks, promotion rushes, third-party mass/slow requests. The core idea matches cross-region active-active: **prioritize core business and the vast majority of users**.
- Distinguishing the means: degradation responds on **function priority** (loses functions); circuit breaking responds to **external dependency failures** (cuts the dependency); rate limiting responds on **access pressure** (drops requests); queuing is a rate-limiting variant (slows requests — waiting may feel no better than rejection).
### The four means compared
| Means | Problem solved | Core mechanism | Implementation essentials | Failures addressed |
|---|---|---|---|---|
| Degradation | Non-core functions dragging down the system | Sacrifice the periphery: lower or stop some functions (forum becomes read-only; stop the log-upload interface) | **System back doors**: reserved degradation URL + parameters + password — cheap but per-server and slow at scale; **a dedicated degradation system**: permission management + batch operations | Internal causes (slow queries, bugs) overloads |
| Circuit breaking | **External dependency** failures dragging you down (B slow → A's threads all stuck) | Stop calling the external interface per rules and return errors immediately; degradation targets yourself, circuit breaking targets external dependencies | (1) A **unified API invocation layer** centralizes sampling and statistics; (2) **threshold design** (30% of requests >1s within 1 minute → break): analyze and set → observe in production → tune | Third-party/downstream failures or slowness |
| Rate limiting | Access pressure beyond capacity | Admit only what capacity sustains, drop the excess; **request-based** (totals: 1M in a live room, 10k rush purchases; rates: 10k/minute, 100k/s peak); **resource-based** (connections, handles, threads, queues, CPU>80% reject) reflects real pressure better | Thresholds cannot be extrapolated linearly from hardware (64 cores ≠ 2× 32 cores — maybe 1.5× or even 1.1×); load test + observe in production and tune; request-based suits functionally simple systems (gateways, rush purchases) | External causes: attacks, rushes, promotions |
| Queuing | A rate-limiting variant: don't reject, let users wait (12306-style) | A standalone system caches requests FIFO and pulls them for processing at a rate matching capacity | Must be a standalone system (Kafka-like). Yihaodian's three modules: **queuing** (one FIFO queue per product) → **scheduling** (a mediator tunes the pull rate) → **service** (calls business + writes back results) | Instant high concurrency tolerating delayed results (flash sales) |
### Rate-limiting modes and algorithms compared
| Mode | Perspective | Common means | Difficulty |
|---|---|---|---|
| Request-based | Outside the system: limit requests | Cap totals (1M in a live room, 10k rush purchases); cap rates (10k/minute, 100k/s peak) | Thresholds hard to set; no linear extrapolation from core counts; suits functionally simple systems (load balancers, gateways, rush purchases) |
| Resource-based | Inside the system: limit key resources | Connections, file handles, threads, request queues (Netty caps 10000, rejects when full), CPU>80% reject | Identifying key resources + thresholds; reflects real pressure better; tune stepwise |

| Algorithm | Essence | Pros | Cons | Fit |
|---|---|---|---|---|
| Fixed window | Count requests in a fixed period; over the limit, rate-limit | Simple to implement | **The boundary problem**: two batches 10 seconds apart across adjacent windows total 200 requests (limit 100/minute) and both pass → crushed | Coarse-grained rate limiting |
| Sliding window | Two counting periods partially overlap | Eliminates the boundary problem | Slightly more complex | The default choice in general scenarios |
| Leaky bucket | **Total-rate control**: requests enter the bucket, draining at constant speed (= the limit speed; with no backlog outflow = inflow); when full, new requests drop | Discards little burst traffic | Bucket size is hard to adjust dynamically (BlockingQueue needs repeated tries); outflow speed cannot be precisely controlled | **Instant high concurrency**: midnight check-ins, on-the-hour flash sales |
| Token bucket | **Rate control**: tokens released at a rate; only token-holders are processed; the bucket accumulates tokens → bursts above the release rate can be processed | Rate dynamically adjustable, flexible | Bursts may still drop a lot; **bucket capacity estimated from processing capacity** (a 30-TPS bank interface: capacity ≈40 — at load test 40 it truly fell over) | Controlling third-party call rates (Alipay calling banks); capping your own max processing speed (load test 100 → limit 100) |

Leaky bucket vs token bucket: the leaky bucket handles **instant high concurrency** (buffer, then process at constant speed); the token bucket's "allowing bursts" only scales small (100 TPS bursting to 120 is fine; to 1000 likely crushes); **for second-scale massive concurrency, use the leaky bucket**.
### Failure type → response decision table
| Failure scenario | First choice | Auxiliary/notes |
|---|---|---|
| Own non-core functions dragging down core (slow queries, bugs, memory exhaustion) | Degradation | Keep reading posts (90% of traffic), stop log uploads |
| Dependent external systems slow/failing (deadlocking thread pools) | Circuit breaking | Unified API invocation layer + threshold tuning |
| Request volume beyond capacity (rushes, attacks, promotions) | Rate limiting | Request-based (gateways/rushes) or resource-based (reflects pressure better) |
| Instant high concurrency minimizing dropped requests (midnight check-ins, flash sales) | Leaky bucket/queuing | Queuing needs a standalone system; users wait for results |
| Need to control third-party calls or own processing rate | Token bucket | Bucket capacity estimated from processing capacity |
| Machine crashes/datacenter failure/city disasters (system-level failures) | The cross-region active-active system | Interface-level means do not apply to system-level failures |
## Selection decision table
| Condition/scenario | Recommended architecture combination |
|---|---|
| Internal back-office, few changes, manual patching acceptable, cost-sensitive | Compute: primary-standby (warm); storage: primary-standby replication |
| Read-heavy online business (reads 10–100× writes or more) | Compute: load-balanced cluster; storage: primary-secondary replication (clients route primary/secondary; watch replication lag) |
| Core online business at four nines, automatic failover required | Compute: load-balanced cluster; storage: primary-standby/primary-secondary failover (interconnected virtual IP, or ZooKeeper/Keepalived mediator, or MongoDB-Replica-Set-style arbitration) |
| Leader/role semantics (metadata, coordination, elections) | Compute: asymmetric cluster (ZAB/Raft election); storage: centralized-data cluster (ZooKeeper, ≈5 machines) |
| Huge data, many machines (hundreds to thousands) | Distributed-data cluster: Hadoop/HBase/Elasticsearch-style sharding + replication (dedicated Namenode or elected master) |
| Datacenter-level failures (fire/power loss/AC) must not interrupt | Same-city cross-zone dual datacenters + high-speed network (optimal for datacenter-level disaster recovery) |
| City-level disasters must be survivable / minute-level recovery required | Cross-city cross-region active-active: grade business → classify data → multi-means sync → the 4-step exception handling |
| Strongly consistent money data (balances/inventory) | No cross-city active-active: single-point writes + same-city cross-zone; or async "transfer request" + SMS result notice |
| Users in different countries / read-only business | Cross-border active-active (Amazon China/US model; Google Search model) |
| Small disruption impact; cannot bear the complexity and cost | Cross-region cold standby or partitioned backups; no cross-region active-active |
| Geo-level disasters must not lose all data; fast recovery required | Data partitioning (city partitions doubling as active-active; replication rule by cost; backup centers must be off-site) |
| Interface-level failures (overload, dependency failures) | Degradation + circuit breaking + rate limiting + queuing combined; run FMEA screening fully before launch |

- Data partitioning vs cross-region active-active: partitioning stresses that a disaster loses only part of the data, recoverable (simultaneous serving not required); active-active stresses multiple sites serving correctly simultaneously. City partitions serving simultaneously = cross-region active-active; partitions not serving externally are disaster-recovery backup only.
- FMEA essentials: granularity is "assume a component fails"; probability high/medium/low suffices; disputes settled by discussion, unresolved — the architect decides; the output is a "follow-up planning" list with resources ranked by risk.
## Common pitfalls
- One CP/AP choice for the whole system: CAP's granularity is **data** — decide per class or you rob Peter to pay Paul; connectionless non-sharing clusters like Memcache and ZooKeeper elections are outside CAP's scope.
- Pursuing perfect CP / cross-city strongly consistent active-active: CAP ignores latency and perfect CP does not exist; balances/inventory can only be single-point writes, and cross-city dual datacenters each deducting = double spending (10000 balance transferring 20000).
- Interconnected state-channel failure → dual primary; extra channels only lower the probability and contradictory information has no solution — cure it with the mediator style (ZooKeeper); the dictatorial style has no answer to decision-maker failure and the negotiated style has the connection-break dilemma (misjudgment → dual primary, missed judgment → no primary) with no perfect answer; the mediator must be highly available — reuse proven highly available components.
- Ignoring split brain and quorum costs: the democratic style needs majority votes to prevent split brain, but when true failures exceed half, remaining healthy nodes cannot elect a primary — total outage.
- Ignoring replication lag's failure amplification: MySQL 5.1 single-threaded replication lagged ten-plus seconds to ten-plus minutes; before Redis 2.8 a reconnect triggered full replication, crushing the read service.
- Primary-primary replication for data that cannot replicate bidirectionally: auto-increment ID conflicts, inventory/balance overwrite losses (99 overwriting 98 = 97).
- Two-machine setups doing replication but not the three failover elements (state judgment / failover decision / conflict resolution): failover complexity is an order of magnitude above replication (1000 lines vs 10000).
- Changing core business rules for the architecture's sake ("one number, many accounts" to make registration active-active): the cure costs more than it's worth.
- Expecting cross-region active-active to be 100% available and all data real-time synced: physically impossible; protect core business + the vast majority of users, "endure" + compensate.
- Merging profile conflicts by multi-center timestamps: with clock error over 1 second the earlier edit wins; globally unique incrementing IDs would need active-active themselves — extremely costly.
- Independent backup centers in the same city as the primary partition: a Chaoyang backup placed in Tongzhou means a city-wide blackout destroys both — must be off-site (Beijing → Tianjin); multi-channel sync/interface access sharing one network with database replication means one break breaks all — use one public and one private network.
- Token-bucket capacity copied from leaky-bucket thinking and set huge (30 TPS set to 1 million — bursts crush yourself or downstream; the bank interface fell over at load test 40); rate-limit thresholds by gut feel or linear hardware extrapolation (64 cores ≠ 2× 32 cores, maybe 1.5× or even 1.1×) — load test + production observation, tuning stepwise.
- Over-design: multi-datacenter/cross-region active-active for an internal admin system; a news site blindly going cross-city active-active — judge by failure cost and business scale first.
- Confusing the two kinds of "primary": a centralized cluster's primary takes **writes**; a distributed cluster's elected "primary" (ES master, HDFS Namenode) only manages **partition assignment**, not writes.
- Ignoring the mediator's dual-standby window: while the mediator is down, dual standby and writes unavailable; depending on MongoDB arbitration/ZooKeeper requires monitoring the mediator cluster's health.
- Cold standby to save power, not time: cold standby needs manual startup of the business system; warm standby only switches traffic — prefer warm standby in general scenarios.
### Key numbers quick reference
| Number | Meaning | Design implication |
|---|---|---|
| 99.99% ≈ 50 minutes/year; 99.999% ≈ 5 minutes/year | Availability metrics | After sacrificing C or A during partition, recover to CA via logs |
| Guangzhou↔Beijing RTT ≈50ms, spikes 500ms–1s | Cross-city transmission delay | Cross-city sync delay is unavoidable; design sync per data traits |
| Replication scheme 1000 lines vs failover scheme 10000 lines | Two-machine failover complexity | Estimate failover effort at 10× replication |
| ZooKeeper ≈5 machines; Hadoop/HBase hundreds to thousands | Cluster-scale boundary | The selection boundary of centralized-data vs distributed-data clusters |
| MySQL single machine 500 GB: 2 TB = 4 machines, 200 TB = 800 | Data volume vs partition complexity | Beyond huge scale, partitioning and geo-disaster-recovery design are required |
| MySQL 5.1 lag ten-plus seconds to ten-plus minutes; Redis 2.8 reconnect = full replication | Storage sync shortcomings | Cross-region active-active cannot rely on storage-system sync alone |
