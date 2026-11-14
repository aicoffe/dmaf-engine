# The Internet Architecture Template and Evolution Roadmap
> When to read: designing an internet business system's overall architecture from zero, planning architecture evolution by business stage, or making open source vs in-house selections

## Technology evolution patterns

### The core judgment method

**Business decides technology; technology evolution can only be judged from the business stage** — do not view technology from technology's own angle; technology is only the means supporting business development.

| Business type | Traits | What users choose by | Technology ↔ business relation |
|---|---|---|---|
| Product class | Users take exclusive possession (browsers, antivirus, phones) | "Features" (strong functions, good performance, smooth experience, good looks) | **Technology innovation drives business development** |
| Service class | Use is non-exclusive; more users make it more valuable (search, e-commerce, IM) | "Scale" (you use it because your friends do) | **Business development drives technology development** (a lookalike product takes half a year to build, but without equivalent service scale it is meaningless) |

Judgment steps: identify the business's primary complexity now and for the near term (feature complexity or scale complexity: performance/availability) → invest only against that complexity; skip the rest. The sole exception: when a new technology can drive or create new business (e.g., multi-touch opened the smartphone era) — and even then it reverts to business-driven afterwards. Evolution case conclusions (business stage → architecture traits; trajectories differ by industry): Taobao — rapid development, buying off-the-shelf systems to adapt → user requests surged, switched to Oracle/Java → sharding, read/write splitting, caching → at 100 million+ products / 250 million+ PV, decomposition (trading center, category management, user center split out); banks — feature complexity (ever more systems) → online banking stability/security/usability → mobile payment (Double 11 floods) performance/consistency.

### The three wrong schools

The three wrong schools: the trend-chasers ("new technology always pays" → guinea-pigging immature technology, high learning cost, discovering inapplicability after mastery = wasted human effort); the conservatives ("stability above all" → missing step-change gains; technology substitution is usually explosive, not linear); the followers ("the competitor uses it, so do I" → when you lead there is nothing to follow; incomplete competitor information invites blind imitation; what fits them may not fit you).

### The evolution timeline

Internet business is "scale decides everything"; development has four stages: **startup, growth, competition, maturity**. Two driving factors: business complexity and user scale — both essentially **quantitative change triggering qualitative change**.

| Business stage | Business traits | Primary complexity | Architecture actions |
|---|---|---|---|
| Startup | One innovative business point; the point is "innovation", not "polish"; rapid iteration and trial | One requirement only: **speed** | Buy what can be bought, use open source where it exists; a monolithic system |
| Growth · piling features | Market validated; new features flood in | Implement requirements fast | Keep piling features into the original system; refactoring/optimization shelved under staffing and business pressure |
| Growth · optimization | Features accreted; one change moves many; ever slower | Development efficiency + performance | Refactor, layer, tune SQL, HDD → SSD, MySQL → Oracle, add Memcache; in most cases the optimizers win (fastest) — whether the business survives until the architects' route pays off is unknown; protect today's competitiveness first |
| Growth · architecture | Optimization exhausted: Oracle cannot take 100 million transactions, minicomputers cannot sustain 1 million online | Performance + availability | The core is one word: **split** — split functions (login/auth, orders, queries, analytics subsystems), split databases (sharding, MySQL one → two → four machines), split servers (Nginx/HAProxy load balancing) |
| Competition | Rivals enter and imitate; system counts explode | (1) reinventing wheels (every system builds its own storage/cache/database); (2) mesh interactions: interaction count grows quadratically (4 systems = 6 paths, 10 = 45) — joint debugging, joint testing, and deployment are disasters | **Platformization**: storage (TFS/JFS), database (DBProxy/TDDL), cache (Twemproxy etc.) platforms; **servicization**: message queues for async notification (Notify/MetaQ/Kafka/ActiveMQ) + service frameworks for sync calls (thrift/Dubbox/HSF) |
| Maturity | Industry leader; the business turns to "refinement": response time, experience, cost | Single-metric optimization | No fixed playbook — find weak points and optimize one by one (200ms → 50ms needs CDN/database/network acting together); all earlier-stage means remain reusable |

User scale's two qualitative jumps: (1) performance — however simple the query and high the spec, one MySQL machine caps TPS/QPS in the ten-thousands (low thousands, high tens of thousands); distribution becomes mandatory (sharding, read/write splitting, replication, sync) and complexity rises sharply; (2) availability — 10000 users down 1 hour costs a few thousand yuan; 1 million users down 10 minutes costs hundreds of thousands, and a reputation collapse can forfeit the next 1 million users. Key signal: **when the business can no longer move "fast", technology has fallen behind the business and it is time for change**; the high-level play is forecasting the next turning point and preparing technology ahead of it.

## The template's seven layers in detail (storage / development / service / network / user / business / platform)

Internet technology evolution converges from different paths; each layer is organized by "what problem it solves / typical solutions / when introduced / selection essentials":

### Storage layer

| Technology | What problem it solves | Typical solutions | When introduced | Selection essentials |
|---|---|---|---|---|
| SQL (relational) | Structured business data storage | MySQL, PostgreSQL (Oracle is too expensive and needs dedicated maintainers — generally avoided on the internet) | Needed inherently | NoSQL is Not Only SQL — a complement to SQL, not a replacement; relational databases will not be abandoned |
| Database middleware | Splitting/combining data after sharding, sparing every business a reimplementation | Large companies in-house: Baidu DBProxy, Taobao TDDL; small/mid companies open source: MySQL Router, 360 Atlas | When one database cannot sustain the load (single MySQL at ten-thousand TPS/QPS) and multiple instances are needed | Automated + platformized sharding demands top-tier engineering — build in-house only at large scale; smaller companies use open source |
| SQL storage platform | Each business running its own SQL cluster: low utilization, waste, ever-rising maintenance cost | Taobao UMP: transparent to business — resource allocation, data backup, migration, disaster recovery, read/write splitting, sharding | At large scale with many SQL clusters | The terminal unified form of storage technology: build a unified storage platform on open-source solutions |
| NoSQL | Remedies SQL's two shortcomings: special data structures + high performance (performance is every NoSQL's shared selling point) | Memcache (key-value), Redis (rich data structures), MongoDB (documents); built-in clusters: Memcache consistent hashing, Redis 3.0 cluster | A baseline requirement of internet business; adopted early | Very convenient at first — nothing like SQL sharding's complexity; generally no platform wrapping needed early on |
| NoSQL storage platform | At 1000+ nodes, two major gains: centralized management lifts ops efficiency; resource utilization rises (10% on 2000 machines = 200 machines saved = hundreds of thousands of yuan a year) | Three functions: dynamic on-demand resource allocation, automated management (businesses just request space), automatic failure takeover | Meaningful only at thousand-machine NoSQL scale; little payoff at dozens of machines | The criterion is the ops and cost returns machine scale brings |
| Small-file storage | Display-type data: files <1MB, enormous counts (up to 350 million uploads/day), enormous reads (1 billion+/day); avoids every business redesigning massive storage | Open-source base: HBase, Hadoop, FastDFS; finished products: Taobao TFS, JD JFS, Facebook Haystack | **Worth considering from the business's startup stage** — wrapping open source is not hard; no need to wait for scale | A unified, business-agnostic platform; open-source barriers are already low |
| Large-file storage | Business big data (video, hundreds of MB to TBs) and massive logs (access/operation/user trajectory); the architecture differs greatly from small-file storage — do not mix them | Hadoop ecosystem (HDFS, HBase), Storm, Hive | When video/massive-log needs arise | Open source essentially monopolizes; large companies wrap big-data platforms on open source |

### Development layer

| Technology | What problem it solves | Typical solutions | When introduced | Selection essentials |
|---|---|---|---|---|
| Unified development framework | Teams each fighting their own war: no common technical language, little communication, duplicated investment per technology, immobile staff, low efficiency | Java: SSH, SpringMVC, Play; Ruby: Ruby on Rails; PHP: ThinkPHP; Python: Django | Once systems multiply and teams coexist, set the company's major technology direction | General principle: **prefer mature frameworks; avoid blindly chasing new technology** — complete documentation, trodden pitfalls, a large talent pool, stability for long-term development |
| Web server | Carries the development framework's real external service; building one from scratch is extremely costly | Java: Tomcat, JBoss, Resin; PHP/Python: Nginx; universal fallback: Apache | Needed inherently | Take it and use it: master the open source, tune parameters and configs; **do not worry about performance too early — switching when it can no longer cope is not late** — by then there is money, people, and time |
| Containers | vs virtual machines: fast startup, almost no resource overhead | Docker | Introduced gradually with cloud-native evolution | Far-reaching: (1) an ops revolution (the basis of automated, intelligent operations); (2) changed design patterns: cheap instance startup encourages service-oriented function splits from the start, avoiding refactoring after traffic grows |

### Service layer

| Technology | What problem it solves | Typical solutions | When introduced | Selection essentials |
|---|---|---|---|---|
| Config center | Systems each managing configs: (1) coordinating config checks for multi-system joint launches is slow; (2) production troubleshooting across systems is inefficient; (3) editing configs as text has no validation — one wrong character (a 0 typed as letter O) is nearly invisible to the eye | A self-built universal config system identifying running instances by "system ID + host + port" | When systems are many and must act in concert | Four gains: centralized operation is efficient; centralized checking makes collaboration efficient; programmatic rule checks (regex validation of IPs/URLs/value ranges); config is backup — after a dead machine, the environment rebuilds fast and business recovers |
| Service center | Cross-system dependency "configuration" and "scheduling": upgrading an interface means editing configs and restarting dozens to a hundred machines; domain access still hits failed machines before cache expiry; IP direct connection means full sync on any machine change | (1) Service Name System: resolves a Service name to host+port+interface name, and the requester calls directly; (2) Service Bus System: the bus performs the call; the requester never touches the provider | When inter-system dependencies grow complex and machine counts rise | The name system is lightweight; the bus system is more thorough; understand their designs by analogy with DNS and a computer bus |
| Message queues | Cross-system **async notification** middleware: turns mesh interactions into a linear structure; supports one-to-one notification and one-to-many broadcast | Open source: RocketMQ, Kafka, ActiveMQ; in-house: Taobao Notify, MetaQ | When the business demands "fast" and needs async | Five gains: mesh becomes linear and clear, producer-consumer decoupling simplifies implementation, new consumers need zero producer changes, the queue centralizes high availability and performance, subsystems focus on business; basic functions are simple, but **reliability, ordering, and transactionality** requirements demand deep study of the open-source implementations or you will step on mines |

### Network layer

Core insight: **one system's high availability/high performance ≠ the whole business's high availability/high performance** — design from the network's perspective as a whole (this "network layer" is not about building LANs). Load balancing's context: internet concurrency past 10000 is common; major promotions/red packets reach hundreds of thousands per second.

| Means | Level/type | Performance | What problem it solves | When introduced / selection essentials |
|---|---|---|---|---|
| DNS | Geographic level | — | Regional scheduling (user → nearest datacenter) | The simplest and most universal, cheap; drawbacks: long caching (users still hit removed machines), blind to backend state; consumes public IPs, so not used at machine level |
| HTTP-DNS | Private DNS | — | Sidesteps DNS caching and flexibility limits | Flexible, controllable, timely (fast updates/failure isolation); drawbacks: high development cost, App rework needed; suits App businesses — hard for Web (browsers control resolution) |
| Nginx | Software layer 7 | Ten-thousands (≈50k/s on a Linux server) | Machine-level balancing within a datacenter; supports HTTP/E-mail | Use software when performance demands are modest; layer 7 is protocol-aware |
| LVS | Kernel layer 4 | Hundred-thousands (≈800k/s) | Protocol-agnostic — chat, databases, almost any application | Layer 4's flexibility is high |
| F5 | Hardware layer 4 | Millions (2–8 million/s) | Ultra-high-performance scenarios | A unit costs hundreds of thousands of yuan, but per-request economics may favor it (1M QPS: 1 F5 vs 20 Nginx) — at very high performance demands, prefer hardware |
| Cloud load balancing | Service | — | No self-building | Alibaba Cloud SLB, UCloud ULB — small/mid companies just buy |

The three multi-datacenter strategies compared (a single datacenter is a global network single point: power loss, network outage, disaster can paralyze everything; the core tension is inter-datacenter **latency**):

| Strategy | Approach | Latency handling | Fit/cost |
|---|---|---|---|
| Same-city multi-datacenter | A private high-speed network within the city | ≈same-datacenter results; business barely notices | Small business impact but heavy investment — only large companies afford it; cannot survive extreme earthquakes/floods |
| Cross-city multi-datacenter | Cross-city data replication (e.g., MySQL primary-standby replication) | Business compromises to eventual consistency (a 10-minute Weibo lag is acceptable; payment transfers' strong consistency is not) | Simple to implement but strongly business-coupled — run a consistency-tolerance analysis first |
| Cross-border multi-datacenter | Farther apart, higher latency | No real-time sync | Generally backup only + serving domestic users (cross-border access is too slow) |

| Technology | What problem it solves | Typical solutions | When introduced | Selection essentials |
|---|---|---|---|---|
| CDN | The "last mile" effect — essentially "trading space for time": content cached nearest the user | Professional CDN vendors and cloud CDNs (Wangsu, Alibaba Cloud, Tencent Cloud); the system: distributed storage, global load balancing, network redirection, traffic control | When static assets/images/live video are needed (live video is unstreamable without CDN) | Self-building costs enormously — **just buy the service**; architects need no deep detail |
| Multi-center | One level harder than multi-datacenter (disaster recovery, tolerating 10 minutes–1 hour of interruption and possible data loss): every center serves simultaneously, failures switch automatically with minimal human action | No universal scheme — each designed independently per business | The higher goal above the multi-datacenter baseline | The difficulties are data consistency and data transactionality; detailed per-business design is required; forcing every business multi-center is not required |

### User layer

| Goal | What problem it solves | Means | Selection essentials |
|---|---|---|---|
| Single sign-on (SSO/unified login) | Multiple subsystems cannot each manage a vast user base | cookie, JSONP, token; the most mature open source: CAS | User data is enormous (tens of millions typically, hundreds of millions for leading products) but cross-user business coupling is weak — **a simple load-balanced architecture copes** |
| Authorized login | The business becomes a platform opening to third-party applications | OAuth 2.0 (the de facto standard) | Pick it when building an open platform; avoid private protocols: more vulnerabilities, painful third-party onboarding |

**Message push** — SMS (carrier interfaces), email (email providers), in-site messages (system API); App push: iOS only via APNS; Android in China via large vendors' self-built channels or commercial third parties (Umeng, JPush). When introduced: when App operation needs arise. Selection: small/mid companies without sensitive data → use third parties directly (professional, guaranteed delivery); with sensitive data → self-build, facing three challenges:

| Challenge | Problem | Response |
|---|---|---|
| Massive device and user management | Countless devices make storage and management complex; business growth needs user-device association and tagging | Extract user traits to classify/tag |
| Connection keep-alive | Devices restrict background running, so connection channels break and messages cannot arrive in time | The densest domain of tricks: apps waking each other up, carrier whitelists, etc. |
| Message management | Selecting push targets by user traits, in ever-varying combinations | Push rules must be extremely flexible — a rule-engine-style microkernel architecture |

| Technology | What problem it solves | Typical solutions | When introduced | Selection essentials |
|---|---|---|---|---|
| Storage cloud | Storage and access of user-uploaded files: huge counts (up to 1 billion uploads/day), small files (hundreds of KB to MB), time-sensitive access (freshest right after upload, decaying over time) | The essence = "CDN + small-file storage" (reusing the storage layer's small-file technology) | When UGC upload scenarios exist | Except for leading companies, **just buy cloud** — fastest and most economical |
| Image cloud | The extra processing chain for images: cropping, compression, beautification, moderation, watermarking | A system separate from storage cloud | When image business is complex | Ordinary files need only storage + access; images do not — hence two systems |

### Business layer

The business layer has no common components/systems to extract (businesses differ wildly); every business ultimately faces the same challenge: **ever-rising complexity**. The answer is "**split**" — divide and conquer (means: layered architecture, microservices, microkernel).

| Means | What problem it solves | Approach | When |
|---|---|---|---|
| Business splitting | One system growing ever larger; businesses multiplying | Three-stage evolution: (1) all functions in 1 system (business startup); (2) split out products and orders — 2 subsystems (functions coupled, complexity rising); (3) each further splits into 6 smaller subsystems (business still swelling; leading companies run hundreds to thousands of subsystems internally) | Continuously, as the business grows |
| Virtual business domains (Facade pattern) | At hundreds to thousands of subsystems, nobody can explain the business call flow; troubleshooting is extremely complex | Merge strongly coupled subsystems into a virtual business domain by "high cohesion, low coupling", presented outward through a gateway; it is a "merge", but not a merge back into one big system | When subsystem counts run away |

### Platform layer

| Platform | What problem it solves | Core duties | When needed |
|---|---|---|---|
| Ops platform | Manual ops: heavy staffing, low efficiency, error-prone | Configuration, deployment, monitoring, emergency response (table below) | When systems are many and hand-deploying/watching dashboards is unsustainable |
| Testing platform | Manual test execution is slow and repetitive; the goal is automation lifting efficiency and quality | The four management areas: cases, resources, tasks, data (table below) | When regression volume is large and quality demands are high |
| Data platform | Data silos; every business re-collecting and re-analyzing | Three layers: data management → data analysis → data application (table below) | **Premised on "big data" scale** — below scale, good statistics suffice; startups should not copy this |
| Admin platform | Each system implementing its own permission management: inefficient, repetitive | **Authentication** (unified enterprise account login, sparing per-system user management) + **authorization control** (against privilege abuse) | When systems/platforms are many and unified control is needed |

The ops platform's four duties (mapping to system life-cycle stages) and the design core, the "four -izations":

| Duty/element | Content | Essentials |
|---|---|---|
| Configuration | Resource management: machines, IP addresses, virtual machines | The life cycle's starting point |
| Deployment | Release to production: package management, canary release management, rollback | — |
| Monitoring | Collect post-launch operating data; surface problems promptly | Automatic alerts — no human staring at data |
| Emergency response | Failure handling: stop programs, take failed machines offline, switch IPs | — |
| -izations · standardization | Set ops standards governing configuration/deployment/monitoring/emergency response | **The foundation — without standardization there is no ops platform**; systems that cannot be adapted are bridged by a middle layer (e.g., a scheduled job calling RESTful interfaces to report logs) |
| -izations · platformization | Ops operations integrated into the platform | Standards are baked into the platform — no rote memorization; operations are simple and error-resistant; one platform supports hundreds to thousands of business systems |
| -izations · automation | Repetitive operations are codified and done by the system | One-click deploy (login/upload/extract/backup/overwrite/start fully automatic), with notification on completion |
| -izations · visualization | Dashboard-style data presentation | Intuitively shows attributes (ranges/units), meanings (color zones), related data; emergency response races the clock — a minute saved = hundreds of thousands of yuan in losses avoided |

The testing platform's four management areas (basic architecture: cases → resources → tasks → data):

| Area | Duties | Essentials |
|---|---|---|
| Case management | Managing script/code-form cases for re-execution | Dimensions: business, system, test type, case code |
| Resource management | Hardware (servers/phones/tablets) + software (OS/DB/JVM) + the system under test | Automated testing is not performance-hungry — VMs/Docker raise hardware utilization |
| Task management | Assigning cases to resources for execution, tracking execution | **The core of testing-platform design** — it stitches the parts into automation |
| Data management | Recording execution time/results/CPU/memory, etc. | Three uses: showing execution status, historical comparison revealing trends (coverage 90% → 70%), data mining (why do two similar-scale businesses differ 10× in case counts?) |

The data platform's three-layer structure:

| Layer | Duties | Content |
|---|---|---|
| Data management | The platform's baseline functions | Collection (logs/user behavior/business data), storage, access (SQL/Hive/Key-Value protocols), security (protecting sensitive data under multi-business sharing) |
| Data analysis | From data to patterns | Data statistics (PV/UV/transaction volume), traditional data mining (data warehouse + rules), machine learning/deep learning (implementations vary widely — designed independently) |
| Data application | Value output | Online applications: recommendations, advertising; offline applications: reports, fraud detection, anomaly detection |

## Template tailoring guide (by business stage)

Follow the architecture's three principles: **fitness, simplicity, evolution** — the evolution direction may reference the mature template (the big picture all roads converge on), but must be tailored to the current stage; do not copy the big companies in one step.

| Principle | Application | Typical misuse |
|---|---|---|
| Fitness | Choose technology by current business complexity and user scale; if the business needs 1000 TPS, do not adopt a 50000-TPS scheme | Introducing technology for influence or résumés |
| Simplicity | At startup, monolith + open source + buying is the optimal architecture; piling features is nothing to be ashamed of | Microservicing and platform-building before the business validates |
| Evolution | The template is the destination map — evolve segment by segment at the turning points (optimization maxed out → split; mesh systems → servicization; thousand-machine scale → platformization) | Copying the big companies in one step; or clutching the old architecture and refusing to evolve |

| Business stage | Needed layers/technology | Explicitly not needed | Decision essentials |
|---|---|---|---|
| Startup (0→1, a few engineers) | A monolithic system; MySQL/PostgreSQL; an open-source framework + Apache/Nginx; with UGC, small-file storage (can be done at startup) and storage/image cloud (buy cloud); NoSQL as needed (built-in clusters, ready to use) | Database middleware/sharding, microservice splitting, config center, service center, multi-datacenter/multi-center, containers, the four platforms, message queues, building anything in-house | The only goal is "speed": buy what can be bought, use open source where it exists; piling features is the correct posture; a simple load balancer copes with user management |
| Growth (users exploding) | Caching (Memcache/Redis); CDN (buy the service); load balancing (DNS geographic + Nginx machine, or cloud SLB); a unified development framework; once optimization is maxed out, "split": split functions/databases (open-source middleware)/servers; once systems multiply: message queues (RocketMQ/Kafka), config center, SSO (CAS); Docker optional | In-house middleware, SQL/NoSQL storage platforms, service center, multi-datacenter, virtual business domains, data platform, in-house push | Optimize first, architect later: optimization is fastest and usually wins — protect today's competitiveness first; architecture actions start when "optimization can no longer hold"; same-city multi-datacenter needs large-company money, usually skipped in the growth stage |
| Maturity (industry leader, hundreds to thousands of systems) | The full template: platformization (SQL/NoSQL/small-file storage platforms), servicization (service center + message queues), virtual business domains + gateway, multi-datacenter (same-city → cross-city, per the business's consistency tolerance), Docker, unified message push (self-built), the four platforms | Multi-center only when a business's consistency and transactionality are dissected one by one — forcing every business multi-center is not required | In the competition stage, platformization + servicization first to kill wheel-reinvention and mesh interaction; in maturity, single-metric optimization (200ms → 50ms needs CDN/database/network acting together); introduce platform-layer items one by one on two signals: "much repeated work + high cost of non-standard collaboration" |

| Symptom | Response |
|---|---|
| Deploying means logging into machines to upload, extract, back up, overwrite, start; monitoring is grep+awk+sed | Build the ops platform (standardization → platformization → automation → visualization) |
| One requirement means a dozen systems change together; joint debugging is a disaster | Servicization (service frameworks sync + message queues async) + config center |
| Every new system reimplements cache/storage/database access | Platformization (storage/database/cache platforms) |
| Upgrading one interface means editing configs and restarting dozens to a hundred machines | Service center (Service Name System / Service Bus System) |
| Too many subsystems — nobody can explain the call chain; troubleshooting brutal | Virtual business domains + gateway Facade |
| A single database at the ten-thousand TPS wall | Read/write splitting/sharding (open-source middleware first), not a pricier single machine |
| Systems each manage configs; misconfigurations (0→O) slip through | Config center (programmatic rule validation) |
| NoSQL past a thousand nodes; ops straining | A NoSQL storage platform (dynamic allocation/automatic management/failure takeover) |

## Open source vs in-house vs commercial

### How to choose an open-source project

General principle: **do not reinvent the wheel, but find the right wheel**.

| Assessment dimension | Criterion | Counterexample/example |
|---|---|---|
| Does it meet the business | **Focus on business needs, not project excellence**: if the business needs 1000 TPS, a 20000-TPS and a 50000-TPS project are identical; worried TPS will grow? The architecture can evolve — refactor when scale arrives (fitness + evolution principles) | Counterexample TT (Tokyo Tyrant): positioned to "replace Memcached + MySQL at once" — in practice every use demands deciding where data lives, with many unfamiliar details to trip on — Memcached + MySQL was entirely sufficient |
| Is it mature | (1) Version at least 1.X (never 0.X), the higher the better; (2) the more and bigger the using companies, the better; (3) community activity (posts/replies/issue-response speed) | Even top developers' systems have bugs; TT corrupted files on unexpected power loss — the day's data lost entirely |
| Operability | (1) Are logs complete (a few scant lines make troubleshooting impossible); (2) is there a command line/admin console to see runtime; (3) are there failure detection and recovery (alerts, failover) | Library projects (e.g., Netty) have no operability of their own — you must log the key points in their Handlers |

### How to use an open-source project

1. **Study deeply, test carefully** — do not go live on a passing Demo: read the design documents/whitepapers to understand the design principles; verify every config item's role and impact and identify the critical config items; performance-test across scenarios, running load tests for days watching CPU/memory/disk IO; failure-test: kill, power cut, unplug the network cable, 100+ restarts, failovers. Counterexamples: launching Elasticsearch on defaults — nodes ping-timed-out and were evicted too slowly, taking the whole site down; MySQL's key parameters (innodb_buffer_pool_size, sync_binlog, innodb_log_file_size) unconfigured — then blamed for being "too slow".
2. **Apply cautiously, release gradually** — even the deepest study and testing only lowers risk; it cannot cover every production scenario (a month of expert code reading + testing still may fail in production); experience: start on non-core business, expand once experienced.
3. **Prepare for emergencies, just in case** — you may hit a bug nobody else in the world has met; back up critical business/data with a mature solution (e.g., MySQL as the fallback under MongoDB/Redis) — higher complexity and cost, but life-saving at the critical moment.

### How to do secondary development

| Strategy | Approach | Case/basis |
|---|---|---|
| Keep it pure, wrap around it (first choice) | Do not modify the original system; build auxiliary systems around it: monitoring, alerting, load balancing, administration | The Redis cluster need → add a proxy layer (Twitter's Twemproxy route); when Redis 3.0's native cluster shipped, switching over was simple. Deep-fork costs: a Redis-class project takes 2+ people over a month, and you lose the ability to track the original project's evolution (divergence too large to merge) |
| File requirements/bugs with the open-source project | When you really want to change the original system, go through the community first | Drawbacks: slow response; if urgent you must patch yourself; if not, prepare backup/contingency and wait for the community |
| Invent the wheel you need (when rich in money, people, and time) | Software has no absolute industry standard — similar wheels are interchangeable (Memcached ↔ Redis); in-house will not be scrapped like custom hardware wheels; open source is built for the general case and may not fit your business | Case: Memcached had no cache backup (one machine down slowed the business) and Redis then had no cluster — 2–4 people spent ≈2 months building a cache framework on LevelDB principles (storage/backup/cluster), later adding cross-datacenter sync; leading companies' mass in-house building is exactly this route |

### Decision table: open source vs in-house vs commercial

| Dimension | Open source | In-house/secondary development | Commercial/cloud |
|---|---|---|---|
| Fit | Generic needs, modest reliability/ordering/transaction demands (RocketMQ/Kafka/ActiveMQ ready to use) | Special business with no ready wheel; extremely high reliability/ordering/transaction demands needing deep study or in-house work; rich in money, people, time | Infrastructure class: CDN, load balancing (cloud SLB), storage/image cloud, database middleware (MySQL Router/Atlas) |
| Cost | Saves staffing and time; accelerates business | Reworking the original is costly and slow; in-house fits the business deeply | Pay for efficiency — except for leading companies, usually fastest and most economical |
| Control/risk | Subject to community evolution; immature projects are extremely risky (half-hour outages to total data loss) | Fully controllable; adapts fast to business evolution | Subject to the vendor, but ops-free |
| Rule of thumb | Meets the business + mature + operable | Keep it pure and wrap around it; fork the kernel only when urgent; reinvent only when no wheel truly fits you | Services that can be bought (CDN/load balancing/storage cloud) are not self-built; those that cannot (multi-datacenter/multi-center) must be |

## Common pitfalls

1. **The three schools' traps**: trend-chasers as guinea pigs, conservatives missing step-change gains, followers with nothing to follow/blindly imitating — the only correct posture: judge the primary complexity from the business stage; business decides technology.
2. **Copying the big companies in one step**: even with an obvious reference, evolution cannot be skipped; big-company architecture is the result of years to decades of quantitative change becoming qualitative — adopting it before your business reaches that scale only adds complexity.
3. **Architecting when you should optimize, optimizing when you should architect**: microservices during the feature-piling/optimization stage is over-design; trusting Oracle/minicomputers without splitting will not hold (no Oracle survives 100 million transactions).
4. **Immature technology/open source straight to production**: 0.X versions, total data loss on unexpected power loss (the TT case); check version, using companies, and community activity in every selection.
5. **Take-it-and-use-it without studying**: deploying to production after a few Demos (ES defaults slowed node eviction, taking the whole site); documentation study + config verification + performance/load/failure testing are mandatory.
6. **Open source without emergency backups**: critical data living solely in one open-source store cannot be manually recovered after failure; a mature fallback (MySQL backing MongoDB/Redis) costs more but saves lives.
7. **Deeply forking open-source kernels**: huge investment (Redis-class: 2 people, a month+) and you lose the ability to track the original project's evolution; keep it pure and wrap around it (proxy/monitoring systems).
8. **Choosing for excellence or for résumés**: similar wheels abound — selection only asks "does it meet the business": a 50000-TPS project is meaningless to a 1000-TPS business.
9. **Building platforms/data platforms too early**: a few dozen NoSQL machines do not justify a storage platform (thousand-machine scale does); below data scale, mining is worthless — good PV/UV statistics suffice.
10. **Judging by technical metrics while ignoring operability**: an open-source project with scant logs, no console, no failure recovery leaves you staring helplessly at failures; library projects need you to add the key logs yourself.
11. **Counting a single datacenter as high availability**: a single datacenter is a global single point (power loss/earthquake paralyzes all); same-city multi-datacenter needs large-company money, and cross-city needs the business to accept eventual consistency (a 10-minute Weibo lag is fine; payment transfers are not).
12. **Worrying about infrastructure performance too early**: do not worry about web-server performance early — switch when it can no longer cope; by then there is money, people, and time.
13. **Heterogeneous technology stacks each doing their own thing**: different frameworks/languages per team mean no common language, duplicated investment, immobile staff; unify the development framework (the unified framework itself becomes a problem needing its own solutions).
