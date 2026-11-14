# React performance conventions (Vite CSR SPA)

> Applies to React 19 (greenfield baseline). Brownfield projects follow `reference/brownfield.md`;
> bundler-related optimizations are assessed against the project's bundler capabilities.

Distilled from the rules in react-best-practices (Vercel) that apply to this stack, ordered by impact. This stack is a Vite CSR SPA:
Next.js App Router / RSC / Server Actions / hydration rules do not apply — do not introduce them; server state goes uniformly through
TanStack Query — do not introduce SWR. Locate hotspots with the React DevTools Profiler / the network panel before acting;
optimize only after the feature is correct and verified.

## Network and async (biggest impact: eliminate waterfalls)

- Independent async operations are always concurrent via `Promise.all`; for partially dependent ones, create the promise first (fire immediately, await later),
  and collect with `Promise.all` at the end; introducing an extra dependency for this is forbidden
- Sink `await` to the branch that truly uses it; for `flag && cheap-sync-condition` combinations, put the synchronous condition first to short-circuit,
  saving the async call on the cold path
- Suspense boundaries let the shell render first: route-level lazy loading + `useSuspenseQuery`, with the fallback only around the data area,
  so the page skeleton is not blocked by a single slow query; data that affects layout decisions (positioning / anti-jitter) does not go into Suspense
- Warm up the API domain with `<link rel="preconnect">` in `index.html`

## Bundle and first paint

- No barrel imports for icon / utility libraries (entry files with thousands of re-exports, +200–800ms dev cold start):
  import directly by subpath (e.g. `@tabler/icons-react/dist/esm/icons/…`); when a deep path lacks `.d.ts` and reports
  implicit any, fall back to named imports — leave production tree-shaking to Rolldown, do not trade type safety for cold start
- Load heavy components on demand: rich text / charts / code editors / large tables via `React.lazy` + `Suspense`
  (a complement to route-level lazy loading)
- Third-party scripts (analytics / logging / error tracking) stay out of the first-paint bundle: trigger a dynamic `import()` or
  `requestIdleCallback` after mount
- Keep `import()` statically analyzable: use an explicit `() => import('./x')` map; concatenating module paths from variables is forbidden
  (when the bundler cannot analyze it, it can only relax conservatively — bundle size and cold start both lose)
- Intent-based prefetch: warm up heavy component entries with `void import(...)` on hover / focus, zero wait at click time
- Legacy projects (brownfield) are not required to enable this section's bundler-related optimizations (barrel-free imports / dynamic import / prefetch assessed against the project's
  bundler capabilities); the "render economy" and "JS hotspots" sections are unaffected

## Render economy

- Wrap high-frequency non-urgent updates (scroll-following, filter linkage) in `startTransition`; expensive input-driven derivations use
  `useDeferredValue` (wrap the expensive computation in `useMemo` depending on the deferred value);
  `query !== deferredQuery` can drive "results lagging" feedback such as reduced opacity
- Pending state of local recomputation uses `useTransition`'s built-in `isPending`; **the pending state of network requests belongs to
  TanStack Query — manual `setIsLoading` is forbidden**
- High-frequency transient values (mouse trails, timer cursors) go into `useRef` with direct DOM style writes, not state (mutating a ref does not trigger a render)
- Narrow effect dependencies to primitives or derived booleans: compute `width < 768` into a boolean before it enters the dependencies; for subscription-style cases use
  `@mantine/hooks` `useMediaQuery` directly (subscribing to a boolean, not continuous pixels)
- Add `{ passive: true }` to scroll-path listeners like `touchstart` / `wheel` (when they do not call `preventDefault`);
  listeners that need `preventDefault` (custom gestures, zooming) are the exception
- When N component instances listen to the same global event: a module-level registry (`Map<key, Set<cb>>`) + a single listener that dispatches,
  N instances, 1 listener
- Give long-list items `content-visibility: auto` + `contain-intrinsic-size` to skip offscreen layout and paint
  (paired with ScrollArea / virtual scrolling)
- Heavy components toggled frequently while preserving state use `<Activity mode="hidden">` (stable since React 19.2+;
  under hidden, state is kept and effects are cleaned up), replacing conditional unmounting
- Make expensive initial values lazy: `useState(() => build(...))` — localStorage reads, large index builds,
  `JSON.parse` should run only on first render
- Version-prefix `localStorage` keys (`prefs:v1`), wrap reads/writes in `try/catch` (incognito mode / full quota throws),
  store only the minimal fields the UI needs; tokens / PII are forbidden

## JS hotspots

- Layout thrashing: batch style writes and layout reads (`offsetWidth` / `getBoundingClientRect`) separately;
  in React, prefer changing className (CSS Modules) over item-by-item inline style writes in loops
- Index repeated lookups: `new Map(list.map(x => [x.id, x]))` — build the table once in O(n), then query in O(1),
  replacing repeated `.find()` inside loops; use `Set` for membership checks; `map` + `filter` chained becomes a single `flatMap` pass
- Find extremes with a single pass; `sort()` then taking an endpoint is forbidden (O(n log n) and it mutates the original array)
- Non-critical work (analytics reporting, local persistence, prefetch) uses `requestIdleCallback` (optionally with `{ timeout }`;
  fall back to `setTimeout` when unsupported); never use it on paths where the user is waiting for the result
- Hoist regexes to module level or `useMemo` per dependency; creating them inside render bodies is forbidden
