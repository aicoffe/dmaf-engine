---
name: secure-coding
description: 'Secure coding. Input validation, injection defense, XSS/CSRF protection, privilege-escalation defense, sensitive-data handling. Use when the user asks for "secure coding", "injection defense", "XSS defense", "input validation", or "security check". Validation placement: unified security checks at the API gateway + defensive re-validation at the service interface layer.'
---

# Secure Coding

Deny by default, allowlist first, trust no input.

## Workflow

### 1. Input validation (two layers)
- Gateway (unified entry point): authentication, field type/length/format, reject unknown fields, reject request bodies > 1MB
- Service interface layer (Controller): defensive re-validation — required fields, format, range, enum, business allowlist (trust no upstream)
- Domain layer: business-rule validation (state transitions, balances), not format validation
- Blocklist-only filtering is forbidden; guard complex regexes against ReDoS (with unit tests and timeout protection)

### 2. Injection defense
- SQL: parameterized queries / ORM; string concatenation is forbidden
- Commands: concatenated execution is forbidden; parameter allowlist
- Rich text: the server allowlist-filters tags and attributes; relying on frontend filtering is forbidden
- File upload: extension allowlist + Content-Type + magic numbers + size cap; rename on storage

### 3. XSS / CSRF
- Output encoding per context (HTML/attribute/JS/URL); embedding user input directly into templates or eval is forbidden
- Endpoints carrying tokens in cookies: SameSite + CSRF protection (anti-CSRF token or custom header verification)

### 4. Privilege-escalation defense
- Interface level: permissions declared via annotations, intercepted uniformly by the framework
- Data level: every query is forced to carry the user/tenant context; deny by default, authorize explicitly
- Self-check: horizontal (changing IDs to access others' data), vertical (low-privilege calls to high-privilege endpoints)

### 5. Sensitive data
- Raw PII must never enter logs or error messages; masked values do not count as PII (e.g. 138****8000)
- Secrets/passwords/tokens must never be hardcoded and must never enter logs (log as [REDACTED] in full)
- Security response headers: X-Content-Type-Options: nosniff; sensitive endpoints Cache-Control: no-store

### 6. Validation-failure responses
- 400 (format) / 422 (semantics) + a details array (field + issue); echoing the raw malicious payload back is forbidden

### 7. Other high-frequency attack surfaces
- **SSRF** (server-side request forgery): server-side requests to user-controlled URLs (webhook callbacks, file fetches) must verify the target domain against an allowlist, disable internal address ranges (127.0.0.0/8, 169.254.169.254, 10.0.0.0/8, etc.), and must not follow redirects into internal networks
- **Insecure deserialization**: deserializing objects from untrusted sources is forbidden (language-native serialization / shared-library reconstruction); cross-boundary data parses only pure data formats such as JSON — automatic type reconstruction is forbidden
- **Path traversal**: when a file path contains user input, after normalization it must still fall within the confined root directory (prefix comparison after resolve); concatenating and then reading/writing directly is forbidden
- **Mass assignment**: binding input parameters to domain objects goes through an explicit field allowlist; framework auto-binding of all request fields is forbidden (prevents privilege tampering with fields such as role/price)

## Boundaries (not in this skill)

- Domain logic / exceptions / async implementation go through `backend-development` (this skill's validations and defenses embed into its implementation flow)
- Log masking lands via `observability`; key management is in §8

### 8. Key management
- Classified custody: infrastructure credentials and third-party API keys go through a key management service (KMS/Vault) or environment-variable injection; certificate private keys never land in logs or code
- Injection: injected at runtime (environment variables / mounted volumes / dynamic KMS fetch); packaging into images or committing to the repository is forbidden; local development provides only `.env.example` (key names, no values), the real `.env` goes into `.gitignore`; credentials are fully isolated across environments; the log-masking list covers PASSWORD/SECRET/TOKEN/KEY
- Smooth rotation (defaults: DB credentials 90 days, third-party keys 180 days, JWT signing keys 180 days with dual-key gradual rollout): add the new first, switch reads next, retire the old last; rotation operations are recorded in audit logs
- Leak response: revoke/rotate within minutes (do not wait for the investigation) → audit the operation records from the leak window to assess the blast radius → purge Git history (filter-repo, though rotation is what matters) → post-mortem and close the mechanism gap
- CI secret scanning (e.g. gitleaks) blocks merges on hits; inventory the key list quarterly and remove zombie keys

## Reference files

- `rules/standards/security-standard.md` / `rules/standards/quality-standard.md` — higher-level constraints
