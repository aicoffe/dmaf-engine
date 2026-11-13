# Rust Backend Idioms and High-Frequency Pitfalls

## Error handling

- thiserror for library-crate error enums; the application-level `AppError: IntoResponse` is the single consolidation point for error outbound — the error chain is logged once here, and repeated per-layer logging is forbidden
- "Theoretically unreachable" cases such as lock poisoning or integer overflow must still be handled explicitly, or commented with a proof of unreachability and then `expect` (startup / tests only)
- derive macros such as `axum_thiserror_tracing` can reduce boilerplate, but they are not hard-bound (low download counts, not a de facto standard)

## Async and concurrency

- No blocking on runtime threads: synchronous IO, long CPU loops, or holding a std-library lock across `.await` — switch to `spawn_blocking` or async primitives instead
- Handles from `tokio::spawn` go into a `JoinSet` or are bound to a lifetime; long-running tasks listen for cancellation — fire-and-forget is forbidden
- Never hold an `Arc<Mutex>` lock across `.await` inside a critical section (a source of deadlock and blocking); share across await points via channels or per-request clones instead
- Put only cancellation-safe branches in `select!`; a cancelled `read_exact` / buffered write loses data — use a dedicated task plus a channel handoff instead

## Axum pitfalls

- **Body-consuming extractors must come last**: no body extractor may follow `Json<T>`; the order of `FromRequestParts` extractors such as `Path` / `Query` / `State` does not matter, but `Json` must be last
- **Add `#[serde(deny_unknown_fields)]` to input DTOs** before validating; directly `Serialize`ing a `FromRow` database model as a response is forbidden
- **Hold state with `AppState` + `.with_state()`**; storing the database pool in `static` / `lazy_static` is forbidden; when finer granularity is needed, use `FromRef` so handlers take only subsets such as `State<PgPool>`
- **Attach auth middleware with `route_layer`** so it applies only to matched routes; unknown paths stay 404 rather than 401
- **Return typed errors from the 404 fallback** so Axum's default plain-text 404/422 never leaks
- **Path parameters use `{name}`** (0.8+), not `:name`; `State` is imported from `axum::extract`
- **Keep error logging at one place — `IntoResponse` or the error middleware**; repeated `map_err` logging per handler is forbidden

## API and data

- The outbound interface layer uses DTOs (`models/`); serializing database models directly as responses is forbidden
- sqlx queries are always parameterized; string-concatenated SQL is forbidden; migration files are append-only; sync `.sqlx/` after query changes
- Configuration is assembled once by figment into typed structs; business code reading environment variables directly is forbidden; sensitive values go through `SecretString`
- spec-first: handlers implement the generated contract's interfaces; field additions, changes, and removals edit `api/openapi.yaml` first and then regenerate — hand-editing generated code is forbidden
- Build `reqwest::Client` once and reuse it (per-request construction throws away connection pooling); the default has no timeout — outbound calls must configure it explicitly

## Engineering habits

- Rampant `Clone` is usually an ownership-design problem: think borrows and lifetimes first, clones second
- Public crate APIs carry doc comments and examples (`missing_docs = "warn"` will remind you); the external API surface is consolidated via `pub use`
- Dependency upgrades and feature changes get their own commits — never smuggled inside feature PRs
