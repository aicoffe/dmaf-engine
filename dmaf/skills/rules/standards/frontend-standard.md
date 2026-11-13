---
name: frontend-standard
level: L1
domain: frontend
description: Load when frontend architecture, state management, component design, browser compatibility, or frontend performance is involved. Governs all frontend work.
owner: Frontend Team
based_on: [organization-principles, ux-standard, architecture-standard, performance-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# Frontend Domain Standard

> This standard governs frontend architecture standards and red lines; implementation details are carried by the corresponding domain skills.

---

## 1. Frontend Architecture

- **Layering**: pages (orchestration) → components (reusable UI) → service layer (API call encapsulation) → state (global state)
- **No business logic in components**: data transformation and business validation go in the service layer / standalone modules; components only render and interact
- **Organize modules by business domain**, with route-level on-demand loading (code splitting)
- Componentization and layering follow high cohesion, low coupling, and one-way dependencies

---

## 2. State Management

- **Distinguish two kinds of state**: server state (request cache) uses the request library's caching capabilities first and is not duplicated into a global store; only client state goes into global state management
- **Minimize global state**: only cross-page shared state (login state, theme, permissions)
- **Traceable changes**: state may only be modified through the official channels of the state management solution; bypassing them is forbidden
- ❌ Using global state to pass values used only between parent and child components is forbidden

---

## 3. Component Conventions

- Reuse component libraries first, Props must be typed, styles must reference Tokens, aria for accessibility, core components have tests and Stories
- Component interface design: explicitly typed Props, clear controlled/uncontrolled modes, semantic event naming
- ❌ Copy-pasting components without extraction is forbidden (duplicated components must be consolidated into shared components)

---

## 4. API Integration

- Interface contracts follow the unified API clause: field naming, pagination, error response structure (`error.code` / `trace_id`)
- **Every request must handle non-2xx**: friendly error message + retry entry; error codes support troubleshooting and the raw code is not shown directly
- Loading / empty / error — all three states must be implemented
- In-flight requests must be canceled on page unload; backend addresses must not be hardcoded; environment configuration must be declared explicitly (P-007)

---

## 5. Compatibility

- The browser support matrix must be declared explicitly (default values: the last two major versions of Chrome / Edge / Firefox + two versions of Safari; adjustable per product)
- Browsers outside the matrix are not adapted for; matrix changes must go through review
- Mobile follows `rules/standards/ux-standard` (breakpoints, minimum clickable area, responsive verification)

---

## 6. Frontend Performance

- Follow `rules/standards/performance-standard`: core page performance budgets must be defined explicitly
- Core page budgets default to alignment with Core Web Vitals: LCP ≤ 2.5s, INP ≤ 200ms, CLS ≤ 0.1 (default values)
- Route-level code splitting; virtual scrolling for long lists; image lazy loading and size-appropriate variants
- Build artifact size budgets must be declared explicitly and checked in CI

---

## 7. Conflict Adjudication with Other Domains

### 7.1 With UX

- **Adjudication basis**: `rules/standards/ux-standard` is the sole standard on the design side
- **Adjudication process**: Design Owner + Frontend Lead adjudicate jointly

### 7.2 With Performance

- **Adjudication basis**: P-009 (user experience first)
- **Example**: animation effects vs performance → degrade to simplified animation; do not remove interaction feedback outright

### 7.3 With Security

- **Adjudication basis**: P-001 (security first)
- **Red lines**: tokens never land in localStorage; rendering user input must prevent XSS; CSP is configured per the security team's baseline
