---
name: backend-java
description: 'Java stack binding (backend). Fixed binding on JDK 25 LTS + Spring Boot 4.1.x + Maven Wrapper + OpenAPI Generator (spring) spec-first contracts; covers version anchors and the 3.x→4.x migration breaking points, the exception hierarchy, connection-pool tuning after virtual threads became the default, graceful shutdown, SLF4J structured logging and Micrometer observability, @ServiceConnection test wiring, the hexagonal Maven multi-module layout, and the verification loop. Loaded as an additional constraint for backend-class skills when the project stack declares backend_stack=java; use when the task involves "Java backend implementation / testing / engineering".'
---

# Java Backend Stack Binding (backend-java)

This skill is the binding implementation for the stack declaration `backend_stack=java`: it lands the neutral spec in Java/Spring idioms.
The fixed binding is **JDK 25 LTS + Spring Boot 4.1.x + Maven**, with no alternative path. Where this document conflicts with the neutral spec, the more specific rule here wins; where it is silent, fall back to the neutral spec.

## Toolchain and build

- **Default combination: JDK 25 LTS + Spring Boot 4.1.x + Maven Wrapper (`./mvnw`)**
  - Spring Boot 4.1.x officially requires Java 17 at minimum and supports up to Java 26; Maven 3.6.3+ (version locked by the Wrapper)
  - JDK 25 is the current LTS; Boot 4.1.x OSS support runs through 2027-07
  - **New projects must not pick Spring Boot 3.x (fully EOL since 2026-06) or 4.0.x (OSS support ends 2026-12)**
- Crossing from 3.x to 4.x means handling three breaking points in the same pass: `javax.*` → `jakarta.*`; Jackson 2 → Jackson 3 (package `com.fasterxml.jackson` → `tools.jackson`); `RestTemplate` deprecated (switch to the declarative HTTP client `@HttpExchange`)
- Versions are anchored in the root `pom.xml`, strictly identical locally and in CI; dependency changes get their own commits
- Spring MVC is the default for the web layer; bind configuration with `@ConfigurationProperties` (records preferred — typed and validatable)

## API contract (OpenAPI, spec-first)

- Bound to **OpenAPI Generator (spring)**: the OpenAPI 3 document is the **single source of truth** (`api/openapi.yaml` in the repo); the `openapi-generator-maven-plugin` (generatorName `spring`, `interfaceOnly=true`) generates the API interfaces and DTOs into `target/generated-sources` at build time (generated code is not committed; the spec is)
- Contract-out via code-first annotations such as springdoc is forbidden — annotations may only describe the implementation and must never define the contract in reverse; spec changes go through separate PR review
- Handlers implement the generated interfaces with no deviation; validation annotations (`@Valid` + jakarta validation) align with the spec's constraints
- The spec itself follows the neutral api-design-guide's REST contract and error-code semantics

## Error handling

- The domain layer defines its own exceptions (unchecked) — thrown, never caught locally; **exception types are subdivided by business semantics** — having every business failure share one `BusinessException` + an error-code field is forbidden (it degenerates into exceptions as flow control, losing hierarchy readability and catchability)
- The interface layer translates to error responses in one place, `@ControllerAdvice` (mapping to the neutral backend-development error-code semantics)
- Checked exceptions appear only at external resource boundaries (IO/JDBC wrappers); swallowing exceptions is forbidden, and so is catching, logging, and doing nothing

## Concurrency model

- On Java 21+, Spring Boot 4.1 **enables virtual threads by default**; still **declare `spring.threads.virtual.enabled=true` explicitly** (P-007: make even default behavior explicit to remove version/upgrade ambiguity), and tune the downstream resource pools in step
- **HikariCP `maximumPoolSize` does not carry over the platform-thread-era default of 10**: once virtual threads scale Tomcat concurrency from roughly 200 to tens of thousands, the connection pool becomes the real bottleneck — keep it at **20–30** (official guidance also discourages exceeding 20–30 for a single service) and shorten `connectionTimeout` (e.g. 3s) to fail fast instead of waiting long
- Enable **`spring.datasource.connection-fetch=lazy`** (Boot 4.1's built-in `LazyConnectionDataSourceProxy` integration): acquire the physical connection only when SQL actually executes, so opening a transaction stops occupying one
- Remote calls or long waits inside `@Transactional` methods are forbidden — how long a transaction holds its connection directly decides whether the pool gets exhausted
- Bare `new Thread` and unbounded queues are forbidden; shared mutable state declares its concurrency strategy explicitly
- Graceful shutdown: `server.shutdown=graceful` + `spring.lifecycle.timeout-per-shutdown-phase` (30s default; tune to the longest business request) — after shutdown begins, new requests are rejected immediately while in-flight requests finish; background tasks close with the context
- Outbound HTTP (`RestClient` / `@HttpExchange` clients) is built once and reused, with explicit connect / read timeouts (`ClientHttpRequestFactorySettings`) — the default has none; retry idempotent operations only

## Data access

- Spring Data JPA by default; complex queries / reports use jOOQ or native queries (received as projection DTOs)
- Transaction boundaries are declared on application-layer methods; domain objects carrying transaction semantics are forbidden
- Migrations are versioned with Flyway/Liquibase; `ddl-auto=update` in production is forbidden

## Logging and observability

- Logging goes through the SLF4J API uniformly; **string concatenation is forbidden** — use parameterized logging or the SLF4J 2 Fluent API: `log.atInfo().addKeyValue("orderId", id).log("order placed")`
- Production enables **structured logging** (native in Boot 4.x): `logging.structured.format.console=ecs` (or `logstash` / `gelf`), with no extra encoder dependency
- Request-level correlation: the MDC `requestId` / `traceId` flow into JSON automatically; in `finally`, filters must **`MDC.remove("requestId")`** — `MDC.clear()` is forbidden (it wipes the Micrometer Tracing context)
- Observability centers on the **Micrometer Observation API**, unifying metrics and tracing; Actuator exposes `health` / `metrics` / `prometheus`, with health detail narrowed in production
- Error logs are written once, at `@ControllerAdvice` or an explicit boundary; the domain / application layers only throw and never log repeatedly

## Testing

- JUnit 5 + AssertJ + Mockito; assertions always in AssertJ fluent style
- Spring slice tests (@WebMvcTest / @DataJpaTest) cover the boundaries; integration tests against external dependencies use Testcontainers, wired with **`@ServiceConnection` first** (Boot 4.x) instead of hand-written `@DynamicPropertySource` item-by-item mapping; only containers without built-in service-connection support (e.g. WireMock) fall back (depends on `spring-boot-testcontainers`, test scope)
- Tests depend on neither execution order nor shared mutable state; test data construction follows the neutral test-design / tdd-guide

## Project layout

- Package by business domain (hexagonal): the four layers `domain` / `application` / `infrastructure` / `interfaces` with one-way dependencies; domain carries no Spring annotations or types (aligned with architecture-standard)
- **Maven multi-module mapping**: each of the four layers is its own module; `interfaces` depends on `application`, `application` depends on `domain`, and `infrastructure` implements the ports `application` defines
- The root `pom.xml` only does `dependencyManagement` and module aggregation — no business code

## Verification loop

```bash
./mvnw verify
```

Run it in full for every task (`verify` covers compilation, unit tests, slice tests, integration tests via Failsafe, checkstyle:check, and spotbugs:check — when configured); the correct way to skip tests and the CI increments are in the reference files.

## Reference files

- `reference/verification.md` — CI checks, the correct way to skip tests (the Boot 4.1 breaking point), and structured-logging verification (required reading when the task touches CI / pipelines)
- `reference/idioms.md` — idioms and high-frequency pitfalls, including the virtual-thread pinning breakdown by JDK level (required reading for implementation tasks)
