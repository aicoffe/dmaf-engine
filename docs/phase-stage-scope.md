# Phase, Stage, Scope Definitions

> Language: English | [中文](zh/phase、stage、scope定义.md)

**Status:** In sync with the in-repo `dmaf/workflows/` definitions (2026-11-12, engine 0.6.0: 12 stages / 2 scopes / 8 agents / 29 skills in 12 categories)
**Reference:** `idea/流程定义/ai-native-sdlc phase、stage、scope定义.md` (the consolidated AI-native SDLC lifecycle)
**Artifacts:** The definitions listed in this document are compiled via `dmaf compile` into `dmaf/compiled/` (byte-deterministic, committed to the repo, guarded by `--check`).
**Authoring guide:** For the full text, see `docs/zh/流程定义指南.md` (single source in the engine repo, distributed with the binary); this document is a quick overview of the sample inventory.

---

## 1. Overview

Three **standard directories** make up the authoring layer:

| Directory | Content | Rules |
|---|---|---|
| `workflows/stages/<phase>/<slug>.md` | Stage library (frontmatter declaration + body execution instructions) | id = file name stem; phase = directory name; must not carry the scope / depends_on fields (ordering belongs to the scope, data supply belongs to hard inputs) |
| `workflows/scopes/<name>.md` | Scope: the **ordered** `include_stages` member list (order of include_stages is the execution order) + process-level metadata (vars / constants / completion_template) | name = file name stem; producers of hard inputs must be members and precede their consumers (enforced at compile time) |
| `agents/`, `skills/` | AI assets (referenced by the agents/skills fields of stages) | Deployed via `dmaf install --harness <name>` (ledger hash gate); `dmaf check` verifies reference completeness |

**A phase is just a directory category**: a pure classification label that takes no part in routing. The single source of truth for execution order is the scope's `include_stages` sequence — the engine advances strictly serially along the sequence (one stage at a time), with no stage-level parallelism; parallelism happens only among multiple agents inside a stage (`collaboration: parallel`).

**Data dependency**: producers and consumers match by **exact path-template equality** on inputs/outputs (equal after rendering). Hard inputs express "must supply data" (the producer must be a member and come before the consumer; the data-dependency edges are derived at compile time — no declaration needed); soft inputs (`required: false`) express "optional reference" — when the producer is absent (on-demand skipped, or pruned as a non-member), the on-demand artifact was simply not produced, and the absence is surfaced as `missing_inputs` in the dispatch instruction.

**Approval gate semantics**: generation + review are merged into a single stage carrying `approval.gate` — after generation completes, the stage halts at `awaiting-approval`; approval (granted → completed) or rejection (rejected → revision loop, dispatch carries `revision_feedback`, unlimited rounds).

## 2. Stage Summary Table (12 stages / 4 phases)

All artifact paths use the session bucket `dmaf-output/{SESSION_ID}/<subdirectory>/{topic}-<artifact>.md` (except the tech-stack declaration); retry is uniformly `1 / abort_workflow` (the 3-attempt task-level verification inner loop of atomic-execution is a task_description convention). "Soft" = a `required: false` soft input.

### Concept (concept)

| Stage | Agent / Skill | Inputs → Outputs | Approval gate | Execution |
|---|---|---|---|---|
| intent-capture | product-manager / requirement-analysis | `{TASK}` requirement verbatim → `{topic}-intent.md` | **intent-accept** | always |

### Requirements (requirements)

| Stage | Agent / Skill | Inputs → Outputs | Approval gate | Execution |
|---|---|---|---|---|
| spec-generation | product-manager / requirement-analysis | intent → `{topic}-spec.md` (v1) | **spec-review** | always |

### Design (design)

| Stage | Agent / Skill | Inputs → Outputs | Approval gate | Execution |
|---|---|---|---|---|
| hifi-ue | ux-designer / ue-prototyper-react-mantine | spec → `{topic}-high-fidelity-ue.md` (delivery index of the proto/** coded prototype) | **ue-review** | conditional (skipped for backend-only / no UI) |
| architecture-analysis | architect / architecture-analysis | spec + UE (soft) → `{topic}-architecture-impact.md` + `dmaf-output/tech-stack.md` | **architecture-review** | always |
| hld-design | architect / hld-design | spec + impact statement → `{topic}-hld.md` | **hld-review** | conditional (skipped when no architecture design is needed) |
| detailed-design | backend-engineer ∥ frontend-engineer / detailed-design-backend + detailed-design-frontend | spec + impact statement + hld (soft) + UE (soft) → `{topic}-lld-backend.md`, `{topic}-lld-frontend.md` | **lld-review** | always |
| conflict-resolution | tech-lead → product-manager (sequential) / — | spec + lld×2 + on-demand artifacts (UE / impact statement / hld, soft) → `{topic}-conflict-resolution.md`, `{topic}-design.md`, `{topic}-spec-v2.md` | **design-accept** | always |

### Development (development)

| Stage | Agent / Skill | Inputs → Outputs | Approval gate | Execution |
|---|---|---|---|---|
| plan-generation | backend-engineer / test-design, git-workflow | spec v1 (**hard**) + spec-v2 / design index / lld×2 / UE (soft) → `{topic}-plan.md` | **plan-review** | always |
| task-breakdown | backend-engineer / test-design | plan → `{topic}-tasks.md` (atomic tasks + TDD steps) | — | always |
| atomic-execution | backend-engineer → frontend-engineer (sequential) / automated-testing + backend-development, frontend-development | tasks (hard) + lld (soft) + UE (soft) → `{topic}-implementation-backend.md`, `{topic}-implementation-frontend.md` | — | always |
| system-testing | qa-engineer / automated-testing, contract-testing | implementation & verification ×2 (hard) + spec-v2 / lld×2 / UE (soft) → `{topic}-system-test-report.md` | — | always |
| pr-preparation | backend-engineer / git-workflow, code-review, documentation | plan + tasks + implementation & verification ×2 + system test report → `{topic}-pr.md` | **pr-review** | always |

## 3. Scope Inventory (2)

| Scope | Members (order of include_stages is the execution order) | Approval gates | Positioning |
|---|---|---|---|
| `sdlc-standard` | intent-capture → spec-generation → hifi-ue → architecture-analysis → hld-design → detailed-design → conflict-resolution → plan-generation → task-breakdown → atomic-execution → system-testing → pr-preparation (12) | Nine: intent-accept / spec-review / ue-review* / architecture-review / hld-review* / lld-review / design-accept / plan-review / pr-review | The standard full process; on-demand stages (*) are resolved by a runtime skipped report (no gate is set) when the project is backend-only / no UI or needs no architecture design |
| `sdlc-bugfix` | intent-capture → spec-generation → plan-generation → task-breakdown → atomic-execution → system-testing → pr-preparation (7) | Four: intent-accept / spec-review / plan-review / pr-review | Lightweight defect-fix path: the five design-chain stages are non-members; plan takes spec v1 as its mandatory hard baseline, and design artifacts are soft references that may be absent on demand. When an impact assessment of the existing system is needed, insert architecture-analysis between spec-generation and plan-generation |

**Naming convention**: scope/agent/skill names are published with the package name as prefix (`sdlc-` ≙ package `dmaf-sdlc`) to avoid cross-package collisions on generic names — the same-family naming of `sdlc-standard` / `sdlc-bugfix` is exactly this convention.

**How sequencing relates to data dependency** (sdlc-bugfix demonstrates the mechanism): removing a member requires also converting the hard inputs that consume its artifacts into soft references (otherwise the compile error "producer not in the member sequence"); soft-referencing artifacts of non-member producers is the legitimate form of on-demand pruning.

## 4. AI Asset Inventory

**agents (8)**: `dmaf-conductor` (the orchestrator, primary) plus 7 SDLC roles: product-manager, ux-designer, architect, backend-engineer, frontend-engineer, tech-lead, qa-engineer.

**skills (29 in 12 categories)**: `dmaf` (orchestration skill) and `rules` (rules charter layer) stay at the top level; the rest are grouped by capability domain — requirements(1), architecture(3), backend(3), frontend(4), engineering(3), testing(3), governance(1), operations(1), security(1), stacks(7: two frontend stack bindings + two UE prototypers + three backend stack bindings).

## 5. Evolution Log (relative to the `idea/` reference document)

The reference document's pruning decisions (Operations / Init / continuous Evals all removed; review stages merged into approval gates) were historically valid. The main evolutions since then:

| Reference form | Current form |
|---|---|
| lofi-ue → hifi-ue, two-stage design drafts | hifi-ue, single-stage coded prototype (proto/**, click-to-annotate review loop) |
| architecture (5 views / ADR produced in one pass) | architecture-analysis (impact judgment + stack declaration) → hld-design (expanded on demand) |
| spec-finalization (a separate finalization stage) | merged into conflict-resolution (tech-lead adjudicates fixes → PO finalizes, `design-accept` gate, producing the design index + spec v2) |
| self-check (a separate self-check stage) | merged into atomic-execution's task-level verification inner loop |
| — (none) | new system-testing (qa-engineer tests only, never fixes) |
| sdlc-lean (demo-pruned) | superseded by sdlc-bugfix (a lightweight path driven by real scenarios, unlocked by engine 0.6.0 scope sequencing) |

Semantic change (0.6.0): the `depends_on` field was removed; order of include_stages is the execution order; data dependencies are expressed as derived edges inferred from hard inputs; dead wiring of soft inputs is blocked with a compile error — the design notes live in the internal dev archive (docs-dev/, not published).

## 6. Compile and Validation

```bash
dmaf compile            # workflows/ → dmaf/compiled/ (all structural validation happens at compile time)
dmaf compile --check    # drift guard: byte-by-byte comparison of sources vs committed artifacts
dmaf check              # reference completeness (defaults to the standard layout when no input source is given)
dmaf doctor             # layout / artifact drift / manifest compatibility / reference completeness / asset drift
```

Key compile-time validations (for the full rules and error mapping see `docs/zh/流程定义指南.md` §8): producers of hard inputs must be scope members and precede their consumers; dead wiring of soft inputs / watches (no member producer precedes the stage) is rejected; a hard input whose sole producer is a conditional stage must be a soft reference (rule 10); template variables ∈ the scope-declared set ∪ engine-reserved variables (SESSION_ID / TASK); id = stem, phase = directory name, and a stage carrying the scope / depends_on fields are all compile errors.

## 7. Extension Guide

**Adding a stage**: create `<slug>.md` under `workflows/stages/<phase>/` (template in `docs/zh/流程定义指南.md` §2.4) → downstream stages add its outputs to their inputs (data dependency is expressed through inputs) → place it before its consumers in the scope's `include_stages` sequence → `dmaf compile` and commit the compiled artifacts → `dmaf check` to confirm the AI assets are complete.

**Adding a scope**: create `<name>.md` under `workflows/scopes/` (name = stem), write `include_stages` in execution order, and declare vars/constants and completion_template → `dmaf compile`.

**Run entry point**: `dmaf start --scope <name> --task "<requirement verbatim>" --var topic=<short-name>` (task-driven; the vars contract follows the scope declaration).
