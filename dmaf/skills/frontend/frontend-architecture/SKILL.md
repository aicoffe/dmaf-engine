---
name: frontend-architecture
description: Frontend engineering architecture. Directory structure, routing organization, request layer encapsulation, environment configuration, build optimization and bundle size budgets, frontend error monitoring integration. Use when the user asks for "setting up a frontend project", "frontend directory structure", "wrapping requests", "frontend build", "environment configuration", "error monitoring", or "error reporting".
---

# Frontend Architecture

A layered frontend project organized by business domain: pages → components → service layer → state, with one-way dependencies.

## Workflow

### 1. Directory structure (by business domain)
```
src/
├── app/        # entry, routing, global Provider
├── domains/    # business domains (self-contained per domain: pages/components/api/model)
├── shared/     # cross-domain shared: common components, utilities, request wrapper
└── config/     # environment configuration (typed, centrally exported)
```
- Cross-domain references may only point at shared/; horizontal imports between domains are forbidden
- Promotion to shared/ criteria: used by ≥ 2 domains and a stable API

### 2. Routing
- Register routes in files split by business domain; route-level lazy loading (code splitting)
- A unified guard for permission routes (no permission renders 403; hide-only permission control is not done)
- Route parameters carry IDs only

### 3. Request layer (single entry point)
- http instance = baseURL (per environment) + interceptor chain
- Request interception: inject Authorization and X-Request-ID, pass the trace context through
- Response interception: 401 → refresh/logout flow; 4xx/5xx → unified error translation (user-readable message + code/trace_id preserved for troubleshooting)
- Global error fallback: window.onerror / unhandledrejection captured and reported uniformly, carrying the release version and trace_id (interlinked with backend tracing); source maps upload to the monitoring platform only — deploying them to production is forbidden
- Business code using bare fetch/axios directly is forbidden

### 4. Environment configuration
- config/ centrally exports a typed object; environment variables scattered through code are forbidden
- Deployment-varying configuration (API addresses, feature switches) is injected at runtime so the build artifacts stay reusable

### 5. Build and artifacts
- Route-level code splitting; dynamic import for large dependencies (charts/editors)
- Bundle size budgets gate CI (the main bundle or any single chunk over budget blocks) — budget-setting method (default): measure the current artifacts first for the baseline → budget = baseline × (1 + growth margin, default 20%), allocated per route to single chunks → downward only, raising it requires a recorded review
- Static assets carry a content hash for long-lived caching

### 6. Engineering quality
- TypeScript strict mode; `any` type escapes are forbidden
- ESLint + Prettier in CI

## Boundaries (not in this skill)

- Component / state / interaction implementation goes through `frontend-development`; page-level design increments go through `detailed-design-frontend`
- New design token values go through the `design-system` inventory review (this skill only consumes the inventory to scaffold the project skeleton)

## Reference files

- `rules/standards/frontend-standard.md` / `rules/standards/ux-standard.md` — higher-level constraints
