# Rust Verification Loop and CI Checks

## Local loop (every task)

The commands follow `SKILL.md`'s "Verification loop" as the single authority (`fmt --all` / `clippy --workspace --all-targets --all-features --locked` /
`test --workspace --locked`); this file does not duplicate that list and only adds the layered execution model and CI increments.

## Layered execution

- Unit tests run in full with `cargo test --workspace` (seconds); during development narrow with `cargo test -p <crate> <module>`
- Axum integration tests (`tests/`): `tower::ServiceExt::oneshot` sends requests at the `app()` Router, occupying no port; every test gets an independent `AppState`
- Cases using testcontainers are marked `#[ignore]` and **must be run before committing** via `cargo test --workspace -- --include-ignored` (requires Docker)
- doc-tests: `cargo test --doc`; documentation examples for exported APIs must compile and run

## Axum integration-test paradigm

- `app.oneshot(Request::get("/x").body(Body::empty()).unwrap()).await` verifies the whole routing path (middleware chain and error outbound included)
- Every test gets an independent `AppState` (its own test database / testcontainers); shared mutable state is forbidden
- External HTTP dependencies use wiremock; real third-party calls inside handler tests are forbidden
- Assert the response status code plus the typed error body; cover both outbound paths — `.fallback()` 404 and custom rejection → `AppError`

## CI checks (added on top of the local loop)

- `cargo build --release --workspace --locked` passes (the release artifact builds)
- `cargo audit` (dependency vulnerabilities); repos with cargo-deny add `cargo deny check` (licenses / duplicate dependencies)
- sqlx projects: after migration or query changes run `cargo sqlx prepare --workspace` and commit `.sqlx/`; CI sets `SQLX_OFFLINE=true` and the build fails when `.sqlx/` is missing
- testcontainers cases need Docker; CI without Docker must state explicitly "integration tests not covered" — silent skipping is forbidden
- figment's `toml` / `env` / `test` features are enabled uniformly at the workspace level (`--all-features` verifies consistency)
- The toolchain version matches `rust-toolchain.toml`; `--locked` guarantees Cargo.lock is never rewritten implicitly
- Coverage (optional): `cargo llvm-cov --workspace`, with the threshold enforced as a CI blocker
- API contract: `api/openapi.yaml` changes get a separate PR; after openapi-to-rust regeneration, `git diff --exit-code` verifies a zero diff

## Prohibitions

- `#![allow(clippy::all)]` or broad allow blocks bypassing lints are forbidden; when an exemption is truly needed, use item-level `#[allow(clippy::specific_lint)]` with a commented reason; `unwrap_used` exemptions go only through the `clippy.toml` test switch
- CI skipping fmt / clippy ("merge first, fix later") is forbidden
- CI implicitly switching toolchain versions, or updating dependencies without `--locked`, is forbidden
