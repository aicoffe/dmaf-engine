**Language:** English | [简体中文](README.zh-CN.md)

# D-MAF Engine (Deterministic Multi-Agent Framework)

The biggest risk in LLM-orchestrated multi-agent workflows is not model capability — it is process unreliability: what happens next is improvised, deviations are untraceable, and progress evaporates when context is lost. **dmaf takes workflow control away from the LLM and hands it to a deterministic engine** — you declare the workflow in YAML, the engine dispatches LLM agents step by step: **the same state always yields the same instruction, every step lands in a hash-chained audit ledger, reproducible, replayable, resumable**.

> **The Engine is the sole routing authority; agents are controlled executors** — routing, state, capability grants, and collaboration topology are all computed by pure code from artifacts + state. The LLM exercises judgment only within the single action the Engine assigns.

## Why D-MAF

Instruction following is not a stable constant for LLMs — it degrades sharply with task scale: the AgentIF benchmark shows instruction-following accuracy dropping significantly as task length, complexity, and constraint count grow; IFEval and "Lost in the Middle" further show that models follow mid-context information worst. Betting multi-agent collaboration on this capability yields five classes of systematic uncertainty:

| # | Source of uncertainty | Manifestation |
|---|----------------------|---------------|
| 1 | Routing | The LLM decides "who does what next" from prompts |
| 2 | State | Task progress is maintained by the LLM from context, drifting as the window compresses |
| 3 | Communication | Agent collaboration rides on natural-language messages; ambiguity compounds hop by hop |
| 4 | Topology | Agents route each other directly; the collaboration structure is unpredictable |
| 5 | Capability grants | Agents rely on fully-injected system prompts and pick their own skills |

All five share one root cause: **handing control flow, state, and communication to LLM improvisation amplifies randomness layer by layer**. Hence D-MAF's goal — predictable workflows, reproducible processes, auditable results, trustworthy collaboration.

## The Approach: Separating Routing from Execution

D-MAF (Deterministic Multi-Agent Framework) applies the ideas of deterministic workflow engines to constrain LLM multi-agent systems: **a decoupled control plane × execution plane**.

```
Control plane  dmaf (deterministic program, pure code)
  ├── Declarative workflow definition ──compile──▶ byte-deterministic JSON artifacts (committed, --check guards drift)
  ├── next pure function: state + artifacts ──▶ the unique next instruction (typed JSON, schema-validated)
  └── report: validate the report → write the audit shard first → then update state (append-only SHA-256 hash chain)
        ▲                              │
        │ report                       │ next (instruction)
        │                              ▼
Execution plane  harness runtime (LLM)
  ├── The orchestrator degrades into a thin forwarding loop: fetch instruction via next → dispatch agent → report
  └── Agents exercise judgment within the single action the instruction defines, never touching control flow
```

The Engine depends on exactly two disk inputs — the workflow definition and the state file — and `next` has no clock, no randomness, no IO: **the same state always produces the same next instruction**. LLM randomness is confined inside action boundaries and can no longer pollute the global flow. Five mechanisms dissolve the five uncertainties above:

### 1. Declarative workflow definitions + compile-time gates → dissolves topology uncertainty

Workflows are declared statically in YAML/MD: stage lists, dependency DAGs, collaboration modes (`single` / `sequential` / `parallel`), retry policies, approval gates. Agents **never route each other directly** — handoffs, messages, and reviews all flow through artifact files injected by the Engine, with producers and consumers matched precisely by path. All structural validation (DAG acyclicity, dependency closure, template variable declarations) happens at compile time; artifacts are byte-deterministic and committed, so the diff is the review of membership changes, and `compile --check` guards drift.

### 2. next/report contractual communication → dissolves communication uncertainty

The only contract between the Engine and the runtime is **typed JSON instructions**: `dispatch` / `await_approval` / `retry` / `park` / `complete` / `fail`, with common fields plus a `payload` carrying the full execution contract (agents, skills, inputs/outputs, retry, approval requirements), all JSON-Schema validated. Workflow semantics never travel through natural language; `report` makes the Engine verify the existence of expected artifacts — a missing artifact is treated as failure. Instructions drive the orchestrator without replacing its execution.

### 3. Externalized state → dissolves state uncertainty

The eight stage states (`pending` / `in-progress` / `awaiting-approval` / `revising` / `completed` / `failed` / `cancelled` / `skipped`) are maintained solely by the Engine, and **the state file is completely invisible to the LLM**; every state transition must go through `dmaf report` — the LLM is forbidden from editing it. The state file is the single source of truth for long-running tasks: context windows can compress, drop, and restart, and the Engine always recovers from disk.

### 4. Audit-first, hash-chained ledger → makes results auditable

Before every state transition the Engine **writes the audit shard first, then updates state** — the two never diverge; if the audit write fails, an exception is raised and state is unchanged. The LLM never writes audit. Ten audit event types (workflow started/failed/cancelled/forked, action dispatched/reported, approval granted/rejected, retry scheduled) map one-to-one to state transitions, and every shard carries a four-way SHA-256 hash chain (`prev_shard_hash` / `state_before_hash` / `state_after_hash` / `shard_hash`) — tampering is detectable; the ledger is the log.

### 5. Precise on-demand skill injection → dissolves capability-grant uncertainty

No dumping the whole skill directory into the system prompt: each instruction carries only the skill list needed for that action, loaded one by one by the orchestrator at dispatch time — the agent sees and can invoke only the capabilities it was granted. Runtime commands have zero harness dependency — `start` / `next` / `report` read no harness configuration; reference integrity is validated at deploy time by `dmaf check`.

**Long-running task support**: externalized state + a stateless Engine (every `next` recomputes from scratch, no reliance on conversation history) + minimal context (each agent receives only the current instruction and its input artifacts) free long tasks from LLM memory; `fork` branches non-destructively from any audit checkpoint, and `replay` rebuilds the state file from the ledger.

## Positioning & Value

Compared with common multi-agent orchestration approaches:

| Dimension | Harnesses (opencode etc.) | Dify (visual orchestration) | LangGraph (graph framework) | **D-MAF** |
|-----------|--------------------------|----------------------------|-----------------------------|-----------|
| Orchestration paradigm | Prompt-driven | Drag-and-drop canvas + node forms | Nodes + edges + State, all code | **Declarative definitions + deterministic Engine, zero orchestration code** |
| Routing authority | The LLM decides ad hoc, no enforcement | Fixed at canvas-wiring time; free inside nodes/agents | Edge functions decide — code or LLM | **The Engine is the sole authority, invisible to the LLM** |
| State ownership | Conversation context, drifts with window compression | Platform variables and session storage | State + checkpoints | **Externalized state file, invisible to the LLM** |
| Communication contract | Natural-language prompts | Platform node configuration (DSL/forms) | Shared State fields | **Typed JSON instructions (schema-validated)** |
| Skill injection | Skill descriptions into context; the model helps itself | Tools/plugins configured per app | Managed by the developer | **Precise on-demand injection at runtime (instruction carries the list)** |
| Audit & replay | Session logs, no native replay | Platform run logs, no hash chain | Checkpoint restore / time travel | **Hash-chained audit ledger + native replay/fork** |
| Determinism | **None** | Deterministic topology, non-deterministic node behavior | Partial, depends on edge implementation | **Strong and verifiable (verification command family)** |

The key difference: **Dify's complexity lives in canvas configuration, LangGraph's in orchestration code, and harnesses lack workflow determinism**. dmaf hands determinism entirely to the Engine — zero orchestration code, zero canvas configuration. People focus on exactly two things: **writing clear workflow definitions (stage/scope), and building the AI assets (agents/skills)**. Direct value for users:

- **Predictable workflows**: collaboration topology is fixed at compile time; the runtime never "figured it out as it went".
- **Reproducible processes**: same state, same instruction — `verify-determinism` proves it with randomized-trajectory regression.
- **Auditable results**: the hash-chained ledger is the log; `verify-audit` chain checks and `replay` state rebuilding make everything replayable.
- **Resumable long runs**: context can be dropped and restarted at any time without losing the session; `fork` branches from any checkpoint.
- **Distributable deliverables**: `publish` / `add` integrate with the market for publishing and installing workflow packages.

## Quick Start

After installing `dmaf` you don't need the engine source — work on workflows in your own project repository following the standard layout. Two tracks by role: **using workflows** (install a ready-made workflow into your project and run your business through it) and **developing workflows** (defining processes and AI assets).

### Install the engine

One line per platform; re-running the same command upgrades in place:

```bash
# macOS / Linux
curl -fsSL https://dmaf.aicoffe.com/install.sh | sh

# Windows PowerShell
irm https://dmaf.aicoffe.com/install.ps1 | iex

# npm (all platforms)
npm i -g @aicoffe/dmaf
```

Download source defaults to the official market with automatic GitHub fallback; pin it with `--source github` (sh) / `-Source github` (ps1). Verify the install with `dmaf doctor`. Building from source (`cargo install dmaf-engine`) remains available as a fallback.

### Track 1: Using a workflow

Take **dmaf-sdlc** (sdlc-standard: the AI-native SDLC standard process), developed in this repository and published to the official market, as the example — install it into your project and run the full requirements → design → development → delivery cycle:

```bash
# 1. Install the workflow
dmaf push <target-project-dir>  # deliver the current workflow project to a target project
dmaf add https://dmaf.aicoffe.com/aicoffe/dmaf-sdlc    # install the workflow into the current project (run inside the target project)
# Multiple source types are supported; see dmaf -h

# 2. Deploy AI assets to the harness and health-check
dmaf install                 # push/add already deployed in sequence; rerun after asset changes (--harness auto-detects; specify or use --harness all when several harnesses coexist)
dmaf doctor --harness opencode   # layout / artifact drift (incl. version compatibility) / manifest engine compatibility / reference completeness / asset drift

# 3. Start a session via the /dmaf skill — no need to memorize dmaf commands
/dmaf add a shopping cart to the existing online mall

# 4. Inspect / operate
/dmaf which stage is the current session on
/dmaf rerun from the design stage
```

Daily self-check loop: after `git pull`, run `dmaf doctor --harness <name>` — `would_change` items are synced by rerunning `dmaf install`; `modified` items are targets you changed locally — add `--force` once you confirm overwriting them.

### Track 2: Developing a workflow

Define the process (stage/scope) and the AI assets (agents/skills). The full definition syntax is documented in the [Flow Definition Guide](docs/flow-definition-guide.md); this repository's `dmaf/` is itself a complete example:

```bash
# 1. Fill in the standard layout (brownfield never overwrites existing files, idempotent; for greenfield use dmaf new --dir my-project)
dmaf new --dir .

# 2. Author definitions and assets (syntax in the guide; with the authoring skill deployed to your harness by dmaf new, you can let AI do this — see the Flow Definition Guide §1)
#    dmaf/workflows/stages/<phase>/<slug>.md            # stage: frontmatter declares the structure, body carries the execution brief
#    dmaf/workflows/scopes/<name>.md                    # scope: include_stages claims members + vars/constants declarations
#    dmaf/agents/<name>.md  dmaf/skills/<name>/SKILL.md # agent and skill assets

# 3. Compile gate (all structural validation happens here; artifacts are byte-deterministic and committed — the diff is the review)
dmaf compile
dmaf compile --check          # drift guard (CI-friendly)

# 4. Deploy and validate (rerun install after AI-asset changes; ledger hash-gate semantics under Install semantics)
dmaf install --harness opencode
dmaf check --harness opencode --stages dmaf/workflows/stages --scopes dmaf/workflows/scopes

# 5. Inline dry runs (prototypes / experiments / tiny workflows; shares the parser with the standard directories)
dmaf validate --workflow <yaml>
dmaf start --workflow <yaml> --task "…"

# 6. Deliver and distribute
dmaf push <target-project-dir>                                   # deliver from the workflow project side (mirror of add, same merge core)
dmaf auth login --token <dmafmt_…> [--market <base>]     # market login (credentials in the user-level config dir; logout/status in the same group)
dmaf publish [<dmaf-dir>] [--intro <file>] [--dry-run] [--bump]   # publish to the market: compile+doctor gates → deterministic packaging → upload (update = republish with bumped version)
```

All commands emit JSON on success (stdout, UTF-8) and JSON errors on stderr; exit codes: `0` success, `2` usage error, `3` schema/definition validation failure, `4` state conflict, `5` filesystem error.

### Workflow variables

Workflow definitions are written before a session starts, while the requirement text, session identity, and business parameters only exist at runtime — **the variable mechanism is the bridge between the two**: templates reference names at definition time, and the engine renders values in after start. Stage artifact paths, `task_description`, delegation payloads, and delivery copy all share one session variable set — declared once, effective in four places. Before referencing variables in a workflow definition, understand where names come from:

- **Business variables** (scope `vars`): parameters only the person running the workflow knows, like the sample's `topic` (artifact filename prefix). Empty value = required, supplied via `dmaf start --var topic=cart`; a literal value = an inline default that may be omitted.
- **Constants** (scope `constants`): literals fixed at definition time, referencable in templates; `--var` cannot override them — changing a constant means changing the definition.
- **Engine-reserved variables**: `SESSION_ID` (session ID; artifact paths are bucketed per session, and forks automatically switch to the new bucket) and `TASK` (the original requirement text, carried only by the first stage's `task_description` — downstream stages always consume upstream artifacts).

At `dmaf start` the engine composes the variable set in order: constants → `vars` inline defaults → `--var` values → reserved variables; a template referencing an undeclared name is a compile-time error, so it never runs sick. Full rules: the [Flow Definition Guide](docs/flow-definition-guide.md) §4.

## Language support

- **English-source definition layer**: the agents/skills/workflows shipped in the package are authored in English, serving users of every language.
- **Output language follows your task language**: write the requirement in English and every deliverable (spec, hld, …) comes out in English, consistently along the pipeline; review feedback follows the language of your latest message and never switches an artifact's body language.
- **The conductor speaks your language**: everything the orchestrator says to you follows your latest message's language; engine-internal payloads and audit records stay English (stable protocol language, localized presentation).
- **Documentation**: `docs/` top level is the English default; Chinese versions live under `docs/zh/` (the Chinese text is the editing source of truth). CLI messages follow the system language (Chinese on zh systems, English otherwise) and can be pinned with `DMAF_LOCALE` / `start --locale`; baked into sessions.

## Install semantics

- **Ledger hash gate**: `dmaf-installed.yaml` at the harness root records the fingerprint of every write — missing targets are installed, unmodified targets are **automatically updated** on source upgrades, user-modified targets are **skipped** and reported (`--force` overwrites explicitly); agents and entry configs are per-file, skills are whole-directory. Reports `installed/unchanged/updated/skipped/overwritten/pruned`.
- **Nesting capability matrix**: opencode and cursor support recursive discovery and keep category subdirectories (`skills/<category>/<skill>/`); all other harnesses (claude/zcode/gemini/codex/dsh) flatten by skill name.
- **Entry configs**: files under `dmaf/harness/<name>/` are placed per-file inside the harness directory (e.g. `.opencode/opencode.json`), under the same ledger hash gate; never at the project root.
- `--prune` removes orphans; `--check-only` is a read-only check; doctor splits drift into `would_change` (rerun install to sync) and `modified` (user-modified, needs `--force`).

## Project layout

```
├── src/                       # Engine core (cargo package dmaf-engine, binary dmaf)
│   ├── workflow.rs / router.rs / state.rs / audit.rs / reports.rs
│   ├── registry.rs            #   harness registry (name → directory/layout/skills nesting capability)
│   ├── compile.rs / install.rs#   compilation & asset installation (ledger hash gate + auto-detection + entry configs)
│   ├── manifest.rs / package.rs #   project manifest (doctor compatibility) / workflow package merge core (add/push)
│   ├── market.rs              #   market integration (auth/publish/add entry URLs: HTTP stack, deterministic packaging, safe extraction)
│   └── forking.rs / schema_check.rs / cli.rs / paths.rs / scaffold.rs / hooks.rs
├── dmaf/                      # Definition layer (committed): the single source of truth for the example workflow & AI assets (fully replaceable)
│   ├── workflows/  agents/  skills/   # stage library + scope grid / agents / skills (grouped by capability domain)
│   ├── docs/流程定义指南.md    #   user guide (single source, embedded by dmaf new into user projects)
│   ├── harness/  manifest.yaml  hooks.yaml / compiled/
├── templates/                 # dmaf new skeleton templates (minimal only; include_dir-embedded in the binary)
├── .dmaf/                     # Runtime layer (gitignored): engine-owned session state & audit
├── docs/                      # User docs (English default: engine-manual / flow-definition-guide / phase-stage-scope + schemas/; Chinese under docs/zh/)
├── docs-dev/                  # Internal dev archive (gitignored, not published)
├── tools/skills/              # Authoring skill source (create-workflow: Q&A + document modes; include_dir-embedded, delivered by dmaf new into the target dmaf/skills/)
├── tests/                     # cargo test (180 cases, fully offline)
└── Cargo.toml / Cargo.lock / .cargo/config.toml
```

## Layout boundaries

| Location | Nature | Version control |
|----------|--------|-----------------|
| `dmaf/` | Definition layer: workflow definitions, AI assets, hook config, compiled artifacts | **fully committed** (`compiled/` guarded by `--check`) |
| `.dmaf/` | Runtime layer: `<timestamp>/` session directories (state, audit); engine-exclusive — no agent or human reads/writes it | gitignored entirely |
| harness directories (`.opencode` etc.) | install derivatives | managed by the engine; do not hand-edit |
| Business artifacts | Full paths live in stage `inputs`/`outputs` (supports `{SESSION_ID}` session bucketing; the sample uses `dmaf-output/`, a fixed prefix avoiding collisions with user directories) | decided by the user project (work products; the engine never writes .gitignore) |

Workflow change loop: edit `dmaf/workflows/` sources → `dmaf compile` → commit `dmaf/compiled/` (the diff is the membership-change review) → `dmaf compile --check` guards drift.

AI-asset change loop: edit `dmaf/agents/`, `dmaf/skills/` → `dmaf install --harness <name>` → `dmaf check --harness <name>` validates reference integrity. Daily self-check: after `git pull`, run `dmaf doctor --harness <name>` and follow its hints.

## Documentation

| Document | Contents |
|----------|----------|
| [Engine Manual](docs/engine-manual.md)（中文：[docs/zh/引擎手册.md](docs/zh/引擎手册.md)） | Arguments / exit codes / semantics for every command (the most complete engine-side reference) |
| [Flow Definition Guide](docs/flow-definition-guide.md)（中文：[docs/zh/流程定义指南.md](docs/zh/流程定义指南.md)） | stage/scope authoring guide (single source, embedded by `dmaf new` into user projects) |
| [Phase, Stage, Scope Definitions](docs/phase-stage-scope.md)（中文：[docs/zh/phase、stage、scope定义.md](docs/zh/phase、stage、scope定义.md)） | The authoritative inventory of the sample phases/stages/scopes |
| [AGENTS.md](AGENTS.md) | Repository development notes (structure, commands, conventions & pitfalls) |

## Build & dependencies

Rust toolchain (rustup 1.85+)

```bash
cargo build --release   # target/release/dmaf
cargo test
```

## License

Dual-licensed: [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) — use either at your option.
