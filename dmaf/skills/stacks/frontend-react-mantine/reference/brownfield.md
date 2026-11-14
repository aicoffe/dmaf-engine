# Brownfield compatibility mapping (legacy-project low-version adaptation)

Single authority for legacy projects (brownfield): when the task precheck determines brownfield, this file overrides the corresponding
greenfield rules of the base skill. Principle: **implement features with the idioms of the project's current versions — no upgrading, no mixing, no drive-by refactoring**.

## Precheck: read versions, determine the compatibility line

At task start, read the project root `package.json` `dependencies` / `devDependencies` (no lock file means greenfield):

| Dependency | Baseline | Common legacy line | When a legacy line is detected |
|---|---|---|---|
| react | ^19.3 | 18.x | Disable React 19-only APIs (see below) |
| @mantine/* | ^9.7 | 8.x / 7.x | Adapt per the Mantine section below |
| @tanstack/react-query | ^5.105 | 4.x | Adapt per the Query section below |
| zustand | ^5.0 | 4.x / 3.x | Adapt per the Zustand section below |
| zod | ^4.6 | 3.x | Adapt per the zod section below |
| react-router | ^8.5 | 6.x / 7.x | Adapt per the router section below |
| vite | ^8.0 | 7.x and below | Adapt per the Vite section below |
| vitest | ^5.0 | 4.x and below | Keep the project's existing config and Node version (note the `clearMocks` default difference) |
| Node | ^22.12.0 \|\| ^24 \|\| >=26 | lower | No upgrade required; do not introduce APIs that depend on this baseline |

Once determined brownfield, declare at the top of the task output:
`Detected <dep>@<version>, adapting per brownfield rules without upgrading.`

## Per-dependency compatibility mapping (the easiest points to get wrong)

**React 18**

- `forwardRef` is still required; do not rewrite it per 19's "ref as a regular prop"
- `defaultProps` still works on function components; do not delete it proactively
- Do not introduce 19-only APIs such as `useActionState` / `useOptimistic` / `useEffectEvent` / `use` / `<Activity>`
- memoization still requires manual `useMemo` / `useCallback` / `memo` (no React Compiler)

**Mantine 8 / 7**

- The `color` prop of `Text` / `Anchor` still works; do not bulk-rewrite legacy code to `c`
- The `light` variant still has opacity semantics; do not apply v9's solid-color CSS variable mapping
- The form resolver follows the project as-is (v7/v8 commonly use the `zodResolver` / `mantine-form-zod-resolver` packages);
  **do not replace it with `schemaResolver`**
- emotion has been removed (since v7); CSS Modules idioms are the same as greenfield
- Mantine 8 + React 18 is a correct combination; **do not suggest upgrading to Mantine 9** (v9 requires React 19.2+)
- Differences at 7.x and below not covered by this file: check each item against the official migration guide for the project's version, handle by the nearest version line, and note it in the delivery summary

**TanStack Query v4**

- The multi-signature `useQuery(key, fn, options)` is valid; do not force a rewrite to the single-object form
- `cacheTime` / `isLoading` / `onSuccess` / `onError` on `useQuery` are valid in v4; do not rewrite them per v5's removal rules

**Zustand v4 / v3**

- v4: named import of `create` (the default export was removed in v4); the second argument of `create`, a custom equality function, still works (deprecated);
  do not proactively migrate to `createWithEqualityFn`
- v3: default-importing `create` is valid
- Shallow comparison of selector objects follows the project version's existing style (`useShallow` is a v5 API; lower versions use the `equalityFn` equivalent)

**zod v3**

- String format checks use method-style calls like `z.string().email()`; do not change them to top-level functions like `z.email()`
- Read errors via `error.errors`, not `.issues`
- `invalid_type_error` / `required_error` still work

**react-router 6 / 7**

- v6 imports from `react-router-dom`; v7 was merged into `react-router` (the `react-router-dom` compatibility package still exists) — follow the project as-is
- Do not introduce v8's new preconditions (Node / React version requirements and ESM conventions follow the project as-is)

**Vite 7 and below**

- Keep esbuild-era config keys such as `build.rollupOptions` / `optimizeDeps.esbuildOptions`
- Do not rewrite config per Vite 8's Rolldown rules

## Legacy-project red lines

1. **No major-version upgrades**: dependency upgrades are a separate migration task requiring an explicit user request; feature implementation uses only the APIs of the project's current major versions
2. **No mixing of two rule sets**: old and new styles must not coexist within one file; unify on the project's actual version
3. **Minimal diff first**: "anti-patterns" in legacy code may carry compatibility logic; no drive-by refactoring
4. **Verification loop follows the project's existing scripts**: read `package.json` `scripts` and use the project's own `typecheck` / `lint` /
   `test` / `build`; do not invent missing commands; run the project's existing type check and tests at least once for version-sensitive changes
5. **Annotate output with the compatibility basis**: e.g. "because @mantine/core@8.x was detected, `zodResolver` is used instead of `schemaResolver`"
