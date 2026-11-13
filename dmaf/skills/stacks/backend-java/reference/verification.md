# Java Verification Loop and CI Checks

## Local loop (every task)

The commands follow `SKILL.md`'s "Verification loop" as the single authority:

```bash
./mvnw verify
```

`verify` should cover: compilation, unit tests, slice tests, integration tests (Failsafe), `checkstyle:check`, and `spotbugs:check` (when configured). This file only adds the layered execution model, the correct way to skip tests, and the CI increments.

## The correct way to skip tests (Spring Boot 4.1+)

- **Using `-DskipTests`** to skip tests or AOT processing **is forbidden** — the Boot 4.1 Maven plugin no longer honors this property
- When skipping is needed, use: `./mvnw verify -Dmaven.test.skip=true`
- Release artifact builds may skip tests, but the reason must be noted separately; merging such changes into feature branches is forbidden

## Layered execution

- Unit tests (no Spring context): pure JUnit 5 + Mockito, must stay in the seconds range
- Slice tests (@WebMvcTest / @DataJpaTest): wire only the relevant beans, an order of magnitude faster than the full context
- @SpringBootTest and Testcontainers integration tests: in `*IT.java` (Failsafe plugin), must run before committing; wire with `@ServiceConnection` first, falling back to `@DynamicPropertySource` only for containers without support

## CI checks (added on top of the local loop)

- `./mvnw -q -Dmaven.test.skip=true package` builds the artifact (note: not `-DskipTests`)
- `mvn org.owasp:dependency-check-maven:check` (dependency CVE scanning)
- `./mvnw -q spotbugs:check pmd:check checkstyle:check` (when not bound to verify)
- The `spring-boot-maven-plugin` version strictly matches the parent POM / Boot version
- The JDK version matches the root `pom.xml`'s `java.version`; Maven 3.6.3+ (locked by the Wrapper)
- Structured-logging verification: at least one profile enables `logging.structured.format.console=ecs`, and CI logs are JSON-parseable
- API contract: `api/openapi.yaml` is validated via `openapi-generator-cli validate`; spec changes get a separate PR, and the build validates spec → interface generation on every run

## Prohibitions

- `-DskipTests` is forbidden (broken in Boot 4.1), as is any "skipping tests and merging into a feature branch" (release builds excepted, with the reason noted separately)
- @SpringBootTest abuse is forbidden: slice when a slice suffices, plain unit-test when a plain unit test suffices
- Silently skipping failing cases in surefire/failsafe configuration is forbidden
- CI implicitly switching JDK / Maven versions is forbidden
