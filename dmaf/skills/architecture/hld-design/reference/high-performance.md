# High-Performance Architecture Patterns

> When to read: after the complexity identification stage confirms "high performance" as a primary complexity, or in detailed design when making high-performance design selections

## Identification signals and performance estimation

### Identification signals

| Signal | Basis | Typical cases |
|---|---|---|
| TPS/QPS scale far beyond one machine | A single business server sustains ≈5000 requests/second; estimate cluster capacity at 80%; at 100k/second the task assigner itself becomes the bottleneck | 2016 Double 11: Alipay peaked at 120k payments/second; 2017 Spring Festival: WeChat red packets at 760k/second |
| Hard response-time requirements | Millisecond-level responses, implementation details matter (one tcp_nodelay parameter moves response time 2ms→40ms; one misplaced debug log line drops TPS 30000→8000) | Trading, payments, real-time interfaces |
| Read-heavy + massive data | Reads are 90%+ of traffic; single tables at tens of millions to billions of rows degrade even with indexes, and backup/recovery times explode | A single Weibo post viewed by tens of millions |
| Performance optimization under cost constraints | Hardware evolution cannot keep up with business growth — software/architecture means must lift performance | Internet businesses running tens of thousands of backend machines |
| High business complexity degrading single-machine performance | The bigger the system, the harder to localize and optimize performance points | WeChat's backend decomposed into access/login/messages/LBS subsystems |

### Performance scale estimation

Estimation chain: average TPS/QPS from business volume → peak = average × 3 → target capacity = peak × headroom factor (2–4, for growth and bursts); cluster actual capacity = theory × 80% (e.g., 2 servers at 5000/s are theoretical 10000, actual ≈8000); compare against single-machine capacity to decide cluster/cache/sharding needs. Example: average TPS 2000 → peak 6000 → target 12000–24000 → 2–3 business machines at 5000/s + 1 assigner; at 100k/s, adding business machines alone won't do (at 25 machines the single assigner becomes the bottleneck) — the assigner itself must scale to multiple, with DNS round robin/smart DNS/CDN/GSLB splitting traffic on the user side; the architecture goes from "1-to-many" to a "many-to-many" mesh, and state management and failure handling complexity surge.

### High performance's two complexity mainlines

- **Single-machine high performance**: OS process/thread models (multi-process, multi-threading, IPC, mutexes; SMP/NUMA/MPP). No universal best: Nginx multi-process, JBoss multi-threaded, Redis single-process, Memcache multi-threaded — all achieve high performance.
- **Cluster high performance**: **task assignment** (add machines + load balancers; the assigner may be hardware F5 / software LVS / load-balancing software Nginx/HAProxy / in-house, and must handle connection management and assignment algorithms; at 100k/s the assigner must scale to multiple, with DNS round robin/smart DNS/CDN/GSLB splitting traffic on the user side); **task decomposition** (split subsystems; optimize and scale bottleneck subsystems independently; splitting is not the finer the better — inter-system calls cross the network at ≈1ms each, so at 50ms of business processing, 2 subsystems cost 51ms and 100 cost 149ms — grasp the granularity).

## Storage high performance (read/write splitting / sharding / NoSQL / caching)

Relational databases (ACID + SQL) remain the core storage; the two ways to build a high-performance database cluster compared:

| Way | Spreads access pressure | Spreads storage pressure | Supported scale | Knock-on problems |
|---|---|---|---|---|
| Read/write splitting | Read pressure spreads (writes stay on the primary) | No (every secondary holds the full data set) | Read-heavy, medium scale | Primary-secondary replication lag, read/write routing mechanism |
| Database and table sharding | Yes | Yes | Millions to tens of millions and up to billions of users | Cross-database joins, distributed transactions, count/order by, expansion migration |

### Read/write splitting

Basic implementation: a primary-secondary cluster (one primary one secondary / one primary many secondaries); the primary reads and writes, secondaries are read-only; primary-secondary replication syncs all data; business writes go to the primary, reads to secondaries (distinct from "primary-standby": the standby only backs up and serves no traffic). **Replication lag**: MySQL lag can reach 1 second, and 1 minute under heavy sync load. Solutions:

| Solution | Mechanism | Pros | Cons/costs |
|---|---|---|---|
| Read the primary after writing | Reads following a write are routed to the primary (e.g., login after registration) | Strong consistency | Tightly bound to business, invasive, easy to miss spots → bugs |
| Secondary read | Reading the primary once more when the secondary read fails | Business-agnostic — wrap the DB access API and done; low cost | Mass secondary reads can crush the primary |
| Key business reads the primary | Key business reads and writes all hit the primary; only non-key business splits reads/writes | Controllable business impact (non-key read lag tolerable; login failure is not) | The primary takes more read pressure |

**Routing mechanism** (how read/write splitting routes):
| Mode | Mechanism | Pros | Cons | Representatives |
|---|---|---|---|---|
| Program code wrapping | An abstracted data-access layer in code manages splitting and connections | Simple to implement, deeply customizable | Re-implement per language; primary-secondary failover means config changes and restarts across all systems | Taobao TDDL (jar package, centrally configured jdbc datasource) |
| Middleware wrapping | A standalone system exposing an SQL-compatible protocol, transparent to business | Multi-language support; failover invisible to business (probes via writes to a test table to judge primary/secondary) | Must support the full SQL grammar and protocol — an order of magnitude more complex and bug-prone; high performance demands on itself (every request passes through) | MySQL Router (official recommendation; MySQL Proxy never GA'd), 360 Atlas |

Selection: generally program code wrapping or mature open-source middleware; only large companies can staff in-house middleware.

### Database and table sharding

**Database split by business**: split business modules into separate databases (users/products/orders each on their own), spreading storage and access pressure. Costs: (1) no joins — two queries and programmatic assembly; (2) no transactions — MySQL XA and other distributed transactions are too slow, defeating high performance, so business code must simulate transactions and humans repair data from logs on exceptions; (3) cost multiplies (1 machine → 3; backups make it 2 → 6).

**When to split databases**:
| Scenario | Advice |
|---|---|
| Small-company startups | Don't split at the start: startup businesses rarely survive (1 in 10); join/transaction limits slow rapid validation; one database sustains ≈100k users — split after the business validates |
| Mature large companies' new business | User scale is naturally massive and solutions are proven — do the business database split, even table splits, from day one |

**Table splits** (when one business's single table hits the wall):
| Mode | Mechanism | Fit | Cost |
|---|---|---|---|
| Vertical split | Split by columns (rarely used, space-hogging large fields moved out) | Few frequently queried columns; large fields are display-only | One query becomes two |
| Horizontal split | Split by rows (same columns, different rows) | Tables with very large row counts | Routing, joins, count, order by all get complex |

**Single-table scale reference**: some companies mandate splitting past 50 million rows; complex tables past 10 million; simple tables can go past 100 million without splitting. Be alert and evaluate splitting once data reaches the **tens of millions**.

**Horizontal-split routing algorithms**:
| Algorithm | Mechanism | Pros | Cons |
|---|---|---|---|
| Range routing | Segment by an ordered column (integer/timestamp), e.g., 1 million rows per segment | Smooth expansion — add new tables, old data untouched | Distribution may skew (one segment 1000 rows, another 9 million); segment sizes of 1–20 million advised |
| Hash routing | Column value hash modulo, e.g., user_id % 10 | Even distribution | Expansion redistributes all data; the initial table count choice is critical |
| Config routing | A standalone routing table maps id → table | Simple design, most flexible expansion (migrate data + update routing table) | One extra lookup hurts performance; an oversized routing table becomes its own bottleneck |

Selection leaning: sustained growth expected, smooth expansion needed → range routing; even distribution required with estimable initial scale → hash routing; frequent flexible resizing and the extra lookup acceptable → config routing.

**Post-split knock-on problems and handling**: joins — business code or middleware runs multiple queries and merges; count() — (1) sum the counts across tables (simple but slow; 20 tables serially can take seconds); (2) a count table (table_name + row_count, updated on insert/delete; fast but complex, not same-transaction, possibly inconsistent); (3) for non-real-time needs, background scheduled updates (combined with (2)); order by — query each sub-table, then merge and re-sort. Implementation again splits into "program code wrapping / middleware wrapping", far more complex than read/write splitting (it must parse SQL for tables, count, order by, group by and rewrite).

### NoSQL

**NoSQL = Not Only SQL — a complement to SQL, not a replacement**; its advantages come from sacrificing some ACID properties (4 relational drawbacks → 4 NoSQL classes):
| NoSQL class | Relational drawback solved | Representatives | Fit | Main cost |
|---|---|---|---|---|
| K-V stores | Cannot store data structures (lists need multiple rows and reassembly; LPOP needs multiple SQL statements) | Redis (string/hash/list/set/sorted set/bitmap/hyperloglog — "a data structure server") | Structured access like caching, queues, counters, follow lists | No full ACID (only I and C; no A or D) — acceptable for most business |
| Document databases | Rigid schema constraints (DDL changes can lock tables for an hour) | MongoDB (no-schema, stores JSON/BSON) | Multi-attribute, fast-changing domains like e-commerce/games; new fields without DDL, backward-compatible history data, complex structures in one JSON | No transactions (inventory deduction + order creation not atomic); no joins |
| Column-oriented databases | High I/O for big-data statistics (row-oriented reads a whole row for one column: 4 bytes needed, 1KB read) | HBase | Offline big-data analytics (write-once, no updates); compression 8:1–30:1 (row-oriented only 3:1–5:1) | Frequent multi-column updates become random writes + decompress/compress — far slower than row-oriented |
| Full-text search engines | Weak full-text search (LIKE scans whole tables; many condition combinations explode indexes) | Elasticsearch (inverted index on every field by default) | Search (keyword → document); relational data must first convert to JSON documents for indexing | Not ACID; data must be converted to sync |

Row-oriented storage's advantage scenarios: reading many columns at once (one I/O reads the whole row) and atomic multi-column row writes — outside these, advantages become liabilities. **NoSQL selection principles**:
(1) Use NoSQL only when the business hits one of the relational database's 4 drawbacks (structure storage / frequent schema changes / big-data column statistics / full-text search) — otherwise stay relational; (2) strongly transactional core paths (money, inventory deduction) stay relational — document stores cannot make "deduct inventory + create order" atomic; (3) join-dependent relational data with many cross-entity queries does not fit document stores; (4) NoSQL's performance/flexibility edge comes from sacrificing one or more ACID properties — accept per business traits; do not veto on ACID alone (e.g., the impact on a follow list is negligible).

### Caching

**Basic principle**: keep reusable data in memory — generate once, use many times (Memcache's simple single-machine K-V queries sustain TPS 50000+). A cache's access policy is tied to storage's access policy; cache design is usually integrated into the storage access scheme, implemented as "program code middle layer / standalone middleware" (the same two wrapping modes as read/write splitting).

**Fit scenarios**:
| Scenario | Explanation | Cases |
|---|---|---|
| Data from expensive computation | The store would recompute every time (e.g., count(*) over masses of data) | A forum's real-time online count |
| Read-heavy, write-light data | Internet reads are 90%+ of traffic; massive selects crush MySQL | Hot content written once, read tens of millions of times |

**The cache architecture's three big problems**:
| Problem | Mechanism | Harm | Solutions |
|---|---|---|---|
| Cache penetration | The cache does nothing; requests pierce to storage: (1) data does not exist; (2) cache generation is hugely expensive (e.g., an order by limit page cache triggered by a full scan) | Non-existent-data attacks or crawler crawls crush or slow storage | (1) Cache a default/empty value even when storage finds nothing; (2) identify and block crawlers (SEO cost) or monitor and handle promptly |
| Cache avalanche | Regeneration after expiry takes tens to a hundred+ ms; under high concurrency masses of requests pierce to storage to regenerate | Slows the system → database down → site-wide cascade | (1) Update locks; (2) background updates (table below) |
| Cache hotspots | All requests hit the same cached item | One cache server over-pressed (tens of millions of reads on one point) | Replicate the cache item (e.g., 100 copies for hot content), append a number to keys and read randomly; **give copies randomized expiry times** to avoid simultaneous expiry avalanches |

**Avalanche solutions, one of two, compared**:
| Solution | Mechanism | Pros | Cons |
|---|---|---|---|
| Update lock | Lock cache updates so only one thread updates; thread locks on one machine, distributed locks (ZooKeeper) in clusters | Cache updates promptly; readers always see fresh data | Distributed locks cost much to implement and operate |
| Background updates | Business threads only read; cache expiry is effectively forever; a background thread updates on schedule + high-frequency probing (read every 1s/100ms, refill immediately if evicted) or message-queue-triggered updates | Works for single machines and distributed clusters alike; simpler than update locks; suits **cache warming** at launch | Within a probe interval the business may read empty values; message-queue dependence adds some complexity |

## Single-server high performance (PPC / TPC / Reactor / Proactor)

The concurrency model's two key decisions — how to manage connections and how to process requests — follow from the I/O model (blocking/non-blocking/synchronous/asynchronous) and the process model (single/multi-process/multi-threading); **architecture design sets the performance ceiling, implementation details set the floor**. Evolution: PPC/TPC assign a process/thread per connection → at many connections creation/teardown costs mount → introduce resource pools for reuse → pooled processes block on one connection's read, dragging down others → switch to non-blocking + polling → polling burns CPU → I/O multiplexing (select/epoll/kqueue: many connections share one blocking object; the OS notifies the process when a connection has data) → combined with process/thread pools, that is Reactor.

### Pattern comparison

| Pattern | Core mechanism | Pros | Cons/costs | Fit | Representatives |
|---|---|---|---|---|---|
| PPC (Process Per Connection) | Fork a process per connection | Simple, no deadlocks, processes isolated, highly stable | Fork is expensive (kernel resources + memory-image copying); parent-child IPC is complex; concurrency caps at a few hundred | Servers with concurrency in the hundreds and few connections | Traditional UNIX network servers, CERN httpd; database servers |
| prefork | Pre-create a process pool at startup accepting the same socket | No fork at connection time; faster response | IPC complexity and concurrency limits remain; accept thundering herd (solved in Linux 2.6+ kernels) | Same as PPC | Apache MPM prefork (256 concurrent by default) |
| TPC (Thread Per Connection) | Create a thread per connection | Threads are lightweight — far cheaper to create than processes; shared-memory communication is convenient | At high concurrency (tens of thousands of connections/second) thread creation still costs; mutexes/shared state deadlock easily; one thread's fault (e.g., memory corruption) can kill the whole process | Similar to PPC; at a few hundred concurrent, PPC is actually more common (more stable) | — |
| prethread | Pre-create threads | Saves creation overhead; more flexible than prefork (main process accepts and dispatches, or worker threads accept independently) | TPC's problems apply | Same as above | Apache MPM worker (multi-process × multi-threading for stability; default 16 × 25 = 400 threads) |
| Reactor (non-blocking synchronous) | I/O multiplexing (select/epoll/kqueue unified listening) + resource pools; "events come, I notify you, you (the code) handle them", a.k.a. the Dispatcher pattern | Resource reuse sustains high concurrency; the multi-Reactor design is actually simpler to implement (single responsibility, independent child processes, no synchronization) | read/send still run synchronously in the user process | The generic choice for high concurrency | See the three variants below |
| Proactor (asynchronous) | The kernel completes I/O then notifies: the Asynchronous Operation Processor completes the I/O → notifies the Proactor → calls back the Handler — "events come, I (the kernel) finish handling, then notify you" | Theoretically above Reactor (uses DMA; I/O overlaps computation) | Requires genuinely asynchronous OS I/O | Windows (IOCP); Linux AIO is incomplete — high concurrency still leans Reactor (Boost.Asio simulates with epoll on Linux) | Windows IOCP |

### The three classic Reactor implementations ("multi-Reactor single-process" exists only in theory)

| Variant | Mechanism | Pros | Cons | Representatives |
|---|---|---|---|---|
| Single Reactor single process/thread | Reactor selects, listens and dispatches; Acceptor connects; Handler does read → business processing → send | Extremely simple: no inter-process communication, no contention | Cannot use multiple cores; the whole process blocks while a Handler processes — an easy bottleneck | **Redis** (scenarios where business processing is very fast; C in a single process, Java single-threaded) |
| Single Reactor multi-threaded | Handlers only respond to events; Processors do business processing in worker threads and hand results back to the Handler to send | Uses multiple cores fully | Multi-threaded shared-data synchronization is complex (e.g., Java NIO's selectedKeys are not thread-safe); the Reactor on the main thread does all listening and responding — instant high concurrency bottlenecks | (No "single Reactor multi-process": returning child-process results to the parent and simulating a connection is very complex) |
| Multi Reactor multi process/thread | The mainReactor (parent) only accepts new connections and dispatches them to child-process subReactors; children complete all subsequent processing independently | Clear responsibilities, simple interaction (only connections pass, no data returns), independent children need no synchronization | Simpler to implement than it looks | **Nginx** (multi-process; variant: children contend on a lock to accept, no second-level dispatch), **Memcache / Netty** (multi-threaded) |

Process vs thread follows language/platform: Java generally uses threads (JVM-internal multi-threading, e.g., Netty); C may use either (Nginx processes, Memcache threads).

### Measured conclusions on single-machine concurrency models (7 models, 1-core cloud server, short-connection HTTP)

| Conclusion | Data/mechanism |
|---|---|
| Process creation ≈ 4× thread creation | forking vs threaded comparison |
| Under 2000 concurrent: preforked, prethreaded, epoll perform nearly identically (the first two occasionally slightly higher) | Low-concurrency systems — internal systems, middleware — don't necessarily need epoll; prefork/prethread are simpler |
| 8000+ concurrent: only threaded and epoll keep running | epoll degrades more smoothly; prethreaded (100 threads) fell from 2200 to 970 between 11000 and 12000 concurrent — C10K hit, thread context switching exceeding I/O processing |
| poll (and select) decline steadily with concurrency | The full descriptor set is traversed each time; select is also capped by FD_SETSIZE |
| epoll only starts declining past 10000 concurrent, and smoothly | Single-process epoll fails with too many connections too — Redis (internal access, <10000 concurrent) can run a single-process Reactor; Nginx (internet-facing, easily past 10000) must use a multi-process Reactor |

### Single-machine concurrency model selection advice

| Condition | Recommendation |
|---|---|
| Concurrency in the hundreds (internal systems, middleware, admin back-ends) | PPC/prefork or TPC/prethread — simple and stable suffice; no epoll needed |
| Thousands to ten-thousands concurrent, fast business processing (e.g., K-V reads/writes) | Single Reactor single process/thread (the Redis model) |
| High concurrency + time-consuming processing (needs multi-core parallelism) | Single Reactor multi-threaded (Java camp) or multi Reactor multi process/thread (the Nginx/Memcache/Netty model) |
| Ten-thousand-scale internet connections | Multi Reactor multi process/thread (the Nginx model) |
| Windows platform | Consider Proactor (IOCP true async) |
| Linux platform | Reactor (epoll) primary; Proactor requires simulation |

## Cluster high performance (load balancing)

The essence of high-performance clusters: add servers to raise total computing capacity; complexity concentrates in the **task assigner and assignment algorithm**. Load balancing's goal is not only "balance" — it may be performance (throughput, response time) or business traits; "tasks" span complete business processing, storage, computation, and caching, so storage/compute/cache systems too can adopt task-assignment architectures; the assigner may be a standalone device or an algorithm embedded in a program (e.g., a Memcache cluster).

### Load-balancer classification

| Class | Level | Mechanism | Pros | Cons | Performance scale | Representatives |
|---|---|---|---|---|---|---|
| DNS load balancing | Geographic level | One domain resolves to different IPs by source IP — nearest access | Simple, cheap (left to the DNS provider); nearby access is faster | Updates lag (long DNS caching); poor extensibility (control sits with the registrar); few algorithms, cannot tell server differences apart, no health checks | Coarse-grained | Registrar DNS; latency/failure-sensitive businesses may self-host HTTP-DNS |
| Hardware load balancing | Cluster level | Standalone hardware network devices | Powerful (all levels, all algorithms, global load balancing supported); strong performance; highly stable (rigorous testing + large-scale validation); built-in firewall and anti-DDoS | Expensive; cannot be extended or customized | 1 million+ concurrent (≈2–8 million/s) | F5, A10 |
| Software load balancing | Machine level | Load-balancing software on Linux servers; layer 7 (Nginx, HTTP/E-mail) and layer 4 (LVS, kernel-mode, protocol-agnostic) | Simple (easy to deploy and maintain); cheap (≈¥10k per ordinary Linux server); flexible (layer 4/7 per need; plug-in/in-house customization) | Mediocre performance; fewer features than hardware; usually no firewall/anti-DDoS | Nginx ten-thousands (≈50k/s); LVS hundred-thousands (≈800k/s) | Nginx, LVS, HAProxy |

The typical combined architecture (only large businesses need it — do not copy): user → DNS (geographic: pick a datacenter) → F5 (cluster: pick a cluster in the datacenter) → Nginx (machine: pick a server in the cluster). A small business needs one Nginx; only at multi-datacenter, multi-cluster, million-concurrent large-business scale introduce DNS → hardware → software layer by layer.

### Load-balancing algorithms

Four families: flat rotation, load-based (by server load), performance-based (by response time), hash-based.
| Algorithm | Mechanism | Complexity | Fit | Misfit/drawbacks |
|---|---|---|---|---|
| Round robin | Assign in turn; ignores server running state (crashes/disconnections can be sensed and evicted) | Extremely low (≈5 lines of code) | Similar server specs and performance; the most common | Cannot sense state or spec differences (a CPU-spinning machine still gets traffic; old and new machines get equal task counts) |
| Weighted round robin | Assign by weight (usually set statically by hardware specs), e.g., 32 cores : 16 cores = 2:1 | Low | Uneven processing capacity (old/new machines mixed) | Still blind to running state |
| Least-load first (dynamic, e.g., least connections) | Assign to the least-loaded server; metrics: connection count (LVS layer 4) / HTTP request count (Nginx layer 7, needs extension) / CPU load (CPU-bound) / I/O load (I/O-bound) | High (possibly 1000 lines of code, developed on both LB and servers); must pick a statistics window (1 minute vs 15 minutes — too short flickers, too long lags) | Scenarios where the LB forwards connections one-to-one to servers (LVS) | Connection-pool setups (e.g., a fixed pool to a MySQL cluster) don't suit least connections; poorly designed, the algorithm itself bottlenecks — in practice less common than round robin |
| Performance-based first (best response time) | From the client's perspective, assign to the fastest responder; response times must be collected/counted, possibly sampled | Very high: sampling rate (too low inaccurate, too high costly) and statistics period (10s/1min/5min) both need per-business tuning | Response-time-sensitive scenarios with large performance differences | Statistics themselves cost performance; often needs continuous tuning after launch |
| Hash family (source-IP hash / destination-IP hash / session-id hash / user-ID hash) | Hash on key information; the same hash always lands on the same server | Low | Transactional/session-bound business (the same server throughout a login session) | Hash redistribution when servers scale up/down |

The main difference between hardware and software load balancing is performance (hardware far above software); layer 4 vs layer 7 differs in **protocol** and **flexibility** — layer 7 can process and customize per application-protocol content; layer 4 is protocol-agnostic but less flexible. "Primary-standby" is not a load-assignment algorithm (tasks go only to the primary; the standby idles until the primary fails) — it belongs to high-availability design's two-machine architecture; see the high-availability reference.

## Selection decision table

| Scenario/condition | Recommended solution | Key basis |
|---|---|---|
| Single machine/single assigner at the wall (≈100k/s) | Multiple assigners + DNS round robin/smart DNS/CDN/GSLB front splitting | The assigner itself bottlenecks; the architecture becomes "many-to-many" |
| Rising business complexity capping overall performance | Task decomposition (split subsystems; scale the bottleneck independently) | WeChat's backend split into access/login/messages/LBS |
| Read-heavy (90%+), single tables under tens of millions | Read/write splitting (primary-secondary + the three lag solutions) | The simplest way to spread read pressure, no data redistribution |
| Write/storage pressure too, single table past 50 million (complex tables 10 million) | Horizontal table splits (range/hash/config routing) | Be alert at tens of millions; split tables first — same-database capacity means no database split yet |
| Business modules fighting over resources; total data volume large | Database split by business (vertical splitting) | Startups don't split first (one database sustains ≈100k users); large companies' new business splits from day one |
| Must store data structures (lists/queues/counters) | Redis and other K-V stores | LPOP etc. take multiple SQL statements in a relational database |
| Fast-changing fields, widely varying entity attributes | MongoDB and other document databases | no-schema, no DDL table locks |
| Offline big-data statistics/analytics | HBase and other column-oriented databases | Column reads save I/O; compression 8:1–30:1 |
| Multi-condition full-text search | Elasticsearch | Inverted index vs LIKE full-table scans |
| Repeatedly read computation results / read-heavy hot data | Caching (Memcache 50000+ TPS per machine) | Generate once, use many times |
| Cache penetration/avalanche/hotspots | Empty-value caching + update locks or background updates + multi-copy randomized expiry | See the cache three-problems table |
| Cache warming at launch | The background-update mechanism (load at launch, don't wait for users) | Background updating natively supports warming |
| Services under 2000 concurrent (internal systems/middleware) | PPC/prefork or TPC/prethread | Nearly identical to epoll in performance; simpler to implement |
| 10000+ concurrent + extremely fast processing | Single Reactor single process/thread (the Redis model) | No locks, no communication; the bottleneck is not in processing |
| 10000+ concurrent + time-consuming processing | Single Reactor multi-threaded or multi Reactor multi process/thread | Multi-core parallelism; the Nginx model sustains 10000+ connections |
| Single-machine concurrency model at its ceiling (e.g., Nginx 50k/s) | Software load-balanced cluster (Nginx/LVS) | Cheap and flexible; LVS layer 4 at 800k/s |
| Cluster-level 1 million+ concurrent, security protection needed | Hardware load balancing (F5/A10) | Strong performance, stable, firewall/anti-DDoS |
| Multi-datacenter/multi-geography | DNS load balancing (or self-hosted HTTP-DNS) | Geographic-level nearest access |
| Uneven server specs | Weighted round robin | Weights set by hardware specs |
| Session/transaction stickiness required | Source-IP hash / session-id hash | The same session pins to the same machine |
| Volatile server loads with direct LB-to-server connections | Least-load first (least connections etc.) | High complexity — confirm necessity before adopting |
| Extremely response-time-sensitive | Performance-based first (response time) | Needs sampling statistics + period tuning |

## Common pitfalls

| Pitfall | Typical case | Avoidance |
|---|---|---|
| Replication lag bites: read the secondary right after writing — "just registered but told unregistered" | MySQL lag 1 second to 1 minute | Read the primary after writing / secondary read / key business all on the primary |
| Underestimating in-house read-write-splitting/sharding middleware | Middleware is an order of magnitude more complex than code wrapping | Prefer program code wrapping or mature open source (TDDL, MySQL Router, Atlas) |
| Sharding too early for performance | Startups survive 1 in 10; one database sustains ≈100k users | Split after business validation, when scale truly arrives |
| Not alerting at tens-of-millions rows, acting only at billions | Complex tables split at 10 million; 50 million is the reference line | Monitor table rows and access performance; start evaluation at tens of millions |
| Believing single Reactor/epoll is universal | Single-process epoll declines past 10000 concurrent | Pick the model by concurrency: 10000+ uses multi Reactor multi process/thread (the Nginx model) |
| Forcing epoll/complex models onto low-concurrency systems | Under 2000 concurrent, epoll performs the same | Prefer simple models — cheaper to implement and maintain |
| Chasing single-machine performance while ignoring cluster and implementation | One debug log line: TPS 30000→8000 | Balance architecture selection + load testing + implementation details (parameters like tcp_nodelay) |
| Splitting tasks/systems too finely, ending slower | 100 subsystems: 149ms vs 50ms of business processing itself | Control split granularity; reduce cross-system network calls |
| Non-existent-data attacks piercing the cache | Mass requests for absent data pierce to storage | Cache a default value even when storage finds nothing |
| Uniform expiry on cache copies | 100 copies expiring together = an avalanche | Give copy expiries a random range |
| Single-machine update locks against cluster avalanches | Dozens to hundreds of servers updating concurrently still avalanche | Use distributed locks (ZooKeeper) or the background-update mechanism |
| Treating NoSQL as a relational replacement | Document stores: no transactions, no joins; Redis lacks A and D | Use as a complement per scenario; strongly transactional core data stays relational |
| Writing joins/transactions in single-database mindset after splitting | Cross-database joins need two queries; XA is too slow | Identify cross-database queries/transactions at design time; change the business scheme or simulate in code + manual fallback |
| Blindly seeking advanced load-balancing algorithms | Least connections ≈1000 lines of code vs round robin ≈5 | Default to round robin/weighted; use dynamic algorithms only for real state differences |
| Least connections misused with connection pools | With a fixed pool, connection count doesn't reflect load | Switch to round robin/weighted or request-count-based extension |

Related references: primary-standby/primary-secondary failover, two-machine hot standby and similar schemes — see the high-availability reference (two-machine architecture); load balancing and clusters' high-availability guarantees — see the FMEA and high-availability cluster sections.
