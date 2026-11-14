# Sources of Complexity and Architecture Design Principles

> When to read: Phase 1 of architecture design ("identify complexity"), when judging a system's primary technical contradictions, or when checking for over-design

## Architecture and complexity basics

### The 4R architecture definition

Software architecture = the **top-level (Rank) structure** of a software system, defining which **roles (Role)** compose the system, the **relations (Relation)** between roles, and the **rules (Rule)** governing their operation.

| R | Meaning | Essentials and examples |
|---|---|---|
| Rank — top-level structure | Architecture is layered | L0 decomposes into multiple L1s, L1 into L2…; no more than 5 layers recommended (L0–L4); design and diagram "top-down, refining step by step", showing one layer or two adjacent layers at a time |
| Role | Which parts compose the system | Splitting by business logic yields "modules" (separation of responsibilities); splitting by physical deployment yields "components" (unit reuse). Example: after microservice splitting, each microservice is a role |
| Relation | How roles connect | Every relation is implemented in code and cannot be wired arbitrarily — you must specify: connection mode (HTTP/TCP/UDP/serial), data protocol (JSON/XML/binary), concrete interfaces |
| Rule | How roles collaborate to complete tasks | Core business scenarios must have rules designed, expressed with a system sequence diagram. Example: the interaction sequence of roles in a QR-code payment |

- Frameworks concern "conventions" (Spring MVC is an implementation of the MVC convention); architecture concerns "structure".
- Architectures decomposed from different angles — business logic, physical deployment, development conventions — are all correct (the reason for the "4+1 views"); system capability is not the sum of individuals, combinations create new capabilities, and architecture must be evaluated as a whole.

### Composition of complexity: structural complexity + logical complexity

**Structural complexity** (component count × component relations): (1) more components fail more easily — with a 10% failure rate per component, 3 components give availability of 0.9³ = 72.9%, 5 components 0.9⁵ = 59.0% (a 13-point gap); (2) changing one component recursively affects related ones → development efficiency drops; (3) many components with complex relations → fault localization is hard (the component showing the failure is not necessarily the root cause).

**Logical complexity**: a single component carrying too many functions (typical: e-commerce stuffing products/search/orders/payment/users into one component), overly complex algorithms (hard to understand → hard to implement/modify/debug); this in turn drags down every engineering link — cloning a million-line codebase takes 30 minutes, merging dozens of branches overwrites easily, dozens of releases per day, failures take dozens of people to localize.

**Function count**: when functions are pairwise related, complexity = function count + connection count — 3 functions = 6, 8 functions = 8 + 28 = 36 (connections make complexity rise explosively).

### The purpose of architecture design = resolving software complexity

Common misconceptions (exclude them when judging):

| Misconception | Rebuttal |
|---|---|
| Architecture is important so we must do it; without it the system won't run | Startups skip formal architecture design, launch quickly, and run fine |
| Architecture design improves development efficiency | Sometimes the simplest design has the highest development efficiency, and architecture design itself costs time and people |
| A good architecture promotes business growth | Copy WeChat's architecture and your business still won't reach 1/10 of WeChat's scale |
| Process requirement / the architect must do something | Forcing architecture design onto requirements that don't need it wastes effort and slows progress |
| We need high performance, high availability, extensibility | Ignoring the system and business and jumping straight to the "three highs" → infinitely complex architecture, indefinitely delayed delivery, one month to add a feature |

The correct answer and its guiding role:

1. Where to start with requirements, and time only allows designing in one direction? — **Identify the complexity points** and design only against the primary ones; do not try to cover everything.
2. Company A uses solution X, company B uses solution Y — which to reference? — Understand which complexity point each solution addresses and **compare against your own complexity points** before picking a similar reference.
3. Is "TPS must reach 100k", "adopt Docker", "copy Taobao" reasonable? — Meaningless if the complexity does not lie in the corresponding direction.
4. Mechanically copying another company's architecture (dressed up as "reference" or "minor adaptation") → poor fit, constant refactoring, even starting over.

Benchmark case (student management system, demonstrating "what counts as complexity"):

- Performance: 10–20k students, <1 visit per person per day → not complex (MySQL suffices, cache optional); extensibility: stable features → not complex; cost: a few servers → negligible.
- High availability: 2 hours of downtime is tolerable → compute high availability can be skipped; but **losing all data is unacceptable** → the primary complexity is storage reliability: same-datacenter primary-standby MySQL (against machine failure) + cross-datacenter sync (against datacenter failure).
- Security: not finance/strong-privacy → Nginx ACL + account-password management + DB access permissions suffice.

## The six sources of complexity

> Screening order: high performance → high availability → extensibility → low cost → security → scale. Most systems have only 1–2 primary complexities (see "identification method").

### 1. High performance

**Definition**: pursuing performance comes with rising complexity. New technology that replaces old (disk → SSD) adds no complexity; only technology that coexists with the old and forces choices and combinations adds complexity.

**Single-machine complexity**: evolved through batch processing → multi-process (time sharing + inter-process communication) → multi-threading (thread = smallest scheduling unit, process = smallest resource-allocation unit, mutex locks ensure data correctness) → multi-CPU parallelism (SMP/NUMA/MPP; SMP most common). Principle: **the newest is not automatically the best, and it is not either-or** — Nginx multi-process, Redis single-process, Memcached multi-threaded all achieve high performance; choose and combine per the business.

**Cluster complexity** (business volume far exceeding one machine: Alipay peaked at 120k payments/second, WeChat red packets at 760k/second — such systems run tens of thousands of backend machines), two paths:

| Path | Approach | How complexity shows up |
|---|---|---|
| Task assignment | Every machine handles complete business; add a task assigner | (1) Assigner selection: hardware (F5), software (LVS), load-balancing software (Nginx/HAProxy), in-house; (2) connection management: establish/detect/interrupt handling; (3) assignment algorithm: round robin/weighted/load-based (load-based needs servers to report status) |
| Task decomposition | Split the monolithic system into small, simple subsystems | Simple systems achieve high performance more easily (few performance points, easy to localize and optimize); you can independently optimize/scale a single bottleneck subsystem without touching the others |

Task-assignment scale reasoning: one machine at 5000/s scales to 2 machines; the theoretical 10000/s works out to ≈8000/s at 80%; at a 100k/s requirement even a single assigner becomes the bottleneck, so multiple assigners are needed (DNS round robin/smart DNS/CDN/GSLB), and the architecture changes from "1-to-many" to a "many-to-many" mesh (e.g., 25 business + 5 assigners) with sharply higher state-management complexity; the assigner can also be an algorithm embedded in another program (e.g., a Memcached cluster).

The "degree" of task decomposition (cross-system call cost model): each cross-system call ≈1ms while the business processing itself is 50ms — split into 2 subsystems: 51ms; 4: 53ms; 100: 149ms; performance has a theoretical ceiling that splitting can only approach, never break — **finer is not better**.

### 2. High availability

**Definition**: the system's ability to perform its functions without interruption. Hardware aging and failure, software bugs, and external disasters are all uncontrollable → the only essential means is **redundancy** (a spare set of machines, datacenters, channels); the essential difference from high performance: **in high performance, added machines "extend" processing capacity; in high availability, added machines "redunduplicate" processing units**.

Three scenario types:

| Scenario | Characteristics | Complexity |
|---|---|---|
| Compute (stateless) high availability | Same algorithm + input gives the same result on any machine; freely migratable | Complexity sits entirely in the task assigner and assignment algorithm |
| Storage (stateful) high availability | Data must travel over links to cross machines | The difficulty is data inconsistency's impact on the business |
| State decision | The common foundation of both: judge normal/abnormal and act | Essential contradiction: **a redundant system's state decision can never be fully correct** |

Compute high-availability assignment algorithms (no absolute best; choose per business): two-machine primary-standby (cold/warm/hot standby), primary-primary; clusters of 1 primary + 3 standbys / 2 + 2 / 3 + 1 / 4 + 0 (ZooKeeper = 1 primary multiple standbys; Memcached = all primary, 0 standbys).

Storage high-availability quantitative background: same-datacenter transfer takes a few ms; cross-region datacenters are stable at ≈50ms, unstable at 1s+; line failures last from ten-odd minutes to hours (a dug-up fiber cable measurably disrupted service for over 4 hours) → at some point data is necessarily inconsistent, and inconsistency is a business error (e.g., after a deposit the query routes to a not-yet-synced datacenter and the balance looks unchanged). **The difficulty is not how to back up data but reducing/circumventing data inconsistency's impact on the business**; the CAP theorem (consistency/availability/partition tolerance — at most two of three) is a theoretical constraint to be traded off per business.

Three state-decision approaches:

| Approach | Mechanism | Defects |
|---|---|---|
| Dictatorial | An independent decision maker collects all reporters' status and decides | If the decision maker itself fails, no decision is possible; wrapping the decision maker in another decision maker recurses forever |
| Negotiated | Two independent parties exchange information and decide by rule (most common = the primary-standby decision: both start as standby → establish connection → exchange status → one decides to be primary) | The connection-break dilemma: the standby thinks the primary failed → dual primary; the standby thinks the primary fine but it truly failed → no primary; multiple connections only lower the probability, never eliminate it |
| Democratic | Multiple parties vote, majority wins (e.g., ZAB leader election) | Algorithms are complex to implement; inherent defect **split brain** (connection break → two subclusters each elect a primary); "votes must exceed half" prevents split brain at the cost that normal nodes also fail to elect a primary when votes fall short |

Availability-grade quantitative table (choose the target by business tolerance):

| Availability | Colloquial | Annual downtime | Daily average | Typical business fit |
|---|---|---|---|---|
| 99% | two nines | 87.6 hours | 14.4 minutes | internal tools, back-office systems |
| 99.9% | three nines | 8.76 hours | 86 seconds | ordinary internet content businesses tolerating minute-level jitter |
| 99.99% | four nines | 52.6 minutes (≈50 minutes) | 8.6 seconds | core transactions, social main paths |
| 99.999% | five nines | 5.26 minutes (≈5 minutes) | 0.86 seconds | payment, wallets, and other money-grade businesses (needs cross-region active-active) |

Judgment essentials: hours of downtime tolerable → no need for load balancing/cross-region active-active; **data loss unacceptable** → compute high availability may be downgraded, but storage reliability is mandatory (same-datacenter primary-standby + cross-datacenter sync is the starting point). Every scheme leaves some users affected → the architecture guarantees 99.99% of users unaffected + manual compensation (vouchers/gift packs) as the fallback.

### 3. Extensibility

**Definition**: when new requirements appear, none or only minor modification is needed; no system rework or rebuild.

**Condition 1: predict change correctly** — you cannot consider extensibility at every design point (the architecture balloons and cannot be implemented), nor ignore it entirely (every new requirement forces a refactor), and all predictions can be wrong. → **The 2-year rule: predict only possible changes within 2 years, not 5 or even 10** (in fast-moving industries 2 years already suffices; in slow ones, predicting 5 years lands about the same).

**Condition 2: handle change perfectly**

| Approach | Core idea | Complexities |
|---|---|---|
| Changing layer + stable layer (the changing layer is also called the adaptation layer) | **Isolate** change behind the changing layer. Example: support XML/JSON/ProtocolBuffer access; support MySQL/Oracle storage | (1) How to split the changing layer from the stable layer (no clear standard); (2) whose convention the inter-layer interface follows (e.g., MySQL's REPLACE INTO vs Oracle's MERGE INTO) |
| Abstraction layer + implementation layer (typical practices: design patterns, rule engines) | **Encapsulate** lower-layer differences behind the implementation layer. The abstraction layer's interfaces stay stable → unified processing rules; new features only add implementation-layer details without touching the abstraction layer | Once fixed, the abstracted rules cannot change easily; what one class could do becomes several classes, and complexity rises visibly |

Practice principle: **write the 1st time, copy the 2nd, refactor the 3rd** (from Martin Fowler's "Rule of three": never on the second occurrence, refactor on the third) — the 1st time implement quickly and directly without extensibility (whether the business takes off is unknown); the 2nd time copy-modify and launch quickly; only the 3rd time refactor, using isolation/encapsulation (template method, strategy pattern).

**Over-design cautionary tale** (the "100-million-user platform"): a team of a few people benchmarked against 100 million users, designing 40+ subsystems up front and taking nearly a year to launch; claimed 50000 TPS, delivered under 500 — every upgrade required a dozen-plus subsystems in sync, localization was hard, and the inheriting team spent 2 years refactoring to under 20 subsystems before it stabilized. Lesson: design for requirements that never arrive is wasted; over-design's demolition cost far exceeds the refactoring cost it "saved".

### 4. Low cost

**Identification signals**: the solution involves hundreds, thousands, even tens of thousands of servers (at a few or a dozen machines, cost is not the focus); startups and businesses burning money for growth. **Quantitative feel**: option A at 10000 machines vs option B at 8000 saves only 20%, but at ≈¥20k per machine-year that is ¥40 million a year.

**Essential conflict**: the generic means of high performance/high availability is adding machines; low cost demands fewer machines. Low cost is usually not the primary goal but an **additional constraint**: set the cost target first → design for high performance/high availability → evaluate against the target → redesign if it misses → only if that fails, adjust the cost target.

**The only means is "innovation"**, in two tiers:

| Means | Primary complexity | Fit | Cases |
|---|---|---|---|
| Introducing new technology | Learning the new technology and integrating it with existing technology | Mostly small and mid-size companies | NoSQL for high-concurrency pressure; full-text search engines for inefficient LIKE; Hadoop for massive data; three-tier caching (SSD as L2) cuts cost and resists penetration |
| Creating new technology | Requires a qualitative leap with an essential improvement over the old | Only large companies have the resources and time | HHVM sped up PHP; Kafka sustains 500 billion events/day; Ruby on Rails/Lua rewritten in Go |

### 5. Security

Security incidents are enormously damaging (data leaks, business-paralysis grade) and split into two classes; architecture design focuses on the second:

| Class | Characteristics | Typical problems | Relation to architecture |
|---|---|---|---|
| Functional security | Exploiting system vulnerabilities to break in | XSS, CSRF, SQL injection, framework vulnerabilities (e.g., Struts2 remote code execution) | Tied to concrete coding, **little to do with architecture**; frameworks' built-in security features prevent common vulnerabilities but not new ones; attack and defense improve step by step — no one-shot fix |
| Architectural security | Direct destruction, far greater impact | DDoS (largest known attack: 665 Gbps) | The traditional answer = firewalls partitioning zones + access control; unsuitable for the internet domain, see below |

Why the internet domain does not rely on firewalls: firewall performance is mediocre (mid-to-high-end ≈¥100k per unit, sustaining ≈25 GB/s; defending 665 Gbps would need nearly 30 units ≈¥3 million, idle when no attack), and DDoS's greatest impact is **exhausting the datacenter's total egress bandwidth** — however strong the internal network, normal user requests cannot get in. Conclusion: **internet systems have no good self-built architectural-security means; they rely mainly on carriers'/cloud providers' strong bandwidth and traffic-scrubbing capacity**.

### 6. Scale

**Quantitative change triggering qualitative change**: many systems have no high-performance/high-availability/extensibility requirements, yet with extremely many functions and logic branches still become "so complex" — long-evolving systems accrete features into black boxes: unreadable, unmodifiable, untouchable, unfixable.

1. **More and more functions**: complexity = function count + connection count, rising explosively (connections are the main driver).
2. **More and more data**: at big-data scale traditional means fail and a new technology system is needed (GFS = file storage, Bigtable = column-oriented storage, MapReduce = computation); even below big-data scale qualitative change arrives — **a MySQL single table is generally recommended at ≈50 million rows**; at 1 billion rows, adding an index can take hours (no inserts during that time — effectively a business outage), schema changes take very long, backups take very long; forced sharding then introduces new complexity: how to define the split rules, how to keep cross-table queries fast.
3. **Long history of accretion**: features piled into a black box, later maintainers unfamiliar with the history — same manifestation as item 1.

## Identification method: the screening method

The original requirements never say where the complexity lies. When experience is thin, use the **screening method** — analyze dimension by dimension, and **quantify every dimension** (gut-feel judgment is the most error-prone).

1. **Screen dimension by dimension**: high performance → high availability → extensibility → low cost → security → scale; estimate and judge "is/is not" a complexity per item against the business context.
2. **Converge conclusions**: real complexity is generally 1, occasionally 2, at most 2–3. If 3+ must all be satisfied simultaneously — either the system was originally designed wrong or this is a misjudgment — you must re-rank by priority.
3. **Rank and trade off**: with multiple complexities present, **rank them by business, technology, and team conditions combined and solve the currently most significant first**; do not fantasize about solving everything at once. The trap of solving all at once: nowhere to start; solutions too complex, delivery indefinitely delayed; design points contradicting each other (availability wants data flushed promptly, flushing in turn drags performance).
4. **Don't fear tearing down and rebuilding**: one complexity has many candidate solutions and you can always pick the best overall value; even a rebuild usually preserves the solved complexities (e.g., Hadoop solves big data's availability/performance/capacity simultaneously).
5. **Cautionary warning**: the complexity lay in heavy business-logic coupling, yet a 50000-TPS high-performance architecture was designed — however perfect and advanced the solution, it is meaningless; the better it is executed, the further off the mark.

**Screening example: the message queue of "Front Wave Weibo" (fictional)**

Background: business subsystems were strongly coupled — posting one Weibo notified a dozen-plus subsystems (review, analytics, ads, …) via interface calls: inefficient and hard to localize. A message queue was chartered to decouple. Constraints: middleware team of ~6, familiar with Java, Linux + MySQL, single-datacenter deployment.

| Screening item | Quantified estimation | Conclusion |
|---|---|---|
| High performance? | 10 million posts/day; each post read by 10 subsystems on average = 100 million reads/day. Converted per second (÷86400): writes 115/s, reads 1150/s. Peak = average × 3 → TPS 345, QPS 3450 → write performance not complex; the business will grow, so reserve headroom (low base takes peak × 4) → design target TPS 1380, QPS 13800 → QPS already substantial | **High-performance reads are the complexity** |
| High availability? | Consequences of lost messages, tiered: the review subsystem losing messages → regulatory violation (very severe); the analytics subsystem losing messages → user churn and lost revenue (severe but second) | **High availability is the complexity** (message writing/storage/reading all need guarantees) |
| Extensibility? | The message queue's function set is clear; little need to extend | Not a complexity |

Final complexity list: high-performance message reads + highly available message writing/storage/reading (design the alternatives from this next).

Capacity-estimation mnemonics (the generic method for judging high-performance complexity): peak = average × 3 (reads/writes are never perfectly even; design to peak); design headroom = peak × 2–4 (4 when the base is low / growth fast, 2 when high), **generally no more than 10×, and estimating 100× up front is strictly forbidden**; day → second conversion = total ÷ 86400 (e.g., 100 million/day ≈ 1150/s); cluster actual capacity estimated at 80% of theory (one machine 5000/s × 2 ≈ 8000/s).

## The three principles: fitness / simplicity / evolution

Programming demands determinism; architecture design is essentially choosing under **uncertainty** — different companies build very different architectures for the same system and both run; there is no syntax to constrain you, so principles must guide choice.

| Principle | Motto | Core argument | Application essentials |
|---|---|---|---|
| Fitness principle | **Fit beats industry-leading** | Three causes of failure: (1) not that many people wanting to do that much work (big-company core teams run dozens to a hundred-plus; most companies' entire R&D is barely 100+); (2) not that much accumulation while wanting to leap to the top in one bound (industry-leading solutions took years of step-by-step refinement); (3) no such outstanding business scenario while fantasizing a flash of inspiration (leading solutions were "forced out" by business volume reaching critical mass, like GFS born of Google's massive data) | An excellent architecture = integrating resources for maximum effect under current staffing, conditions, and business constraints, and landing quickly. A big-company architect moving to a small company and copying big-company practice achieves nothing — the platform, resources, and accumulation are gone; the "100-million-user platform" hit all three failure causes — the failure was not inability to build, but never reaching the original goal |
| Simplicity principle | **Simple beats complex** | In manufacturing/construction, complexity signals sophistication; in software it signals exactly "problems" — software keeps changing after release, and complexity taxes the whole life cycle. Structural complexity's three problems: more components fail more easily (0.9³ = 72.9% vs 0.9⁵ = 59%), changes propagate recursively, localization gets harder; logical complexity: one component carrying too many functions, overly complex algorithms | When both options meet the need, **pick the simple one** (KISS, Keep It Simple, Stupid!). Primary-standby state decision works with a heartbeat — no need to bring in ZooKeeper/ZAB to look impressive; when the function set is relatively simple, etcd's Raft is easier to understand and implement than ZAB and is the better choice |
| Evolution principle | **Evolution beats one-step completion** | A building, once finished, cannot change; in software, change is the theme — no one can predict years ahead at the start. Both one-step approaches (copying big companies' published designs / spending vast resources on predictive analysis and design) end with huge investment and indefinitely delayed delivery, and the predictions are unreliable | Three-step evolution: (1) the architecture first meets the business needs of the moment; (2) iterate and refine in real use (keep the good designs, fix defects, drop the useless); (3) when the business changes, extend, refactor, even rewrite, with valuable experience/designs carried into the new architecture. Even the biggest team must follow this — no one can perfectly predict the business's path |

**Case lessons** (reusable conclusions from Taobao's and Mobile QQ's evolution):

1. Switch means with business stage: buy a solution → buy performance → refactor → build in-house.
2. When the business demands "fast", technology picks "fast": when launch is urgent, buy the lightweight option; don't rush to refactor.
3. When technology starts hurting the business (e.g., middleware deadlock-restarts), switch from "buy" to "refactor".
4. Cost moving from secondary to primary contradiction is an architecture-upgrade signal.
5. Each order-of-magnitude scale jump relocates the bottleneck; patches have limits — a rewrite is also evolution.
6. Capacity bottlenecks can be estimated: per-user memory × user count (2KB/user → 2GB ≈ 1 million online); at the ceiling, upgrade.

## Complexity-judgment quick-reference table

| Condition/scenario signal | Primary-complexity conclusion | Read next |
|---|---|---|
| Ten-thousand-scale users, low daily visits (admin/back-office systems), peak TPS below thousands | Neither performance nor availability is a complexity; the only possibility is unacceptable data loss → storage reliability | internet-template.md (the universal foundation) |
| Peak QPS in the tens-of-thousands to hundred-thousand range after conversion, or one machine + one assigner insufficient | High performance (task assignment → task decomposition; compute actual capacity at 80% first) | high-performance.md |
| Data must not be lost; message loss = money/regulatory-grade consequences | High availability, focused on storage reliability and state decisions | high-availability.md |
| Hours of downtime tolerable but data must survive | Compute high availability may be downgraded; do storage redundancy only (start with same-datacenter primary-standby) | high-availability.md |
| Requirements changing frequently / integrating many third parties / multiple channels and protocols | Extensibility (changing layer/abstraction layer + write-copy-refactor-at-three + the 2-year rule) | extensibility.md |
| Hundreds to thousands of servers, cost share high; startups burning cash | Low cost (as an additional constraint: set the cost target before designing) | security-cost-scale.md |
| Finance/payment/strong-privacy data | Security (functional security goes to frameworks and standards; architectural security relies on cloud traffic scrubbing) | security-cost-scale.md |
| Dozens of features, years of accretion, unreadable and unmodifiable | Scale complexity (prioritize refactoring to cut subsystem count before anything else) | security-cost-scale.md + internet-template.md |
| A small team wanting a big-company-grade all-in-one platform | Violates the three principles; cut scope first, evolve in phases | this file's "three principles" section |
| Screening finds 3+ "all mandatory" complexities | First suspect a misjudgment or a flawed original design; rank by business/technology/team and solve only the top 1–2 | this file's "identification method" section |

## Common pitfalls

1. **Treating complexity as a static checklist**: ignoring the business and applying the "high performance + high availability + extensibility" trio to every system → infinitely complex architecture, indefinitely delayed delivery. Re-identify from requirements every time; there are usually only 1–2 primary complexities.
2. **Missing quantified estimation**: adding caches/clusters/cross-region active-active on gut feeling without numbers. First compute the true scale with peak = average × 3, headroom = peak × 2–4 (<10×), then choose solutions.
3. **Mistaking copying for referencing**: directly duplicating Taobao/WeChat architecture without understanding whether the complexity they solved matches yours → poor fit. Even big-company architects achieve nothing at small companies by copying.
4. **Chasing industry-leading/impressiveness**: bringing ZooKeeper into a heartbeat-solvable primary-standby decision; designing a few-hundred-QPS system for a million QPS. Fit beats industry-leading; simple beats complex.
5. **Over-designing extensibility**: leaving extension points everywhere for imagined requirements → architecture balloons and cannot be implemented. Follow the 2-year rule and write-copy-refactor-at-three; the "100-million-user platform" is the lesson (40+ subsystems, designed TPS 50000 delivered <500, 2 years to dismantle).
6. **Fantasizing about solving all complexity at once**: with multiple complexities present, no ranking, one solution to fix everything → nowhere to start, design points contradicting each other (availability wants flushing, performance fears it). List, rank by business/technology/team, solve the top 1–2 first.
7. **Underestimating structural complexity**: blindly splitting microservices/adding components, ignoring that more components fail more easily (the 0.9ⁿ effect), changes propagate recursively, localization gets harder; task decomposition past the "degree" (cross-system calls grow with split count) and performance drops instead.
8. **Confusing the two security classes**: piling firewalls to fight DDoS at the architecture level — expensive (665 Gbps needs ≈¥3 million of gear) and useless (egress bandwidth exhausted, business still down). Functional security relies on frameworks + standards improving step by step; architectural security relies on carrier/cloud bandwidth and traffic scrubbing.
9. **Ignoring storage high availability's real difficulty**: assuming "backed up = highly available", ignoring cross-datacenter transmission delay/interruption causing data inconsistency (cross-region ≈50ms, a dug-up cable can sever for hours; CAP constraints), without designing inconsistency avoidance and compensation for the business.
10. **Wanting state decisions to be perfect**: the dictatorial style fears decision-maker failure, the negotiated style fears connection breaks (the dual-primary/no-primary dilemma), the democratic style fears split brain — a redundant system's state decision is essentially never fully correct; you can only choose the trade-off point per business scenario.
11. **One-step-completion thinking**: a new system launching with 40+ subsystems / cross-region active-active / full-chain high availability → indefinitely delayed delivery and failed predictions. Evolution beats one-step completion: meet the current business first → iterate and refine → refactor as scale grows (Taobao and Mobile QQ both did).
