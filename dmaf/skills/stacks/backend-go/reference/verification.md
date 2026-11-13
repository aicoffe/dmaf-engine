# Go Verification Loop and CI Checks

## Local loop (every task)

The commands follow `SKILL.md`'s "Verification loop" as the single authority (`gofmt -l .` / `go vet ./...` / `golangci-lint run ./...` /
`go test ./... -race -count=1`); this file does not duplicate that list and only adds the layered execution model and CI increments. `-count=1` disables test caching
to guarantee real execution every time; `-race` is mandatory across the board.

## Layered execution

- Unit tests run in full with `go test ./... -race`; during development narrow with `go test ./internal/<domain>/ -run <case>`
- Handler tests: `httptest` + `NewRouter(deps)`, no real port; independent dependency injection per test, no shared state
- Integration tests (testcontainers-go): `_integration_test.go` or build tag; must run before committing via
  `go test -tags=integration ./... -race`
- envconfig configuration tests: use `t.Setenv` (Go 1.17+, auto-cleanup) to verify that missing required values error, defaults apply,
  and type conversions are correct
- slog tests: an `io.Discard` handler avoids noise; capture output with a custom `io.Writer` when asserting log content
- Graceful-shutdown verification: in an integration test, start the server, send SIGTERM, and assert that in-flight requests complete and `Shutdown` returns within the timeout
- Race detection runs in full every time; enabling it only in CI is forbidden

## CI checks (added on top of the local loop)

- `go build ./...` and the release artifact build pass
- `govulncheck ./...`: **reachable vulnerabilities fail, unreachable ones are logged only**
- The golangci-lint version is compatible with the Go version in go.mod; `.golangci.yml` (v2 format) changes get separate review
- Integration tests (testcontainers): run when Docker is available; CI without Docker must state explicitly "integration tests not covered" — silent skipping is forbidden
- sqlc projects: `sqlc generate` output matches the committed content with zero diff (generated code has not drifted)
- Migrations: `golang-migrate ... up` applies successfully against the test database
- The Go version matches go.mod; CI implicitly switching versions is forbidden, as is implicit dependency upgrading via `go get -u` in CI
- `go mod tidy -diff` (Go 1.23+): zero drift in go.mod / go.sum (aligning with Rust's `--locked` semantics)
- API contract: `api/openapi.yaml` changes get a separate PR; after oapi-codegen regeneration, `git diff --exit-code` shows zero diff

## Prohibitions

- `//nolint` without a comment is forbidden (enforced by `nolintlint`); when an exemption is truly needed, write `//nolint:<linter> // reason`
- CI skipping vet / race ("merge first, fix later") is forbidden
- Logging through the global default slog logger inside handlers / services is forbidden
- Passing request DTOs into the service layer without `Validate.Struct` validation is forbidden
- Dependency changes go through a separate go.mod commit — never smuggled inside feature PRs
