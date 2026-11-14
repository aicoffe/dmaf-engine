---
name: frontend-vue3-naiveui
description: Vue 3 + Naive UI pinned stack binding (frontend). Pins Vue 3.5+ Composition API (script setup + TypeScript), Naive UI (on-demand registration, v-model:xxx binding naming, createDiscreteApi as the single feedback channel), Pinia + @tanstack/vue-query, vue-router 4, Vite, Vitest + Vue Test Utils; covers responsiveness and component contracts, form validation, data tables, theme tokens, the verification loop, and the brownfield low-version compatibility mode. Loaded as an additional constraint on top of frontend-class skills when the project's stack declaration sets frontend_stack=vue3-naiveui; use when the task touches "Vue/Naive UI implementation/testing/engineering".
---

# Vue 3 + Naive UI Stack Binding (frontend-vue3-naiveui)

This skill is the binding implementation for the stack declaration `frontend_stack=vue3-naiveui`: it lands the neutral spec in Vue 3 and Naive UI idioms.
greenfield pins **Vue 3.5+ / Naive UI 2.37+ / Pinia / @tanstack/vue-query v5 / vue-router 4 / Vite / Vitest**;
existing projects (brownfield) adapt downward per `reference/brownfield.md` without upgrading dependencies.
Where this document conflicts with the neutral spec, the more specific one here wins; where this document is silent, fall back to the neutral spec.

## Project mode (must be determined before starting a task)

| Mode | Detection condition | Behavior |
|---|---|---|
| greenfield (default) | No lock file, or no critical dependency below the baseline | Execute all conventions of this skill |
| brownfield | Any critical dependency detected below the baseline | Load `reference/brownfield.md` and adapt downward per the compatibility mapping |

**Covers Vue 3.x only**: detecting Vue 2.x is judged a stack change that goes through architecture review and is not handled within this skill
(Naive UI does not support Vue 2; the "Vue 2 + Naive UI" combination does not exist).

**Under brownfield, upgrading any major dependency on your own is forbidden**; enter the migration flow only when the user explicitly requests "migration / upgrade"
(red lines in the legacy-project red lines section of `reference/brownfield.md`).

## Framework conventions

- Vue 3.5+ (greenfield), version anchor subject to the root package.json; legacy low versions adapt per the Vue version downgrade mapping in `reference/brownfield.md` (e.g. `useTemplateRef` → a typed `ref`, props destructuring defaults → `withDefaults`), no unauthorized upgrades
- **greenfield**: Composition API + `<script setup lang="ts">` is the only style; Options API and mixing the two are forbidden. **brownfield**: new components use `<script setup>`, legacy Options API components are not forcibly migrated, coexistence is allowed. SFC block order is `<script>` → `<template>` → `<style>`
- TypeScript strict mode; props / emits / slots use the typed macros (`defineProps<P>()`, `defineEmits<{ save: [payload: P] }>()`, `defineSlots`); mixing runtime declarations with type declarations inside the same macro is forbidden
- Read `reference/pitfalls.md` before starting, effective for the whole task (not something to consult only after something breaks)
- For general composition capabilities (event listeners, debounce/throttle, local storage, element size etc.) prefer VueUse; do not re-write them
- For non-trivial features, draw component boundaries first: who owns the state, and which of props / emits / `v-model` / typed provide-inject carries the cross-boundary contract; local fixes keep the existing boundaries
- Optional features — slot forwarding, KeepAlive, Teleport, Suspense, render functions — are introduced only when a requirement appears; render functions only where templates cannot express it (e.g. the `render` of an `n-data-table` column)
- Optimize performance only after the feature is correct and verified; locate hotspots with Vue DevTools first

## UI library (Naive UI)

- Naive UI 2.37+ (greenfield; `n-flex`, the `warnings` of `validate()` etc. depend on this version), version anchor subject to the root package.json; legacy low versions adapt per the Naive UI version downgrade mapping in `reference/brownfield.md` (e.g. `n-space` in place of `n-flex`), no unauthorized upgrades
- **greenfield**: components in templates are registered on demand via unplugin-vue-components + `NaiveUiResolver`; full registration (`app.use(naive)`) is forbidden. **brownfield**: keep the project's existing registration style (full registration or manual imports), new components follow the project's existing approach, and introducing unplugin is not forced. Components used in render functions are always imported by name from `naive-ui`
- Component APIs follow the official Markdown docs (`https://www.naiveui.com/zh-CN/components/<component>.md`, index `https://www.naiveui.com/zh-CN/llms.txt`; offline, defer to naive-ui's type declarations in node_modules); look up uncertain props / event names before writing, never guess from memory
- Two-way binding is written `v-model:xxx` per the component's prop name: inputs use `v-model:value`, `n-modal` / `n-drawer` use `v-model:show`, `n-checkbox` uses `v-model:checked`, and `n-data-table` selection uses `v-model:checked-row-keys` — a bare `v-model` binds `modelValue`, which has no effect on Naive UI components
- Clearing a controlled value assigns `null`: a value of `undefined` equals not passing it, and the component falls back to uncontrolled, continuing to show its stale internal value (the same applies to resetting forms)
- Icons use the officially recommended xicons (`@vicons/*`, one icon set per project), rendered via `<n-icon :component="Icon" />`
- One `n-config-provider` at the application root, unifying `theme` / `theme-overrides` / `locale` / `date-locale` (Chinese products use `zhCN` / `dateZhCN`, the default is English); theming details in reference/theme-tokens.md
- Style customization goes only through `theme-overrides` and the components' own props; `:deep(.n-*)` overrides of component internal classes and global CSS hacks are forbidden; introducing a second UI library is forbidden

## State management

- Local first, then lift: UI-local state stays in components; client state shared across components goes into Pinia; module-level exported mutable `reactive` objects posing as global stores are forbidden
- Client state: Pinia setup stores (the same mental model as `<script setup>`), split by business domain as `use<Domain>Store`; all state must be returned (what is not returned stays out of DevTools and plugins); destructure state / getters inside components with `storeToRefs`, destructure actions directly
- Server state: @tanstack/vue-query v5 (`useQuery` / `useMutation`); cache keys, precise invalidation, and staleTime follow the state-management conventions of the neutral frontend-development skill; copying query results into stores is forbidden; legacy v4 projects keep the v4 API and adapt per the v5↔v4 comparison in `reference/brownfield.md`, without being rewritten per v5
- Page-level list filters / sorting / pagination / active tab go into the URL query (shareable, survives refresh, replayable via back/forward), not into stores
- Traps such as install order, reactive `queryKey` parameters, and using stores inside guards are in reference/state-routing.md

## Forms and validation

- `n-form` binds `:model` + `:rules` (type `FormRules`); every `n-form-item` sets `path` matching a model field exactly (nested: `user.name`, `items[0].name`) — items missing `path` get neither the model value nor a matching rule
- Rule `trigger` aligns with the interaction (`blur` validates a single field on blur, full validation at submit); async validation (uniqueness checks etc.) is a `validator` / `asyncValidator` returning a Promise; duplicate submission during validation and submission is forbidden
- Submit via `formRef.value?.validate()`: returns a Promise, rejects with `FormValidationError[]` on failure and must be caught; bypassing validation to submit directly is forbidden
- Custom form controls use `defineModel`; object / array models are replaced wholesale, not mutated in place (in-place mutation never emits `update:modelValue`)
- Reset: reassign the model's initial value via a factory function (clearing a controlled value uses `null`), then call `restoreValidation()` — it only clears validation state, never values
- Conventions for date / select / upload input components and data tables are in reference/forms-tables.md

## Routing and engineering

- **greenfield**: directory layout per the neutral frontend-architecture-guide — `src/app/` (`main.ts`, root component, router aggregation), `src/domains/<domain>/` (pages / components / api / model, composables `useXxx` in model/), `src/shared/` (cross-domain components, the request wrapper, the feedback channel singleton); **brownfield**: keep the project's existing directories, place new code per the existing structure, no forced refactoring
- vue-router 4: register routes split by business domain; lazy-load route components as `component: () => import(...)`; wrapping route components in `defineAsyncComponent` is forbidden
- Guards use return-value semantics (`false` cancels, returning a route location redirects, returning nothing passes); the third argument `next` is forbidden; exclude the target route itself before redirecting to prevent loops; permissions are uniformly decided in `beforeEach` per `meta` (no permission renders 403)
- When only the params of the same route change, the component instance is reused and lifecycle does not re-run: data depending on params is tracked via `watch(() => route.params.id)` or a `queryKey`; fetching only in `onMounted` is forbidden
- `app.use(createPinia())` before `app.use(router)`, both before `mount()`; Vite builds, bundle size budgets go into CI (neutral frontend-architecture-guide)

## Feedback channel

- **greenfield**: a single channel — `src/shared/feedback.ts` builds a singleton via `createDiscreteApi(['message', 'dialog', 'notification', 'loadingBar'], { configProviderProps })`, and everything inside or outside components (pages, request interceptors, route guards, store actions) takes it from there; do not mount `n-message-provider` and other feedback providers, do not use `useMessage` / `useDialog` / `useNotification` / `useLoadingBar` — discrete is unaffected by in-app providers, and the official docs advise against mixing it with the regular APIs. **brownfield**: keep the legacy Provider + `useMessage` pattern, new feedback prefers the project's existing feedback wrapper, and migration to discrete is not forced (details in `reference/brownfield.md`)
- The singleton must not be created inside setup; pass `configProviderProps` as a computed of the same source as the root `n-config-provider` (theme / themeOverrides / locale / dateLocale), otherwise feedback components do not follow light-dark and language switches
- Semantic split: light notices `message`, blocking confirmations `dialog`, important events `notification`, route transitions `loadingBar`; destructive operations require double confirmation (`n-popconfirm` for lightweight inline cases, `dialog.warning()` / `dialog.error()` for batch or irreversible operations); native alert / confirm / prompt in business code is forbidden
- Durations: `messageProviderProps.duration` set to 2000 (aligning with the neutral interaction's 2-second success notice; the library default is 3000); `notification` without a `duration` never auto-closes — set it explicitly as needed
- Rich-content modals (forms, components needing router or i18n injection) use in-component declarative `n-modal` (`v-model:show`): discrete is a separate app, and dialog content cannot access application-level injections

## Verification loop

**greenfield**:

```bash
vue-tsc --build && eslint . && vitest run
```

Run the full loop once per task; this section is the single authority for local loop commands. Type checking must go through `--build`: the root tsconfig of the official scaffolds (create-vue, create-vite) contains only `references` (`files: []`), so `vue-tsc --noEmit` at the root checks zero files and always passes. Component tests use Vitest + Vue Test Utils black-box assertions; visual regression against screenshot baselines (separate baselines for the light and dark themes); test paradigms and CI checks are in the reference files.

**brownfield**: read `scripts` in the project root `package.json` first and use the project's existing `typecheck` / `lint` / `test` / `build` commands; do not invent missing commands, and the baseline above is not required; run the project's existing type check and tests at least once for version-sensitive changes (details in `reference/brownfield.md`).

Pre-delivery self-check: derived values all go through computed, side effects are cleaned up with their owners, external contracts (props / emits / model) are typed, optional features are genuinely needed, and feedback goes only through the project's agreed feedback channel (`shared/feedback.ts` in greenfield).

## Reference files

- `reference/pitfalls.md` — Vue 3 idioms and high-frequency pitfalls: responsiveness, templates, component contracts, composables, lifecycle and async, performance (required reading for implementation tasks, effective for the whole task)
- `reference/components.md` — Naive UI component conventions: selection (layout, icons), controlled values and bindings, feedback channel implementation, menus and routing, wrapped components, overlays and modals (required reading when a task touches components / interactions / page framework)
- `reference/forms-tables.md` — forms, input components (date, select, upload) and data tables (required reading when a task touches forms, data entry, or table lists)
- `reference/state-routing.md` — Pinia, vue-query, and vue-router conventions and pitfalls (required reading when a task touches stores, API data requests, routing / guards / URL state)
- `reference/theme-tokens.md` — theming entry, design token mapping, style conflicts (required reading when a task touches theming / design system / style adjustments)
- `reference/verification.md` — test layering, component test paradigms (including Naive UI overlays and the feedback channel), CI checks (required reading when a task touches writing or fixing tests, CI / pipelines)
- `reference/brownfield.md` — legacy-project low-version compatibility mapping: precheck checklist, Vue 3.5→3.2 and Naive UI downgrade rules, the vue-query v5↔v4 comparison, relaxed architecture conventions, legacy red lines (loaded when the project-mode precheck determines brownfield; required reading when the user requests migration / upgrade)
