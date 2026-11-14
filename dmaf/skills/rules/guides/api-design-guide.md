---
name: api-design-guide
level: L2
domain: api-design
description: Load when designing, modifying, or reviewing API interfaces. Constrains the design of every external and internal interface.
owner: Architecture Group
based_on: [architecture-standard, security-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: every six months
---

# API Design Work Specification

---

## 1. Path Conventions (URL Design)

### 1.1 Base Format

```
/api/{version}/{resource}
```

- **Prefix**: `/api` (fixed)
- **Version**: `v1`, `v2` (carried in the path)
- **Resource**: plural nouns, lowercase, hyphen-separated

**Examples**:
```
GET  /api/v2/claims
GET  /api/v2/claims/{claim_id}
POST /api/v2/claims
GET  /api/v2/claims/{claim_id}/status
```

### 1.2 Resource Naming

| Rule | Correct | Wrong |
| :--- | :--- | :--- |
| Plural nouns | `/claims` | `/claim` |
| All lowercase | `/claim-status` | `/ClaimStatus` |
| Hyphen-separated | `/claim-status` | `/claim_status` |
| No more than 3 nesting levels | `/claims/{id}/status` | `/a/{id}/b/{id}/c/{id}/d` |

### 1.3 Prohibitions

- ❌ Verbs as resource names are forbidden (e.g., `/getClaim`, `/createClaim`)
- ❌ Underscores are forbidden (e.g., `/claim_status`)
- ❌ Uppercase letters are forbidden (e.g., `/ClaimStatus`)
- ❌ Exposing internal implementation in the path is forbidden (e.g., `/api/v2/claims-dao`)
- ❌ Paths without a version number are forbidden (e.g., `/api/claims`)
- ❌ Exposing Swagger UI in production is forbidden (e.g., `/swagger-ui/**`)

### 1.4 Special Paths

| Path | Purpose | Auth required |
| :--- | :--- | :--- |
| `/health` | Health check | ❌ Anonymous |
| `/metrics` | Metrics exposure | ⚠️ Intranet only |
| `/api/v2/public/**` | Public endpoints | ❌ Anonymous |

---

## 2. HTTP Method Semantics

### 2.1 Method Usage Conventions

| Method | Semantics | Idempotent | Safe | Example |
| :--- | :--- | :--- | :--- | :--- |
| **GET** | Read a resource | ✅ Idempotent | ✅ Safe | `GET /claims/{id}` |
| **POST** | Create a resource / perform a non-idempotent operation | ❌ Not idempotent | ❌ Not safe | `POST /claims` |
| **PUT** | Fully replace a resource | ✅ Idempotent | ❌ Not safe | `PUT /claims/{id}` |
| **PATCH** | Partially update a resource | ⚠️ Depends on implementation | ❌ Not safe | `PATCH /claims/{id}` |
| **DELETE** | Delete a resource | ✅ Idempotent | ❌ Not safe | `DELETE /claims/{id}` |

### 2.2 Prohibitions

- ❌ Using GET for side-effecting operations is forbidden (e.g., `GET /claims/{id}/delete`)
- ❌ Using POST for every operation is forbidden (e.g., `POST /claims/query`)
- ❌ Using PUT for partial updates is forbidden (use PATCH)
- ❌ Using DELETE to remove large volumes of data without a confirmation mechanism is forbidden

### 2.3 Idempotency Requirements

| Case | Requirement |
| :--- | :--- |
| **GET / PUT / DELETE** | Must be idempotent; repeated calls yield the same result |
| **POST create** | Must support the `Idempotency-Key` header to prevent duplicate creation |
| **POST non-create** | If idempotency is needed, it must be designed explicitly (e.g., a request number on the payment endpoint) |

---

## 3. Versioning

### 3.1 Version Strategy

- **Version location**: carried in the path, e.g., `/api/v2/claims`
- **Version format**: `v{major}`; only major increments (e.g., `v1`, `v2`)
- **Minor versions**: not reflected in the path; delivered through backward-compatible field additions
- **Version lifecycle**:
  - After a new major version ships, the old version is kept for at least **one major version cycle**
  - When the next major version ships, the old version is marked `deprecated`
  - After being marked `deprecated`, it is kept for at least **6 months**
  - All callers must be notified before retirement; retire only after the retention period ends and zero traffic is confirmed

### 3.2 Version Bump Triggers

| Change type | Version bump required |
| :--- | :--- |
| Adding an optional field | ❌ No |
| Adding an endpoint | ❌ No |
| Removing a field | ✅ Yes |
| Changing a field type | ✅ Yes |
| Changing a field's semantics | ✅ Yes |
| Changing a path | ✅ Yes |
| Changing an error code | ✅ Yes |
| Changing requiredness (optional → required) | ✅ Yes |

### 3.3 Version Compatibility

- **Within the same major version**: backward compatibility is mandatory
- **Across major versions**: breaking changes are allowed, but a migration guide is mandatory
- **Deprecated fields**: marked `deprecated: true`; the retention period follows the §3.1 version lifecycle

---

## 4. Request Conventions

### 4.1 Request Headers

| Header | Required | Description |
| :--- | :--- | :--- |
| `Authorization` | ✅ Required (except allowlist) | `Bearer {token}` |
| `Content-Type` | ✅ Required for POST/PUT/PATCH | `application/json` |
| `Accept` | ⚠️ Recommended | `application/json` |
| `X-Request-ID` | ⚠️ Recommended | Unique request ID, for distributed tracing |
| `Idempotency-Key` | ⚠️ Recommended on POST create | Idempotency key |
| `X-Client-Version` | ⚠️ Recommended | Client version, for gradual rollouts |

### 4.2 Request Body

- **Format**: JSON (`application/json`)
- **Field naming**: `snake_case` (consistent with the database)
- **Required fields**: must be annotated explicitly with `required: true`
- **Unknown fields**: must be rejected (`400 Bad Request`)
- **Body size**: default limit **1 MB**; exceeding it returns `413 Payload Too Large`

### 4.3 Query Parameters

| Parameter type | Naming convention | Example |
| :--- | :--- | :--- |
| **Pagination** | `page`, `size` | `?page=1&size=20` |
| **Sorting** | `sort` | `?sort=created_at,desc` |
| **Filtering** | `filter_{field}` | `?filter_status=approved` |
| **Field selection** | `fields` | `?fields=id,status,created_at` |
| **Search** | `q` | `?q=claim+123` |

### 4.4 Path Parameters

- **Format**: `{param_name}`, snake_case
- **Required**: path parameters must be present, otherwise `404`
- **Validation**: path parameters must be format-checked (IDs must follow the unified ID convention: `{prefix} + 15 digits`, 20 characters in total, e.g., `CLAIM123456789012345`)

---

## 5. Response Conventions

### 5.1 Response Structure

**Success response** (returns the resource directly):

```json
{
  "id": "CLAIM123456789012345",
  "status": "under_review",
  "created_at": "2026-09-25T10:00:00Z"
}
```

**List response**:

```json
{
  "items": [
    { "id": "CLAIM123456789012345", "status": "under_review" },
    { "id": "CLAIM123456789012346", "status": "approved" }
  ],
  "pagination": {
    "page": 1,
    "size": 20,
    "total": 156,
    "total_pages": 8
  }
}
```

**Error response**:

```json
{
  "error": {
    "code": "CLAIM_NOT_FOUND",
    "message": "Claim not found",
    "details": [
      { "field": "claim_id", "issue": "not found" }
    ],
    "trace_id": "abc123def456"
  }
}
```

### 5.2 Response Field Conventions

| Field type | Naming convention | Example |
| :--- | :--- | :--- |
| **ID** | `{resource}_id` | `claim_id` |
| **Timestamp** | `{action}_at`, ISO 8601 UTC | `created_at: "2026-09-25T10:00:00Z"` |
| **Amount** | `{name}_amount`, integer (cents) or string (BigDecimal) | `amount: 10000` (= CNY 100.00) |
| **Enum** | `snake_case` string | `status: "under_review"` |
| **Boolean** | `is_{name}` or `has_{name}` | `is_active: true` |
| **List** | Plural noun | `items`, `claims` |

### 5.3 Prohibitions

- ❌ Returning `null` to mean "no data" is forbidden (omit the field or return an empty array)
- ❌ Exposing internal implementation details in responses is forbidden (e.g., database table names, SQL errors)
- ❌ Exposing PII in responses is forbidden (e.g., ID numbers and phone numbers must be masked)
- ❌ Representing amounts as floating-point numbers is forbidden (use integer cents or a string BigDecimal)
- ❌ Wrapping business errors in HTTP 200 is forbidden (e.g., `{ "code": 500, "message": "..." }`)

### 5.4 HTTP Status Codes

| Status code | Use case |
| :--- | :--- |
| **200 OK** | Successful read or update |
| **201 Created** | Created; the response includes a `Location` header |
| **204 No Content** | Deleted; no response body |
| **400 Bad Request** | Parameter validation failed |
| **401 Unauthorized** | Not authenticated or token expired |
| **403 Forbidden** | Authenticated but not authorized |
| **404 Not Found** | Resource not found |
| **409 Conflict** | Resource conflict (e.g., duplicate creation) |
| **422 Unprocessable Entity** | Semantic error (e.g., an illegal state transition) |
| **429 Too Many Requests** | Rate limit triggered |
| **500 Internal Server Error** | Internal service error |
| **503 Service Unavailable** | Service unavailable (circuit breaker / maintenance) |

---

## 6. Error Code Conventions

### 6.1 Error Code Format

- **Format**: uppercase with underscores, e.g., `CLAIM_NOT_FOUND`
- **Length**: no more than 50 characters
- **Naming**: `{RESOURCE}_{ERROR_TYPE}`

### 6.2 Error Code Categories

| Prefix | Category | Example |
| :--- | :--- | :--- |
| `AUTH_*` | Authentication | `AUTH_TOKEN_EXPIRED` |
| `PERM_*` | Permissions | `PERM_INSUFFICIENT` |
| `VALIDATION_*` | Validation | `VALIDATION_PHONE_FORMAT` |
| `{RESOURCE}_*` | Business | `CLAIM_NOT_FOUND`, `CLAIM_STATUS_INVALID` |
| `SYS_*` | System | `SYS_INTERNAL_ERROR` |
| `RATE_*` | Rate limiting | `RATE_LIMIT_EXCEEDED` |

### 6.3 Error Codes Must Be Enumerable

- All error codes must be defined in the `ErrorCode` enum
- Dynamically assembling error codes is forbidden
- Error code changes must go through a version bump

### 6.4 Error Message Conventions

- **User-readable**: error messages address the user and expose no technical details
- **No internal leakage**: stack traces, SQL, and internal paths are forbidden
- **Internationalization**: error messages support multiple languages (via `Accept-Language`)
- **trace_id included**: every error response must carry `trace_id` for troubleshooting

---

## 7. Pagination, Sorting, Filtering

### 7.1 Pagination

- **Parameters**: `page` (starts at 1), `size` (default 20, max 100)
- **Response**: includes a `pagination` object (`page`, `size`, `total`, `total_pages`)
- **Deep offsets**: when `page * size > 10000`, cursor pagination (`cursor`) is recommended

### 7.2 Sorting

- **Parameter**: `sort={field},{direction}`; multiple fields supported, `sort=created_at,desc&sort=status,asc`
- **Allowed fields**: only fields on the sorting allowlist
- **Default sort**: a default sort is mandatory (e.g., `created_at,desc`)

### 7.3 Filtering

- **Parameter**: `filter_{field}={value}`
- **Multiple values**: `filter_status=approved,rejected` (comma-separated)
- **Ranges**: `filter_created_at_gte=2026-01-01&filter_created_at_lte=2026-12-31`
- **Fuzzy search**: `q={keyword}`, effective only on allowlisted fields

---

## 8. Idempotency and Concurrency Control

### 8.1 Idempotency

| Case | Requirement |
| :--- | :--- |
| **POST create** | Support the `Idempotency-Key` header; the same key within 24 hours returns the same result |
| **PUT** | Idempotent by nature |
| **DELETE** | Idempotent by nature (repeat deletes return 204 or 404) |
| **Payments / transfers** | Must use a business request number (e.g., `payment_request_id`) to guarantee idempotency |

### 8.2 Optimistic Locking

- **Header**: `If-Match: {etag}` or `If-Unmodified-Since: {timestamp}`
- **Response**: returns an `ETag` header
- **Conflict**: returns `409 Conflict`

### 8.3 Pessimistic Locking

- Implemented via an `If-Locked` header or distributed locks
- The lock timeout must be configured explicitly
- Lock release must be reliable (timeout release + explicit release)

---

## 9. Timeouts, Retries, Rate Limiting

### 9.1 Timeouts

| Type | Default value | Description |
| :--- | :--- | :--- |
| **Client connect timeout** | 3 seconds | — |
| **Client read timeout** | 10 seconds | — |
| **Server processing timeout** | 30 seconds | On timeout the API gateway returns `504 Gateway Timeout` |
| **Long tasks** | Go async | Operations over 30 seconds must go async (return `202 Accepted` + a task ID) |

### 9.2 Retries

- **Client retries**: retry only idempotent operations (GET / PUT / DELETE)
- **Retry policy**: exponential backoff, at most 3 retries
- **Retry idempotency**: retries must carry the same `Idempotency-Key`

### 9.3 Rate Limiting

- **Granularity**: per user, per IP, global (thresholds in `rules/standards/security-standard`)
- **Response headers**:
  - `X-RateLimit-Limit`: quota
  - `X-RateLimit-Remaining`: remaining
  - `X-RateLimit-Reset`: reset time (Unix timestamp)
- **Over-limit response**: `429 Too Many Requests` + a `Retry-After` header

---

## 10. Documentation Conventions

### 10.1 OpenAPI Specification

- **Format**: OpenAPI 3.0+
- **Location**: `openapi.yaml` at each service's root
- **Required content**:
  - Paths, methods, parameters, and responses of all endpoints
  - Definitions of all error codes
  - Type, format, and constraints of all fields
  - Request/response examples

### 10.2 Documentation Generation

- **Tooling**: the OpenAPI generator bound by the stack binding + Swagger UI (e.g., SpringDoc, utoipa, swag; exposed in test environments only)
- **Production**: exposing Swagger UI is forbidden
- **Publishing**: published through the API gateway or an internal documentation platform

### 10.3 Change Log

- **Format**: `CHANGELOG.md`
- **Content**: changes per version (added, modified, deprecated, removed)
- **Traceability**: every change links to its ADR or spec

---

## 11. Backward Compatibility

### 11.1 Compatibility Rules

| Change type | Compatible | Handling |
| :--- | :--- | :--- |
| Adding an optional field | ✅ Compatible | Add directly |
| Adding a required field | ❌ Incompatible | Add as optional first, make it required in the next version |
| Removing a field | ❌ Incompatible | Mark `deprecated` first; deprecate per the §11.2 process |
| Changing a field type | ❌ Incompatible | Add a new field and keep the old one |
| Changing a field's semantics | ❌ Incompatible | Add a new field and keep the old one |
| Changing an endpoint path | ❌ Incompatible | Add a new path; keep the old one and redirect |
| Changing an error code | ❌ Incompatible | Add a new error code and keep the old one |

### 11.2 Deprecation Process

```
1. A new major version ships; the old version enters its retention period (at least one major version cycle)
2. When the next major version ships, the old version is marked deprecated
   (deprecated: true in OpenAPI + a Deprecation header on responses)
3. All callers are notified
4. After being marked deprecated, it is kept for at least 6 months
5. After the retention period ends and zero traffic is confirmed, the old version is retired
```

---

## 12. Conflict Adjudication

Conflicts with other domains are adjudicated by the L0 principles and the L1 domain standards:

| Conflict | Adjudication basis |
| :--- | :--- |
| Security vs convenience | P-001; authentication must not be weakened (`rules/standards/security-standard`) |
| Compatibility vs cleanliness | P-005; breaking changes require Architecture Group approval |
| Experience vs technical concerns | P-009; friendly error messages, pagination and field selection |
| Performance vs design | P-009; solve via pagination / field selection / caching without breaking the contract |
