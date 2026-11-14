---
name: security-standard
level: L1
domain: security
description: Load when authentication, authorization, sensitive data handling, API security, or secret management is involved.
owner: Security Team
based_on: [organization-principles]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Security Domain Standard

---

## 1. Authentication

### 1.1 Authentication Protocols

- **Protocols**: OAuth2.0 + OIDC
- **Token type**: JWT
- **Token lifetime**: 30 minutes
- **Refresh token lifetime**: 7 days
- **SSO**: OIDC

### 1.2 Token Conventions

- **Signing algorithm**: RS256
- **Required claims**: sub, iss, aud, exp, iat, jti
- **Transmission**: `Authorization: Bearer {token}`
- **Storage**: frontend: access token in frontend memory, refresh token in an HttpOnly Cookie / backend: memory
- **Forbidden**: storing tokens in localStorage is forbidden
- **CSRF protection**: any endpoint carrying a token via cookies (e.g., the refresh token) must set the `SameSite` attribute and enable CSRF protection (anti-CSRF token or custom header verification)

### 1.3 Authentication Failure Handling

- **Failure response**: 401 Unauthorized
- **Failure count limit**: lock for 15 minutes after 5 failures
- **Failure logs**: record IP, time, and failure reason (passwords are never recorded)

### 1.4 Session Management

- **Session timeout**: 30 minutes of inactivity
- **Concurrent sessions**: multiple devices per user are allowed
- **Forced logout**: administrators may force users offline
- **Logout and revocation**: logout must immediately invalidate the refresh token (server-side session table or `jti` blacklist); the access token cannot be revoked while valid, so its lifetime must be short (see 1.1)

---

## 2. Authorization

### 2.1 Authorization Model

- **Model**: RBAC
- **Permission granularity**: API level + data level
- **Default policy**: deny all, grant explicitly

### 2.2 Permission Checks

- **Check location**: performed in the application/domain layer of backend services (the gateway only authenticates; authorization decisions must be enforced inside the service and must not rely on the gateway)
- **Data-level permissions**: users can only access their own data
- **Privilege escalation protection**: every query must carry user context

### 2.3 Multi-Tenant Isolation

- **Isolation level**: row-level isolation
- **Isolation field**: tenant_id
- **Cross-tenant access**: forbidden unless explicitly authorized

---

## 3. API Security

### 3.1 Authentication Requirements

- **Authentication failure**: return 401 without disclosing the specific reason

### 3.2 Input Validation

- **Validation location**: the API gateway (unified entry layer). This validation is performed by the gateway, not by each backend service's Controller / Service layer
- **Defensive secondary validation**: the interface layer (Controller) of backend services must still perform defensive validation on inputs; it must not be omitted (trust no upstream caller)
- **Validation rules**:
  - Field type, length, format
  - Reject unknown fields
  - Reject oversized request bodies (> 1MB)
- **Validation failure**: return 400 with the specific error field

### 3.3 Rate Limiting and Anti-Replay

- **Rate limit granularity**:
  - Per user: 100 req/min
  - Per IP: 1000 req/min
  - Global: 10000 req/min
- **Rate limit algorithm**: token bucket
- **Anti-replay**:
  - Requires nonce + timestamp
  - Signing algorithm: HMAC-SHA256
  - Time window: 5 minutes

### 3.4 CORS

- **Allowed origins**: declared explicitly as a whitelist
- **Allowed methods**: GET, POST, PUT, PATCH, DELETE
- **Allowed headers**: Authorization, Content-Type
- **Forbidden**: using the `*` wildcard is forbidden

### 3.5 API Signing (If Applicable)

- **Signing algorithm**: HMAC-SHA256
- **Signed fields**: method + path + body + timestamp
- **Signature location**: Header: X-Signature

---

## 4. Data Security

### 4.1 Data Classification

The definitions, storage, and access requirements of the data classification levels (L1 public / L2 internal / L3 sensitive / L4 confidential) take `rules/standards/data-standard` as the single source of truth; this standard does not redefine them.

Additional security-side requirements:

| Level | Security requirements |
| :--- | :--- |
| **L3 sensitive** | Field-level encryption + masking on display/export/logging + access auditing |
| **L4 confidential** | Field-level encryption + key isolation + least privilege + access approval + access auditing |

### 4.2 PII Definition

**PII fields that must be protected**:
- ID card number
- Phone number
- Bank card number
- Email address
- Home address
- Biometric features

**PII protection requirements**:
- **Logs**: raw values must never appear in logs; masked values are not PII and may be logged per the masking rules (see 4.5; log structure: see `rules/standards/observability-standard`)
- **Error messages**: must never appear in error messages
- **API responses**: must be masked (e.g., phone number 138****8000)
- **Database**: must be stored encrypted
- **Export**: must be masked

### 4.3 Transport Encryption

- **Protocol**: TLS 1.3 end to end
- **Certificates**: use certificates issued by the corporate CA
- **Forbidden**: self-signed certificates are forbidden (production)
- **Internal communication**: service-to-service communication must also use TLS

### 4.4 Storage Encryption

- **Encryption algorithm**: AES-256-GCM
- **Key management**: no hardcoding
- **Encrypted fields**: PII fields must be encrypted
- **Encryption scope**: database field-level encryption

### 4.5 Data Masking

| Scenario | Masking rule | Example |
| :--- | :--- | :--- |
| **Frontend display** | Mask the middle 4 digits of phone numbers | 138****8000 |
| **API responses** | Return only the last 4 digits of ID card numbers | ****1234 |
| **Logs** | Raw values must never be logged; phone numbers and the like may be logged in masked format; secrets / passwords / tokens must be fully replaced | `138****8000` / `[REDACTED]` |
| **Export** | Mask by role | Admins see full values; regular users see masked values |

### 4.6 Data Destruction

- **Soft delete**: soft delete by default, flagging `deleted=1`
- **Hard delete**: periodically purge soft-deleted data (after 30 days)
- **Retention period**: user data is retained for 3 years
- **User-requested deletion**: completed within 30 days

---

## 5. Secrets and Credentials

### 5.1 Secret Management

- **Forbidden**: hardcoding secrets, passwords, tokens is forbidden
- **Injection**: injected via environment variables
- **Auditing**: secret access is recorded in audit logs

### 5.2 Credential Protection

- **Forbidden to commit**: committing credentials such as `.env` or `credentials.json` to Git is forbidden
- **Environment isolation**: development, testing, and production use different credentials
- **Least privilege**: each service is granted only the permissions it needs

### 5.3 AI Security Boundary

Security constraints on AI usage (allowed/forbidden boundaries, generated-code review focus, context data classification, cost control) are unified in `rules/standards/ai-usage-standard`.

Red lines:
- AI must not execute production deployment commands (unless there is a release approval)
- AI-generated code must not contain hardcoded secrets

---

## 6. Security Baseline

### 6.1 OWASP Top 10 (2021)

Vulnerability types that must be defended against:
- [ ] A01 Broken Access Control
- [ ] A02 Cryptographic Failures
- [ ] A03 Injection (including SQL / NoSQL / command injection)
- [ ] A04 Insecure Design
- [ ] A05 Security Misconfiguration
- [ ] A06 Vulnerable and Outdated Components
- [ ] A07 Identification and Authentication Failures
- [ ] A08 Software and Data Integrity Failures
- [ ] A09 Security Logging and Monitoring Failures
- [ ] A10 Server-Side Request Forgery (SSRF)

---

## 7. Conflict Adjudication with Other Domains

### 7.1 With UX

- **Adjudication basis**: P-001 (security over experience)
- **Adjudication process**: Security Owner + Designer adjudicate jointly
- **Accommodation**: look for solutions that accommodate both first, such as extending the timeout + secondary authentication for sensitive operations

### 7.2 With Business

- **Adjudication basis**: P-002 (compliance over business flexibility)
- **Adjudication process**: Security Owner + Product Manager adjudicate jointly
- **Accommodation**: find alternative approaches, such as displaying data only after masking

### 7.3 With Performance

- **Adjudication basis**: P-001 (security first)
- **Adjudication process**: Security Owner + Architect adjudicate jointly
- **Accommodation**: e.g., pick encryption algorithms at the balance point between performance and security
