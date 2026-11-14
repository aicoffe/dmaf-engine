# Pinia, vue-query, and vue-router conventions and pitfalls

> Applies to @tanstack/vue-query v5 / Pinia v3 / vue-router 4.1+. When project dependencies below the baseline are detected
> (brownfield mode), this document's "forbidden / required" items downgrade to suggestions; the compatibility mapping in `reference/brownfield.md` governs
> — in particular, vue-query v4 projects keep the v4 API (`isLoading` / `cacheTime` / `keepPreviousData: true`),
> not rewritten per v5; the v5↔v4 comparison table is in brownfield.md.

Required reading when a task touches stores, API data requests, routing / guards / URL state.

## Pinia

- Install order: `app.use(createPinia())` → `app.use(router)` → `app.mount()`; using a store in a guard while Pinia is not installed yet errors with "getActivePinia() was called but there was no active Pinia"
- `useXxxStore()` is called only in setup, composables, and the function bodies of guards / interceptors / request functions; calling at module top level is forbidden (module loading precedes Pinia activation)
- setup store: `ref` is state, `computed` is getter, functions are actions; return all state — "internal" state is a `_`-prefix convention, not omission
- setup stores have no built-in `$reset()`; implement a `reset` action yourself when needed
- Inside components, `const { items, total } = storeToRefs(store)` takes state / getters and `const { load } = store` takes actions (`storeToRefs` excludes actions)
- Stores can use only application-level injections (e.g. `useRouter()`); values provided by component-level providers are unreachable — feedback always goes through the `shared/feedback.ts` singleton

## Server state (@tanstack/vue-query v5)

- `app.use(VueQueryPlugin, { queryClient })` installed globally once; `QueryClient` defaults (`staleTime`, `retry`) configured centrally
- queryKey is organized as `[resource, dimension, parameters…]` (aligned with the neutral frontend-development skill's cache key convention) and must contain all request parameters; pass parameters into queryKey as refs / computeds / getters — vue-query tracks their reactive values; a `.value` snapshot or destructured props will not refetch when the parameter changes
- props enter queries as getters: `useXxx(() => props.id)`; only `queryKey` and `enabled` accept getters, other options accept refs; read parameters inside `queryFn` with `toValue()`
- `queryFn` passes `signal` through to the request wrapper: in-flight requests cancel automatically on unmount or parameter switch (matching the neutral "automatic cancellation on unmount")
- Writes use `useMutation`, with `queryClient.invalidateQueries({ queryKey })` in `onSuccess` for precise invalidation; full cache clears are forbidden
- Three-state mapping: first paint with no data `isPending` (skeleton), background refresh `isFetching`, failure `isError` / `error`; keeping the previous page's data across pagination uses `placeholderData: keepPreviousData` (the v5 style; the old `keepPreviousData: true` has been removed)

## Routing (vue-router 4)

- Route tables split by domain into files, aggregated in `src/app/router`; route components are written `component: () => import('…')` (routing already handles async; wrapping in `defineAsyncComponent` again is forbidden)
- Guards: `async` functions + return-value semantics (`false` cancels, a route location redirects, `undefined` / `true` passes); async checks must be `await`ed; the third argument `next` is a legacy style kept officially for compatibility — forbidden in new / modified guards (legacy `next()` in brownfield code is not forced to change immediately, see brownfield.md)
- Redirect guards must exclude the target route itself (e.g. `to.name !== 'login'`); the protection scope is declared via meta fields like `meta.requiresAuth`
- `beforeEnter` fires only when entering from another route; params / query / hash changes on the same route do not trigger it: parameter-level validation goes in `beforeEach` or in-component `onBeforeRouteUpdate`
- When only params change on a same-component route, the instance is reused and `onMounted` does not re-run: track data via `() => route.params.id` (watch or queryKey); `<RouterView :key="route.fullPath" />` forces a costly rebuild — use it only when state must fully reset
- Route params carry only IDs (neutral frontend-architecture-guide); route config sets `props: true` and components receive params as props, easing testing
- URL query state: read `route.query`, write `router.replace({ query })` (`replace` for input-like filters to avoid history pile-up); defaults are not written into the URL; `@vueuse/router`'s `useRouteQuery` is available
- KeepAlive only for list pages that truly need "return and keep": set `max`; `include` matches by component name (`<script setup>` infers it from the filename, `defineOptions({ name })` when necessary); cached pages decide whether to refetch data in `onActivated`; for nested routes with KeepAlive, verify by testing whether child routes remount
