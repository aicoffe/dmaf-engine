# Brownfield compatibility mapping (legacy-project low-version adaptation)

Single authority for legacy projects (brownfield): when the task precheck determines brownfield, this file overrides the corresponding
greenfield rules of the base skill. Principle: **implement features with the idioms of the project's current versions — no upgrading, no mixing, no drive-by refactoring**.

**Coverage boundary: Vue 3.x + Naive UI only**. Detecting Vue 2.x (e.g. 2.7 + Element Plus) is judged a stack change that goes through architecture review
and is not handled within this skill (Naive UI does not support Vue 2; the "Vue 2 + Naive UI" combination does not exist).
Unlike pure API downgrades, this stack's brownfield also includes a set of **relaxed architecture conventions**
(Options API, full registration, Provider feedback, directory structure) — see the "Legacy-project architecture downgrades" section.

## Precheck: read versions, determine the compatibility line

At task start, read the project root `package.json` `dependencies` / `devDependencies` (no lock file means greenfield):

| Dependency | Baseline | Common legacy line | When a legacy line is detected |
|---|---|---|---|
| vue | ^3.5 | 3.4 / 3.3 / 3.2 | Downgrade APIs per the "Vue version downgrade mapping" section |
| naive-ui | ^2.37 | 2.3x / 2.2x | Downgrade components and feedback per the "Naive UI version downgrade mapping" section |
| pinia | ^3 | ^2 | setup store syntax unchanged; confirm the @pinia/testing version |
| @tanstack/vue-query | ^5 | ^4 | Downgrade APIs per the "vue-query v5↔v4 comparison" section |
| vue-router | ^4.1 | ^4.0 | Guard return-value semantics unchanged; confirm createMemoryHistory availability |
| vue-tsc / typescript | project baseline | older | `--build` not forced; use the project's existing typecheck |
| vite | project baseline | ^5 / ^6 | Config keys follow the project as-is; do not rewrite per the newer version |
| vitest | project baseline | ^1 / ^2 / ^3 | Adjust clearMocks / globals per the project version |
| @vue/test-utils | ^2 | ^2 | Stable, little compatibility pressure |

Once determined brownfield, declare at the top of the task output:
`Detected <dep>@<version>, adapting per brownfield rules without upgrading.`

## Vue version downgrade mapping (3.5 → 3.4 → 3.3 → 3.2)

**Vue 3.4 (no `useTemplateRef` / reactive props destructuring / `useId` / `onWatcherCleanup`)**

- `useTemplateRef<T>('name')` → typed `ref`: `const formRef = ref<FormInst | null>(null)`, template `ref="formRef"`
- Reactive props destructuring `const { size = 'medium' } = defineProps<P>()` → `const props = withDefaults(defineProps<P>(), { size: 'medium' })` (array / object defaults must be factory functions); access via `props.size` from then on, and write `watch` as `() => props.size` — destructuring breaks reactivity on 3.4, keeping the destructured style is forbidden
- `useId` → a module-level counter or a `Symbol` scheme
- `onWatcherCleanup` → the `watch` callback's third argument `onCleanup` (same async race-protection semantics)

**Vue 3.3 (`defineModel` still experimental; `defineOptions` / generic components / `defineSlots` already available)**

- `defineModel` defaults to manual `modelValue` + `update:modelValue`; keep using `defineModel` only if the project has explicitly enabled the experimental flag; do not turn on the compiler flag on your own
- `defineOptions`, generic components (`<script setup lang="ts" generic="T">`), `defineSlots` are available

**Vue 3.2 (no `defineOptions` / generic components / `defineSlots` / `:foo` same-name shorthand)**

- `defineOptions({ name, inheritAttrs })` → a second plain `<script>` block with `export default { name, inheritAttrs }` (under 3.2, at most one plain `<script>` block may coexist with `<script setup>`)
- Generic components → narrow types at call sites; fall back to a minimal `any` in transitions, no runtime overhead
- `defineSlots` → take at runtime via `useSlots()`
- `v-bind` same-name shorthand `:foo` → write the full `:foo="foo"` (the shorthand arrives in 3.4+)

## Naive UI version downgrade mapping

**2.36 and below (no `n-flex`)**

- `n-flex` → `n-space`; their layout semantics differ (`n-space` wraps children by default and slots them by `size`), so measure spacing and wrapping after the one-to-one swap

**2.28 and below (no `createDiscreteApi`)**

- Feedback goes through the Provider pattern: mount `n-message-provider` / `n-dialog-provider` / `n-notification-provider` / `n-loading-bar-provider` at the application root, and inside components use `useMessage()` / `useDialog()` / `useNotification()` / `useLoadingBar()`; providers automatically inherit the `n-config-provider` configuration, no `configProviderProps` needed
- Non-component environments (request interceptors, route guards) cannot get `useMessage` (providers live inside the component tree): keep the project's existing bridging; if there is none, building a new `shared/feedback.ts` is not forced — handle it the simplest way the project allows

**Early 2.3x (`validate()`'s `warnings` / rule `level: 'warning'`)**

- Whether `validate()`'s return includes `warnings` and whether `level: 'warning'` takes effect depends on the project version — verify against the official docs; when unsupported, use ordinary rules + custom notice styling and do not write `level: 'warning'`

**Theme-related (`preflight-style-disabled`, `<meta name="naive-ui-style">`)**

- Verify support per the project's Naive UI version; when unsupported, keep the project's existing global style approach; applying the new style is not forced

## vue-query v5↔v4 comparison

| v5 style | v4 equivalent |
|---|---|
| `isPending` (first load with no data) | `isLoading` (semantic difference: v4's `isLoading` = `isPending && isFetching`, `false` on cache-hit refetches) |
| `placeholderData: keepPreviousData` | `keepPreviousData: true` |
| `gcTime` | `cacheTime` |
| `onSuccess` / `onError` on `useQuery` | still available in v4; **under brownfield, do not rewrite them per v5's removal rules** |
| `useQuery({ queryKey, queryFn })` single object | v4 has both single-object and multi-signature forms; follow the project as-is, no forced rewrite |

Under brownfield, keep the v4 API and do not rewrite per v5; version-independent conventions — reactive parameters in queryKey, `signal` pass-through cancellation, precise invalidation — all still apply in full.

## Pinia / vue-router / Vite / Vitest

- **Pinia v2**: setup stores and `storeToRefs` are available; setup stores still have no built-in `$reset()` (implement a reset action yourself); confirm the `@pinia/testing` version and the `createSpy` requirement
- **vue-router 4.0**: `createMemoryHistory` and guard return-value semantics are available; "no `next`" holds across all 4.x — legacy `next()` calls are not forced to change immediately, but new / modified guards always use return-value semantics
- **Vite 5 / 6**: config keys such as `build.rollupOptions` and `optimizeDeps` follow the project as-is; do not rewrite per newer versions
- **Vitest 1 / 2 / 3**: verify and adjust the `clearMocks` default and the `globals` config per the project version; component test paradigms are largely stable

## Legacy-project architecture downgrades

| Skill convention (greenfield) | brownfield behavior |
|---|---|
| Directories `src/app/` / `src/domains/<domain>/` / `src/shared/` | Keep the project's existing directories; place new code per the existing structure, no forced refactoring |
| `<script setup>` as the only style, Options API and mixing forbidden | New components use `<script setup>`; legacy Options API components are not forcibly migrated, coexistence allowed; no mixing of the two styles within one file |
| On-demand registration (unplugin + NaiveUiResolver), full registration forbidden | Keep the project's existing registration style (full registration or manual imports); new components follow the project's existing approach, introducing unplugin not forced |
| Feedback only via the `createDiscreteApi` singleton | Keep the legacy Provider + `useMessage` pattern; new feedback prefers the project's existing feedback wrapper, migration not forced |
| Introducing a second UI library forbidden | Already-mixed legacy setups are not forcibly removed; new components prefer Naive UI, do not actively introduce a second one |
| Icons unified on xicons | Keep the project's existing icon library (`@ant-design/icons-vue` / `@heroicons/vue` / `@iconify/vue` etc.); `<n-icon :component="Icon" />` works with any icon component returning a VNode |
| Copying server state into stores forbidden | Already-copied legacy code is not forcibly refactored; new queries do not copy |

## Verification loop (brownfield)

Read the project root `package.json` `scripts` first and use the project's existing
`typecheck` / `lint` / `test` / `build` commands; do not invent missing commands.
When changes touch version-sensitive APIs, run the project's existing type check and tests at least once.
Legacy projects are not required to meet the greenfield `vue-tsc --build && eslint . && vitest run` baseline.

## Legacy-project red lines

1. **No major-version upgrades**: dependency upgrades are a separate migration task requiring an explicit user request; feature implementation uses only the APIs of the project's current major versions
2. **No mixing of two rule sets**: old and new styles must not coexist within one file (e.g. 3.5's `useTemplateRef` alongside 3.4's typed `ref` in the same file); unify on the project's actual version
3. **Minimal diff first**: "anti-patterns" in legacy code may carry compatibility logic; no drive-by refactoring
4. **No forced migration of legacy architecture**: Options API, full registration, Provider feedback, and old directory structures do not change because of this skill
5. **Annotate output with the compatibility basis**: e.g. "because vue@3.4 was detected, a typed `ref` is used instead of `useTemplateRef`"
