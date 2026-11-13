# Naive UI component conventions

> Applies to Naive UI 2.37+. Entries note their version lines (`2.37+` / `2.29+` etc.); when project dependencies below the baseline are detected
> (brownfield mode), this document's "forbidden / required" items downgrade to suggestions; the compatibility mapping in `reference/brownfield.md` governs
> (e.g. at 2.28 and below, feedback goes through the Provider + `useMessage` pattern).

Required reading when a task touches components / interactions / page framework implementation. Forms, input components, and data tables are in reference/forms-tables.md.

## Selection

1. Ready-made Naive UI components (`n-*`); structured data uses `n-data-table`, display-only simple tables use `n-table`
2. `shared/` wrapped components; 3. new in-domain components (promotable to `shared/` after review)
- When Naive UI lacks a capability, first check `theme-overrides` and component props / slots, then consider building your own; introducing a second UI library is forbidden
- Layout: one-dimensional arrangement uses `n-flex` (2.37+; at 2.36 and below use `n-space`, see brownfield.md), grids use `n-grid` (CSS Grid based, responsive), not `legacy-grid`; page frames use `n-layout` + `n-layout-sider`
- Icons: render xicons icon components via `<n-icon :component="Icon" />`; hand-drawn SVGs must carry a `viewBox`

## Controlled values and bindings

- Controlled = passing the value `xxx` (`v-model:xxx` is controlled); listening only to `update:xxx` without passing the value is uncontrolled. `v-model:xxx` follows the component API's prop name (`value` / `show` / `checked` / `page` / `checked-row-keys` etc.)
- When the value is `undefined` (or absent), the component works uncontrolled: assign `null` to clear a controlled value; assigning `undefined` lets the component keep showing its stale internal value
- Additional callbacks alongside `v-model:value` are written `@update:value` (the two merge into one execution); writing `:on-update:value` is forbidden — it collides with the same-named prop compiled from v-model, the former gets overwritten, and two-way binding breaks
- In render functions: callbacks use camelCase `onUpdateValue`, and child content is always written as slot functions `h(NButton, { onClick }, { default: () => 'Edit' })`; passing strings / arrays directly as children triggers Vue's "Non-function value encountered for default slot" warning

## Feedback channel implementation

Implementation details of SKILL.md's "Feedback channel".

- `src/shared/feedback.ts` exports the `message` / `dialog` / `notification` / `loadingBar` created by `createDiscreteApi` (2.29+); create at module level — when the theme source lives in a Pinia store, create lazily after `app.use(pinia)`; creating inside setup is forbidden. At 2.28 and below there is no discrete API; use the Provider + `useMessage` pattern (see brownfield.md)
- Pass `configProviderProps` as a `computed` taking the same source as the root `n-config-provider` (theme / themeOverrides / locale / dateLocale) — discrete is a separate app, in-app providers have no effect on it, so sync manually
- Durations and counts: `messageProviderProps: { duration: 2000, max: 3 }` — the 2-second success notice aligns with the neutral interaction spec (the library default is 3000), and `max` prevents floods when concurrent requests fail; errors that must linger take per-call `duration` / `closable`; `notification` without a `duration` never auto-closes — set it explicitly as needed
- `message.loading()` returns a mutable MessageReactive: change its `type` / `content` when done, or call `destroy()`, leaving no residual notices
- The request layer (neutral frontend-architecture-guide) translates errors into user-readable text in the response interceptor and notifies via `message.error`, never surfacing raw technical detail; routing calls `loadingBar.start()` / `finish()` / `error()` in `beforeEach` / `afterEach` / `router.onError` respectively
- The three loading layers do not substitute for each other (aligned with the neutral interaction loading states): page / block first paint uses `n-skeleton`, button level uses the `loading` prop, route level uses loadingBar; `n-spin` only for local block refreshes, full-screen spinners are forbidden
- Empty states use `n-empty` (distinguish "no data" from "no results for this filter" — the latter offers a clear-filters entry), error states land in `n-result` + a retry button; success feedback uses `message.success`, no modal interruptions
- Destructive operations are double-confirmed with wording that states the consequence: lightweight cases like table rows use `n-popconfirm` (`@positive-click` runs the operation), batch or irreversible operations use `dialog.warning()` / `dialog.error()`

## Menus and routing

- The side menu uses `n-menu` `options`: `label` written as `() => h(RouterLink, { to: { name } }, { default: () => 'Title' })`, `key` as the route name, `icon` written as `() => h(NIcon, null, { default: () => h(Icon) })`
- Highlight `:value` is a `computed` from the current route (e.g. `route.name`); no duplicate state kept on the side
- Pair with `n-layout-sider` collapse: `n-menu`'s `collapsed` follows the sidebar, and both sides set the same `collapsed-width` (the menu must set it explicitly)

## Wrapped components (wrapping `n-*` under `shared/`)

- When the root element is not the wrapped `n-*`: `defineOptions({ inheritAttrs: false })`, and `v-bind="$attrs"` on the target `n-*`
- Forward all slots wholesale: `<template v-for="(_, name) in $slots" #[name]="slotProps"><slot :name="name" v-bind="slotProps ?? {}" /></template>`
- Optional boolean props forwarded to `n-*` (especially `disabled`) default explicitly to `undefined`: Naive UI inherits the disabled state from `n-form` / `n-form-item` only when `disabled === undefined`; Vue's default conversion to `false` cuts off the inheritance; do not default `size` at the wrapper layer either, for the same reason
- Import prop types from `naive-ui` (`InputProps`, `SelectProps` etc.) to extend; do not hand-write mirrors; when the parent needs to call internal methods (e.g. a form's `validate`), `defineExpose` a minimal forward

## Overlays and modals

- `n-modal`, `n-drawer`, `n-popover`, `n-dropdown`, `n-select` menus, `n-tooltip` etc. teleport their content to `body` by default: own elements inserted inside still carry this component's scoped attribute, so class selectors work; but selectors relying on DOM ancestry like `.wrapper :deep(.n-xxx)` fail on them — adjust overlay appearance via the component's own `class` / `style`-like props (per the component API) or `theme-overrides`
- When rendering in place is needed (e.g. positioning breaks inside a transform container): the popover family (popover, popconfirm etc.) and `n-select` set `:to="false"` to stay inside the trigger container; `n-modal`'s `to` only accepts a selector / element, designating the mount point
- `n-modal`'s `display-directive` defaults to `if`: closing destroys the content and its state; set it to `show` when filled-in progress must be preserved
- Form modals set `:mask-closable="false"` (default `true`; a mis-click on the mask loses input); rebuild the model with a factory function before each open and call `restoreValidation()`
- In tests, overlay content is not inside the `wrapper` subtree — see reference/verification.md for how to find it
