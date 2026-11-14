# D-MAF Engine Manual (dmaf / dmaf-engine)

> Language: English | [中文](zh/引擎手册.md)

A deterministic multi-agent routing engine. Core principle: **the Engine is the sole routing authority, and Agents are controlled execution units** — the same state file plus workflow definition always produces the same next-hop instruction; the LLM exercises judgment only within the single action the Engine specifies.

## Layout and command surface (v0.5.4)

```
dmaf/                       # Definition layer (committed): all "sources" of dmaf
├── workflows/stages/<phase>/<slug>.md   # stage library (input to compile)
├── workflows/scopes/<name>.md           # scope member grid (include_stages)
├── agents/  skills/         # AI assets (installed into the harness by install)
├── hooks.yaml               # project-level hook configuration (optional)
└── compiled/                # compiled artifacts (engine-managed; committed + --check drift guard)
.dmaf/<session>/            # Runtime layer (gitignored): engine-exclusive session state and audit
<business-dirs>/            # workflow artifacts: full paths are declared in stage inputs/outputs ({SESSION_ID} session bucketing supported)
src/  tests/                # engine implementation (cargo package dmaf-engine, binary dmaf)
```

The command binary is **`dmaf`** (`target/release/dmaf`, or `cargo run --release --` during development).

## Commands

### Definition and assets (one-time project setup / compile / deployment)

| Command | Description | Exit code |
|---|---|---|
| `dmaf new [--template minimal] [--dir <path>] [--force]` | Scaffolds the standard dmaf directory layout: greenfield creates a new project / brownfield `--dir .` completes an existing repository; only missing files are created and existing ones are **never overwritten**, .gitignore only gets missing entries appended (.dmaf/, .opencode/; the user's existing entries are untouched); idempotent (the `dmaf init` semantics are reserved for one-time initialization before a workflow starts, not yet implemented). Two embedded assets ship with the skeleton: the flow definition guide (`dmaf/docs/`) and the definition skill (`tools/skills/` embedded → `dmaf/skills/create-workflow/`, Q&A + document-driven dual mode, engine slot, excluded from package distribution) | 2 |
| `dmaf compile [stages] [scopes] [--out dmaf/compiled]` | Compiles the stage library + scope definitions into two byte-deterministic JSON artifacts; all structural validation happens at compile time (scope sequence data dependencies (hard-input producers must be members and precede their consumers), soft-input dead wiring, template variables, enums, id=stem, phase=directory name) | 3 validation failure |
| `dmaf compile … --check` | Drift guard: recompiles and compares byte-by-byte against the committed artifacts; any mismatch fails (source changed but compile forgotten) | 3 |
| `dmaf compile … --harness <name>` | Optional: additionally validates at compile time that the referenced agent/skill exist in the specified harness | 3 |
| `dmaf install [--harness <name>] [--prune] [--check-only] [--force]` | Installs the definition layer's `dmaf/agents/` and `dmaf/skills/` into the specified harness (source-side frontmatter validation; **ledger hash gate**: `dmaf-installed.yaml` at the harness directory root records fingerprints at write time — target missing → installed, identical to source → unchanged, target unmodified while source upgraded → updated (auto-overwrite), user-modified → skipped (`overwritten` with `--force`); agents are replaced file by file, skills as whole directories; JSON report with layout/installed/unchanged/updated/skipped/overwritten/pruned/ledger; orphans are not deleted by default; the toml-agents layout converts agents to TOML). When `--harness` is omitted, **auto-detects** harness directories already installed under the project root: exactly one → install directly (`auto_detected: true`); several → list the candidates and refuse; none → print installation guidance. `--harness all` installs into **all detected** harnesses (the report carries per-harness `results` + aggregate counts). Bare-name harnesses additionally **place the entry config**: files under `dmaf/harness/<name>/` are copied **file by file** into the harness directory (e.g. `.opencode/opencode.json`, not the project root), subject to the same ledger hash gate (report `entry_config.created/unchanged/updated/skipped/overwritten`) | 2 none/multiple candidates; 3 asset validation failure; 5 IO |
| `dmaf check --harness <name> (--workflow <file> \| --session <dir> \| --stages <dir> --scopes <dir>)` | Validates that the agents referenced by the workflow/stage library (existing and dispatchable) and the skills exist in the harness; when all input sources are omitted, defaults to the standard layout `dmaf/workflows/{stages,scopes}` (a usage error is reported if absent) | 2/3 |
| `dmaf add (<market-entry-URL> \| --from <path\|local .tar.gz archive\|URL> [--tag vX]) [--dry-run] [--no-install] [--force]` | Merges a workflow package into the **current project** (consumer perspective). The source must contain the `dmaf/` layout and a package manifest. Merge semantics: if the target has no `dmaf/`, a minimal skeleton is added first; merging follows the **hash gate** of the `dmaf/packages.yaml` ledger (target missing → created, identical content → unchanged, package-owned and target unmodified → updated, everything else → conflicted and **nothing is written at all**); project-owned slots are excluded (`dmaf/manifest.yaml`, `dmaf/README.md`, `dmaf/docs/流程定义指南.md`, `dmaf/packages.yaml`, the engine-issued definition skill `dmaf/skills/create-workflow/`) along with `dmaf/compiled/`; recompiles after merging (the naming-conflict detection point); chained harness auto-detection and deployment follow (`--no-install` disables this; `--force` applies to the chained deployment as well); the ledger records package name/source/tag/per-file hashes (market sources additionally record the digest). **market entry URLs** (the install-command form on the entry page) go through anonymous source fetch → SHA-256 double check against `X-Content-Digest` (fail closed) → safe unpacking (rejects path traversal/symlinks/oversize) → the same merge core; `--tag` pins the version (default latest; the ledger records the actual version from `X-Entry-Version`); local `.tar.gz` archives go through the same safe unpacking and merge (offline install) | 2 invalid source/URL; 3 entry 404/envelope error/invalid archive; 4 merge conflict; 5 IO/network/digest mismatch/deployment failure |
| `dmaf push <target-project> [--dry-run] [--no-install] [--force]` | Delivers the **current workflow project** to a target project (the mirror of add: same merge core and `--force` semantics, executed inside the source project). Verification gates: the source manifest (with a non-empty name) must exist; if the source has a stage library, its artifacts must not have drifted (run `dmaf compile` first). The target project must already exist | 2 manifest missing/target missing; 3 artifact drift/compile failure after merge; 4 merge conflict; 5 IO/deployment failure |
| `dmaf validate --workflow <file>` | Structural/semantic validation of an inline workflow (no harness binding) | 3 |
| `dmaf doctor [--artifacts <dir>] [--harness <name>]` | Environment checkup (read-only): binary version, standard layout, artifact drift (including **artifact format version compatibility**: when an artifact's version field differs from the binary's, upgrade/recompile guidance is printed instead of doing a byte comparison), `dmaf/manifest.yaml` engine range compatibility (optional file; absent = normal state); with `--harness`, additionally checks reference completeness and **asset sync drift** (per-file comparison of the definition layer vs the assets installed in the harness, reusing install --check-only; `would_change` = missing/unmodified drift that a rerun of install would sync, `modified` = user-modified targets that need `--force` to overwrite); JSON `ok=false` when problems exist | 0/3 |

### market integration (auth / publish / add entry URL)

Three commands for working with the market (the marketplace for workflows/agents), plus the shared foundation:

| Command | Description | Exit code |
|---|---|---|
| `dmaf auth login --token <PAT> [--market <base>]` | Local format pre-check (starts with `dmafmt_`, total length 50; no request is sent on mismatch) → validation via `GET /api/me` → credentials written to the user-level config directory (`%APPDATA%\dmaf\credentials.toml` / Unix `~/.config/dmaf/credentials.toml`, keyed by base, 0600 on Unix). Calls no `/api/auth/*` endpoints (the CLI does not use OIDC/refresh) | 2 format/address; 3 validation failed (401 includes revocation) |
| `dmaf auth logout [--market <base>]` | Deletes local credentials (server-side revocation is done via the Web tokens page) | — |
| `dmaf auth status [--market <base>]` | Calls `/api/me` to report the currently logged-in identity | 0 (0 also when not logged in) / 3 |
| `dmaf publish [<dmaf-dir>] [--project] [--version] [--bump] [--intro <file>] [--github-url] [--market <base>] [--dry-run]` | Publish pipeline (the first four steps make zero requests): ① login state → ② gate 1, compile (zero errors) → ③ gate 2, doctor core (**only error-level findings reject**: layout/artifacts_drift/manifest; `.dmaf` hygiene is a warning and passes) → ④ local pre-checks (manifest name, version and project regexes, intro three fields 1–2000/1–2000/1–4000 plain text, package integrity, E-9 **excludes files belonging to other installed packages** per the publisher's ledger and lists them, member count/uncompressed total size/compressed size caps) → ⑤ **deterministic packaging** (tar.gz: sorted paths, mtime/uid/gid=0, mode 0644, gzip header mtime=0/OS=255/fixed compression level — the market judges idempotency by the server-side byte SHA-256) → ⑥ multipart upload `POST /api/publications` (meta JSON + package byte stream, Bearer PAT) → ⑦ response handling (201: idempotent replay, safe to retry; 409 version_conflict: suggests `--bump`/`--version` to raise the version, no retry; `content_integrity_violation`: retry forbidden). `--bump` is a one-step version bump: increments the numeric tail segment of the manifest version and writes it back; `--intro` defaults to interactive follow-up prompts; `--dry-run` packages and prints the digest without uploading. **publish-only**: no update/overwrite/takedown; updating = republish with a bumped version | 2 not logged in/address/usage; 3 gate/pre-check/4xx; 4 version conflict; 5 rate limit/5xx retries exhausted/integrity anomaly |

market foundation conventions: address resolution order `--market` > environment variable `DMAF_MARKET_URL` > user config (`[default] market` in `credentials.toml`) > built-in default instance (`https://dmaf.aicoffe.com`); only `https://` is accepted (`http://` only for localhost/127.0.0.1/[::1]); the credentials environment variable `DMAF_MARKET_TOKEN` can override temporarily without touching disk; output/logs show only the first 12 characters of the token; each command generates one `X-Request-ID`; error envelopes (code/message/details/trace_id) map uniformly to engine exit codes 2/3/4/5; retryable errors (network/503 blob_unavailable/500 sys_internal_error) retry the same bytes up to 3 times with exponential backoff.

`--harness` takes a **name** (shared by install/check/doctor/compile); the engine resolves it to a directory and layout under the project root:
`opencode`→`.opencode`, `claude` (alias `claude-code`)→`.claude`, `zcode`→`.zcode`, `cursor`→`.cursor`, `gemini` (alias `gemini-cli`)→`.gemini` (flat-md: md files copied as-is);
`codex`→`.codex`, `dsh`→`.dsh` (toml-agents: agents converted to TOML, `mode` preserved as a `# dmaf-mode:` comment);
skills are always copied as whole directories (the source side supports one level of category subdirectories `skills/<category>/<skill>/`; **opencode and cursor support nested discovery**, installation keeps the category path, other harnesses flatten by skill name; skill names must be globally unique). Path-like values (containing a separator or starting with `.`) pass through as flat-md; an unknown bare name is an error (exit code 2).
Optional companions on the project side: `dmaf/harness/<name>/` is the **session entry config source** for that harness (placed at install time, under the project root),
`dmaf/manifest.yaml` declares the workflow package identity and the `engine` version range (doctor judges compatibility; shared by the add/push merge core).

### Runtime (zero harness dependency: runtime commands never read the harness/registry)

| Command | Description | Exit code |
|---|---|---|
| `dmaf start --scope <name> --task "…"` | scope mode: loads the flow from the `dmaf/compiled/` artifacts; the raw requirement text is injected into the reserved variable TASK (extraction is owned by the first stage in the flow); machine callers can pass `--var k=v`; `--locale zh|en` sets the session's human-facing message language (baked into the session; defaults to DMAF_LOCALE, then the system language, then en) | 0/3 |
| `dmaf start --workflow <id> --task "…" [--var k=v]` | inline mode (both tracks are kept in the engine; `--workflow-dir` defaults to `dmaf/workflows/inline`; the repo samples carry no inline definitions, see the fixtures in `tests/fixtures/`) | 0/3 |
| `dmaf next --session .dmaf/<dir>` | Computes the next-hop instruction (JSON). Writes an `action.dispatched` shard when it has side effects; resume/resend has no side effects | — |
| `dmaf report --session <dir> --payload <file\|->` | Reports an action result or an approval decision; the Engine verifies artifact existence and then transitions the state | 3 schema rejected; 4 state conflict |
| `dmaf inspect --session <dir>` | Human-readable progress view | — |
| `dmaf log --session <dir> [--type T] [--stage S] [--limit N]` | Audit timeline (a head warning is shown when the chain is anomalous) | 2 |
| `dmaf verify-audit --session <dir>` | Hash chain verification + replay comparison | 4 |
| `dmaf cancel --session <dir>` | Cancels the workflow | 4 |
| `dmaf replay --session <dir> [--overwrite]` | Rebuilds the state file from shards (writes to a side file by default) | 4 |
| `dmaf fork --session <dir> --seq N` | Forks a new session from an audit checkpoint | 4 |
| `dmaf verify-determinism --session <dir>` | Empirical determinism replay (re-derives all routing decisions and compares them against the ledger) | 1 mismatch; 3; 4 |

Errors are output as JSON (stderr); normal output is JSON (stdout, UTF-8).

## Instruction payload and harness decoupling

The action of a `next` instruction carries `skills: [name list]` (from the stage definition) and **no longer includes the `skill_brief` carried over from the registry** — skill descriptions are supplied by the harness at execution time. A skill missing from the harness is an execution-time error: agent execution fails → report failed → the user fixes the harness → run `next` again for the same `--session` (resume; pending actions are resent as-is). `compute_next` is therefore a pure function of `(artifacts/definitions, state)`.

## Flow Definition Guide

The user-facing guide for writing stages/scopes lives at [docs/flow-definition-guide.md](flow-definition-guide.md) (Chinese source: `docs/zh/流程定义指南.md`, embedded at compile time); `dmaf new` copies it into the user project under `dmaf/docs/`, and the template READMEs all link to it.

## Built-in samples

## Language (locale)

Human-facing messages follow the system language: zh-family systems get Chinese, everything
else (including undetectable environments) gets English; the `DMAF_LOCALE=zh|en` environment
variable pins the process-level choice. `dmaf start --locale zh|en` sets the session-level
language and bakes it into the session (session commands read the session, not the env;
forks inherit; legacy sessions fall back to zh). `dmaf new` writes the flow definition
guide in the chosen language (Chinese or English edition).

## Built-in samples

The authoritative inventory of phase/stage/scope definitions: see [docs/phase-stage-scope.md](phase-stage-scope.md).

The repo samples implement the AI-native SDLC standard flow: the `sdlc-standard` scope (12 stages under
`dmaf/workflows/stages/{concept,requirements,design,development}/`), with 8 agents
(`dmaf-conductor` orchestrator + 7 SDLC roles) and 29 skill directories (organized into 12 category directories by capability domain).

## Inline workflows (dual track)

The engine keeps the inline single-file mode: `dmaf start --workflow <yaml>` and `dmaf validate` work as before. The repo samples are
scope-only and carry no inline definitions; parse equivalence between the two tracks is guarded by the synthetic dual-track fixtures in
`tests/fixtures/wf-lesson.yaml` and `tests/test_compile_install.rs` (`golden_compiled_matches_inline_workflow`).

## Session directory structure

`.dmaf/<YYYYMMDD-HHmmss>/` is the engine's exclusive runtime area (the whole directory is gitignored) and contains only engine bookkeeping — **workflow artifacts are not here**; they are written into business directories under the project root at the full paths declared by the stage definitions' `inputs`/`outputs` ({SESSION_ID} session bucketing supported):

```
.dmaf/<YYYYMMDD-HHmmss>/     # Engine-exclusive; neither LLMs nor humans should read or write
├── dmaf-state.md            # state file
├── last-report.json         # draft of a single report, written by the Orchestrator
└── audit/shard-*.json       # append-only SHA-256 hash chain audit shards
```

## Key semantics

- **State machine**: `pending → in-progress → (awaiting-approval ⇄ revising) → completed / failed / cancelled`; the full transition table is in `src/state.rs`.
- **Artifact validation is in the Engine**: on report, the Engine checks that the expected output files exist; missing ones are handled as failed. For stages with no output artifacts (such as submit), the subagent's confirmation is authoritative.
- **Retries**: `retry.max_attempts` caps the retries after the initial attempt (`1` = at most 1 retry, 2 attempts in total); a retry is audited as `retry.scheduled` and dispatched automatically at the next hop.
- **Approval gates**: a stage with `approval.required: true` stops at `awaiting-approval` after success; `granted` → completed, `rejected` → revising (the revision dispatch carries `revision_feedback`); revision loops are unlimited.
- **Approval reviewers**: with `approval.reviewer: <agent>`, the `await_approval` instruction carries `payload.reviewer` (the reviewer + the paths of the artifacts under review); the Orchestrator dispatches the reviewer first and then files the approval report based on its verdict; the `approval.granted/rejected` shards record `reviewer` for traceability. A missing/undispatchable reviewer is validated at deployment time by `check`/`compile --harness`.
- **Delegation payload pre-construction**: every action of a dispatch instruction carries a `delegation` object (the path fields share the names of the stage definition's `inputs`/`outputs`, with values being the template-resolved full paths, always present; protocol fields + flow variables are flattened per "session vars ∪ the agent's declared vars set"; spec at docs/schemas/delegation.schema.json); the Orchestrator forwards it to the subagent as-is; the action top level carries no business fields (knowledge_point has been removed, and the summary's top-level course_id removed alongside; both are now carried by vars).
- **Delivery text template**: scope/inline definitions may declare a `completion_template` (`{vars.name}` / `{outputs.stage_id}`, placeholders validated at definition/compile time); the complete instruction renders it as `payload.completion_text`; without a template the field is omitted and the Orchestrator falls back to paraphrasing the summary.
- **Parallel aggregation**: the last attempt of each agent is authoritative; with `on_exhausted: continue` the flow continues with failures present, and downstream inputs do not drop missing items.
- **Determinism**: `instruction_id = ins-<workflow_id>-<audit sequence number>`; instruction content contains no timestamps — the same state always yields the same instruction (randomized-trajectory regression in `tests/test_determinism.rs`).
- **fork**: a non-destructive branch; the hash chain continues; `SESSION_ID` is live-bound to the new session ID and the artifact buckets switch accordingly — artifacts completed before the fork point are copied from the source bucket to the new bucket (the `copied_artifacts`/`missing_artifacts` in the result JSON are new-bucket paths), and the two branches never overwrite each other; paths without `{SESSION_ID}` live at the same location on both sides and are shared by the branches. The fork point must have no in-flight action. The recommended way to retry a failed stage: fork back to the checkpoint before that stage's dispatch and rerun.

## Hook (hooks.yaml)

Project-level external integration points: **the engine only invokes them in configuration order and never reads their results**. Configuration lives in the definition layer at `dmaf/hooks.yaml` (read at runtime, not part of compiled artifacts; missing = no hooks, silently):

```yaml
timeout_secs: 120          # per-hook wait cap (seconds, global, default 60; on timeout the hook is killed and the list continues)
init:                      # optional: run sequentially before dmaf start creates the session; results do not affect startup
  - python dmaf/hooks/preflight.py
on-event:                  # optional: keys are audit event types (ten of them; see the audit event vocabulary)
  action.reported:
    - python dmaf/hooks/upload.py
  workflow.completed:
    - [cmd, /c, npm, run, notify]   # Windows .cmd/.bat wrappers must be invoked via cmd explicitly
```

- **Two hook points**: `init` (before start creates the session; stdin carries a workflow/scope/session_dir/vars/stages snapshot); `on-event` (**whenever an audit shard is written to disk**, the shard's event_type is looked up in the table and matches are run sequentially; stdin = `{"session_dir": ..., "shard": {…shard content…}}`). Read-only commands (next resend/resume, inspect, log, verify-*, replay) produce no shards and never trigger.
- **Command syntax**: uniform argv semantics — strings are split on whitespace (quotes are not parsed; use a list for paths containing spaces), lists pass through as-is; hooks are spawned directly without going through a shell; a hook inherits the engine process's env and CWD.
- **Zero result interpretation**: the engine waits synchronously for the hook to exit (sequential execution, no concurrency); exit codes and output are never read or judged; a single hook failure neither blocks the rest of the list nor affects the enclosing command's stdout JSON, exit code, state, or ledger. Hook stdout is forwarded to the terminal's stderr (the engine's stdout contract is pure JSON and must not be polluted).
- **No long-running work**: the engine waits synchronously for hooks, so do not do time-consuming work synchronously inside a hook — put uploads/external calls into task files consumed by a background process.
- **Switch**: `--no-hooks` (supported by start/next/report/cancel/fork) or the environment variable `DMAF_NO_HOOKS=1`.
- **Config degradation**: bad YAML / unknown event keys / invalid timeout_secs → a one-line warning on stderr, while the valid parts still take effect; `dmaf doctor` reports configuration health.
- **Responsibility boundary**: hooks do not participate in routing — with identical input, the ledger is byte-identical with or without hooks, and a hook breaking determinism is the user's responsibility (`verify-audit` / `replay` / `compile --check` as the backstop). The right answer to runtime "interception" needs is an approval gate (`approval.gate`: halt → external decision → report lands in the audit), not a hook.

## Recovery (resume)

All state lives on disk: after a session is interrupted, a new session can resume by running `next` directly against the same `--session` (pending actions are resent as-is; no new audit is produced). Use `verify-audit` for tamper detection.

## Maintenance conventions

- **The definition layer is the single source of truth**: `dmaf/workflows/` (workflow definitions), `dmaf/agents/`, `dmaf/skills/` (AI assets; this repo's content is the sample). After changing AI assets, install them into the harness with `dmaf install --harness <name>` (e.g. `opencode` → `.opencode`); no more manual cp.
- **Workflow changes require recompilation**: after changing `dmaf/workflows/stages|scopes`, run `dmaf compile` and commit `dmaf/compiled/`; `dmaf compile --check` guards against drift.
- **Harness binding validation**: at deployment time use `dmaf check --harness <name>` (workflow references vs the actual harness content).
- Adding a stage: add an md file under `dmaf/workflows/stages/<phase>/` → `dmaf compile` → claimed into a scope's `include_stages`; business copy such as `task_description` lives entirely in the definitions — Orchestrator prompts carry no business copy.
- Engine changes must keep `cargo test` fully green (179 tests), especially the determinism tests.
- The JSON Schema specs for instructions/reports/audit shards are in `docs/schemas/`; the equivalent Rust-side validation is in `src/schema_check.rs` — both sides must be changed in sync.
