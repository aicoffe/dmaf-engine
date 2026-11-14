---
name: ue-prototyper-vue3-naiveui
description: 'Vue 3 + Naive UI high-fidelity UE prototype generation (code-native): builds runnable, clickable, reviewable, progressively productionizable UE prototypes inside the project''s real Vue 3 + Naive UI stack, replacing Axure/Figma static prototypes. Spec-first (proto/ue-spec.yaml), all output fenced inside proto/** (greenfield: whole app shell inside the fence), backend mocked at the HTTP layer, PROTOTYPE banner, point-and-click review loop (?uer=1), gaps.md productionization ledger; prototype page code always follows the frontend-vue3-naiveui skill conventions. Use when the project stack declares frontend_stack=vue3-naiveui and the user asks for a prototype / high-fidelity prototype / clickable demo / UE prototype / PRD-to-prototype / prototype review / Axure replacement, or wants to try out and review a feature before development. Other frontend stacks use their own prototype skill; production development does not go through this skill (this skill feeds it).'
---

# Vue 3 + Naive UI High-Fidelity UE Prototype (ue-prototyper-vue3-naiveui)

Build the prototype inside the product's own code: the same Naive UI components, the same theme tokens, the same layout shell, real interactions (validation, modals, table pagination, feedback channels), with the backend mocked at the HTTP layer. The prototype runs on the normal dev server, clickable end to end, supporting point-and-click annotation review; once review passes, the prototype code evolves directly into the production feature instead of being thrown away like an Axure file.

The discipline that keeps all of this from decaying into unreviewable vibe coding is **spec-first**: draft the structured UE spec first (`proto/ue-spec.yaml` — pages, roles, state matrix, component mapping, mocks), post it for confirmation, then generate page by page against the spec. A prototype can only be as coherent as the spec it rests on; revisions happen in the spec and propagate to the pages — never the other way around.

## Stack binding (must hold before you start)

- This skill is the prototype-generation implementation for `frontend_stack=vue3-naiveui`: **prototype page code always follows the `frontend-vue3-naiveui` skill** (`<script setup lang="ts">`, `v-model:xxx` binding naming, controlled values cleared to `null`, `n-data-table` remote pagination, `n-form` validation, component API checked against the official docs before writing) — read that skill and its reference routing before writing code; on brownfield projects also adapt per its `reference/brownfield.md`; prototype pages neither upgrade dependencies nor mix conventions
- The tech-stack declaration names another frontend stack → use that stack's prototype skill; do not mix stacks inside this skill
- Directory conventions follow the stack skill: greenfield uses `src/app/` / `src/domains/<domain>/` / `src/shared/`; brownfield keeps the project's existing directories

## Project modes (decide right after the survey)

| Mode | Detection | Behavior |
|---|---|---|
| brownfield anchored (primary mode) | The project already has a Vue 3 frontend app | The prototype lives in an isolated `proto/**` inside the real app, consuming the app's real component library / theme / layout shell, with a mocked backend |
| greenfield whole shell inside the fence | The project has no frontend yet (the product repo has no frontend directory yet) | The entire app shell is set up self-contained inside `proto/` (`proto/` is the Vite app root, `cd proto && npm run dev`), zero touch points and zero residue at the repo root; the prototype is the starting point of the future `web/` app, with the same spec-first and review discipline |

The minimal composition of the greenfield whole-shell-in-fence setup (everything lives inside `proto/`, no new files at the repo root — dependency manifest, entry HTML, build / typecheck / lint config, `src/` entry and request layer, `proto/.gitignore`): Vite + vue-router 4 + root `n-config-provider` (theme and themeOverrides) + the feedback singleton (`src/shared/feedback.ts`) + `@tanstack/vue-query` — the data plane goes straight through the project's request layer to the mock channel (mocks live at the HTTP layer, vue-query unaware), so the request layer needs zero rewrites at productionization time; the page skeleton grows out of the prototype's first page — no pre-created empty directories, no unused dependencies installed. A detected Vue 2.x is a stack change — route it through architecture review, not handled inside this skill. **The first step of productionization is promoting `proto/` wholesale to `web/` (a one-time mechanical move), see Step 6 — it holds precisely because of the whole-shell-in-fence setup here: with all artifacts inside the fence, the move is purely mechanical.**

## Hard rules (each one exists because it was violated before — and it fooled someone)

1. **Spec first.** Draft `proto/ue-spec.yaml` first, post its path and ask for confirmation, then generate pages against the spec — not by feel. Specs are cheap to revise; a set of pages is not. Confirmation is soft: send the invitation and continue if the user delegates, with the spec marked `draft`.
2. **Real constraints win.** The prototype consumes the app's real Naive UI components, theme / themeOverrides, icon set (xicons or the project's existing icon library), and layout shell; feedback goes through the project's real channels too (greenfield: the `shared/feedback.ts` singleton; brownfield: the project's existing Provider / feedback wrapper) — the prototype's interaction feel must be the product's feel. Never introduce a second component system, CSS framework, CDN link, or a forked theme; native alert / confirm / prompt are forbidden.
3. **Component resolution ladder.** When a page needs a component: first project business components, then Naive UI (including its props / slots / column `render` combinations), and only then is a minimal temporary self-built component allowed — built only from the app's theme variables, isolated in `proto/components/`, and registered in `gaps.md` as "pending replacement with a component-library component". A self-built component is deliberate debt, not a shortcut.
4. **Fence off and stay inside.** The prototype's **entire output** — for greenfield the whole app shell (dependency manifest, entry HTML, build / typecheck / lint config, `src/` entry and request layer), plus prototype pages, mocks, the review runtime, and the manifest / ue-spec / gaps ledgers — **lives inside `proto/**` without exception**. This fence invariant is the precondition of the productionization guidance: precisely because every artifact is already inside `proto/`, productionization can be a one-time mechanical move that promotes `proto/` wholesale to `web/` (see Step 6); if the shell is scattered across the repo root, no wholesale promotion exists. Brownfield: the prototype lives in `proto/**` under the frontend app root (fall back to `src/proto/**` only when the build cannot see `proto/`, and record the deviation in the manifest); business code changes only on a minimal, **pre-declared touch list** (one DEV-guarded vue-router registration in one place ± one tsconfig / vite include). Greenfield: whole shell inside the fence, `proto/` is the self-running app root, **the touch list is always empty — writing any file at the repo root (including package.json / tsconfig / .gitignore) is a violation**. Before writing any file outside the fence, add it to the touch list first; if a change cannot be expressed as a touch-list entry, stop and ask.
5. **Never ships.** Prototype routes register in dev only (`import.meta.env.DEV` guard + dynamic import, the whole block DCE'd from the production build); they never appear in production navigation, menus, or the static segment of the route table. Mocks never intercept outside dev. A prototype that leaked into the production build is an incident, not a shortcut.
6. **Honest mocks.** Mocks are deterministic and clearly labeled: every prototype page carries a visible "PROTOTYPE" banner, plus delay / error toggles so any state is available on demand. A page-level `?mockError=1` only takes effect when the interception layer injects it into all /api requests, and it must exempt identity-probe / session endpoints (e.g. `GET /api/me`) — otherwise "service error" gets misread as "not logged in" (logged-in users appear logged out); the no-permission state's semantics take precedence over error demos. A real backend channel is **read-only (GET) and opened only when the user explicitly asks** — never let one reviewer click execute a real write.
7. **State matrix complete and demonstrable.** Each page declares its state matrix in the spec — at least success, loading, empty, error, disabled, no-permission, plus edge data rules (long text, overflow, max rows) and the product's own responsive scope — and every declared state must be **demonstrable** (mock toggles, seed data, or role switching). A state that cannot be demonstrated does not count as covered; the approval of a prototype that only demos the happy path is void the moment the API returns its first 500.
8. **Manifest as contract.** `proto/manifest.json` always reflects reality: stack snapshot, pages / routes, touch list, mock channel, review rounds, productionization path. When it goes stale, fix it first, then do anything else — the prototype is auditable and removable only because of it.
9. **Zero-pollution review layer.** The review bootstrap lives only inside `proto/**` (vendored runtime + `?uer=1` dynamic import); never paste a script tag into any committed artifact.
10. **Anchored feedback.** Every review comment anchors to a uniqueness-verified selector bundle (CSS + XPath + fingerprint), exported as JSON — the export file, not chat memory, is the single source of truth for "what to change".
11. **Productionization ledger from day one.** `proto/gaps.md` is maintained from scaffolding onward: every mock-only endpoint, temporary self-built component, permission shortcut, missing instrumentation, and performance suspect gets logged the moment it appears. The productionization path (mock swap / spec handoff / discard) is recorded in the manifest at scaffolding time; cleanup must stay deterministic — delete `proto/**` + revert touch-list entries.
12. **Pages are the product's final cut.** Prototype pages **target the product's final result**: every word on a page must be what the end user will see — real product copy, real data shapes. **Presenting discussion and review content is forbidden**: review comments, review conclusions, decision notes, to-be-confirmed markers, and process explanations never enter the page DOM — discussion and review live only in the review layer (banner / pill / panel / pin / composer) and in ue-spec and the feedback export files; to leave the reviewer a note, put it in the review package or a ue-spec note.

## Workflow

### Step 0 — Triage: does a prototype already exist?

Look for `proto/manifest.json`, scan `proto/**`. Manifest exists and matches the requirement → **iterate** (straight into the review loop, or new pages enter the spec first and then the fence); exists but the requirement is new → extend (revise the ue-spec first, then add the new pages to the same fence, routes, and mock layer); missing → Step 1.

### Step 1 — Survey the codebase (one pass, then fix the mode)

Every conclusion must have an opened file as evidence: vue / naive-ui / @tanstack/vue-query / vue-router versions (judge greenfield / brownfield mode against the stack skill's baseline), route mount point (`src/app/router` or the project's existing aggregate file), HTTP client and request layer (axios instance / fetch wrapper / MSW — decides the mock interception point), theme source (where the `n-config-provider` theme / themeOverrides come from), layout shell (`n-layout` / menu / header), icon set, business components worth reusing, the existing page closest to the requirement (pattern reference), and the project's `package.json` scripts (they are the per-page quality gates). No frontend → greenfield whole shell inside the fence. Survey table: `reference/fence-mocks.md`.

### Step 2 — Draft the UE spec (spec first)

Bind the requirement to the strongest contract available, in this order (schema and protocol: `reference/ue-spec.md`):

1. The repo spec (produced by requirement-analysis) contains page / interaction / control-level contracts → they are the contract; ue-spec references and implements it, it does not invent it.
2. intent.md exists → its expected outcomes and design intent frame the ue-spec; where ambiguous, go back to the user — never guess.
3. Neither exists → interview (PRD, user stories, flow text, old Axure exports are all raw material): which pages, which scenarios, who uses it, which decisions this prototype must settle, what data to show, hard constraints.

Then write `proto/ue-spec.yaml`: pages with routes, roles, state matrices, component mappings (via the ladder, with `source` recorded), interactions, flows, mock data needs. **Post the spec path and ask for confirmation before generating pages** — if the user delegates, continue with the spec marked `draft`; revisions are expected and cheap.

### Step 3 — Scaffold, then generate page by page

The full recipe for fence layout, route gating, mock channel, and review bootstrap: `reference/fence-mocks.md`. The invariant structure:

```
proto/
├── manifest.json      # runtime contract
├── ue-spec.yaml       # design contract
├── gaps.md            # productionization ledger, logged as you build
├── routes.ts          # route-record array, dev-gated registration
├── ProtoLayout.vue    # PROTOTYPE banner + the app's real layout shell
├── pages/<page>.vue   # one file per page, built from its spec entry
├── components/        # ladder rung 3 only — registered temporary components
├── mocks/             # HTTP-layer interception handlers, one file per page
├── feedback/          # exported review comments land here (one JSON per page)
└── review/            # vendored review runtime
```

When greenfield puts the whole shell inside the fence, `proto/` additionally contains the entire app shell alongside the directories above: `package.json` / `package-lock.json`, `index.html`, `vite.config.ts`, `tsconfig*.json`, `eslint.config.js`, `.gitignore`, `src/` (entry `main.ts`, router `app/router`, feedback singleton `shared/feedback.ts`, request layer, domain types) — `proto/` is the Vite app root; run the gates and the dev server inside `proto/`, zero additions at the repo root.

**Generate one page at a time**: scaffold first (manifest, ue-spec, routes, ProtoLayout, mock skeletons), then loop page by page in spec order — build the page from its spec entry, wire the mock, run the project's lint + typecheck, smoke-check (route reachable, state toggles demonstrable, banner visible), then move to the next page. Never generate ten pages at once and go back to debug; small steps keep every page accountable to its spec entry and the gates. After scaffolding, report to the user: what was created, the touch list, the spec path, how to run it.

### Step 4 — Quality gates and the review package

Run the full gate set before delivery (any failure is a fix, not a delivery):

- The project's own lint, typecheck, and build pass (greenfield baseline `vue-tsc --build && eslint . && vitest run`); the dev server starts clean, with no console errors
- Every page route in the spec is reachable; the main flow is clickable end to end; the state matrix is demonstrable page by page (toggles work); the PROTOTYPE banner is visible
- Visual and accessibility floors pass item by item (`reference/design-decisions.md` checklist): dual-theme contrast, icon discipline, focus visibility, color never the only signal, no scattered hardcoded token values
- Token discipline: no hardcoded colors / spacing where theme variables exist; no self-built components off the ladder and unregistered
- Footprint guard: `git status --porcelain` shows only `proto/**` and touch-list files

Then deliver the **review package** (in the user's language; template in `reference/fence-mocks.md`): the run command; one URL per page plus what to try (key interactions and the states to look at); a mock-toggle quick reference; a summary of known gaps from `gaps.md`; the decisions that need a human call.

### Step 5 — Inline review loop (built in, zero dependencies)

The review runtime ships with this skill (`assets/ue-inline-review.js`, a zero-dependency IIFE): copy it to `proto/review/` when scaffolding; `?uer=1` activates it dynamically — after the reviewer presses M to enter pick mode, they click any element on a prototype page and write feedback in place (the review pill is draggable so it never blocks an element); comments anchor to uniqueness-verified selector bundles and export to `proto/feedback/comments-<page>.json` (the filename follows the panel's page name); the review session is global — client-side page switches automatically load the matching comments for the current page (pick mode persists across pages), page identity resolves via the `__UE_REVIEW_META__.pages` registry baked into ProtoLayout, with unregistered routes falling back to the last path segment. The shepherding talk track, export schema, triage tree (visual → rule → new requirement → question), and the apply / wrap-up checklists are in `reference/review-loop.md`.

### Step 6 — Productionization

After the user accepts the prototype, execute the productionization path recorded in the manifest, with `gaps.md` as the work list:

0. **Wholesale promotion (greenfield first step)**: the precondition is rule #4's fence invariant — every artifact (including the app shell) already lives inside `proto/`. When it holds, productionization is promoting `proto/` **wholesale to `web/`** — a one-time mechanical move (rename/move the whole directory + install dependencies in the new location), not file-by-file cherry-picking and copying; when it does not hold (shell scattered across the repo root), return to the fence discipline and gather the shell into the fence first, then talk promotion. From then on, all productionization work (mock swap, moving into the official tree, removing the banner, adding tests) happens inside `web/`; after the move, fix the path references in the manifest / delivery index accordingly.
1. **Mock swap** (the default for lightweight features): move prototype components from `proto/**` into the app's real directories (keep DOM structure and class names; switch code style to production — add tests, pass all stack-skill gates), swap each mock-only endpoint for the real service call one by one (the vue-query layer untouched — mocks live at the HTTP layer), register real routes, remove the banner, and close out the ledger items one by one.
2. **Spec handoff**: register the prototype paths, ue-spec, and review conclusions as the design reference for detailed-design / frontend implementation tasks; keep the prototype until the feature ships.
3. **Discard**: once the prototype has answered its questions it can retire. Delete `proto/**`, revert touch-list entries, mark the manifest `retired` (or delete it).

Update the manifest status after productionization. Never leave a prototype half-moved: the mock layer either fully swaps to real, or the prototype stays exactly as it is.

## Manifest Schema

```json
{
  "version": 1,
  "mode": "brownfield-anchor | greenfield-shell",
  "status": "draft | in-review | accepted | productionized | retired",
  "stack": { "framework": "vue@3.5.x", "ui": "naive-ui@2.4x", "build": "vite@…", "router": "vue-router@4.x", "http": "axios@1.x" },
  "mount": { "basePath": "/proto", "gate": "import.meta.env.DEV + dynamic import", "entry": "proto/routes.ts" },
  "ueSpec": "proto/ue-spec.yaml",
  "gaps": "proto/gaps.md",
  "pages": [
    { "id": "order-list", "route": "/proto/order-list", "file": "proto/pages/order-list.vue",
      "comments": "proto/feedback/comments-order-list.json" }
  ],
  "touchList": ["src/app/router/index.ts"],
  "mocks": { "channel": "intercept | live-readonly", "handlers": "proto/mocks/", "toggles": "delay=600ms, ?mockError=1 per handler" },
  "reviewRounds": 0,
  "graduation": { "path": "mock-swap | spec-handoff | discard", "notes": "" }
}
```

The schema is add-only: add new fields, never rename old ones. Mode semantics: brownfield-anchor typically has 1–2 `touchList` entries; greenfield-shell is **whole shell inside the fence** — the app shell lives entirely inside `proto/`, self-running (`cd proto && npm run dev`), `touchList` is always `[]`, and productionization starts from promoting `proto/` wholesale to `web/` (a one-time mechanical move).

## Verification commands

```bash
# —— project quality gates (probed in Step 1; run inside proto/ for greenfield) ——
vue-tsc --build && eslint . && vitest run

# —— review runtime syntax (after vendoring) ——
node --check proto/review/ue-inline-review.js

# —— footprint guard (every line must be explainable by the manifest) ——
git status --porcelain   # greenfield: ⊆ proto/**; brownfield: ⊆ proto/** ∪ touch list
```

Browser-side review behavior (click → comment → export → schema fields) is exercised once per session — no browser automation needed.

## Red flags — stop immediately

| Impulse | Reality |
| :--- | :--- |
| "Just one page, skip the spec" | The spec is everything that keeps ten pages coherent and revisions cheap. Page one without a spec is where vibe prototyping begins. |
| "Tailwind / hand-written CSS would be faster" | A second styling system makes the prototype lie about the product. Visual customization goes only through themeOverrides and component props (stack-skill red line); "convenient" is the trap. |
| "No good fit in the component library — I'll build one quickly" | Ladder rung 3: build only from theme variables, isolate in `proto/components/`, register in `gaps.md`. An unregistered self-built component is hidden debt. |
| "I'll put the mock in the real service file, commented out" | Spread outside the fence makes the prototype unremovable. All mock logic stays in `proto/mocks/`; the touch list stays minimal. |
| "The reviewer wants real data — wire up the real API" | Read-only (GET) and only on explicit request. A reviewer clicking a real DELETE is the worst thing a prototype can do. |
| "Unsure about this page's design — I'll go by feel" | Design authority is ordered: the app's real system > the user's explicit direction > the built-in floors. Verify first (official docs / existing pages as reference), then act; floors in `reference/design-decisions.md`. |
| "Happy path is enough for round one" | The state matrix is the review checklist. Any pass that skips it is void at the first API error. |
| "This feedback is actually a feature — I'll add it while I'm here" | New requirements go back to spec / intent (or get an explicit scope confirmation from the user). A prototype is not a requirements document. |
| "Inject the review script straight into index.html / App.vue" | That file belongs to the app. The review bootstrap lives only in `proto/**`, dynamically imported via `?uer=1`. |
| "The greenfield shell is easier at the repo root" | The repo root gets occupied by package.json / node_modules / tsconfig, the prototype stops being wholesale removable, and the repo's identity changes from a workflow repo to a Node repo. Whole shell inside the fence: `proto/` is the app root, zero repo-root touch points; productionization promotes `proto/` wholesale to `web/` in one move. |
| "Add a 'pending review confirmation' section on the page so reviewers understand better" | The page is a rendering of the product's final result and carries only what the end user will see; review comments, decision notes, and to-be-confirmed markers are discussion content that lives only in the review layer and the ue-spec / feedback exports — notes for reviewers go in the review package, never into the page DOM. |

## Completion criteria

- [ ] ue-spec.yaml exists, was posted for confirmation, and matches the built pages (revisions propagated)
- [ ] The prototype runs on the normal dev server; every page is clickable end to end and every declared state in its matrix is demonstrable
- [ ] Only project business components and Naive UI were used (self-built went through the ladder and got registered); feedback goes through the project's real feedback channels; footprint ⊆ `proto/**` (greenfield whole shell inside the fence, zero repo-root touch points; brownfield plus the touch list)
- [ ] Project lint / typecheck / build pass; no console errors
- [ ] manifest, ue-spec, gaps.md are current; mock channel and toggles recorded; PROTOTYPE banner visible
- [ ] Review round complete: comments exported, triaged, applied or answered; status and spec rows updated
- [ ] Productionization path recorded; footprint clean after productionization (mock fully swapped to real, or the prototype preserved as-is)

## Reference files

- `reference/ue-spec.md` — required when drafting or revising the UE spec: YAML schema, contract precedence, state-matrix enumerations, component-mapping ladder (Naive UI mapping examples), confirmation and revision protocol
- `reference/fence-mocks.md` — required when scaffolding / wiring mocks after the survey: survey checklist, fence layout, vue-router gated mounting, the three mock channels and read-only passthrough, vue-query wiring, review bootstrap, review-package template
- `reference/design-decisions.md` — required for design questions the app does not answer (new page composition, empty states, banner, new visual direction): authority order and verification discipline, spacing / contrast / icons / motion / form UX / content realism floors, token discipline, pre-delivery visual checklist
- `reference/review-loop.md` — required when entering the review loop: activation and shepherding, export schema, triage tree, apply and wrap-up checklists
