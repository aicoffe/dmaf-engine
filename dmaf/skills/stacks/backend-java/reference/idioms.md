# Java Backend Idioms and High-Frequency Pitfalls

## Error handling

- Organize the exception hierarchy per domain (base classes + semantic subclasses) and subdivide exception types by business semantics; translation happens in one place, @ControllerAdvice — each Controller assembling responses with its own try-catch is forbidden
- "One BusinessException to rule them all + an error-code field to tell them apart" is forbidden — it is equivalent to exceptions as flow control
- Exception messages carry business context and key identifiers (order number / ID); blanket catch(Exception) and empty catch blocks are forbidden
- Resources always go through try-with-resources; never throw from finally

## Virtual threads and pinning (layered by JDK version)

- **JDK 24+ (including 25 LTS)**: `synchronized` no longer causes pinning (JEP 491); a virtual thread blocking inside a `synchronized` block releases its carrier thread — **rewriting business code to `ReentrantLock` for this reason is unnecessary**
- What still needs attention on JDK 24+ is **`synchronized` / native blocking inside third-party libraries** (especially older JDBC drivers and DBCP) — upgrading to the latest versions mitigates it
- **JDK 21–23**: blocking inside a `synchronized` block causes pinning; avoid long `synchronized` critical sections on virtual-thread paths and switch to `ReentrantLock`

## Spring and concurrency

- After virtual threads become the default, check the connection pool first: HikariCP `maximumPoolSize` 20–30, `connectionTimeout` 3s, `connection-fetch=lazy` (see the SKILL's "Concurrency model")
- @Transactional marks application-layer methods only; private methods / internal self-invocation take no effect — same-class self-invocation bypassing the proxy is the classic trap; remote calls and long waits inside transactional methods are forbidden
- Beans are singletons by default: no request-scoped data in member fields; ThreadLocal must be removed after use
- @Async needs an explicit thread pool; the default SimpleAsyncTaskExecutor is forbidden

## API and data

- The outbound interface layer uses DTOs (records preferred); serializing entities directly is forbidden — lazily loaded fields blow up the moment they cross the serialization boundary, so isolate with DTOs
- JPA: writes go through managed entities or explicit JPQL; complex read-only projections use interface projection / jOOQ; depending on open-in-view is forbidden
- Migrations are versioned with Flyway/Liquibase; ddl-auto=update in production is forbidden
- Remote calls use the declarative HTTP client (`@HttpExchange`); RestTemplate is deprecated and forbidden in new code
- The JPA N+1 is the most classic performance trap: list pages trigger lazy loading in a loop — use `@EntityGraph` / fetch join for collection associations and configure `default_batch_fetch_size` globally; pagination stacked on fetch join degrades into in-memory paging, so prefer `@EntityGraph`
- spec-first: handlers implement the generated API interfaces; field changes edit `api/openapi.yaml` first, then build — hand-editing generated code is forbidden
- The outbound `RestClient` has no timeout by default — set it explicitly at construction; creating a new client instance on every hot-path call is forbidden

## Engineering habits

- Prefer records / immutable objects; mutable shared state must come with a stated concurrency strategy
- Lombok use is confined to data carriers (@Data/@Builder); behavior classes do not abuse it
- Jackson 3 lives under the `tools.jackson` package (Boot 4.x) — mixing in Jackson 2 `com.fasterxml.jackson` annotations / classes is the most common compile error in the 3.x→4.x migration
- Dependency upgrades and configuration changes get their own commits — never smuggled inside feature PRs
