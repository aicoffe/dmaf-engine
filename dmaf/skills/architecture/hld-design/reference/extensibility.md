# Extensible Architecture Patterns

> When to read: after the complexity identification stage confirms "extensibility" as a primary complexity, or when making architecture splitting, microservice, or plug-in design decisions
>
> Typical triggers: (1) requirements iterate frequently and the system is asked to "support new business quickly"; (2) the team grew and the system must be split; (3) debating whether to adopt microservices/SOA/rule engines; (4) concrete extension-point design such as multiple access protocols, multiple databases, fast-changing rules.

## Identification signals

**Definition of extensibility**: the extension capability a system provides against future requirement changes — when new requirements appear, none or only minor modification is needed, with no system-wide rework or rebuild.

When requirements show these traits, "extensibility" should be identified as a primary complexity:

| Signal | Explanation |
|---|---|
| Fast-changing business, frequent requirement iterations | Every new requirement forces a major rework — extremely costly |
| Features expected to keep growing | The system keeps being modified and evolving after release |
| Multi-person team developing in parallel | Skill levels vary; reasonable splits forcibly bound the blast radius of errors |
| Demands for fast experimentation and delivery | Typical: web-based internet innovation businesses |
| Rule/policy logic that keeps recombining | Billing, insurance, promotions, etc. (points to microkernel/rule engines) |
| Replaceable extension points such as multiple access protocols/stores | E.g., XML/JSON/ProtocolBuffer access, MySQL/Oracle/DB2 storage (points to changing/stable layer or abstraction/implementation layer) |
| Predictable sustained team growth | Service granularity re-splits as teams grow (Three Musketeers) |

Extensibility is usually not the only complexity; rank it together with high performance, high availability, etc. by importance. There is no universal standard for how far to predict — calibrate with experience and review.

**The two basic conditions for designing extensible systems**: (1) predict change correctly; (2) handle change perfectly (handling = **isolating** change behind a changing layer, or **encapsulating** change behind an implementation layer).

- The three difficulties of predicting change: you cannot consider extensibility at every design point; you cannot ignore it entirely; every prediction can be wrong.
- **The 2-year rule**: predict only possible changes within 2 years, not 5 or 10; in fast-moving industries 2 years suffices, and in slow ones looking further adds little.
- **Write the 1st time, copy the 2nd, refactor the 3rd** (Rule of three): implement directly the 1st time, copy-modify the 2nd; only when a similar case appears the 3rd time, refactor (isolate or encapsulate). Example: integrate WeChat Pay → copy for Alipay → only when the third provider arrives, wrap with template method/strategy pattern.

## The basic idea of extensibility and split granularity

**The basic idea is one word: split** — break the monolithic system into smaller parts so extensions touch only part of it, shrinking the change scope and the risk.

### Three splitting approaches (scope from large to small: process > service > function)

Using the TCP/IP stack as an example: process = the four-layer model; service = HTTP/FTP/SMTP in the application layer; function = HTTP's GET/POST, FTP's uploads and downloads. Student-system example:

| Splitting approach | Split (student-system example) | Matching architecture pattern | Advantage when extending |
|---|---|---|---|
| Process-oriented splitting | Presentation → business → data → storage | Layered architecture | Extensions usually touch one layer (e.g., storage MySQL → also Oracle touches only the storage + data layers) |
| Service-oriented splitting | Services for registration, login, profile management, security settings | SOA, microservices | Extending/adding touches only related services (e.g., adding "student-ID registration" touches only registration + login) |
| Function-oriented splitting | Each service further splits into fine-grained functions (phone-number registration, email login, etc.) | Microkernel architecture | New functions add only new function modules + adjacent-function changes |

**Key conclusion: the splitting approach inherently determines the system's extension approach.** Patterns combine: microservices overall (service-oriented), layering inside services (process-oriented), microkernel for individual services like login (function-oriented).

### Two refinement schemes for handling change

| Scheme | Core idea | Structure | Complexities |
|---|---|---|---|
| Scheme 1: refine a "changing layer" + "stable layer" (adaptation layer) | **Isolate** change behind the changing layer | The changing layer may depend on the stable layer or the reverse — per business (multi-protocol access uses form 1; multi-database storage uses form 2) | (1) The line between changing and stable layers is often unclear — review debates flare; (2) inter-layer interface design is hard: the stable layer's interface must be as stable as possible and must find the common ground across divergent implementations (e.g., MySQL's `REPLACE INTO` vs Oracle's `MERGE INTO` differ functionally — whose convention does the interface follow?) |
| Scheme 2: refine an "abstraction layer" + "implementation layer" | **Encapsulate** change behind the implementation layer | Abstraction-layer interfaces stay fixed; processing is unified against the interface; new features only add implementations in the implementation layer, without touching the abstraction layer | "Flexible design" is itself complex: e.g., once fixed, the decorator pattern's rules cannot change easily; typical practices are design patterns and rule engines |

### Split granularity: the cost of too fine and too coarse

| Granularity | Cost |
|---|---|
| Too fine (over-splitting) | (1) Inter-system complexity: n services ≈ n×(n-1)/2, rising explosively with count; (2) team efficiency: 5–6 people over 30+ services — 5+ services per person, one requirement crossing multiple services and 6–7 interfaces, dev/test/ops switching nonstop; (3) performance: every call crosses the network; production interfaces average ≈50ms, so 6 calls per request = 300ms; (4) fault localization: failures spread; root cause can take hours; (5) deployment cost rises explosively |
| Too coarse (no split or too few) | Single services grow hugely complex; extensions move the whole body; change scope cannot be bounded and errors affect everything |

**Granularity criterion — the "Three Musketeers" rule**: one microservice is developed by 3 people; derive service count from team size (6 people → 2 services, 12 → 4). Rationale: system complexity is comprehensible to everyone yet divisible; when 1 is on leave, 2 can hold the line (avoiding single points); discussion is effective yet consensus is fast. In maintenance, 1 person may maintain 1 or even several services, but each service is best backed by 2 people.

## Layered architecture

Also called N-tier, N at least 2. Three forms by what is divided and along which dimension:

| Form | Object | Dimension | Dependencies | Typical |
|---|---|---|---|---|
| C/S, B/S | The whole business system | User interaction (interaction as one layer, backend as another) | Layer by layer | Desktop clients, browser/server systems |
| MVC, MVP | A single business subsystem | Responsibility | **Flexible — layers interact pairwise** | MVC: Model for data and business logic, View for presentation, Controller for forwarding and dispatch; MVP is its variant |
| Logical layering | A subsystem or the whole system | Responsibility | **Top-down dependency** | OS kernels, TCP/IP, Android, J2EE's four layers (Presentation → Business → Persistence → Database) |

Choice basis: interaction shape decides C/S vs B/S; flexible pairwise interaction inside a subsystem uses MVC/MVP; clear one-way dependencies and full-chain constraints use logical layering. Logical layers' responsibilities (student-system example): presentation (pages/interaction), business (business logic), data (data access), storage (MySQL, Memcache, etc.).

- Extensibility's essence is **separation of concerns**: each layer handles only its own logic; extending one layer leaves others untouched (e.g., adding a new filesystem to the Linux kernel touches only the filesystem layer).
- Inter-layer dependencies must be **stable**: Linux abstracts the filesystem interface via VFS, so a new filesystem only adapts to VFS. Whether the interface warrants its own layer depends on complexity — complex systems (operating systems) can afford a dedicated interface layer; forcing one into a simple business system is overkill.
- **Layer-by-layer passing, pairwise dependency** is a hard constraint: benefit — dependencies are forcibly bounded and overall complexity drops; cost — wrapper functions at each layer are **redundant** (low implementation complexity, acceptable) and requests traverse all layers (negligible on modern hardware in most scenarios). **Do not bypass layers** (e.g., Presentation calling Persistence directly) or the architecture tangles, extensions run wild, and every change moves the whole body.
- Layer-count guidance: commonly 2–4 layers; 5+ is rare (only extremely complex systems like OS kernels reach it); **the core is that layer differences are clear enough and boundaries distinct enough — readable straight off the architecture diagram**, or ownership arguments and chaotic implementation follow.

## SOA

Service Oriented Architecture, landed mainly in traditional enterprises (manufacturing, finance). **Background** (enterprise IT systems duplicated and heterogeneous): departments built independent systems with duplicated functions (deleting a departing employee's permissions means visiting multiple systems); systems came from different vendors with different technologies and could not be reworked; processes needed multi-system cooperation with no standard way — new flows meant heavy custom development.

| Concept | Content |
|---|---|
| Service | Every business function is a service exposing open capability other systems use without custom development; granularity is yours to judge per enterprise reality |
| ESB (Enterprise Service Bus) | Connects heterogeneous services: protocol conversion (REST→RMI/AMQP, etc.), data conversion (JSON→Java, etc.), transparent dynamic routing. **Widely criticized**: the combinations of protocols (JMS/WS/HTTP/RPC) and data formats (XML/JSON/binary/HTML) are vast; conversion is complex and expensive; at high message volume the ESB becomes the global performance bottleneck; yet rewriting heterogeneous legacy systems costs too much, so only the ESB can bridge them |
| Loose coupling | Reduce inter-service dependencies and mutual influence; true full backward compatibility is hard |

**Position**: SOA is an enterprise-level architecture idea ("this enterprise adopted SOA"); do not split SOA + a dedicated ESB inside a single system (high cost, no benefit).

### SOA vs microservices comparison table

| Dimension | SOA | Microservices |
|---|---|---|
| Service granularity | Coarse ("employee management system" is one service) | Fine (split into employee profile/attendance/leave/benefits services) |
| Service communication | ESB heavyweight: service definitions, routing, message conversion, delivery | Unified lightweight protocols (RESTful, RPC); "Smart endpoints and dumb pipes" (pipes carry messages only, not format or content) |
| Service delivery | No special requirement (the focus is compatibility with existing systems) | Requires fast delivery → automated testing, continuous integration, automated deployment are mandatory; past ≈20 microservices without automation, fast delivery is impossible |
| Application scenario | Vast, complex, heterogeneous enterprise systems (years of legacy, multiple vendors, cannot start over) | Fast, lightweight, web-based internet systems (fast-changing business; external interfaces naturally HTTP RESTful) |

**Conclusion**: the two are essentially different architecture philosophies that intersect only at "service" — no better or worse, only scenario fit. Forcing microservices onto enterprise heterogeneous legacy systems may cost far more than implementing SOA. Internet systems are mostly greenfield web systems with naturally uniform HTTP RESTful interfaces, no large legacy needing an ESB to shield, and a fast-delivery requirement — hence microservices became the internet mainstream.

## Microservices (method + infrastructure)

### Merits and pitfalls

**Definition**: develop a single application as a suite of small services, each running in its own process, with lightweight communication (usually an HTTP resource API), built around business capabilities, independently deployable via fully automated deployment machinery. Three keywords: **small, lightweight, automated**. Merits: per-service complexity drops; independent deployment and independent scaling; smaller failure blast radius; faster iteration; splitting by reliability keeps non-core failures off core; splitting by performance keeps high-pressure modules from dragging down others.

**The six pitfalls (microservices are not a panacea):**

| # | Pitfall | Key facts |
|---|---|---|
| 0 | Bandwagon adoption | Ignoring team size, business development, and technical foundations, believing microservices solve everything; the "micro" itself is a trap |
| 1 | Services split too fine, relations too complex | Complexity moves from within systems to between them, n×(n-1)/2 explosive growth; 3 coarse services have simple relations; fine-grained single services shrink but relations get far more complex |
| 2 | Too many services, team efficiency collapses | 5–6 people over 30+ services — 5+ per person; one requirement crosses multiple services and 6–7 interfaces; developers juggle many repos, testers deploy many environments and services, ops release many interdependent services |
| 3 | Call chains too long, performance drops | Microservices talk HTTP/RPC, ≈50ms per call × 6 calls = 300ms; high-performance scenarios suffer and need a lot more hardware — cost rises |
| 4 | Call chains too long, localization hard | Failures spread (C's slow query → C → B → A → user error); layer-by-layer checks cost half an hour to an hour each; simultaneous failures in multiple services make root cause even harder |
| 5 | No automation, no fast delivery | No automated tests (massive manual regression), no automated deployment (hand-typing shell on dozens of machines), no automated monitoring (hand-reading logs on dozens of machines) — less efficient than one big system |
| 6 | No service governance, chaos at scale | Service routing (60 nodes across 20 machines — how do dependents know?), fault isolation (5 nodes down — now what?), registration and discovery (60 → 80 nodes — how to sync?); "lightweight" eventually grows a governance system nearly as complex as an ESB |

**Signals you should not use microservices**: a 5–6 person team splitting 30+ services; no automated testing/deployment/monitoring; high-performance scenarios sensitive to call-chain time (≈50ms per call); innovation-phase businesses with undecided direction (validate fast first, split after the business takes off); enterprise heterogeneous legacy (choose SOA); microservices already stable in maintenance need no further subdivision.

### Service splitting method

Granularity baseline: first compute the service-count range with the "Three Musketeers", then choose the split. **Granularity varies with team size — never discuss business granularity apart from the team:**

| Team size | Service count | Granularity example |
|---|---|---|
| 10 people | ≈4 services | "login, registration, user profile management" all fold into a "user service" |
| 100 people | ≈40 services | "user login" is its own service |
| 1000 people | more | even "user connection management" stands alone |

| Splitting method | Basis | Essentials |
|---|---|---|
| By business logic | Business-module scope of responsibility | Most common; most contested (should e-commerce split into 3 or 6 services?) — team size decides, and most cases over-split |
| By extensibility | Module stability | Mature, rarely changing modules become **stable services** (coarse granularity OK; weakly related services may share a system); frequently changing ones become **volatile services** (fine granularity, but cap the total); goal: fast iteration without hurting mature features |
| By reliability | Module priority | Separate core from non-core, focus on core availability; benefits: non-core failures leave core unaffected (e.g., log-reporting floods), core high-availability solutions get simpler, machine/bandwidth costs drop |
| By performance | Performance pressure | High-pressure modules stand alone (e.g., rush-purchase entry queuing); may split web services, databases, caches, etc. |

The methods **combine freely** (e.g., split A by reliability, B by performance, C/D/F by extensibility, plus existing X — 6 services total).

### Infrastructure checklist

**Core judgment**: what really decides microservice success is "automated". Unreasonable granularity can be re-split/merged after landing; missing infrastructure is hard to remedy. Microservices do not reduce complexity — they move it from the ESB into infrastructure ("service discovery" and "service routing" were ESB functions all along). Optional open-source suite: Spring Cloud (service discovery, routing, gateway, config center, etc.). With few services, priority 3/4 items can be carried manually; their importance rises with node count.

| Priority | Infrastructure | Purpose |
|---|---|---|
| 1 (most basic) | Service discovery, service routing, service fault tolerance | The minimum for running microservices |
| 2 | Interface framework, API gateway | The interface framework boosts internal service development efficiency; the API gateway boosts external integration efficiency |
| 3 | Automated deployment, automated testing, config center | Boost testing and ops efficiency |
| 4 | Service monitoring, service tracing, service security | Further boost ops efficiency |

| Facility | Essentials |
|---|---|
| Service discovery | Nodes are many and change often (scaling/fault isolation/canary upgrades); manual configuration cannot take effect in real time; the core is the **service registry** (auto-registration at startup; callers or the LB query). Self-service: each service does it via a shared library — simple, no obvious risk; proxy-based: the LB does it uniformly, but an LB failure hits every call and concentrated traffic easily bottlenecks — cluster it |
| Service routing | Pick one eligible available node per call; implemented together with discovery; core algorithms: random, round robin, least pressure, fewest connections |
| Service fault tolerance | Node counts explode, overall failure probability explodes and spreads — must be automatic; means: request retry, flow control, service isolation, usually integrated into discovery/routing |
| Interface framework | Beyond uniform HTTP/REST or RPC, the data format must be uniform (e.g., JSON and its specification) or every microservice adapts dozens of formats; provided as libraries/packages (Java package, Python package, C library) |
| API gateway | The single external entry: authentication, authorization, transport encryption, request routing, traffic control; internal point-to-point access is not exposed |
| Automated deployment | Nodes are several to tenfold more and frequency way up (one example business deployed on 70% of workdays — tens of times a monolith); functions: version management, resource management, deploy operations, rollback |
| Automated testing | Interface counts explode + fast delivery — manual regression cannot keep up; cover unit tests, single-system integration tests, inter-system interface tests; when short-handed, **at minimum automate interface tests** |
| Config center | Runtime config changes dynamically and must hit all nodes instantly; functions: config version management (different node sets of one service may hold different values), CRUD, node management, config sync-push |
| Service monitoring | Objects (machines/network/processes/interfaces) explode — manual collection and analysis is unrealistic; roles: real-time collection and analysis to shorten handling time + early warning to shrink blast radius; record request counts, response-time mean/max, error-code distribution; a dedicated system is advised |
| Service tracing | Records one request's full path (initiation/response time, error codes, parameters, returned JSON); tracing vs monitoring is micro vs macro; most implementations build on Google's Dapper paper |
| Service security | Nodes may reach each other, but sensitive data/operations are for only some microservices; split into access security, data security, transport security, usually integrated via the config center, with generic policies packaged as libraries |

**"Infrastructure before microservices"**: past ≈20 services, fast delivery without automation (testing/deployment/monitoring) is impossible and deployment costs explode — before adopting microservices, assess automation capability and prefer building infrastructure first (or adopting Spring Cloud) before splitting services; with very few services (e.g., 2–4), some facilities can be carried manually and filled in by priority.

## Microkernel architecture

Microkernel Architecture, a.k.a. plug-in architecture, is the **function-oriented splitting** extensible architecture, typically for **product-based** (multi-version, downloaded and installed) applications: IDEs like Eclipse, UNIX operating systems, clients like the Taobao App; some enterprise business systems use it too, e.g., insurance valuation logic systems (different product types wrapped as plug-ins).

**Essence**: a core system (core system) + plug-in modules. The core system handles business-agnostic generic functions (module loading, inter-module communication) and stays stable as the business extends; plug-ins implement concrete business logic and extend as the business needs. **Wrap the changing parts inside plug-ins to extend quickly and flexibly without destabilizing the whole.**

| Key point | Problem | Implementation |
|---|---|---|
| Plug-in management | The core system must know which plug-ins exist, how to load them, when to load them | A plug-in registry (config file/code/database all work) recording plug-in name, location, load timing (at startup or on demand) |
| Plug-in connection | How plug-ins attach to the core system | The core defines the connection specification; plug-ins implement it; common mechanisms: **OSGi**, **message pattern**, **dependency injection** (Spring's approach), distributed protocols (RPC, HTTP) |
| Plug-in communication | Plug-ins are decoupled, but business flows often need multi-plug-in cooperation | Communication must be forwarded via the core system |

**OSGi in brief**: OSGi is a plug-in standard, not a runnable framework (Eclipse uses Equinox; peers include Apache Felix and Spring DM). Three logical layers: Module layer (plug-in management: plug-ins are Bundles, Java JAR files containing MANIFEST.MF metadata); Lifecycle layer (plug-in connection: install/update/start/stop/uninstall via BundleActivator's start/stop); Service layer (plug-in communication: register/look up services via BundleContext.registerService — note this is the inter-plug-in communication mechanism, not the plug-in registry).

**Rule engines in brief**: a concrete microkernel implementation (execution engine = microkernel, rules = plug-ins) for billing, insurance, promotions, risk control; e-commerce promotion rules run to dozens or hundreds, and their combinations yield hundreds to thousands of schemes — pure code development cannot keep up with business change. Three merits: extensible (business logic separated from the business system), understandable (rules described in natural language that business people can read and operate), efficient (visual rule authoring, approval, management). Flow: developers distill business functions into rules stored in the **rule repository** (usually database storage) → business people combine rules into business flows stored in the **business repository** → the engine executes. Mapping to the trio: plug-in management = rule repository; plug-in connection = rule language; plug-in communication = data flows and event flows (rules do not communicate actively — the engine passes results to the next rule). The most common open-source implementation: **JBoss Drools** (Java, Rete-algorithm based, JSR-94 compliant, with the web BRMS Guvnor); its rule language is close to a programming language — high learning cost for business people — so practice wraps it with visual configuration.

**Fit and misfit:**

- Fits: businesses whose rules recombine frequently (billing/insurance/promotions/risk control); product-based products needing third-party/dynamic extension; cases where business people (non-programmers) configure business logic.
- Misfit/use cautiously: simple systems with few, stable functions and no plug-in payoff; logic where the core/plug-in boundary is unclear or plug-ins couple tightly to the core; server-side web businesses should prefer layering/microservices over microkernel.

## Selection decision table

| Condition/scenario | Recommendation |
|---|---|
| Simple stable business, tiny team (1–3 people), few changes | A monolith with good code layering; do not over-design for the future (changes invisible within 2 years are not predicted) |
| Organizing by responsibility inside one system, extensions concentrated in one kind of logic (presentation/business/data/storage) | Layered architecture (C/S, B/S, MVC/MVP, logical layering); keep inter-layer dependencies stable and pairwise, no layer bypassing |
| Logic is concentrated but the change point is rare and clear (1st–2nd occurrence) | Don't split; write-copy: implement directly or copy-modify |
| The same kind of change appears the 3rd time | Refactor: isolate (changing/stable layer) or encapsulate (abstraction/implementation layer); design patterns/rule engines |
| Must support multiple access protocols or multiple stores (XML/JSON/ProtocolBuffer; MySQL/Oracle/DB2) | Changing/stable-layer isolation, or abstraction/implementation-layer encapsulation; design the interface on the common ground of the implementations and keep it stable |
| Presentation-logic separation inside a subsystem, flexible dependency relations | MVC/MVP (pairwise layer interaction); for top-down one-way dependency use logical layering (J2EE-style, OS-style structures) |
| product-based clients/IDEs/operating systems; rules recombining frequently (billing, promotions, risk control); business people configuring logic themselves | Microkernel architecture (the OSGi standard, or rule engine Drools + visual wrapping) |
| Enterprise heterogeneous legacy needing interop (multiple vendors, years of history, cannot start over) | SOA (services + ESB + loose coupling); use the ESB to bridge heterogeneity — do not force microservices |
| Internet business, fast-changing, fast trial and fast delivery, team size supports it (≈3 people per service) | Microservices; confirm the automation base first (testing/deployment/monitoring) and cap count with the "Three Musketeers" |
| Team of 10 → ≈4 services; 100 → ≈40 services; re-split as the team grows | The microservice splitting rhythm (Three Musketeers rule) |
| Need fault isolation / core protection, or one module's performance pressure dominates | Within microservices, split core services by reliability and pressure modules by performance (combined with business-logic splitting) |
| Starting microservices with few services (e.g., 2–4) | Priority-1 infrastructure first (discovery/routing/fault tolerance) + interface framework + API gateway; carry priority 3/4 manually and fill in as nodes grow |
| External systems/partners need to integrate your capabilities | Unified API gateway entry (auth, authorization, encryption, routing, flow control); internal point-to-point access is not exposed |

**Evolution signals** — evolving from monolith/layered to microservices requires all of: (1) fast-changing business with clearly divergent iteration rhythms across modules (some change weekly, some are mature); (2) team growth to where one system cannot be divided/understood by 3 people (service count = team headcount ÷ 3); (3) need for independent deployment rhythms and reliability/performance-tiered guarantees; (4) automation in place (or Spring Cloud et al. adopted) with service count projected not to run away (a governance system as backstop).

**Evolution path**: a 6-person team runs 2 microservices → team grows to 12 → split into 4; unreasonable granularity (too coarse/too fine) can be re-split/merged after landing; missing infrastructure is hard to remedy. **Combine**: microservices (service-oriented) + layering inside services (process-oriented) + microkernel for individual services (function-oriented) — the three are not mutually exclusive.

## Common pitfalls

| Pitfall | Consequence and countermeasure |
|---|---|
| Treating microservices as a panacea | Adopting microservices for high performance/high availability violates the fitness and simplicity principles |
| Bandwagon adoption, résumé-driven | Ignoring team size, business development, and technical foundations, believing adoption solves everything |
| Over-splitting, worshipping "small" | Inter-service complexity n×(n-1)/2 explosive; 5–6 people over 30+ services, 5+ per person; granularity is decided by team size via the Three Musketeers — do not argue granularity from the business angle |
| Ignoring the "automated" infrastructure | Without automated testing/deployment/monitoring, microservices are less efficient than one big system; past ≈20 services automation is mandatory — build infrastructure before splitting |
| Worshipping "lightweight", skipping governance | Routing/fault isolation/registration-discovery by hand means chaos; a governance system is no simpler than an ESB — complexity only moves, never vanishes |
| Interface protocols not unified | Unifying HTTP/REST but not the JSON data specification means every microservice adapts dozens of formats; solve with a unified interface framework (library/package) |
| Predicting extensibility at every point | The architecture balloons and ultimately cannot land; bound predictions with the 2-year rule and defer abstraction with write-copy-refactor-at-three |
| Predicting nothing | The system needs a refactor the moment it launches — the upfront investment is wasted; predict the key change points within 2 years |
| Bypassing layer constraints | View calling Dao directly for convenience ends in a tangled architecture, runaway extensions, whole-body moves; the wrapper-function redundancy between layers is low complexity — tolerate it |
| ESB performance bottleneck | Protocol/data-format conversions are many and expensive; at high message volume the ESB becomes the global bottleneck; SOA scenarios must plan ESB capacity |
| Call-chain performance and localization | 6 calls ≈300ms; failures spread and localization takes hours; needs service tracing (Dapper-style) + service fault tolerance (retry/flow control/isolation) + service monitoring with alerts |
| Handing rule engines straight to business people | Drools' rule language is costly to learn; only visual configuration wrapping delivers "understandable and efficient" |

**Pre-split/pre-extension self-check:**

1. Is the primary complexity really "extensibility", or is it performance/availability? (Identify complexity first; never default to microservices)
2. Will the predicted change arrive within 2 years? Which occurrence is this? (<3rd: no abstraction)
3. The splitting approach (process/service/function) determines the extension approach — does the desired extension direction match the split?
4. Team headcount ÷ 3 = reasonable service count — exceeded?
5. Are automated testing/deployment/monitoring ready? Past 20 services, can you still deliver fast?
6. Are layer dependencies pairwise stable? Will someone bypass a layer?
7. Is the core-system/plug-in boundary clear for plug-in/rule-engine designs? Is the core system truly stable?

**Overall decision principle**: follow the fitness and simplicity principles — do not split when you can avoid it, split late when you can split late; complexity never disappears, only moves (ESB → microservice infrastructure, within systems → between systems), and whether you can master it after the move depends on team size and automation capability.
