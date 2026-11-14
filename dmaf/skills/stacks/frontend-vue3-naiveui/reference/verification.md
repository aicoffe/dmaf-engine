# Vue 3 + Naive UI verification loop and test paradigms

> Applies to Vitest / Vue Test Utils at the greenfield baseline (`clearMocks` and other defaults change with Vitest major versions).
> Brownfield projects follow the verification loop in `reference/brownfield.md`: commands per the project root `package.json`
> `scripts`; this section's greenfield baseline is not required.

## Local loop (per task)

Commands take `SKILL.md`'s "Verification loop" as the single authority (`vue-tsc --build` / `eslint .` / `vitest run`); this document does not duplicate that list and only adds test paradigms and CI increments.

- Vitest major-version differences: verify the `clearMocks` default (`true` from the newest major on) and the `globals` config per the project version; brownfield keeps the project's existing Vitest config, no unauthorized changes

## Layered execution

- Unit (pure functions, stores, composables) and component tests: Vitest (shares config and the transform pipeline with Vite), DOM environment happy-dom or jsdom
- Cases depending on real layout, computed styles, or native focus / drag events: Vitest Browser Mode (jsdom does not render CSS)
- E2E: Playwright; visual regression: screenshot baselines (one set each for the light and dark themes)

## Component test paradigm (Vue Test Utils)

- Black-box assertions (the neutral "Testing Library behavior assertions" principle, landed in this stack with VTU): query by visible text, role, `data-testid`, interact via `trigger` / `setValue`, assert rendered results and `emitted()`; reading/writing `wrapper.vm` internal state is forbidden, snapshot-only assertions are forbidden
- Async: `await trigger()` / `await setValue()`; after requests and timers `await flushPromises()` (`nextTick` cannot wait for Promises)
- Wrap components using top-level `await` in a `<Suspense>` before mounting, then assert with `findComponent` after `flushPromises`
- Composables with lifecycle hooks or inject are tested by mounting a host component (the `withSetup` pattern), with `unmount` at case end to trigger cleanup
- Routing-related components prefer a real router built via `createRouter` + `createMemoryHistory`, asserting after `await router.isReady()`

## Naive UI specifics

- Overlays (modal / drawer / popover / dropdown / select menus) teleport to `body`: `mount(C, { attachTo: document.body })`, query via `document.body`, and `wrapper.unmount()` at case end; `wrapper.find()` cannot see overlay content
- Feedback channel: `vi.mock` the `shared/feedback` module and assert calls like `message.error`; do not assert Toast DOM in jsdom
- Components depending on `useThemeVars` or a specific theme / language use a test wrapper component with `NConfigProvider`
- Selectors prefer `data-testid`; depending on `.n-*` internal class names is forbidden (they change between versions)

## Pinia and vue-query testing

- Component tests: `global.plugins: [createTestingPinia({ createSpy: vi.fn })]` (`createSpy` is required when Vitest `globals` is off); actions are stubbed by default — set `stubActions: false` for real execution; preset `initialState` per store id; mount first, then `useXxxStore()` to grab the same instance for assertions
- Store unit tests: `beforeEach(() => setActivePinia(createPinia()))`, fresh state per case; a global store's core mutation logic must have unit tests (neutral frontend-development hard rule)
- vue-query: a fresh `QueryClient` (`retry: false`) per case, injected via `global.plugins: [[VueQueryPlugin, { queryClient }]]`; mock the API at the request wrapper or network layer, do not mock `useQuery`

## CI checks (added beyond the local loop)

- `vite build` passes, bundle size budget (neutral frontend-architecture-guide: an over-budget main bundle or single chunk blocks)
- Dependency lock file committed, CI frozen installs (`npm ci` / `pnpm install --frozen-lockfile`)
- `eslint . --max-warnings 0` (CI blocks warnings)
- Playwright E2E covering critical paths; screenshot baseline comparison (light and dark sets), baseline changes reviewed with the PR
- Coverage (optional): `vitest run --coverage`, thresholds blocking in CI

## Prohibitions

- Running `vue-tsc --noEmit` on a root tsconfig containing only `references` and passing it off as type checking is forbidden
- Whole-file `// @ts-nocheck` / `/* eslint-disable */` are forbidden; when an exemption is truly needed, use per-line `// @ts-expect-error` or `eslint-disable-next-line` with the reason noted
- Skipping type checking / lint in CI for "merge first, fix later" is forbidden
