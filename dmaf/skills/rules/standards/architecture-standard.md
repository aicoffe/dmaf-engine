---
name: architecture-standard
level: L1
domain: architecture
description: Load when system architecture design, module decomposition, technology selection, service boundaries, integration patterns, or architecture evolution is involved. Governs all architecture-decision-related work.
owner: Architecture Group
based_on: [organization-principles, security-standard, data-standard]
enforcement: mandatory
version: v1.1.0
review_cycle: Once every six months
---

# Architecture Domain Standard

## 1. Architecture Styles

### 1.1 Allowed Architecture Styles

| Style | Applicable scenarios | Description |
| :--- | :--- | :--- |
| **Monolith** | Small teams, simple business, rapid validation | Single-process deployment, organized into modules |
| **Modular Monolith** | Medium scale, clear boundaries, evolving teams | Single-process deployment, strict module boundaries |
| **Microservices** | Large teams, multiple business domains, independent evolution | Multi-process deployment, independent services |
| **Event-Driven** | Async decoupling, high throughput | Asynchronous architecture based on message queues |
| **Serverless** | Burst traffic, low-frequency tasks | Functions as a service |

### 1.2 Architecture Style Selection Principles

| Criterion | Monolith | Microservices |
| :--- | :--- | :--- |
| **Team size** | < 10 people | > 20 people |
| **Business complexity** | Single business domain | Multiple business domains |
| **Deployment frequency** | Low (weekly/monthly) | High (daily/hourly) |
| **Tech stack** | Unified | Polyglot |
| **Independent scaling needs** | None | Yes |

**Default recommendation**:
- New projects start with a **modular monolith** by default
- Split into microservices only when "independent deployment" and "independent scaling" become real pain points
- Starting with microservices from day one is forbidden (unless the team has mature experience)

### 1.3 Architecture Style Prohibitions

- ❌ "Distributed monolith" is forbidden (services are split, but strongly coupled with overly long synchronous call chains)
- ❌ "Nanoservices" are forbidden (service granularity too fine — one service doing only a single CRUD)
- ❌ Microservice splitting without clear boundaries is forbidden

---

## 2. Layered Architecture

### 2.1 Standard Layers

```
┌─────────────────────────────────────────────────────────────┐
│ Interface Layer (Interface / Controller)                    │
│   Responsibilities: protocol adaptation, parameter          │
│   validation, routing                                       │
│   Forbidden: business logic, direct database access         │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Application Layer (Application / UseCase)                   │
│   Responsibilities: use-case orchestration, transaction     │
│   boundaries, cross-domain coordination                     │
│   Forbidden: domain logic, direct database access           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Domain Layer (Domain)                                       │
│   Responsibilities: business rules, domain model,           │
│   aggregate roots                                           │
│   Forbidden: infrastructure dependencies, framework         │
│   annotations                                               │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Infrastructure Layer (Infrastructure)                       │
│   Responsibilities: database, cache, messaging, external    │
│   services                                                  │
│   Forbidden: business logic                                 │
└─────────────────────────────────────────────────────────────┘
```

> Note: "parameter validation" in the interface layer means defensive format validation; security checks such as authentication, anti-replay, and rate limiting are performed uniformly by the API gateway.

### 2.2 Layering Constraints

| Rule | Description |
| :--- | :--- |
| **Dependency direction** | Upper layers depend on lower layers; lower layers do not depend on upper layers (dependency inversion) |
| **Cross-layer calls** | Cross-layer calls are forbidden (e.g., a Controller calling the DAO directly) |
| **Domain layer purity** | The domain layer depends on no frameworks and no infrastructure |
| **Thin interface layer** | The Controller only adapts protocols; business logic lives in the application/domain layer |

### 2.3 Prohibitions

- ❌ Business logic in the Controller is forbidden
- ❌ Controllers calling the DAO / Repository directly are forbidden
- ❌ The domain layer depending on application framework annotations and types is forbidden (e.g., Spring's `@Service`/`@Autowired`, web frameworks' request context types)
- ❌ Business decisions in the infrastructure layer are forbidden
- ❌ Circular dependencies are forbidden

---

## 3. Module Decomposition

### 3.1 Module Decomposition Principles

| Principle | Description |
| :--- | :--- |
| **High cohesion** | Elements within a module are tightly related |
| **Low coupling** | Few inter-module dependencies, narrow interfaces |
| **Single responsibility** | One module owns one business capability |
| **Clear boundaries** | Module boundaries align with business boundaries |
| **Independent evolvability** | Modules can be modified, tested, and deployed independently |

### 3.2 Module Decomposition Method

```
1. Identify bounded contexts
   - Divide by business domain (DDD)
   - Each context has its own domain model

2. Identify aggregates
   - The aggregate root is the transaction boundary
   - Strongly consistent within an aggregate, eventually consistent across aggregates

3. Define module boundaries
   - One module per bounded context
   - Modules communicate through interfaces

4. Define module dependencies
   - Dependency direction: one-way
   - Circular dependencies are forbidden
```

### 3.3 Module Communication

| Mode | Applicable scenario | Requirement |
| :--- | :--- | :--- |
| **In-process calls** | Modular monolith | Through interfaces; depending on implementations directly is forbidden |
| **Synchronous HTTP/RPC** | Microservices | Must have timeout + retry + circuit breaking |
| **Async messaging** | Event-driven | Must be idempotent + reliably delivered |
| **Shared database** | ❌ Forbidden | Sharing a database between modules is forbidden |

### 3.4 Module Boundary Checks

- **Static checks**: check module boundaries with dependency analysis tools (e.g., ArchUnit)
- **CI gate**: inter-module dependency violations must block the merge
- **Documentation**: module boundaries must be recorded in the architecture documentation

---

## 4. Service Boundaries (Microservice Scenarios)

### 4.1 Service Decomposition Principles

| Principle | Description |
| :--- | :--- |
| **Decompose by business capability** | Each service corresponds to one business capability |
| **Decompose by bounded context** | Service boundaries align with bounded contexts |
| **Independent deployment** | Services can be deployed and scaled independently |
| **Independent data** | Each service has its own database |
| **Team autonomy** | Each service is owned by an independent team |

### 4.2 Service Decomposition Prohibitions

- ❌ Splitting by technical layer is forbidden (e.g., Controller services, Service services)
- ❌ Splitting by entity is forbidden (e.g., a User service, an Order service, a Product service)
- ❌ Sharing a database between services is forbidden
- ❌ Strong coupling between services is forbidden (synchronous call chains > 3 levels)
- ❌ Circular dependencies between services are forbidden

### 4.3 Inter-Service Communication

| Mode | Applicable scenario | Requirement |
| :--- | :--- | :--- |
| **Synchronous HTTP/REST** | Queries, simple commands | Connect timeout 3s, read timeout 10s, 3 retries, circuit breaking |
| **Synchronous gRPC** | Internal high-performance calls | Connect timeout 3s, read timeout 10s, 3 retries, circuit breaking |
| **Async messaging** | Event notification, decoupling | Idempotent, reliable delivery, dead-letter queue |
| **Event sourcing** | Auditing, state rebuilding | Events are immutable, version-managed |

### 4.4 Service Contracts

- **External interfaces**: defined with OpenAPI 3.0
- **Internal interfaces**: defined with Proto (gRPC)
- **Events**: defined with AsyncAPI
- **Contract tests**: interfaces between services must have contract tests
- **Versioning**: interface changes follow the backward compatibility principle

---

## 5. Technology Selection

### 5.1 Technology Selection Principles

| Principle | Description |
| :--- | :--- |
| **Maturity first** | Prefer mature technologies with active communities |
| **Team familiarity** | Prefer technologies the team knows |
| **Organization standards** | Prefer organization-approved technologies |
| **Minimal dependencies** | Minimize third-party dependencies |
| **Replaceability** | Key dependencies must be replaceable |

### 5.2 Organization Standard Tech Stack

- New projects default to selections within the organization-supported tech stack
- Using technologies outside the supported set must go through the 5.3 introduction process and record an ADR
- The supported tech stack set is maintained by the Architecture Group

### 5.3 New Technology Introduction Process

```
1. Proposal (ADR draft)
   - Background, problem
   - Comparison of alternatives
   - Recommended option and rationale

2. Evaluation
   - Technology maturity
   - Team capability
   - Community activity
   - Long-term maintenance cost

3. Review
   - Architecture Group review
   - Security team review
   - Operations team review

4. Pilot
   - Pilot in non-core business
   - Validate feasibility

5. Rollout
   - Become an organization standard
   - Update the tech stack inventory
```

### 5.4 Prohibitions

- ❌ Introducing unapproved tech stacks is forbidden
- ❌ Introducing unmaintained open source projects is forbidden
- ❌ Introducing open source projects with incompatible licenses is forbidden
- ❌ Introducing versions with known Critical vulnerabilities is forbidden
- ❌ Introducing technologies that conflict with the existing tech stack is forbidden

---

## 6. Integration Patterns

### 6.1 Integration Pattern Selection

| Pattern | Applicable scenario | Pros | Cons |
| :--- | :--- | :--- | :--- |
| **Synchronous HTTP** | Real-time queries | Simple, intuitive | Coupling, cascading failures |
| **Synchronous gRPC** | Internal high performance | Efficient, strongly typed | Coupling, cascading failures |
| **Async messaging** | Decoupling, event notification | Decoupled, peak shaving | Eventual consistency, higher complexity |
| **Event sourcing** | Auditing, state rebuilding | Complete history | Higher complexity |
| **CQRS** | Read/write separation | Performance optimization | Higher complexity |

### 6.2 Synchronous Call Conventions

| Convention | Requirement |
| :--- | :--- |
| **Timeout** | Connect timeout 3s, read timeout 10s |
| **Retry** | Idempotent operations only, at most 3 times, exponential backoff |
| **Circuit breaking** | Triggered at error rate > 50%, half-open recovery |
| **Degradation** | Must have a degradation plan |
| **Rate limiting** | Client-side rate limiting to protect downstream |
| **Call chain depth** | ≤ 3 levels (beyond that, go asynchronous) |

### 6.3 Async Messaging Conventions

| Convention | Requirement |
| :--- | :--- |
| **Message format** | JSON + schema validation |
| **Message versioning** | Must carry a version number |
| **Idempotency** | Consumers must be idempotent |
| **Reliable delivery** | Persistence + acknowledgment mechanism |
| **Dead-letter queue** | Failed consumption enters the dead-letter queue |
| **Message ordering** | Use a single partition when ordering is required |
| **Message size** | ≤ 1MB (use object storage beyond that) |

### 6.4 Distributed Transactions

| Pattern | Applicable scenario | Requirement |
| :--- | :--- | :--- |
| **Saga** | Long transactions, cross-service | Compensation transactions must be idempotent |
| **TCC** | Strong consistency, short transactions | Try-Confirm-Cancel, three phases |
| **Local message table** | Eventual consistency | Message table + scheduled compensation |
| **Transactional messages** | Eventual consistency | Half message + backward check |

**Principles**:
- Prefer **eventual consistency**; avoid strongly consistent distributed transactions
- Cross-service transactions must have compensation mechanisms
- Compensation transactions must be idempotent
- Sharing a database across services is forbidden (avoid 2PC)

---

## 7. Non-Functional Architecture

### 7.1 Performance Architecture

| Requirement | Description |
| :--- | :--- |
| **Performance budget** | Every interface must have a performance budget (P95 latency) |
| **Caching strategy** | Frequently-read, rarely-written data must be cached |
| **Asynchronization** | Time-consuming operations must be made asynchronous |
| **Batching** | Batch operations must merge requests |
| **Pagination** | List queries must paginate |
| **Indexes** | Query fields must be indexed |
| **N+1 queries** | N+1 queries are forbidden |

### 7.2 Availability Architecture

| Requirement | Description |
| :--- | :--- |
| **Redundancy** | Critical services must be deployed with multiple instances |
| **Circuit breaking** | External dependencies must be circuit-broken |
| **Degradation** | Must have a degradation plan |
| **Rate limiting** | Must have rate limit protection |
| **Health checks** | Every service must have a health check endpoint |
| **Fault isolation** | A failing service must be isolated and must not affect other services |

### 7.3 Scalability Architecture

| Requirement | Description |
| :--- | :--- |
| **Statelessness** | Services must be stateless; state is stored externally |
| **Horizontal scaling** | Services must support horizontal scaling |
| **Sharding** | Large data volumes must be sharded |
| **Asynchrony** | Time-consuming operations must be made asynchronous |

### 7.4 Security Architecture

- See `rules/standards/security-standard`
- Architecture design must have security built in (Security by Design)
- All service-to-service communication must be authenticated
- All external entry points must go through the gateway

---

## 8. Architecture Evolution

### 8.1 Evolution Principles

| Principle | Description |
| :--- | :--- |
| **Incremental** | Architecture evolution must be incremental; no big-bang refactoring |
| **Rollback-able** | Every evolution step must be rollback-able |
| **Compatibility** | Backward compatibility must be maintained throughout evolution |
| **Observable** | The evolution process must be observable |
| **Canary** | Evolution must be validated via canary releases |

### 8.2 Evolution Patterns

| Pattern | Applicable scenario |
| :--- | :--- |
| **Strangler Fig** | Monolith → microservices |
| **Branch by Abstraction** | Replacing technical implementations |
| **Parallel Run** | Old and new systems running in parallel for validation |
| **Feature Flag** | Gradual/canary rollout |

---

## 9. Architecture Governance

### 9.1 Architecture Reviews

| Review type | Timing | Participants | Output |
| :--- | :--- | :--- | :--- |
| **Architecture design review** | New projects / major changes | Architecture Group + relevant teams | Review record |
| **Technology selection review** | Introducing new technology | Architecture Group + Security + Operations | ADR |
| **Architecture compliance review** | Periodic | Architecture Group | Compliance report |

- Architecture decisions must be approved by humans

### 9.2 Architecture Checks

> The tools in the table below are organization default examples and may be replaced with equivalent tools.

| Check item | Tool | Frequency |
| :--- | :--- | :--- |
| **Module boundaries** | ArchUnit | Every PR |
| **Circular dependencies** | ArchUnit / jdeps | Every PR |
| **Layering violations** | ArchUnit | Every PR |
| **Dependency vulnerabilities** | Dependabot / Snyk | Every PR |
| **Tech stack compliance** | Custom scripts | Every PR |

### 9.3 Architecture Metrics

| Metric | Description | Target |
| :--- | :--- | :--- |
| **Module coupling** | Number of inter-module dependencies | Lower is better |
| **Circular dependency count** | Number of circular dependencies | 0 |
| **Layering violation count** | Number of cross-layer calls | 0 |
| **Technical debt** | Number of technical debt items | Decreasing step by step |
| **Architecture decision coverage** | Share of decisions with an ADR | 100% |

---

## 10. Architecture Documentation

### 10.1 Architecture Document Types

> The location column lists conventional relative paths (relative to the architecture documentation root).

| Document | Content | Location |
| :--- | :--- | :--- |
| **Architecture overview** | System context, module decomposition, tech stack | `architecture/overview.md` |
| **Architecture 5 views** | Logical, development, process, physical, scenario views | `architecture/` |
| **ADR** | Architecture decision records | `architecture/adr/` |
| **API contracts** | OpenAPI / Proto | `architecture/api/` |
| **Data model** | ER diagrams, DDL | `architecture/data-model/` |

### 10.2 Architecture Documentation Requirements

- **Version control**: all architecture documents must be version-controlled
- **Explicit decisions**: architecture decisions must be recorded as ADRs
- **In sync with code**: architecture documents must be updated in sync with code
- **Executable**: architecture constraints must be checkable by tools
- **AI-readable**: architecture documents must be AI-friendly (Markdown)

---

## 11. Conflict Adjudication with Other Domains

### 11.1 With Security

- **Adjudication basis**: P-001 (security first)
- **Adjudication process**: Security Owner + Architect adjudicate jointly
- **Example**: microservice splitting vs security boundaries → security first; service boundaries are security boundaries

### 11.2 With Performance

- **Adjudication basis**: P-009 (user experience is part of the feature)
- **Adjudication process**: Architect + SRE adjudicate jointly
- **Example**: modularity vs performance → modularity first; performance issues are solved with caching/asynchrony

### 11.3 With Delivery Schedule

- **Adjudication basis**: P-008 (maintainability over short-term delivery)
- **Adjudication process**: Tech Lead + Product Manager adjudicate jointly
- **Forbidden**: breaking the architecture on "schedule pressure" grounds is forbidden

### 11.4 With Backward Compatibility

- **Adjudication basis**: P-005 (backward compatibility first)
- **Adjudication process**: the Architecture Group approves breaking changes
- **Example**: architectural refactoring vs compatibility → incremental evolution while keeping compatibility
