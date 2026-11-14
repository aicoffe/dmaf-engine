# API Design

> Deep norms for the lld API design chapter. Trigger condition: must-read when the lld includes API design (paths/methods/inputs-outputs/error codes/idempotency/secure access).

Design consistent, secure, evolvable REST APIs.

## Workflow

### 1. Paths and methods
- Format: `/api/v{major}/{plural noun}`, lowercase with hyphens, hierarchy ≤ 3; the version only ever increments major
- GET reads (idempotent and safe), POST creates (non-idempotent), PUT full replace, PATCH partial update, DELETE removes
- Forbidden: verb resource names, GET with side effects, POST-as-anything, underscores, missing version

### 2. Requests and responses
- Request bodies in JSON snake_case; unknown fields rejected (400); default size ≤ 1MB
- Responses: success returns the resource directly; lists include `pagination`; errors include `error.code/message/details/trace_id`
- Fields: time as `{action}_at` ISO 8601 UTC, money as integer cents or a string, booleans with the `is_/has_` prefix
- Status codes: 200/201/204/400/401/403/404/409/422/429/500/503; wrapping business errors in HTTP 200 is forbidden
- PII must be masked; exposing table names, SQL, or stack traces is forbidden

### 3. Versioning and compatibility
- Compatible changes (new optional fields/endpoints) ship directly; incompatible changes (removing fields / changing type semantics / changing error codes / optional to required) bump major
- Life cycle: the old version is kept for one major cycle → marked deprecated when the next major ships → kept at least 6 months after deprecation → retired once no callers remain
- Deprecation markers: OpenAPI `deprecated: true` + the `Deprecation` header + notify callers

### 4. Idempotency and concurrency
- POST creation supports `Idempotency-Key` (the same key within 24h returns the same result); payment operations use a business request number
- Optimistic locking via `If-Match`/ETag, conflict 409

### 5. Secure access checklist
- Authentication on by default; public endpoints are explicitly registered in a whitelist (`/health`, `/api/v2/public/**`)
- Gateway-level unified security validation: type/length/format, unknown fields, >1MB rejected; defensive second validation at the service interface layer
- The CORS whitelist is enumerated item by item; `*` is forbidden; replay protection via nonce+timestamp (5-minute window)
- Rate-limit response header trio + 429 + Retry-After
- Pagination/sorting/filter parameters go through a whitelist

### 6. Documentation
- Full OpenAPI 3.0+ definitions (paths, error codes, constraints, examples); exposing Swagger UI in production is forbidden

### 7. Error code system (fixed at design time; a contract once released)
- Segmented structure: `{domain}_{resource}_{reason}` in uppercase with underscores (e.g. `CLAIM_STATUS_NOT_EDITABLE`) or a layered enumeration — one style, organization-wide; dynamic concatenation is forbidden
- Error code registry: registered centrally (enum class / registry table); new codes are checked for duplicates first; retired codes keep their definition and are never reused (error codes are a contract, frozen at release)
- Every error code registers three things: HTTP status mapping, a user-readable message, and whether it is retryable (transient failures are retryable; deterministic failures must never be retried)

## Hard rules

- Authentication is never weakened for convenience; error codes are enumerable (an ErrorCode enum; dynamic concatenation forbidden)
- Error messages are user-readable, leak no internal information, carry trace_id, and support internationalization
- Retired items keep their retention period per the version life cycle; early deletion is forbidden

## Reference files

- `rules/guides/api-design-guide.md` — full REST norms (primary document)
- `rules/standards/architecture-standard.md` / `rules/standards/security-standard.md` — higher-level constraints
