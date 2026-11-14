# npm packaging (@aicoffe/dmaf-engine)

npm distribution channel for the engine binary. The main package ships a `bin/dmaf.js`
shim; platform binaries are prebuilt and delivered through `optionalDependencies` —
npm skips non-matching platforms via each platform package's `os`/`cpu` fields, so
there is zero download at install time.

## Package layout

- Main package `@aicoffe/dmaf-engine`: `bin/dmaf.js` resolves the platform package for
  the running platform and execs its binary; all five platform packages are declared in
  `optionalDependencies` (strict same-version discipline).
- Platform packages `@aicoffe/dmaf-engine-{win32-x64, darwin-arm64, darwin-x64, linux-x64, linux-arm64}`:
  one binary under `bin/` plus `os`/`cpu` fields; the linux side reuses the musl static
  binary (usable on glibc systems, no dual build).
- `npm i -g @aicoffe/dmaf-engine` lands on the npm global bin (already on PATH) —
  no PATH handling.

## Version placeholder

The committed `npm/dmaf-engine/package.json` always stays at `0.0.0-dev` (version and
optionalDependencies). Release versions exist only in build outputs: `build.cjs`
rewrites them to the release version at pack time, and nothing is committed back — the
repo never tracks release numbers, avoiding drift against the engine's `Cargo.toml`.
Never publish the main package without running `build.cjs` first.

## Assembly (standalone)

```bash
node npm/build.cjs <version> [distrib-dir]   # default distrib dir: target/distrib
```

Reads the five dist archives from the distrib dir, assembles `npm/pkg/<slug>/`
platform packages, and rewrites the main package's version + optionalDependencies to
`<version>` (ephemeral in the working tree, not committed).

## Release automation

A tag push `vX.Y.Z` runs `.github/workflows/release.yml`; its dist-generated
`custom-npm-publish` job invokes `.github/workflows/npm-publish.yml`, which:

1. downloads the five release assets + `sha256.sum` and verifies checksums (fail closed);
2. assembles via `build.cjs` with the version taken from the tag;
3. publishes platform packages first, then the main package — `npm view` prechecks make
   reruns safe, and dist skips the whole job for prereleases;
4. runs the installed-binary gate: `npm i -g @aicoffe/dmaf-engine@<version>` →
   `dmaf --version` → `dmaf compile --check` against the checked-out sample repo →
   `dmaf new` scaffold check.

Auth is tokenless OIDC trusted publishing: all six packages have Trusted Publishers
configured on npmjs.com with **workflow filename `release.yml`** (the top-level
triggered workflow — GitHub's OIDC `workflow` claim names the caller, not the reusable
callee) plus org/user `aicoffe` and repository `dmaf-engine`; publishing needs no
secret. `id-token: write` is granted by the calling job in `release.yml` and must be
**re-declared on the publish job itself** — a reusable workflow job without an explicit
`permissions:` block never receives the id-token grant (no `ACTIONS_ID_TOKEN_*`
injection, npm fails with ENEEDAUTH). The publish step also strips the `_authToken`
line setup-node wrote, so no stray token can shadow OIDC (npm now rejects bypass-2FA
tokens for direct publishing with a bare 404 on PUT). The installed-binary gate polls
the registry for up to two minutes before installing: npm's packument CDN (~60s cache)
may still serve pre-publish metadata right after a publish (ETARGET). If a Trusted
Publisher was configured with an environment name, the publish job must declare the
same `environment:`. Manual/local publishing is not possible under trusted publishing
(OIDC works only from the configured CI); emergency fallback is token auth: re-add
`env: NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` to the publish step.

## dist template hand-fixes

The dist 0.34 generator emits two defects into the generated `custom-npm-publish` job;
both are hand-fixed in the committed `release.yml` and must be re-applied after every
`dist generate` (enforced by the template guard in ci.yml):

1. the job's `uses: $/...` line must read `./...`;
2. `"contents": "read"` must be present in the job-level `permissions:` — job-level
   permissions replace the top-level block, and without it checkout and `gh` fail with 403.

Because of these hand-fixes, `allow-dirty = ["ci"]` is set in `Cargo.toml`
`[workspace.metadata.dist]`: dist host otherwise verifies the committed `release.yml`
against its own regeneration and refuses to run, which the hand-fixes would always trip.
(The value is a list of config classes to skip, enum `"ci"`/`"msi"` — a plain boolean is
rejected by the schema.) The ci.yml template guard is the drift enforcement in its place.

## After publishing

npmmirror picks up new versions automatically (a first sync may need a manual trigger).
Post-publish sanity in a clean environment: `npm i -g @aicoffe/dmaf-engine` →
`dmaf --version` → `dmaf doctor`.

Design record: `docs-dev/引擎安装与分发实施方案.md` §6.5/§6.5.1 (internal archive, not committed).
