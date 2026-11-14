---
name: backend-rust
description: 'Rust stack binding (backend). Fixed binding on the Axum 0.8+ web framework, figment typed configuration, error outbound via AppError implementing IntoResponse directly, and openapi-to-rust spec-first contracts; covers the toolchain and workspace lints, tracing observability, tokio concurrency and graceful shutdown, sqlx connection pooling and offline mode, oneshot integration testing, and the Axum standard layout with its verification loop. Loaded as an additional constraint for backend-class skills when the project stack declares backend_stack=rust; use when the task involves "Rust backend implementation / testing / engineering".'
---

# Rust Backend Stack Binding (backend-rust)

This skill is the binding implementation for the stack declaration `backend_stack=rust`: it lands the neutral spec in Rust idioms.
The fixed binding is **Axum + figment + sqlx** and no alternative path is offered. Where this document conflicts with the neutral spec, the more specific rule here wins;
where it does not cover a topic, fall back to the neutral spec.

## Toolchain and build

- Rust stable, **edition 2024 by default**; the toolchain version is anchored by the repo's `rust-toolchain.toml`, identical locally and in CI
- cargo workspace; the lint policy is configured centrally in `[workspace.lints]`, and each crate opts in with `[lints] workspace = true`:
  - `[workspace.lints.clippy]`: `unwrap_used` / `expect_used` both `"deny"` (test exemption: in `clippy.toml` set `allow-unwrap-in-tests = true`)
  - `[workspace.lints.rust]`: business crates `unsafe_code = "forbid"`; public crates `missing_docs = "warn"`
- rustfmt and clippy `-D warnings` join the verification loop; run `cargo audit` when dependencies change
- Dependency versions are anchored centrally in `[workspace.dependencies]` and referenced from each crate with `workspace = true`; cross-crate drift is forbidden. Cargo.lock is committed and builds always run `--locked`
- Enable feature flags minimally, on demand; figment's format/provider features (`toml`/`env`/`test`) are declared once in the workspace — per-crate drift is forbidden

## Error handling

- Library crates: define error enums with thiserror; application/bin crates: the `AppError` enum **implements `IntoResponse` directly**, with optional anyhow for wrapping internal context
- Inside `IntoResponse`: 4xx responses return actionable information; 5xx responses return a generic message only, and the internal error chain is **logged once** with `tracing::error!`
- Logging is consolidated at a single point — the `IntoResponse` implementation or the error middleware; **repeated per-`?` logging via `map_err` inside handlers is forbidden**
- Custom extractors unify their `Rejection` as `AppError`; Axum's default rejection text must never reach the client
- Propagate with `?`; handlers return `Result<Json<T>, AppError>`
- `unwrap()` / `expect()` are forbidden on the request path; allowed only during startup and in tests, with a comment stating the reason

## Web framework (Axum)

- The default and only binding is **Axum 0.8+** (maintained by the tokio team, built on Hyper + Tower); no Actix-web alternative is introduced
- The **router-centric, macro-free** model: thin handlers + Tower middleware + typed errors; handler signature errors surface at compile time instead of panicking at runtime
- Axum 0.8 highlights: path parameters are written `{id}`, not `:id`; `State` is imported from `axum::extract`; `axum::serve` replaces the old `Server`
- Serialization with serde; logging with tracing + tracing-subscriber; configuration with figment (next section)
- Middleware ordering: trace → timeout → body-limit → auth → handler; timeouts via tower-http / `tokio::time::timeout` — unbounded waits are forbidden
- Return typed errors from `.fallback()` for 404s; leaking Axum's default plain-text 404/422 to the client is forbidden

## API contract (OpenAPI, spec-first)

- Bound to **openapi-to-rust**: the OpenAPI 3 document is the **single source of truth** (`api/openapi.yaml` in the repo) and generates the serde types plus the
  axum server skeleton; generated code is committed — code-first annotation paths such as utoipa / aide are forbidden
- Spec changes go through separate PR review; CI verifies a zero diff after regeneration (the build fails whenever generated code and spec drift apart)
- Handlers implement exactly the interfaces the generated contract defines — no deviation; `deny_unknown_fields` aligns with the spec's `additionalProperties: false`
- The spec itself follows the neutral api-design-guide's REST contract and error-code semantics

## Configuration (figment)

- Standardize on **figment**: priority CLI > Env > config file > defaults; after merging, `extract()` into serde-typed structs
- Load once at startup; on failure, report **with the field name** and exit; scattered `std::env::var` reads in business code are forbidden
- Sensitive values use `secrecy::SecretString` (`Debug` prints `[REDACTED]`); putting secrets into plain `String`s or logs is forbidden
- Non-secret operational parameters get defaults; cross-field validation runs after deserialization
- Profiles (dev / staging / prod) are layered in figment and merged by environment selection; writing environment-specific if branches inside business code is forbidden

## Observability (tracing)

- tracing-subscriber is wired up once at startup: production JSON output + `EnvFilter` levels (driven by `RUST_LOG`, default info);
  scattered println! / log calls are forbidden
- `#[instrument]` covers the service / repository boundaries, with span fields carrying business identifiers (never sensitive values)
- TraceLayer (tower-http) sits at the head of the middleware chain: extract/inject `traceparent`, aligned with the gateway's trace propagation; request_id / trace_id appear in every log entry
- `/metrics` exposes process and business metrics via a prometheus exporter; the health endpoint `/healthz` skips access logging

## Concurrency model

- tokio async runtime; blocking calls go into `spawn_blocking` (uncancellable — they run to completion; the blocking pool is finite by default, pair long blocking work with a `Semaphore`); blocking runtime threads is forbidden
- Prefer channel message passing for shared state; use `Arc<Mutex>` only for short critical sections, and critical sections must never span an `.await`
- Graceful shutdown: `axum::serve(...).with_graceful_shutdown(shutdown_signal())`, listening on SIGTERM / SIGINT and waiting for in-flight requests to finish; abort after the shutdown timeout
- Manage background tasks with `JoinSet` and join them with a timeout at shutdown; spawning and discarding the JoinHandle is forbidden
- `select!` branches must use cancellation-safe futures (`sleep`, `mpsc::recv`, `TcpStream::read`, etc.); `read_exact` and buffered writes are not cancellation-safe — when unsure, use a dedicated task plus a channel handoff
- Build the outbound HTTP client (reqwest) once and reuse it; set explicit `connect_timeout` / `read_timeout` and an overall timeout — the default is no timeout, exactly the unbounded wait forbidden on the request path
- No remote calls or long waits inside a transaction; transaction boundaries are managed explicitly at the service layer

## Data access

- sqlx by default (compile-time SQL checking); migrations via sqlx migrate; the pool is reused and placed in `AppState` — building a pool per request is forbidden
- Offline mode: after migration or query changes run `cargo sqlx prepare --workspace` and commit `.sqlx/`; CI sets `SQLX_OFFLINE=true`
- Pool parameters are configured explicitly and live in the config file: `PgPoolOptions`'s `max_connections` (default 10; size it against the target concurrency and the PG `max_connections`
  budget), `acquire_timeout` (the 30s default is too long — 3s for fast failure is recommended), `idle_timeout` / `max_lifetime`; hardcoding is forbidden

## Testing

- Unit tests live beside the code in `#[cfg(test)]`; integration tests in the `tests/` directory; exported APIs get doc-tests
- Axum integration tests use `tower::ServiceExt::oneshot` to send requests directly at the Router returned by `app()`, with no real port; every test gets its own `AppState`
- Async tests use `#[tokio::test]`; integration tests against external dependencies use testcontainers-rs, external HTTP uses wiremock
- Test data construction and case design follow the neutral test-design / tdd-guide

## Project layout

- A workspace with crates split by business domain / bounded context; lib and bin separated; common types consolidated in a common crate
- Recommended structure for a service crate: `main.rs` (tokio::main + serve + graceful shutdown), `app.rs` (`fn app(state: AppState) -> Router`, reusable by tests), `config.rs`, `state.rs`, `error.rs`, `handlers/` (modules per domain), `services/`, `repositories/`, `models/` (DTOs)
- Start with a single crate and modules; split into separate crates only when independent release or compile-isolation needs arise — avoid splitting for its own sake

## Verification loop

```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings && cargo test --workspace --locked
```

Run it in full for every task; this section is the single authority for the local loop — CI incremental checks and the integration-test paradigm are in the reference files.

## Reference files

- `reference/verification.md` — CI checks, sqlx offline and testcontainers prerequisites, and the oneshot integration-test paradigm (required reading when the task touches CI / pipelines or integration testing)
- `reference/idioms.md` — idioms and high-frequency pitfalls, including the Axum specifics (required reading for implementation tasks)
