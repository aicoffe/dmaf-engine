# Detailed Design Techniques (Interfaces / Distributed Transactions / Idempotency / Cross-Region Active-Active Implementation)

> When to read: the architecture solution is settled and detailed design has begun — when you need interface schemes, transaction consistency, idempotency, and cross-region active-active implementation decisions at the microservice/distributed landing level

## Background constraints (memorize first)

The microservice architecture's signature is "two distributeds"; every technique in this file targets these two challenges:

- **Distributed services**: functions provided by multiple microservices → interface aggregation, multi-client adaptation, circular calls (BFF, GraphQL)
- **Distributed data**: data spread across multiple independent storage systems → cross-service data consistency (distributed transactions + global idempotency are the paired solution; retries must be idempotent)

## Interface design techniques

**First judge the challenge type**: the frontend goes from "1 interface" to "calling multiple microservice interfaces" → read the BFF/GraphQL sections; interfaces suspected of dependency cycles → read the circular-calls section.

### BFF (Backend For Frontend)

**What problem it solves**: after splitting into microservices, one frontend page needs many requests to many microservices — 3 problems:

| Problem | Concrete manifestation |
|---|---|
| Request performance | Many round trips between frontend and backend; on mobile networks one slow request drags the whole page load |
| Network bandwidth | The protocol overhead of many small requests (e.g., HTTP headers) pushes up datacenter ingress bandwidth cost |
| Frontend development cost | The frontend must integrate with many microservices and multiple interface versions — a heavy development/joint-debugging/adaptation burden |

**Core mechanism**: the BFF sits between the frontend and the backend microservices as a middle layer doing data aggregation, conversion, and adaptation. Simple pages: frontend → API gateway forwarding directly to microservices; complex pages: frontend → API gateway → BFF → requests multiple microservices → assembles and returns.

**Key discipline**: **do not do interface aggregation inside the API gateway**. The gateway is business-agnostic network infrastructure with extremely high stability requirements (one failure = every business system fails); it should avoid business-flavored changes; the BFF is business-related and iterates frequently with versions — the two positions conflict.

**3 application scenarios**:

| Scenario | Approach |
|---|---|
| Interface aggregation (most common) | Microservices provide atomic interfaces; the BFF accesses, converts, merges, and trims per page needs; the frontend receives the optimal data structure |
| Multi-client adaptation | Tailor service interfaces per frontend type (App/H5/desktop client) |
| Interface compatibility | Isolate backend API version changes; support old and new versions simultaneously; the frontend migrates smoothly |

**Who builds the BFF (two landing modes)**:

| Option | Tech stack | Pros | Cons | Fit |
|---|---|---|---|---|
| Frontend owns the BFF | Node.js | The frontend best understands page aggregation needs and can iterate independently; both sides use JavaScript — one stack, low communication cost | The frontend lacks storage/cache experience; the stack differs from the backend mainstream (Java/.Net/PHP/Golang/Python), making DevOps infrastructure maintenance/monitoring/deployment troublesome | Uncomplex business, small team, lightweight BFF, mostly interface aggregation |
| Backend owns the BFF | Backend stack | Familiar with microservices, efficient development, can do complex BFFs (e.g., caching); reuses existing backend development/maintenance/monitoring/deployment infrastructure | Frontend-backend communication cost rises; the backend must understand fast-changing frontend interface needs | Complex business, large team, heavyweight BFF, multi-client adaptation, high concurrency |

**BFF vs calling microservices directly**:

- One BFF interface per page is not recommended (development cost too high; most pages are simple and low-traffic — little benefit)
- The mature practice: pages with high traffic and complexity get BFF aggregation/multi-client-adaptation interfaces; first- and second-level pages (home and navigation) are likely BFF material
- When unsure: launch the business first and decide from monitoring data (traffic, load time) whether to add a BFF

### GraphQL

- GraphQL: an API query-language specification, developed by Facebook, open-sourced in 2015; the client sends Query/Mutation specifying exactly the data it needs, and the server fetches from multiple sources (local storage, NoSQL, other microservices, third-party services) and returns per the request structure. Value: one interface call retrieves all needed data, replacing "multiple calls to different backend APIs to assemble fragments" — fewer frontend calls, no over-fetching
- **Essential difference**: GraphQL is an API interface specification; the BFF is an API interface architecture. It can be implemented in a gateway, a dedicated service, or the BFF — **implementing the GraphQL specification in the BFF is the best choice (a golden pairing)**

### Circular interface calls

**Problem**: interfaces calling themselves directly/indirectly form infinite loops — resource consumption grows explosively (CPU/memory/bandwidth) and can crash the system; with shared-resource locks (database locks, file locks) deadlock is possible; in microservices the call chains are complex, making it more hidden and harder to localize.

**Preventive measures (the industry has no cure, and the measures are ineffective)**:

| Measure | Approach | Why it fails |
|---|---|---|
| Sort out the call chains | Manually map interface dependencies and draw the call graph to find cycles | Real production call relations cannot be fully covered (a real case: one business operation triggered nearly 100 internal interface requests — nobody could explain the whole picture); cycles often trigger only on special branches, making pre-analysis practically impossible |
| Unique request identifier | Every call carries a globally unique request ID; before calling, check whether the target already carries the same ID | Request IDs must be stored and checked per call — a severe performance impact; yet 99.999% of requests never trigger a circular call |

**Practice conclusion**: handle it "problem-driven" — analyze and optimize after real production incidents; do not invest heavily in prevention.

## Distributed transaction solutions

**Why it's hard**: with distributed data, cross-database ACID transactions are infeasible; when microservices use different stores per data characteristics (e.g., MongoDB + MySQL), cross-store ACID is even less possible; microservices can only call each other via interfaces — "stored procedures" lose their habitat.

**Business-level distributed transactions**: distributed transactions implemented at the microservice layer (as opposed to database XA transactions), implementable via interface calls or message queues. Four mature industry patterns:

### Pattern comparison table

| Pattern | Core mechanism | Consistency | Performance/complexity cost | Fit | Not fit |
|---|---|---|---|---|---|
| Local transactional message (Transaction Outbox) | Business data + the transaction message table persist in the **same local transaction**; downstream is notified via interface/MQ; downstream checks the message table first for idempotency; on rollback, read the message table and send downstream a rollback message | Eventual consistency; the message table keeps a full transaction trail | No third-party components needed, but many exception branches and overall logic fairly complex | Needing convenient failure recovery and manual correction; self-built transaction frameworks | Pursuing implementation simplicity |
| MQ transactional message (Apache RocketMQ) | Two-phase commit bound to the local transaction: send a Half message (persisted but not yet deliverable) → execute the local transaction → second-phase Confirm/Rollback; if confirmation times out, RocketMQ checks back on the transaction status; delivered only after Commit | Eventual consistency; **guarantees only consistency between the local main-branch transaction and message sending, not between consumption results and the upstream transaction** — the consumer must retry as the fallback, with manual handling on persistent failure | The MQ carries the transaction flow, greatly reducing implementation complexity; high performance, easy to use | Already using RocketMQ; eventual-consistency scenarios pursuing low implementation cost | Strong-consistency requirements; consumers unable to retry as fallback |
| TCC (Try-Confirm-Cancel) | The application layer conforms to the 2PC protocol: Try checks and reserves resources → the main business commits/rolls back per Try results → the coordinator calls each service's Confirm to commit (if Try succeeds, Confirm must succeed) or Cancel to release reservations | Quasi-strong consistency (resources locked during reservation) | Invasive: the business must implement and expose Try/Confirm/Cancel interfaces — complex to design | Database-independent; atomic operations across heterogeneous stores and cross-application resources; complex core business like money/trading | Limited development resources, non-core paths |
| SAGA (from the 1987 "SAGAS" paper, Hector Garcia-Molina and Kenneth Salem) | Compensation protocol: a long transaction splits into a sequence of sub-transactions executed in order, each with a corresponding compensating transaction; any forward operation failing (e.g., T3) runs compensations in reverse back to the initial state | Eventual consistency (intermediate states visible externally) | Every sub-transaction must implement compensation logic; sub-transactions can be designed independently — easy to extend as business changes/adds steps | Long-running transactions, complex multi-step business flows | Scenarios requiring isolation / invisible intermediate states |

**Key flow details (consult during implementation)**:

- Local transactional message, normal flow in 4 steps: (1) in A's local transaction, write business data + the transaction message table (global ID, sub-transaction ID, commit/rollback operation, initiation time, key parameters, execution status); (2) A sends the transaction message to B via interface/MQ; (3) B checks its local message table for idempotency, then in one transaction writes business data + the transaction message; (4) B returns the result and A updates the message-table status. Exception handling: if step (2)/(4) fails, A resends the message; when A decides to roll back, it reads the message table and sends B a rollback message
- MQ transactional message, 7-step essentials: send the Half message (persisted but "not yet deliverable") → execute the local transaction → second-phase Confirm/Rollback → on confirmation timeout RocketMQ proactively **checks back on the transaction status** → after Commit, deliver to the Subscriber → the Subscriber executes its local transaction → submit the consumption result (success/failure)
- TCC in 4 steps: the main business asks the transaction coordinator to start the transaction → calls each downstream's Try to reserve resources → sends the coordinator commit/rollback per Try results → the coordinator uniformly calls Confirm or Cancel

**Open-source landing reference**: Apache Seata (open-sourced by Alibaba) provides AT, TCC, SAGA, and XA modes — a recommended starting point for learning and adoption.

## Global idempotency schemes

**Why it's needed**: distributed transactions cooperate via interfaces/messages, calls fail, messages get lost — eventual consistency is reached only through **retries**, and retries presuppose idempotency (Idempotence: performing the operation multiple times has the same effect as once). Typical: the network drops during payment and the user refreshes and pays again; without idempotency one order is paid twice; with it, once. **Why "global"**: the same request is not necessarily handled by the same microservice instance — the guarantee must hold across multiple microservices globally. **Overall design principle**: abstract the business operation into an object = **globally unique identifier + state**, and achieve idempotency through state control; the patterns differ only in three respects: how the identifier is generated, how the idempotent state is judged, and what scenario it applies to.

**Globally unique identifier generation**: generated by the caller (a unique frontend Token passed with the parameters); assembled from interface parameters (e.g., "user ID + order ID" identifies one payment request); assigned by the initiator (service A assigns the transaction a global ID and writes it into the event record: execution status, initiation/update time, business parameters). **State-recording convention**: after handling, B synchronously records the global ID, the status (done/fail), and the completion time; already-handled requests read the locally recorded result and return it directly without re-executing the business.

### Pattern comparison table

| Pattern | Core mechanism | Storage/interaction | Fit | Key details |
|---|---|---|---|---|
| Transaction-level synchronous processing | A assigns the global ID and writes the event record → sends the event message to B → B extracts the global ID and judges idempotency (already handled: return the locally saved result directly) → on completion, synchronously records the result → returns the processing message → A updates the event status to "completed" | Interface calls | Synchronous processing, simple handling, fast scenarios; e.g., TCC's Try interface deducting inventory | The 4 possible reasons A times out without receiving a result (A's sent message lost / B's return message lost / B crashed / B's processing errored) — whatever the cause, resend the event message; generally retry 3 times, then alert and hand to humans |
| Transaction-level asynchronous processing | Overall the same as the synchronous pattern; the difference is in B: first save the event message, then process the business **asynchronously**, updating the local event-message status on completion | Message queue | Slow, long-running handling; e.g., SAGA processing a project-management flow | Frees the caller — no blocking wait |
| Interface-level automatic idempotency | The provider implements it inside the interface handling logic: extract/assemble the global ID → judge idempotency → already handled: return the previously saved result directly; otherwise proceed normally | Interface parameters carry the global ID | De-duplication for key business operations: placing orders, payments, etc. | Global ID: generated by the caller (frontend Token) or assembled from parameters ("user ID + order ID") |

### Idempotency-judgment techniques

Core idea: **judge idempotency through "state"**. Simple states use yes/no for handled-or-not; complex states use a "state machine" (e.g., an order: Pending Payment, Paid, Shipped).

| Technique | Mechanism | Fit | Notes |
|---|---|---|---|
| Database unique index | The globally unique identifier as a unique index; insert succeeds = first handling, fails = duplicate | Data-creating scenarios: user registration, order creation | Simple and reliable, yes/no-level judgment |
| Redis SETNX | Sets successfully only when the key does not exist; key name = globally unique identifier | Distributed transactions, scheduled-job scheduling | The lightweight first choice |
| ZooKeeper node creation | Creation fails when the node exists; the same yes/no judgment | Same as Redis SETNX | Somewhat lower performance and requires maintaining a ZK cluster; prefer Redis when the system already has it |
| State-machine judgment | State saved in the database, updated with `Update ... Where ... and status = XXX` | Idempotency for complex states (order flows, etc.) | Combinable with unique index/SETNX |

## Mature cross-region active-active architecture patterns

**Relation to the "4-step active-active design" (the high-availability mainline)**: the high-availability reference covers architecture shapes, the three physical deployment patterns (same-city cross-zone/cross-city/cross-border), the 4 major design techniques, and the "4-step" process — answering "whether to do it and how to plan"; **this file focuses on landing**: how to choose and build among the industry's three mature patterns. The business-customized pattern is a continuation of the classic pattern (still valid; cloud/cloud-native optional); the business-general and storage-general patterns are newer, arising from paired supporting services and the rise of distributed-consistency databases.

**Overall approach** (common to all patterns; the architecture has three parts):

| Part | Decisions | Essentials |
|---|---|---|
| Network architecture | How traffic is scheduled | Network traffic scheduling: DNS/GSLB assigns requests to multiple remote datacenters; business traffic scheduling: routed per business rules (e.g., the user's home datacenter) |
| Compute architecture | How compute is redundant | Static allocation (comparatively money-wasting) vs dynamic generation (complex to implement, potentially risky) |
| Storage architecture | How key business data syncs | Guarantee consistency; handle data inconsistency's business impact during failures |

### Pattern comparison table

| Pattern | Core idea | Data sync/consistency | Pros | Cons | Fit | Typical cases |
|---|---|---|---|---|---|---|
| Business-customized | "Customization" thinking: keep only core business active-active per business priority (e.g., the user center prioritizes "login"; "registration/profile management" may skip), with the scheme customized around core business flows and data (e.g., "database + message queue" fast sync, "CDC + algorithms" dual writes) | Customized per business: message queues, CDC, etc. combined as needed | 1. No hard infrastructure requirements; supports long-distance two-city deployment; survives regional disasters; 2. Relatively low complexity; 3. Low landing cost (two active datacenters each holding 50% redundant resources suffice; no large supporting systems; on failure, traffic switches wholesale at the network layer) | 1. Not generalizable (each business is reworked separately); 2. Hard to extend (major changes to core business require redesign, e.g., Weibo shifting from posts to short video as the core) | Enterprises/teams with modest business scale, uncomplex business, few businesses | Alibaba Games' cross-region active-active |
| Business-general | Multi-site through unified supporting services, no prioritization of businesses; **businesses that accept BASE (eventual consistency) may deploy active-active; those requiring strong consistency deploy centralized** | Data-sync services: database replication, message queues, CDC real-time/scheduled sync; **beyond long distance (inter-city latency over 30ms), whatever the technology, it is essentially eventual consistency** | 1. No hard hardware-infrastructure requirements; long-distance two-city deployment; survives regional failures; 2. Businesses barely need rework | 1. Supporting services are complex (traffic scheduling, disaster-recovery failover, site-provisioning platform, config management, data sync); 2. Global single-point businesses exist (e.g., inventory, sellers); 3. High development cost for supporting services — needs a dedicated team | Large companies with many businesses and no high strong-consistency requirements | Taobao/Alipay's unit-based architecture (LDC, Logic Data Center) |
| Storage-general | Storage systems with native distributed consistency (Raft, Paxos); the architecture supports active-active by nature; under network partitions and node failures (short of extreme multi-datacenter simultaneous failure) it stays available and consistent | Strong-consistency sync built into the storage system | 1. Supports active-active by nature — apart from switching storage, businesses barely change; 2. Supports strong data consistency | 1. Few eligible systems: ZooKeeper, etcd (neither supports large-scale data storage), OceanBase, TiDB (database class, more widely used); 2. Hard datacenter requirement: must deploy in adjacent cities, latency ≤10ms (4 city pairs in China: Beijing+Tianjin, Guangzhou+Shenzhen, Chengdu+Chongqing, Shanghai+Hangzhou); 3. High cost: "two cities, three datacenters" or even "three cities, five datacenters" | Businesses with hard requirements on data consistency and disaster recovery (finance, telecom) | MYbank (OceanBase underneath) |

**The business-general pattern's 4 key supporting services**:

- **Traffic scheduling**: simple scenarios do network traffic scheduling only; complex ones also schedule microservice-application traffic to databases and storage systems
- **Config center**: centrally manages configuration (database connections, cache settings, etc.) with dynamic updates; the key to active-active failover/takeover — without it, fast takeover and recovery are out of the question
- **Site-provisioning platform**: container-based; on failover, temporarily requests hardware/software resources to spin up microservice applications and take over traffic; little idle redundancy needed normally — a large running-cost saving; the key foundation is the config center
- **Data sync**: unified low-level data-sync tools/systems syncing across storage media, systems, and datacenters

## Selection decision table

### Distributed transaction selection

| Condition/scenario | Recommended solution |
|---|---|
| Already using RocketMQ; pursuing simple implementation, high performance, eventual consistency | MQ transactional message (RocketMQ) |
| No third-party components wanted; need the full transaction trail for failure recovery and manual correction | Local transactional message (Transaction Outbox) |
| Across heterogeneous stores / cross-application resources; complex core business in money/trading | TCC |
| Long transactions, complex multi-step flows, frequently changing/extending business steps | SAGA |
| Consumer may fail persistently | Rely on MQ transactional messages cautiously (no consumer-vs-upstream guarantee); prepare a manual fallback |
| Need a unified distributed transaction framework | Apache Seata (AT/TCC/SAGA/XA) |

### Global idempotency selection

| Condition/scenario | Recommended solution |
|---|---|
| Short distributed transactions, synchronous interface interaction (e.g., TCC Try inventory deduction) | Transaction-level synchronous processing (retry ≈3 times; on persistent failure alert and hand to humans) |
| Long-running flows, message-queue interaction (e.g., SAGA flows) | Transaction-level asynchronous processing |
| De-duplication for key business interfaces (orders, payments) | Interface-level automatic idempotency (global ID: frontend Token, or "user ID + order ID" assembly) |
| Data-creating business (user registration, order creation) | Database unique index |
| De-duplication in distributed transactions and scheduled-job scheduling | Redis SETNX |
| yes/no idempotency when the system already runs Redis | Redis SETNX, not ZooKeeper |
| Complex state flows (orders, etc.) | State machine (database + `Update ... Where ... and status = XXX`) |

### BFF / interface selection

| Condition/scenario | Recommended solution |
|---|---|
| Simple pages, low traffic | Call microservices directly; no BFF |
| First-/second-level pages (home, navigation), high traffic and complex | BFF aggregation interfaces |
| Uncomplex business, small team, lightweight aggregation | Frontend owns the BFF (Node.js) |
| Complex business, large team, multi-client adaptation, high concurrency | Backend owns the BFF (backend stack) |
| Frontend needs precise field control, fetching multiple sources at once | Implement GraphQL in the BFF |
| Where to put aggregation logic | In the BFF, not the API gateway (the gateway must stay stable; business-flavored changes avoided) |
| Circular interface calls | No cure exists; problem-driven — handle after production incidents |

### Cross-region active-active pattern selection

| Condition/scenario | Recommended pattern |
|---|---|
| Modest business scale, uncomplex, few businesses; cost-sensitive | Business-customized (dual-active two datacenters each holding 50% redundant resources) |
| Many businesses; most tolerate eventual consistency; large company | Business-general (unit-based/LDC + the 4 supporting services) |
| Finance/telecom-grade strong consistency + disaster-recovery requirements | Storage-general (OceanBase/TiDB; adjacent-city deployment ≤10ms) |
| A business must be strongly consistent (e.g., inventory, sellers) | Centralized deployment within business-general (accept the global single point) |
| Only core businesses must survive regional disasters; long-distance deployment | Business-customized or business-general (beyond 30ms latency, eventual consistency is the ceiling) |

### Key numbers (selection basis)

| Number | Meaning |
|---|---|
| 30ms | Inter-city latency over 30ms = long distance; data sync is essentially eventual consistency (the business-general premise) |
| 10ms | Latency cap between datacenters for storage-general (Raft/Paxos); adjacent-city deployment mandatory (4 pairs: Beijing+Tianjin, Guangzhou+Shenzhen, Chengdu+Chongqing, Shanghai+Hangzhou) |
| 50% | Resource redundancy each of the two datacenters must hold in business-customized dual-active deployment |
| 3 times | The usual number of event-message resends by service A in transaction-level synchronous processing; beyond that, alert and hand to humans |
| 99.999% | Share of interface requests that never trigger a circular call (full-coverage request-ID checks are not worth it) |
| 2015 | Year Facebook open-sourced GraphQL |
| 1987 | Origin of SAGA theory (the "SAGAS" paper, Hector Garcia-Molina and Kenneth Salem) |
| 4 pairs | Number of adjacent-city pairs usable for storage-general deployment in China |
| two cities three datacenters / three cities five datacenters | Deployment patterns storage-general needs to support distributed-consistency storage (high cost) |

## Common pitfalls

- **Doing interface aggregation inside the API gateway**: the gateway is business-agnostic infrastructure; mixing in frequently iterating aggregation logic means one failure fails the whole site.
- **A BFF interface for every page**: high cost, low benefit; filter by traffic/complexity and validate with post-launch monitoring data.
- **Manually sorting call chains to prevent circular calls**: cannot cover real production call relations, nearly useless; full-coverage request-ID checks severely hurt performance (99.999% of requests checked for nothing).
- **Assuming MQ transactional messages are end-to-end consistent**: RocketMQ guarantees only local transaction ↔ message sending; consumption results vs upstream are not guaranteed — pair with consumer retries + a manual fallback.
- **Writing stored procedures inside microservices**: after database splits, local transactions no longer span; stored procedures are hellish to read/debug/change/port — unnecessary under microservices anyway.
- **Retries without idempotency**: distributed transactions rely on retries for eventual consistency; retries lacking global idempotency cause double charges and duplicate orders.
- **Unlimited retries**: the synchronous pattern retries ≈3 times; beyond that, alert and hand to humans.
- **Treating a "failure result" as "not completed" and resending forever**: once B returns a handling result (success or failure), A should update the status to "completed", not keep retrying.
- **Using ZooKeeper for idempotency when Redis already exists**: ZK is slower and requires maintaining an extra cluster — unnecessary.
- **Expecting cross-region active-active to provide strong consistency over long distances**: beyond 30ms inter-city latency, every sync technology is essentially eventual consistency; even storage-general's strong consistency requires adjacent-city deployment (≤10ms).
- **Underestimating storage-general's constraints**: few eligible systems (ZooKeeper/etcd do not support large-scale data), "two cities three datacenters"/"three cities five datacenters", high cost — non-finance/telecom businesses should think twice.
- **Not re-reviewing the active-active scheme after major business shifts**: business-customized is hard to extend; when the business pivots (posts → short video), the scheme must be redesigned from scratch.
- **Designing distributed transactions and global idempotency separately**: they are a paired solution — choosing TCC/SAGA should settle the sync/async idempotency pattern and global-ID generation in the same stroke; do not patch the design later.
