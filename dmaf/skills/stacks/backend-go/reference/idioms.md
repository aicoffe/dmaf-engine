# Go Backend Idioms and High-Frequency Pitfalls

## Error handling

- Wrap errors with verb-phrase context (`fmt.Errorf("decode body: %w", err)`); judge with errors.Is/As only —
  string-comparing error messages is forbidden
- Sentinel errors are part of the API: changes to exported ErrXxx values require a compatibility assessment
- The error chain is logged once, only at the top-level error middleware; per-layer repeated logging of the same error is an anti-pattern (slog best practices explicitly forbid it)

## Context and concurrency

- context carries only cancellation / timeouts / request metadata; using it as a generic parameter bag is forbidden
- Three goroutine-leak checks: a for-select with no exit path, a channel no one closes, a ticker / time.Timer no one stops
- With errgroup concurrent subtasks, any failure cancels the rest; bound concurrency with a buffered channel or `x/sync/semaphore`
- Background tasks hold a context derived from the shutdown context; living past the shutdown signal is forbidden

## Chi pitfalls

- **Global middleware must be `Use`d before routes are registered**: Chi middleware only affects routes registered after it
- **auth on route groups, not global**: `r.Group(func(r chi.Router) { r.Use(auth); ... })` or
  `Use` inside `r.Route("/admin", ...)`; global auth intercepts healthz and similar endpoints too
- **`chi.URLParam` is unavailable inside (global) middleware** (routes not yet matched); to get path parameters in middleware use
  `chi.RouteContext(r.Context()).URLParams`, but prefer `r.Route` subrouters and avoid path special-casing in generic middleware
- **All four `http.Server` timeouts are required**: setting only `ReadTimeout` without `ReadHeaderTimeout` still leaves slowloris exposure;
  `WriteTimeout` should be relaxed for streaming scenarios
- **`Shutdown` must take a timeout context**: without a timeout it can block forever; after the timeout, `server.Close()` forces shutdown
- **`WriteHeader` can be called only once**: once middleware has written the header (e.g. the logger recorded the status code), further handler writes are ignored;
  emit responses uniformly via `writeJSON` at the end of the handler
- **`middleware.Timeout` only cancels the context**: if the handler ignores `r.Context()`, the goroutine still runs to completion while the response is already dropped
  (side effects may have happened) — long operations inside handlers must respect ctx

## envconfig pitfalls

- **Fields not marked `required` silently become zero values when missing** — envconfig's biggest pitfall; every field with business meaning must carry
  `required:"true"` or `default:"..."`
- **`default` for `time.Duration` and similar types must be a parseable string** (e.g. `"30s"`), not an integer;
  `default:"0"` on an `int` is indistinguishable from unset
- **Nested-struct env prefixes are not auto-derived**: nested configuration either flattens fields or writes the full variable name in the tag
- **When prefix and `env` tag coexist, the prefix concatenates first**: `Process("MYAPP", &cfg)` + `env:"PORT"`
  reads `MYAPP_PORT`; do not mix prefix with full-variable-name tags, to avoid unexpected variable names

## slog pitfalls

- **The global default logger carries no request context**: calling `slog.Info` directly loses `request_id` / `trace_id`;
  it must be constructed in main and injected
- **`slog.Any` can lose structure for errors**: a plain error emits only its message; when structure is needed, implement `slog.LogValuer`
  or use explicit `slog.String("error", err.Error())` + `slog.String("error_type", ...)`
- **Unstable key names break log queries**: the same business field must use a consistent key across layers (`user_id`, not `userId`);
  key names are agreed at project level
- **Check the level before building expensive attributes on hot paths**: `if logger.Enabled(ctx, slog.LevelDebug)` avoids pointless construction

## API and data

- The outbound interface layer uses DTOs; JSON-serializing database models directly as responses is forbidden; sqlc-generated models stay inside the repository only
- sqlc queries are parameterized; fmt.Sprintf-assembled SQL is forbidden; migration files are append-only
- Configuration is assembled once by envconfig; business code reading os.Getenv is forbidden; sensitive values must not default — missing means startup failure
- oapi-codegen strict-server: contract changes edit the spec first, then regenerate — hand-editing generated code is forbidden
- The `http.Client` zero value has no timeout; outbound calls must set `Timeout` explicitly; build the client once and reuse it

## Engineering habits

- Interfaces are defined on the consumer side (small interfaces), not pre-abstracted on the implementation side
- Exported symbols carry doc comments; implementation types are not exposed outside `internal/`
- Zero-value semantics are explicit: the JSON behavior differences between nil slices and empty maps are handled at the DTO layer
- In tests, `t.Setenv` and `t.Parallel` are mutually exclusive (combining them panics): tests that need parallelism never touch environment variables — configuration arrives via dependency injection
- Dependency upgrades and configuration changes get their own commits — never smuggled inside feature PRs
