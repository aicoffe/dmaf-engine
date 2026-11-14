---
name: frontend-react-mantine
description: React 19 + Mantine 9 pinned-version stack binding (frontend). Function components and hooks conventions (defaultProps / forwardRef removal, memoization handed to React Compiler), Mantine 9 components and theming, TanStack Query v5 / Zustand v5 state, useForm + schemaResolver form validation, async/bundle/re-render performance conventions, the greenfield baseline version matrix, and the brownfield low-version compatibility mode with its verification loop. Loaded as an additional constraint on top of frontend-class skills when the project's stack declaration sets frontend_stack=react-mantine; use when the task touches "React/Mantine implementation/testing/engineering".
---

# React + Mantine Stack Binding (frontend-react-mantine)

This skill is the binding implementation for the stack declaration `frontend_stack=react-mantine`: it lands the neutral spec in React and Mantine idioms.
greenfield pins **React 19 / Mantine 9 / TanStack Query 5 / Zustand 5 / zod 4 / react-router 8 / Vite 8 / Vitest 5** per the baseline matrix;
existing projects (brownfield) adapt downward per `reference/brownfield.md` without upgrading dependencies.
Where this document conflicts with the neutral spec, the more specific one here wins; where this document is silent, fall back to the neutral spec.

## Project mode (must be determined before starting a task)

| Mode | Detection condition | Behavior |
|---|---|---|
| greenfield (default) | No lock file, or dependency versions fall within the baseline matrix | Execute all conventions of this skill |
| brownfield | Any critical dependency detected below the baseline matrix | Load `reference/brownfield.md` and adapt downward per the compatibility mapping |

**Under brownfield, upgrading any major dependency on your own is forbidden**; enter the migration flow only when the user explicitly requests "migration / upgrade"
(red lines in the legacy-project red lines section of `reference/brownfield.md`).

## greenfield baseline version matrix

All greenfield dependencies are generated per the matrix below — **never write `latest`, never leave it empty**; the matrix does not apply to brownfield projects
(version probing and per-dependency adaptation in `reference/brownfield.md`).

| Dependency | Version range |
|---|---|
| react / react-dom | ^19.3.0 |
| @mantine/core / hooks / form / notifications / modals | ^9.7.0 |
| @tanstack/react-query | ^5.105.0 |
| zustand | ^5.0.0 |
| zod | ^4.6.0 |
| react-router | ^8.5.0 |
| vite | ^8.0.0 |
| vitest | ^5.0.0 |
| @testing-library/react | ^16.3.0 |
| @testing-library/user-event | ^14.6.0 |

**Node runtime baseline**: `^22.12.0 || ^24 || >=26` (a hard requirement of Vitest 5).

When generating `package.json`, use the exact caret ranges from the table above — **never write `latest`, never leave it empty**.

## Framework conventions (React 19)

- Function components + hooks; class components are forbidden
- **`defaultProps` is forbidden**: React 19 removed function-component support; defaults are always provided via parameter destructuring
- **No `forwardRef`**: pass `ref` directly as a regular prop; a ref cleanup function can be returned directly from the ref callback
- TypeScript strict mode; props are declared explicitly as interfaces, types co-located with the component file
- React Compiler has taken over automatic memoization (1.0 released 2025-10): **hand-writing `useMemo` / `useCallback` around every value and callback is no longer the default**; intervene manually only when the Profiler proves a hotspot
- Prefer React 19 native capabilities for async actions: `useActionState` replaces hand-written pending/error state, `useOptimistic` handles optimistic updates, `useEffectEvent` stabilizes callback references inside effects
- Optimize performance only after the feature is correct and verified; locate hotspots first with the React DevTools Profiler / the network panel; techniques in `reference/performance.md`
- Read `reference/pitfalls.md` before starting, effective for the whole task (not something to consult only after something breaks)

## UI library (Mantine 9.7)

- Mantine 9.7.x, CSS Modules + CSS variables, no runtime CSS-in-JS (emotion has been removed since v7, **referencing it again is forbidden**)
- Prefer `@mantine/hooks` over hand-written hooks
- Feedback channels: `notifications` (`@mantine/notifications`) and `modals` (`@mantine/modals`); confirmation dialogs use `openConfirmModal` (details in reference/components.md)
- **The style color prop is uniformly `c`**: the `color` prop of `Text` / `Anchor` was removed in v9 and replaced by `c`
- The v9 default radius is `md` (v8 was `sm`); new projects keep `md` as-is, no fallback needed

## State management

- Server state: **TanStack Query v5.105+**; cache key / invalidation / staleTime follow the state-management conventions of the neutral frontend-development skill, **never copied into Zustand**
- All server-state fetching goes through Query (the same `queryKey` across instances dedupes and merges requests automatically); hand-written **`useEffect` + fetch + `setState` is forbidden**
- Client state: **Zustand v5**, stores split by business domain; component-local state stays in `useState`, never global
- Use `useShallow` when a Zustand selector returns objects / arrays; subscribing to the whole store is forbidden
- **TanStack Query v6 APIs are forbidden**; the v6 React adapter has not reached a stable release

## Forms and validation

- `useForm` with **explicit `mode: 'uncontrolled'`** (the officially recommended mode: values live in refs, typing does not re-render the form; the default is controlled);
  every input gets `key={form.key('path')}`, event handlers read `form.getValues()`, and render-time linkage uses `form.useWatchValue(path)`
- The in-stack validation default is zod + **`schemaResolver`** (Mantine 9 has built-in Standard Schema support; **`zodResolver` has been removed**);
  import zod from `zod` (or `zod/v4`); schema and types share one source (`z.infer`), hand-written parallel interfaces are forbidden
- Submit via `form.onSubmit`, button `loading={form.submitting}`; bypassing validation to submit directly is forbidden; server field errors
  are written back per dotted path with `form.setErrors`
- To swap records in an always-mounted edit modal, use `form.setInitialValues(record)` + `form.reset()`; the key-remount hack is forbidden
- Async validation (uniqueness checks etc.) returns a Promise from the rule and takes an `AbortSignal`; submission is blocked while `form.validating`
- Full conventions (binding details / list fields / dirty baseline / cross-component forms) in `reference/forms.md`, required reading before starting any form task

## Routing and engineering

- **react-router 8.5+**: register routes split by business domain; route-level lazy loading (`React.lazy` + `Suspense`); a single unified guard for permission routes
- **Vite 8** (Rolldown as the single bundler); bundle size budgets go into CI (neutral frontend-architecture-guide)
- This stack is a Vite CSR SPA: no SSR / hydration; **no Next.js-related dependencies or APIs** (`next/*`, RSC, Server Actions) —
  SSR needs are a stack change that goes through architecture review; do not slip them in inside a frontend task

## Provider mount order (application entry)

```
MantineProvider (theme) → Notifications → ModalsProvider
```

`notifications.show` must be called after `Notifications` is mounted.

## Verification loop

**greenfield**:

```bash
tsc --build && eslint . && vitest run
```

Run the full loop once per task; this section is the single authority for local loop commands. Type checking must go through `--build`: the root tsconfig of the official scaffold (create-vite react-ts) contains only `references` (`files: []`), so `tsc --noEmit` at the root checks zero files and always passes.

- Node baseline: `^22.12.0 || ^24 || >=26`
- Vitest 5 defaults `clearMocks` to `true`; if tests depend on mock call records across cases, set `clearMocks: false` explicitly in `vitest.config.ts`
- Component tests use RTL 16 + user-event 14 black-box assertions
- Visual regression against screenshot baselines (separate baselines for the light and dark themes)

**brownfield**: read `scripts` in the project root `package.json` first and use the project's existing `typecheck` / `lint` / `test` / `build` commands; do not invent missing commands; run the project's existing type check and tests at least once for version-sensitive changes (red lines in `reference/brownfield.md`).

## Reference files

- `reference/pitfalls.md` — high-frequency pitfalls of React 19 / Mantine 9 / TanStack Query v5 / Zustand v5 / zod 4 (required reading for implementation tasks, effective for the whole task)
- `reference/components.md` — component selection, the Combobox decision table, Provider mounting, wrapping Mantine-style components (required reading when a task touches components / interactions / dropdown selection / custom wrappers)
- `reference/forms.md` — full @mantine/form conventions: uncontrolled mode, bindings, validation, list fields, the dirty baseline, server errors, cross-component usage (required reading when a task touches forms / data entry / validation)
- `reference/performance.md` — performance techniques: network waterfalls, bundle and first paint, render economy, JS hotspots (required reading when a task touches performance optimization / long lists / jank / first-paint size)
- `reference/theme-tokens.md` — `createTheme` and design token mapping (required reading when a task touches theming / design system / style adjustments)
- `reference/brownfield.md` — legacy-project low-version compatibility mapping: precheck checklist, per-dependency downgrade rules, legacy red lines, verification strategy (loaded when the project-mode precheck determines brownfield; required reading when the user requests migration / upgrade)
