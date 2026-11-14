# Fencing and mocks — survey, scaffolding, route gating, mock channels, review bootstrap

This file serves anchoring the prototype inside a real React + Mantine app (shared by the brownfield anchored mode and the greenfield whole-shell-in-fence). The recipe is a pattern, not an incantation — the project's own conventions always take precedence over this file.

## 1. Survey checklist (one pass; every conclusion backed by an opened file)

| What to survey | Where to look | What it means for the prototype |
| :--- | :--- | :--- |
| react / @mantine/* / @tanstack/react-query / react-router versions | `package.json` + lockfile | Judge greenfield / brownfield conventions against the stack skill's baseline matrix (React 18 / Mantine 8 etc. per the stack skill's brownfield.md; prototype pages do not upgrade) |
| Provider mounting | `main.tsx` / `App.tsx` (MantineProvider → Notifications → ModalsProvider order) | Prototype pages render inside the app tree and inherit theme and feedback automatically; greenfield whole-shell builds to the stack skill's mount order |
| Route mount point | the react-router aggregate file (`src/app/router.tsx` etc.) | Touch list: one DEV-guarded registration |
| HTTP client and request layer | `package.json`, `src/api/**` or `src/services/**` (axios instance / fetch wrapper) | Mock interception point; handlers mirror real endpoint shapes |
| Theme source | where `createTheme` lives, CSS variables, PostCSS config (Mantine PostCSS plugin) | The prototype's visual authority; self-built component styles use `--mantine-*` variables |
| Layout shell | AppShell / navigation structure, default layout component | ProtoLayout wraps it — no bespoke shell |
| Icon set | icon package imports (one set across the project) | Same icons as the product; never emoji as structural icons |
| Business components | `src/components/**` etc. | Ladder rung 1 — reuse before building |
| The existing page closest to the requirement | search `src/**` for the nearest cousin | Pattern reference for file organization, naming, table / form / modal usage |
| Quality gates | `package.json` scripts: lint / typecheck / build | The gate commands after each page (run per page) |
| Does tsconfig include cover `proto/` | root `tsconfig*.json` | If not, add one line (touch list +1), otherwise tsc never checks prototype pages |

greenfield (no frontend in the project): this table serves only to confirm the "truly no frontend" judgment itself; the survey target becomes the greenfield whole-shell recipe in §2 — no pre-created empty directories.

## 2. The fence: `proto/**`

Canonical location: the frontend app root (next to `src/`). When the build cannot see `proto/`, do a minimal include (tsconfig `include` or vite alias) and put that config file on the touch list; only if truly infeasible fall back to `src/proto/**` and record the deviation in the manifest.

**greenfield variant (whole shell inside the fence)**: when the repo has no frontend yet, the entire app shell is set up inside `proto/` — `proto/` is the Vite app root, sharing the directory with `package.json` / `package-lock.json`, `index.html`, `vite.config.ts`, `tsconfig*.json`, `eslint.config.js`, `.gitignore`, `src/` (entry `main.tsx`, router `app/router.tsx`, theme, request layer, domain types). Run the prototype and all quality gates with `npm run dev` inside `proto/`; **zero new files at the repo root** (even `.gitignore` is `proto/.gitignore`). The first step of productionization = promoting `proto/` wholesale to `web/` (a one-time mechanical move, see SKILL.md Step 6) — that promotion is purely mechanical precisely because of this variant's fence invariant: the app shell lives inside the fence without exception and the repo root keeps zero residue; if the shell is scattered at the repo root, the first step can only be gathering it into the fence before talking promotion.

Layout:

```
proto/
├── manifest.json      # runtime contract (SKILL.md schema)
├── ue-spec.yaml       # design contract (reference/ue-spec.md)
├── gaps.md            # productionization ledger
├── ProtoRoutes.tsx    # route table + ProtoLayout wrapper
├── ProtoLayout.tsx    # PROTOTYPE banner + the app's real layout shell
├── pages/<page>.tsx   # one file per page, built from its spec entry
├── components/        # ladder rung 3 only — registered temporary components
├── mocks/             # HTTP-layer handlers, one file per page
├── feedback/          # exported review comments (one JSON per page)
└── review/            # vendored ue-inline-review.js
# greenfield whole-shell adds: package.json · index.html · vite.config.ts ·
# tsconfig*.json · eslint.config.js · .gitignore · src/ (entry/router/theme/request layer/types)
```

Touch-list discipline: brownfield typically 1–2 entries (router registration ± tsconfig / vite include); **greenfield whole-shell is always 0 — writing any file at the repo root is a violation**. If you find yourself changing business components, stores, or services — you have left the prototype and entered feature development; stop and confirm scope with the user.

## 3. Route gating (never ships)

`proto/ProtoRoutes.tsx` aggregates the prototype pages (one entry per page, all wrapped in ProtoLayout), default-exporting the route element:

```tsx
import { Routes, Route, lazy, Suspense } from 'react-router'
import { ProtoLayout } from './ProtoLayout'

const OrderList = lazy(() => import('./pages/order-list'))

export default function ProtoRoutes() {
  return (
    <Routes>
      <Route path="order-list" element={<ProtoLayout><Suspense fallback={null}><OrderList /></Suspense></ProtoLayout>} />
      {/* … one entry per page */}
    </Routes>
  )
}
```

Mount at `/proto/*` in the app's route aggregate file (the touch list's single business change) with a DEV guard + lazy:

```tsx
// src/app/router.tsx
import { lazy, Suspense } from 'react'
const ProtoRoutes = lazy(() => import('../../proto/ProtoRoutes'))

const router = createBrowserRouter([
  ...appRoutes,
  ...(import.meta.env.DEV
    ? [{ path: '/proto/*', element: <Suspense fallback={null}><ProtoRoutes /></Suspense> }]
    : []),
])
```

- **The DEV guard guarantees the routes never register in production and the lazy branch never loads**; a static import would bundle the prototype into the main chunk.
- Run the production leak guard before delivery: `vite build && grep -rl "PROTOTYPE\|/proto/" dist/` must output nothing; if a proto chunk shows up in dist (dead weight not DCE'd), switch to the whole-module dynamic-import variant and note it in the manifest.
- A second lock (optional for paranoid repos): the `/proto/*` host element checks `location.search` for `proto=1` and renders null otherwise; if used, note it in the manifest.
- Never write prototype routes into menus, navigation data, or sitemaps — URL reachability is enough for review.
- greenfield whole-shell mode: the app's route table itself lives inside the fence (`proto/src/app/router.tsx`, dev is the only entry) — the DEV guard still handles mock installation and `/proto/*` route registration (a top-level `lazy(() => import(...))` declaration is collected into a chunk unconditionally by the bundler; the `import()` must be written inside the guard branch, whole-module dynamic-import variant), the manifest records `gate: "greenfield-shell"`; after productionization (once `proto/` is promoted wholesale to `web/`) these routes become regular route declarations.

## 4. Component resolution, landed inside proto

- Ladder rung 1: explicit imports of project business components — reuse before building.
- Ladder rung 2: `@mantine/core` / `hooks` / `form` / `notifications` / `modals` (packages the project already has) per the stack skill's conventions — function components, no `defaultProps` / `forwardRef` (React 18 legacy projects per the stack skill's brownfield.md), the color prop is `c`, forms `useForm` uncontrolled + `form.key('path')`, confirmation modals `openConfirmModal`.
- Ladder rung 3 temporary self-built components: styles only from Mantine CSS variables (`--mantine-color-*` / `--mantine-spacing-*`) + CSS Modules (the project's PostCSS pipeline); no emotion, no inline hardcoded colors, no second styling system; files live in `proto/components/`, one gaps.md line each.
- React Compiler present (greenfield): no hand-written memoization; brownfield React 18 per brownfield.md.

## 5. ProtoLayout: banner + layout shell + review bootstrap

```tsx
// proto/ProtoLayout.tsx
// Review runtime: dynamically loaded only during a review session, never in the business entry
if (new URLSearchParams(location.search).has('uer')) {
  import('./review/ue-inline-review.js')
}
// Review page registry (recommended, baked page by page from the ue-spec page list): when pages switch,
// the runtime resolves page identity from it, and comments, anchor naming, and export follow the current page;
// falls back to the last route segment by default (only pageSource / ueControlId absent)
// window.__UE_REVIEW_META__ = {
//   pages: [
//     { match: '/proto/order-list', page: 'order-list', pageSource: 'proto/pages/order-list.tsx', controls: [] },
//     { match: '/proto/order-detail', page: 'order-detail', pageSource: 'proto/pages/order-detail.tsx', controls: [] },
//   ],
// }

export function ProtoLayout({ children }: { children: React.ReactNode }) {
  // Review entries: "Review" appends ?uer=1; "mockError" toggles ?mockError=1 (other params preserved)
  // The banner text during review guides the reviewer (press M, then point and click) — guidance text
  // lives in the banner (the review layer), never in the business DOM
  const q = new URLSearchParams(window.location.search)
  const reviewing = q.has('uer')
  const mockErrorOn = q.get('mockError') === '1'
  const bannerHref = (patch: Record<string, string | null>) => {
    const next = new URLSearchParams(window.location.search)
    for (const [k, v] of Object.entries(patch)) {
      if (v === null) next.delete(k)
      else next.set(k, v)
    }
    return `${window.location.pathname}?${next.toString()}`
  }
  return (
    <>
      <Alert color="orange" variant="light" mb="md">
        PROTOTYPE · mock: intercepted ·{' '}
        {reviewing ? (
          <>Reviewing · press M for point-and-click annotation (click any element to comment, Enter saves, Esc exits)</>
        ) : (
          <a href={bannerHref({ uer: '1' })} style={{ fontWeight: 600 }}>
            Review
          </a>
        )}
        ·{' '}
        {mockErrorOn ? (
          <>mockError=1 error-state demo (<a href={bannerHref({ mockError: null })}>off</a>)</>
        ) : (
          <a href={bannerHref({ mockError: '1' })}>mockError error state</a>
        )}
      </Alert>
      {/* The app's real layout shell: brownfield reuses the project's AppShell / layout components; greenfield uses a minimal shell (navigation lists prototype pages only) */}
      {children}
    </>
  )
}
```

- The banner text must guide the reviewer: when not reviewing, the "Review" link (appends ?uer=1) and the "mockError" link (appends/removes ?mockError=1) are both clickable with other params preserved; while reviewing it shows the operating guide "Reviewing · press M for point-and-click annotation (click any element to comment, Enter saves, Esc exits)" — guidance text lives in the banner (the review layer), never in the business DOM.
- Visually the banner must be loud (a screenshot must not pass for the product) without getting in the reviewer's way; it shows the current mock channel (intercepted / live-readonly).
- Params the "Review" link preserves (e.g. `?mockError=1`) must actually demo: the interception layer must do page-level injection and exempt identity-probe endpoints (rules in §6 state toggles).
- Prototype page feedback always goes through the project's feedback channels: `notifications.show` (success / error), `openConfirmModal` (destructive double-confirm) — the prototype's interaction feel must be the product's feel.

## 6. Mock backend — channel one: HTTP-layer interception (default)

Pick the interception point by the app's HTTP layer, not by taste. One handler file per page; handlers mirror real endpoints' paths and response shapes (copy shapes from service-layer types — the prototype must be both mockable and swappable to real).

| App HTTP layer | Interception point | Notes |
| :--- | :--- | :--- |
| axios shared instance | request / response interceptors, registered in `proto/mocks/index.ts`, matched by method + URL prefix | Cheapest, zero new deps; the interceptor registration is imported only where proto mounts, never in business code |
| fetch (global or wrapped) | monkey-patch `window.fetch` in the same proto-only bootstrap, restored on HMR dispose | Same shapes as above |
| MSW already present | reuse the app's MSW worker + proto-scoped handler files | First choice when the app already uses MSW |
| None fits (e.g. dev proxies straight to a real backend) | dev-server middleware (vite `configureServer` / proxy bypass) serving `/proto-api/**` | Prototype pages call `/proto-api/**`; the vite config file goes on the touch list |

Handler rules:

- **Deterministic**: seeded / fixed data, stable ordering — if a reviewer sees different rows twice in a row, they stop trusting the prototype.
- **Realistic volume**: enough to page / scroll (≥ 3 screens), plus edge rows — the longest text a column can hold, minimum / maximum numbers, special characters, one record with every optional field empty.
- **Content realism**: thousand separators on numbers, currency and precision on amounts, localized date formats, and copy from realistic business samples in the product's language (not lorem / test123) — reviewers lose trust when they spot fake data (floor in reference/design-decisions.md, "content realism").
- **Pages are the product's final cut**: present only end-user-facing product content; review comments, decision notes, to-be-confirmed markers, and other discussion content never enter the page DOM (they live in the review layer and the ue-spec / feedback exports) — notes for reviewers go in the review package, not on the page.
- **State toggles**: every handler gets a `delay` (default 300–800ms so loading states are visible) and an `error` toggle (an exported flag or `?mockError=1`) — the state matrix stays demonstrable through them, with no code changes mid-review.
- **Page-level `?mockError=1` must really work**: the interception layer injects the page URL's `?mockError=1` into all /api requests — only then is the convention "append a URL param to demo the error state" honored (the banner's "Review" link preserves the param, so it survives entering review).
- **Identity probes are exempt from error injection**: page-level injection must exempt identity-probe / session endpoints (e.g. `GET /api/me`) — taking one down makes the page misread "service error" as "not logged in", visually logging out logged-in users; the no-permission state's semantics take precedence over error demos: not logged in still shows the login guidance (correct), and data-error states are demoed only after login.
- **Realistic latency**: a prototype that returns data in 0ms will fall over in the real world's 400ms.
- **Writes**: mock writes mutate in-memory state (create → it appears in the list; the UI stays honest), and every call logs a `[proto-mock]` console marker.

## 7. Mock backend — channel two: read-only passthrough (only on explicit request)

Only when the user explicitly asks in the current session and the data is read-safe: GET passes through to the real backend, **all non-GET methods are intercepted locally** (`405` + a console warning naming the blocked calls). Implemented as a whitelist variant of channel one's interceptors; the manifest records `channel: "live-readonly"` and the review package notes which numbers are real. Channel one is always the default.

## 8. TanStack Query wiring (the key to zero-rewrite productionization)

- Prototype pages use TanStack Query for the data plane (greenfield v5; brownfield v4 projects keep the v4 API, per the stack skill's brownfield.md): queryKey organization, enabled derivation, precise invalidation all per the stack skill's conventions — the same patterns as production pages; **hand-rolled `useEffect` + fetch + `setState` data fetching is forbidden**.
- `queryFn` calls the project's request layer → mocks intercept at the HTTP layer, the query layer is unaware of mocks → zero query-layer rewrites when swapping to real at productionization.
- greenfield whole-shell: `QueryClient` defaults (`staleTime` / `retry`) configured centrally per the stack skill; per-page mock data and queryKey namespaces (e.g. `['proto-orders', filters]`).

## 9. Review bootstrap (everything lives inside proto/**)

1. Copy this skill's `assets/ue-inline-review.js` → `proto/review/ue-inline-review.js` (vendored inside the fence; deleting `proto/**` removes everything).
2. Dynamic `?uer=1` import inside the ProtoLayout module (see §5) — the review bootstrap never enters business entry files.
3. Bake the `window.__UE_REVIEW_META__ = { pages: [...] }` registry inside ProtoLayout from the page list (per page `match`/`page`/`pageSource`/`controls`); when absent, the runtime falls back to the last route segment — only pageSource and ueControlId anchor naming are missing, page-switch following and export are unaffected.
4. Reviewer conventions (write them into every review package): **once review is on, switching pages freely is fine** — the review session is global, the runtime automatically follows the current page and loads its comments, the panel title shows the current page name; the review pill is draggable (position remembered) — drag it aside when it covers a target element.

## 10. Per-page loop (page-by-page discipline)

1. Generate `proto/pages/<page>.tsx` from that page's spec entry (component mapping, interactions, states).
2. Wire the handler (data per the spec's `edgeData`).
3. Run the project's lint + typecheck (the commands probed in Step 1); only then touch the next line.
4. Smoke test: route reachable, main interactions work, declared states demonstrable via toggles, banner visible.
5. This page introduced a rung-3 component or a new touch point → update `gaps.md` / manifest. Next page.

Never sweep through ten pages and then debug: a new page inherits a green pipeline, so any failure belongs to the most recent ~100 lines — locality is the whole point.

## 11. Review-package template (deliver in the user's language)

```
The prototype is ready (proto/**, routes /proto/**, dev-only)
Spec: proto/ue-spec.yaml (status: draft, confirmed / pending confirmation)
- Run: brownfield `<package manager> run dev` at the app root; greenfield whole-shell `cd proto && <package manager> run dev`. Then visit:
  - /proto/order-list — order list: pagination, filtering, row click into detail;
    focus on: empty-state copy, the error toast under ?mockError=1
  - /proto/order-detail — detail: edit modal, validation, save (mocked)
- State toggles: ?mockError=1 error state; loading defaults to a 600ms delay; click the "Review" link on the banner (appends ?uer=1 to the current URL) to start point-and-click annotation
  (**press M** to enter pick mode, click page elements to comment, Esc to exit; keystrokes inside inputs do not trigger it.
  The review pill is draggable anywhere — drag it aside when it covers a target element. The review session is global:
  switching pages follows automatically, and the panel shows the current page's comments; exports are named after the current page)
- Known gaps (excerpt from gaps.md): the advanced filter is a temporary self-built component, pending the design system
- Decisions needed from you: default list order — placement time or status?
```

Screenshots / screen recordings: only existing repo tools (if any), recording the path; never add Playwright-like dependencies for the review package — a clickable prototype is itself the evidence.
