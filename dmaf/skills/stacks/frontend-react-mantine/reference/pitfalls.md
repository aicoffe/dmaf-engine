# React 19 + Mantine 9 high-frequency pitfalls

> Applies to React 19 / Mantine 9.7 / TanStack Query v5 / Zustand v5 / zod 4. Each section's heading notes its
> version line; when project dependencies below the baseline matrix are detected (brownfield mode), a section's "forbidden / required" items follow the
> compatibility mapping in `reference/brownfield.md` (e.g. React 18 keeps `forwardRef` / `defaultProps`,
> Mantine 8 keeps the `color` prop, Query v4 allows the multi-signature form).

## Framework and hooks (React 19)

- **`defaultProps` has been removed from function components**: provide defaults via parameter destructuring; if a third-party component still sets `Component.defaultProps`, it has no effect under React 19
- **`forwardRef` has been removed**: pass `ref` directly as a regular prop; do not generate new code out of React 18 `forwardRef` + `useImperativeHandle` habit
- **No default hand-written memoization**: with React Compiler in charge, per-value `useMemo` / per-callback `useCallback` are outdated patterns; intervene manually only when the Profiler proves a hotspot
- Declare `useEffect` dependency arrays honestly; **suppressing `exhaustive-deps` with `eslint-disable` is forbidden** — incomplete dependencies are a source of race conditions
- Destructuring props in event callbacks creates stale closures: prefer `useEffectEvent` (React 19.2+) to stabilize callback references inside effects
- List keys use stable business IDs, **index is forbidden** (guaranteed breakage in sort/delete scenarios)
- Clean up side effects on unmount: event listeners, timers, cancellation of unfinished requests (`AbortController` / Query handles it automatically)
- **Defining components inside components is forbidden** (not even just to capture a parent's variable): every render creates a new component type and React unmounts and remounts the whole tree —
  inputs losing focus, animations restarting, effects running repeatedly are usually this; data always flows through props
- Compute derived values directly during render; the props→state sync effect is forbidden (state drift + extra renders); the response to "a specific event" goes
  in the event handler, not modeled as state + effect
- Updating state based on "the current value" always uses the functional form `setX(curr => ...)`; stale values in closures are the top stale-closure source
- Put app-level one-time initialization in the entry module or a module-level `didInit` guard, not in a component's `useEffect([])` (remounts re-run it, StrictMode runs it twice)
- **The return value of `useEffectEvent` must not enter dependency arrays** (its identity changes every render; entering the array = re-runs every render + lint error); depend on the actual reactive values

## Conditional rendering and immutability

- Conditional rendering uses explicit ternaries: `{count && <Badge />}` renders the literal `0` when `count = 0` (`NaN` likewise) —
  `&&` short-circuiting is forbidden when the left side may be a number
- Updating arrays / objects in state must be immutable: sort with `toSorted()`, add/remove with `toSpliced()` / `with()`;
  `sort()` / `splice()` mutate in place, the reference stays identical, and React skips the render (the page does not update)
- Regexes with the `g` flag carry `lastIndex` state: repeated `.test()` on the same `/g` regex flips its results;
  remove `g` or create a new regex when repeated matching is needed

## TanStack Query v5 (v6 APIs forbidden)

- The React stable line is **v5.105.x**; the v6 React adapter is not stable yet — **introducing `@tanstack/react-query@6` or v6-only APIs is forbidden**
- **Do not write v3/v4 legacy APIs**:
  - the package is `@tanstack/react-query`, not `react-query`
  - all hooks take a single-object argument, not `useQuery(key, fn, options)`
  - `cacheTime` → `gcTime`
  - `isLoading`'s "first load with no data" semantics → `isPending`
  - `keepPreviousData: true` → `placeholderData: keepPreviousData` (import the helper)
  - `onSuccess` / `onError` on `useQuery` have been removed (mutation keeps them)
- `queryKey` must contain **all** request parameters (missing one parameter = cached data bleeding between queries)
- Race protection: when query components switch, rely on Query to ignore stale responses; hand-written fetch must use `AbortController`
- In Suspense scenarios use `useSuspenseQuery` / `useSuspenseInfiniteQuery`, where `data` can no longer be `undefined`; do not set `suspense: true` manually anymore
- Hand-written `useEffect` + fetch + `setState` data fetching is forbidden (no dedupe, no cache); all server state goes through `useQuery` /
  `useSuspenseQuery` — the same `queryKey` across instances merges into a single request automatically

## Zustand v5

- **Use `useShallow` when a selector returns objects/arrays** (import from `zustand/react/shallow`); otherwise the changed reference after every `set()` triggers re-renders
- Subscribing to the whole store is forbidden; take slices with selectors
- Zustand is built on `useSyncExternalStore`, compatible with React 19; `create` no longer accepts a custom equality function — use `createWithEqualityFn` (`zustand/traditional`) when custom comparison is needed

## Mantine 9 specifics

- **emotion has been removed since v7** and `createStyles` is deleted; installing/referencing `@emotion/react` and legacy styling usage is forbidden — custom styles use CSS Modules (`*.module.css`) or inline style
- **The `color` prop of `Text` / `Anchor` has been removed**: replaced by `c` (`<Text c="red">`)
- **Default radius**: v8 was `sm`, v9 is `md`; new projects keep `md` as-is, no fallback needed
- **The `light` variant's CSS variables now use solid colors** (no longer opacity-based); new design systems map design tokens per v9's solid-color semantics
- **`TypographyStylesProvider` has been renamed to `Typography`**
- **`Popover` / `Tooltip` no longer accept `positionDependencies`**; just remove it — position recalculates automatically
- **The second generic of `useForm` changed meaning**: it now denotes the **type of the transformed values**, no longer a transform function type
- **Form resolver**: `zodResolver` is no longer exported from `@mantine/form`; zod v4 uses the built-in `schemaResolver` (Standard Schema)
- `notifications.show` must be called after `Notifications` is mounted (the application root is fully wired; mind the ordering at dynamic entry points)
- Select / Combobox with controlled `value`; async options pair with `use-debounced-value` debouncing

## Mantine form pitfalls (@mantine/form)

- **uncontrolled mode missing `key={form.key('path')}`**: the input stops responding to `setFieldValue` / `setValues` / `reset` —
  the top cause of "clicked reset / backfill and nothing happened"
- **Reading `form.values` during render under uncontrolled**: it does not update while typing — event handlers use `form.getValues()`,
  render-time linkage uses `form.useWatchValue(path)`
- The `required` prop adds the native attribute, triggering browser submit blocking (bypassing `form.onSubmit`): for the visual required asterisk use `withAsterisk`
- onBlur validation errors push the submit button away and the click lands on nothing: put the button where errors cannot move it, or switch to onChange validation
- `form.initialize()` **takes effect only once**; swapping records in an always-mounted edit modal uses `setInitialValues(record)` + `reset()`,
  the key-remount hack is forbidden
- Hidden conditional fields keep their values and still validate: rules depend on the visibility-controlling value (the rules' second argument is the full values / zod
  `superRefine` + `path`)
- `isEmail` / `isUrl` / `matches` reject empty strings: check for empty first on optional fields before calling the validator
- Validation methods returned by `useFormContext()` always type as synchronous: with async rules, always `await` `validate()`
- Inputs that transform user input mid-typing (uppercasing etc.) **get no `key`**: `setFieldValue` changes the key, and the remount loses focus

## Mantine Combobox / wrapped-component pitfalls

- Mixing selected (the keyboard-highlighted item) with active (the item holding the current value, unstyled by default): reflect the current value with the `active` prop,
  and render your own check icon
- Calling `selectFirstOption()` on a free-text search box: Enter gets hijacked by the highlighted item, and text with no matches cannot be submitted
- After clicking a button inside the dropdown, the input blurs and the dropdown closes by mistake: interactive elements get `onMouseDown={(e) => e.preventDefault()}`
- Custom wrappers redeclaring the `variant` prop: `StylesApiProps` already types it as own variants + string; redeclaring overrides it
- Using `withProps` for type narrowing: it only presets at runtime and required stays required; declare props meant for presets as optional
- Collecting subcomponents with `Children.map`: fragments / conditional rendering / wrapping components break it; use `createSafeContext` to let subcomponents self-register

## zod 4

- String format checks use **top-level functions**: `z.email()` / `z.uuid()` / `z.url()`; `z.string().email()` etc. are deprecated
- **`ZodError.errors` has been removed → use `.issues`**; error-handling logic reading `.errors` always finds nothing under zod 4
- `invalid_type_error` / `required_error` have been removed, unified as `error`
- `z.record` now **requires explicit key and value schemas** (`z.record(z.string(), value)`)
- Import from `zod/mini` when a tree-shakable smaller bundle is needed

## Types

- Event types use the types React provides (`ChangeEvent` / `MouseEvent` etc.), **bare `any` is forbidden**
- Infer types from zod schemas with `z.infer`; **hand-written interfaces duplicating a schema in parallel are forbidden**
