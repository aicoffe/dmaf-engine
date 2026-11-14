---
name: backend-go
description: 'Go stack binding (backend). Fixed binding on the Chi v5 web framework, envconfig typed configuration, log/slog structured logging, and oapi-codegen spec-first contracts; covers the toolchain and golangci-lint v2, single-point error outbound mapping, the four http.Server timeouts and graceful shutdown, the sql.DB connection pool with sqlc/golang-migrate, the httptest testing paradigm, project layout, and the verification loop. Loaded as an additional constraint for backend-class skills when the project stack declares backend_stack=go; use when the task involves "Go backend implementation / testing / engineering".'
---

# Go Backend Stack Binding (backend-go)

This skill is the binding implementation for the stack declaration `backend_stack=go`: it lands the neutral spec in Go idioms.
The fixed binding is **Chi + envconfig + slog** and no alternative path is offered. Where this document conflicts with the neutral spec, the more specific rule here wins;
where it does not cover a topic, fall back to the neutral spec.

## Toolchain and build

- Go 1.22+, go modules; the Go version is anchored by the go directive in go.mod, identical locally and in CI
- gofmt (`gofmt -l .` must produce no output) and `go vet` are mandatory; static checks go through golangci-lint uniformly, with its config committed
- golangci-lint uses the **v2 config format** uniformly (`.golangci.yml` first line `version: "2"`): after `default: none`,
  the core linters are enabled explicitly — `errcheck`, `govet`, `staticcheck`, `ineffassign`, `unused`,
  `misspell`, `revive`, `gocritic`, `errname`, `errorlint`, `noctx`, `bodyclose`,
  `copyloopvar`, `nolintlint`; `errcheck` enables `check-blank: true`, with `_test.go` exempt
- `//nolint` must carry the specific linter name and a reason (enforced by `nolintlint`)
- Dependency vulnerability scanning uses `govulncheck`: **reachable vulnerabilities fail CI, unreachable ones are logged only**

## Error handling

- Wrap errors with `%w` and test them with `errors.Is` / `errors.As`; define sentinel errors for expected failures (`var ErrXxx = errors.New`)
- panic is only for unrecoverable programming errors; recovering and swallowing on the request path is forbidden
- The application layer defines `AppError` (or reuses sentinel errors) and handlers return `error`; the **top-level error middleware maps them to
  HTTP status codes in one place and logs once**; the handler / service / repository layers are forbidden from repeatedly logging the same error chain
- `AppError` separates externally exposed information from internal-only information: 4xx returns actionable text; 5xx returns a generic message only, with the internal error chain going to slog only
- `context.Context` is the first function parameter, threaded through the call chain; error messages carry key context (`fmt.Errorf("query claims: %w", err)`)
- `log.Fatal` / `os.Exit` are forbidden on the request path; fail-fast is allowed only for startup configuration errors

## Web framework (Chi)

- Bound to **Chi v5** (`github.com/go-chi/chi/v5`): a thin decoration layer with `http.Handler` in and out, fully interoperable with the
  `net/http` ecosystem; **Fiber is forbidden** (fasthttp-based, incompatible with net/http), as is gorilla/mux (maintenance mode); Gin / Echo are not introduced
- Routing via `chi.NewRouter()`, path parameters via `chi.URLParam(r, "id")`; subrouters with `r.Route()` (inline) or
  `r.Mount()` (external), and **subrouters inherit middleware** — hang permission control on the subrouter group's `Use` instead of judging inside every handler
- Global middleware order: RequestID → RealIP → Logger → Recoverer → Compress → Timeout → business;
  auth hangs on the route groups that need protection, not mounted globally and then exempted
- The four `http.Server` timeouts are configured explicitly: ReadHeaderTimeout / ReadTimeout / WriteTimeout / IdleTimeout
- Graceful shutdown: `signal.NotifyContext` listens on SIGINT / SIGTERM and `server.Shutdown(ctx)` carries a timeout
  (15s default, `server.Close()` after it expires); `ListenAndServe` returning `http.ErrServerClosed` is expected, not an error
- The outbound interface layer uses DTOs + validation (go-playground/validator/v10 struct tags; run
  `Validate.Struct` immediately after parsing the request body and return 400 + field-level information on failure); JSON-serializing sqlc-generated models directly as responses is forbidden

## API contract (OpenAPI, spec-first)

- Bound to **oapi-codegen** (spec-first): the OpenAPI 3 document is the **single source of truth** (`api/openapi.yaml` in the repo); `cfg.yaml`
  configures generation of the chi-server + models + strict-server typed interfaces; generated code is committed
- Code-first annotation paths such as swaggo are forbidden; spec changes go through separate PR review, and CI verifies a zero diff after regeneration
- Handlers implement the strict-server generated interfaces with no deviation; request validation wires the validator to the spec's constraints
- The spec itself follows the neutral api-design-guide's REST contract and error-code semantics

## Configuration (envconfig)

- Bound to **kelseyhightower/envconfig**: assemble typed structs in one pass; run `envconfig.Process(prefix, &cfg)` at startup and exit on failure
- Business code is forbidden from scattered `os.Getenv` reads; every configuration field must be declared via a struct tag
- `required:"true"` marks required fields; `default:"..."` is for non-sensitive parameters only; **sensitive values (secrets, tokens) must never have defaults — a missing one fails startup**
- Keep naming consistent with `split_words:"true"` or explicit `env:"..."`; isolate nested configuration with nested structs or prefixes
- envconfig's silent zero values are a known risk: a missing field without `required` silently yields the zero value and **raises no error** — every field with business meaning must carry an explicit `required` or `default`, and cross-field validation (ranges, enums) runs after startup
- Configuration structs are injected as dependencies, never accessed through package-level globals

## Logging (slog)

- Bound to **`log/slog`** (Go 1.21+ standard library): build the logger once at process start and pass it via dependency injection;
  **a global default logger is forbidden** (the context-less `slog.Info` forms)
- Handler choice: dev/test `slog.NewTextHandler`; production `slog.NewJSONHandler(os.Stdout, ...)`
- Log with strongly typed attributes only (`slog.String` / `slog.Int` / `slog.Duration` / `slog.Any`);
  `fmt.Sprintf` message building is forbidden — the message is a static description and variables travel as key-value pairs
- Key names are lowercase-with-underscores and consistent across modules (`request_id`, `user_id`, `duration_ms`, `error`)
- Build request-scoped child loggers with `logger.With` (binding `request_id` / `trace_id` / `method` / `path`);
  handlers / services take the child logger from the context or parameters instead of reconstructing it
- Error logs are written once, at the top-level error middleware or the handler boundary; inner layers only propagate with `%w`
- Level discipline: DEBUG for development diagnostics / INFO for normal business events / WARN for recoverable anomalies / ERROR for issues needing human attention;
  recording expected 4xx business failures as ERROR is forbidden

## Concurrency model

- Every goroutine has a lifecycle owner: errgroup management, context cancellation, or an explicit exit path; bare `go func` with no exit is forbidden
- Channels have a clear sender/closer of record — ownerless bidirectional channels are forbidden; guard with `-race` and leak detection (`go.uber.org/goleak`)
- Timeouts and cancellation travel through the context; the `http.Server`'s `BaseContext` and the handler's `r.Context()` must be cancellable by the
  shutdown signal so in-flight requests exit promptly after the shutdown timeout
- Background jobs (timers, consumers) listen to the shutdown context as well; `main` returns only after every background job has exited
- Bound concurrency with a buffered channel or `golang.org/x/sync/semaphore`; unbounded goroutine production is forbidden
- Build the outbound HTTP client (`http.Client`) once, reuse it, and set an explicit `Timeout` (the zero value means no timeout; for fine-grained control use Transport-level
  `DialContext` / `TLSHandshakeTimeout` / `ResponseHeaderTimeout`); bare `http.Get` is forbidden on production paths
- Manage transaction (`BeginTx`) boundaries explicitly and constrain them with the context timeout; remote calls and long waits inside a transaction are forbidden

## Data access

- **database/sql + sqlc** by default (SQL first, type-safe generation); migrations with **golang-migrate**
- Queries are always parameterized; the connection pool is reused and timeouts pass in via context
- sqlc-generated models stay inside the repository layer and **must never double as HTTP response DTOs** — convert through the handler layer's DTOs
- Migration files are append-only; CI runs `golang-migrate ... up` against them to verify applicability
- `sql.DB` is **unbounded** by default (`MaxOpenConns=0`) with immortal connections (`ConnMaxLifetime=0`) — you must set `SetMaxOpenConns` /
  `SetMaxIdleConns` / `SetConnMaxLifetime` / `SetConnMaxIdleTime` explicitly, with the parameters in configuration rather than hardcoded; an unbounded pool will exhaust PG `max_connections`

## Testing

- Table-driven tests as the mainstay, `_test.go` in the same package as the code under test; testify is optional, with style uniform within a module
- Handler tests use `net/http/httptest`: build `NewRouter(deps)`, `httptest.NewRecorder()` +
  `router.ServeHTTP`, no real port; every test gets independent dependency injection
- Integration tests against external dependencies use testcontainers-go, in a separate `_integration_test.go` or build tag, and must run before committing
- Mock external HTTP with httpmock (or the testcontainers WireMock module); real third-party calls inside handler tests are forbidden
- Race detection is mandatory: `go test -race`
- envconfig configuration tests: set values with `t.Setenv`, then `Process`, and verify that missing required values error, defaults apply, and type conversions are correct
- slog tests use an `io.Discard` handler to avoid noise; when asserting logs, use a capturable `io.Writer`
- Test data construction and case design follow the neutral test-design / tdd-guide

## Project layout

- `cmd/<app>/main.go` entrypoint: build the logger → load configuration → build the router → start the `http.Server` → register graceful shutdown
- `internal/<domain>/` implementation layers (internal is the module boundary):
  `handler/` (HTTP handlers, DTOs, validation; one `Routes(deps) http.Handler` per domain returning the subrouter),
  `service/` (business logic), `repository/` (sqlc queries + data access),
  `config/` (envconfig structs + startup validation), `middleware/` (error mapping, request logger, auth)
- Router construction is separated from `ListenAndServe`: expose `func NewRouter(deps) http.Handler` so tests can inject directly via `httptest`
- Use `pkg/` sparingly — only for code that truly must be shared across repositories

## Verification loop

```bash
gofmt -l . && go vet ./... && golangci-lint run ./... && go test ./... -race -count=1
```

Run it in full for every task (`-count=1` disables test caching; this section is the single authority for the local loop); CI increments and the layered paradigm are in the reference files.

## Reference files

- `reference/verification.md` — CI checks, sqlc/migration verification prerequisites, and the graceful-shutdown and configuration-test paradigms (required reading when the task touches CI / pipelines or integration testing)
- `reference/idioms.md` — idioms and high-frequency pitfalls, with Chi / envconfig / slog specifics (required reading for implementation tasks)
