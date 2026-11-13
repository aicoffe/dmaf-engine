# Vue 3 idioms and high-frequency pitfalls

> Applies to Vue 3.5+ / Naive UI 2.37+. Entries note their version lines (`3.5+` / `3.4+` etc.); when project dependencies below the baseline are
> detected (brownfield mode), an entry's "forbidden / required" items downgrade to suggestions; the
> compatibility mapping in `reference/brownfield.md` governs (e.g. Vue 3.4 uses a typed `ref` and `withDefaults`).

Required reading for implementation tasks, effective for the whole task. Only error-prone spots and Vue-specific conventions are collected here; general JS / Web common sense is not repeated.

## Responsiveness

- Minimize source state; derive with `computed` always: deriving by assigning to another ref via `watch` / `watchEffect` is forbidden; computed getters stay pure (no requests, no state mutation, no emits) — side effects belong in a separate `watch`
- computed return values are read-only: never `sort()` / `reverse()` / `push()` them in place (it mutates the source array) — copy with `[...arr]` first; "computations" that take arguments become plain functions or computeds returning functions
- Selection: values replaced wholesale use `ref`; properties of a single state object mutated in place use `reactive` (wholesale reassignment forbidden); external instances (charts, editors, SDK clients), large read-only data, and component objects use `shallowRef` or `markRaw` — deep-proxying a component object triggers a Vue warning
- Destructuring a `reactive` object or store directly breaks reactivity: use `toRefs` / `storeToRefs`, or keep `obj.x` access
- Reading/writing a ref in script requires `.value`; templates auto-unwrap only top-level refs (`obj.count` still needs `.value` if it is a ref), and refs inside arrays / Maps do not auto-unwrap either
- `watch` sources: a reactive object's property is written as a getter `() => state.x`; `watch(state.x)` only captures a one-off value; for run-on-first too, use `{ immediate: true }` instead of duplicate `onMounted` + `watch`

## Templates

- `v-for` always carries a stable primitive `:key` (business id; index and objects forbidden); `v-if` and `v-for` never sit on the same element (filter with computed, wrap whole show/hide on an outer element)
- Templates carry only declarative bindings: filtering / sorting / assembly go into computeds; functions called in templates must be side-effect free
- `v-html` renders only sanitized (e.g. DOMPurify) trusted content
- Template refs use `useTemplateRef<T>('name')` (3.5+; at 3.4 and below use a typed `ref` + `ref="name"`, see brownfield.md); it is `null` before mount and under `v-if`, so always access via optional chaining

## Component contracts

- props are read-only: to change them, emit, use `v-model`, or keep a local copy; at 3.5+ use reactive destructuring `const { size = 'medium' } = defineProps<P>()`, and when passing into `watch` / composables wrap it as a getter `() => size` (3.4 and below lack reactive destructuring — use `withDefaults` + `props.` access, see brownfield.md)
- `withDefaults` array / object defaults must be factory functions `() => []`, otherwise all instances share one reference
- An optional boolean prop absent from the attributes is converted by Vue to `false`, not `undefined`; when "unset means inherit from above" semantics are needed, default explicitly to `undefined` (the Naive UI wrapping case in reference/components.md "Wrapped components")
- All events a component emits go into `defineEmits`: undeclared ones like `click` fall into `$attrs` and bind native listeners to the root element, causing double triggers; component events do not bubble — forward explicitly across layers or switch to provide / inject
- `defineModel` (stable at 3.4+; at 3.3 and below, manual `modelValue` + `update:modelValue`, see brownfield.md): the parent must supply the initial value (the child's `default` is not written back to the parent, leaving the two inconsistent); object / array models are replaced wholesale
- `defineOptions({ name, inheritAttrs })` (3.3+; at 3.2 and below export from a second plain `<script>` block, see brownfield.md): typically for a wrapped component's `inheritAttrs: false` and the component name for KeepAlive
- Parent-to-child prefers props / emits; when imperative calls are truly needed, the child `defineExpose`s a minimal API, which must be called before any `await` (the parent cannot reach it afterwards)
- provide / inject: `InjectionKey<T>` + read-only state + explicit actions; the provider sits at the nearest common ancestor and is called synchronously in setup; throw a clear error when a required dependency is missing, never silently fall back to a default; object defaults use factory functions

## Composables

- Extract stateful logic by responsibility (fine even with a single caller); pure computations are plain functions (utils), not wrapped as `useXxx`; optional configuration uses an options object; state that must change through actions is returned `readonly`
- Call only synchronously at setup top level (or inside lifecycle hooks): calling after an `await`, in callbacks, conditionals, or loops is forbidden — lifecycle hooks and inject registered inside silently stop working
- Accept `MaybeRefOrGetter<T>` arguments and read them with `toValue()` inside `watch` / `watchEffect` / `computed`; reading them early outside loses dependency tracking
- Side effects are cleaned up with their owner: event listeners, timers, and subscriptions released in `onUnmounted` (`onScopeDispose` inside composables), or via VueUse wrappers with automatic cleanup

## Lifecycle and async

- Lifecycle hooks, `watch`, and `provide` register synchronously in setup; a top-level `await` in `<script setup>` is the only exception (the compiler restores instance context); registering inside nested async functions silently stops working; components using top-level `await` need an ancestor `<Suspense>` — business pages do not abuse it, data fetching goes through vue-query's state
- Watchers created in `await` / `setTimeout` callbacks do not die with the component: prefer creating them synchronously and checking conditions inside the callback; when unavoidable, keep the stop handle and call it on unmount
- Race-proof async watches: use the callback's third argument `onCleanup` (or 3.5+'s `onWatcherCleanup`, called before the first `await`) to abort the previous request (`AbortController`) or discard stale results
- `watchEffect` only tracks dependencies accessed before the first `await`; for async scenarios switch to `watch` with explicit sources
- Read the DOM in `onMounted` or after `await nextTick()`; watchers that need the DOM after an update use `flush: 'post'`

## TypeScript

- Do not pass generic parameters to `reactive()`; annotate the variable's type instead (the generic parameter diverges from the ref-unwrapped return type)
- Narrow unions with `v-if` in templates before access; DOM event handlers annotate the event type explicitly
- `any` is blocked per the neutral spec; Naive UI types (`FormInst`, `FormRules`, `DataTableColumns<Row>`, each component's `XxxProps`) are imported from `naive-ui` for reuse; hand-written mirrors are forbidden

## Performance (only after the feature is correct and verified)

- Measure first, then change: confirm hotspot components with Vue DevTools before optimizing
- Virtualize large lists / tables: tables use `n-data-table`'s `virtual-scroll` (must also set `max-height`), ordinary long lists use `n-virtual-list`
- Keep list-item props stable: the parent computes `:active="item.id === activeId"` before passing, instead of handing down `activeId` for every item to compare itself (otherwise every switch re-renders all children)
- Reduce wrapper-component depth in hotspot list items; `v-memo` / `v-once` only when measurements show gains, and a `v-memo` dependency array must cover every field used in rendering
- Large dependencies like charts and editors are dynamically imported on demand (neutral frontend-architecture-guide)
