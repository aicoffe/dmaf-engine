---
name: create-workflow
description: 'Guides creating dmaf workflow definitions in two modes: Q&A mode (a short round of questions gathers the business information; fits spot additions of a stage, new workflows (scopes), or adding an approval gate to a flow) and document mode (the user supplies a process definition document, treated as the single source of truth and converted wholesale into dmaf definitions). Generates dmaf/workflows/stages and dmaf/workflows/scopes definition files (all boilerplate fields filled in automatically), then runs dmaf compile for validation after the user confirms. Use when the user wants to add a stage, create a new workflow (scope), add an approval gate to a flow, mentions "add a phase/step/stage/flow", or hands over a document describing phases/stages/inputs and outputs/review gates/roles to convert into dmaf definitions — even when they never say dmaf or workflow.'
---

# create-workflow: dmaf process definition authoring assistant

Help the user create dmaf workflow definitions; pick the mode automatically from the input:

- **Q&A mode** (default): used when there is no written process document. The user only answers business questions (what it does, inputs and outputs, dependencies, whether human approval is needed); you fill in all boilerplate fields from the templates and defaults.
- **Document mode**: the user supplied a process definition document (typical shape: a phase overview, a per-stage input/output table, a Gate table, a roles-and-skills table — e.g. the engine repo's `idea/00-工作流定义.md`). **The document is the single source of truth** — extract everything stated, derive the implied where reasonable, flag every assumption explicitly, ask only about points that are genuinely ambiguous or self-contradictory, and never interrogate field by field. If the user gave a document path or attached a document to the message, take this mode; if not, ask once for the document (asking for it does not count as "Q&A mode"). When the document covers only part of the flow, do document-mode extraction plus Q&A only for the gaps.

Both modes share steps 2–4 (generate, confirm, compile).

## Language

Reply in the language the user writes in. Language policy (the producers' language policy): user-visible text in generated definitions (stage/scope bodies, `task_description`, `description`, delivery text) is written in the language of the user's original task statement; when consuming upstream artifacts, match their established language and never switch a document's language midway. Identifiers (`id`, `phase`, agent/skill names, business path segments) use English kebab-case.

## Prerequisites

- Use inside a dmaf project (created by `dmaf new`, containing the `dmaf/workflows/` definition layer).
- Locating the dmaf binary: `dmaf` on PATH. If not found, tell the user to install it (`cargo install dmaf-engine`, or download a binary from Releases); do **not** try to reference the engine source directory.
- For document mode, first survey the target project's current state: existing `dmaf/workflows/stages|scopes`, `dmaf/agents|skills`, and existing scope declarations — new definitions must stay consistent with the project's current state (variable names, artifact path style, agent naming, approval gates).

## Material inside this skill (read on demand)

- `references/field-reference.md` — quick reference for every stage/scope field + template-variable rules + the compile error reference table. **Read it once before generating any definition**; when unsure about a field, it wins.
- `references/mapping-guide.md` — mapping from document concepts to dmaf fields (review stage → approval gate, execution order and data dependencies, "on-demand" outputs vs the outputs promise, revision loops, external inputs, multi-role stages, scope assembly) + a pitfall list. **Read it before generating anything in document mode**; in Q&A mode read as needed.
- `references/examples.md` — seven real-shape examples taken from the official SDLC samples (intent admission / single agent + approval gate / parallel dual agent / multi-input confluence + soft references / task-driven scope / constants and variable picking / trimmed scope).
- `references/doc-example.md` — a complete document-mode walkthrough: document excerpt → extraction list → mapping decisions and assumption list → generated files.
- `assets/stage-template.md`, `assets/scope-template.md` — the skeletons used to generate files.

## Step 1: Gather intent

### Q&A mode: ask everything in one round

One round, all at once (skip what the user already said, ask only what is missing):

1. What does this stage **do** (one sentence)? → suggest `id` (kebab-case) and `phase` from that (list the project's existing phases to choose from, or create a new one)
2. **Input artifacts**: what are they called, and which existing stage produces them (or are they provided externally)?
3. **Output artifacts**: what are they called?
4. Which stages does it **depend on** (decided by the producers of its inputs; they must precede the consumer in the scope `include_stages` sequence)?
5. Is **human approval** needed? What is the gate called (e.g. `risk-review`)?
6. Which **agent** to use (list the existing ones under the project's `agents/` to choose from)? skills as needed (default `[]`)
7. Join which **scope** or scopes, or create a new scope?
8. **Input shape**: is the workflow aimed at natural-language requirements from humans (task-driven: the first stage uses `{TASK}` for intent/admission judgment and may declare no vars; downstream stages consume only upstream artifacts and never reference `{TASK}`), or structured parameters from machines/upstream systems (the scope declares a `vars` contract, provided via `start --var`)? Both can coexist.

When the user says "stop asking, just build it", fill in reasonable defaults and flag the assumptions in the result.

### Document mode: extract the process model

Read the document through and build an extraction list in working notes:

- Each phase and its stages (number, name, one-line purpose)
- Per stage: inputs, outputs, review gate, human role, required skills
- Relationships: serial/parallel, revision loops ("rejected → sent back for rework"), "triggers the next phase"
- Artifact paths and lifecycle (feature-level vs service-level)

While extracting, record contradictions inside the document — a stage consuming an artifact nobody produces, a Gate with no producing stage, a "review" that both adjudicates and rewrites. These become the material for the single concentrated round of questions at the end of this step.

If you have not yet read `references/mapping-guide.md`, read it now. The load-bearing mappings (details and the pitfall list live in that file):

- **Review stage / Gate** — a review where humans only "accept or reject" maps to `approval: {required: true, gate: <slug>}` on the *producing stage*: the dmaf approval gate has the "reject → revise → re-dispatch" loop built in. A "review" that *produces new artifacts* (an index, corrections, a merged draft) stays an independent stage, which may itself carry an approval gate.
- **Order and parallelism** — the scope's `include_stages` **is the execution order** (strictly one stage at a time; the engine never reorders); producers of hard inputs must be members and precede their consumers, and the data-dependency edges are derived automatically from the inputs — never declared explicitly. "A and B can run in parallel" in the document has no stage-level parallelism: place them into the sequence in document order, and if they truly must be produced simultaneously, merge them into one `collaboration: parallel` stage (sharing the approval gate and retry policy). Pure precedence with no artifact handoff is just sequence position — no declaration needed.
- **Artifacts** — `inputs`/`outputs` carry full path templates (project-root-relative, `{SESSION_ID}` session bucketing allowed); after rendering, a consumer's path must match the producer's exactly; `outputs` are promises — list only artifacts that are necessarily produced; on-demand/trimmable upstreams use soft references (`{artifact: …, required: false}`). When the documented flow takes natural-language input from humans, the first stage carries the original requirement text via the reserved variable `{TASK}` for intent/admission judgment — downstream stages consume only upstream artifacts and never reference `{TASK}`.
- **Roles → agents, skill columns → skills** — missing assets are acceptable at definition time: write the names derived from the document anyway and flag them as "assets to be created" (deployment-time `dmaf check --harness` will surface the gap).

Then **ask one round only**: what can be derived from context or convention → decide directly and record one line in the **assumption list** (what was chosen and why); what is genuinely ambiguous or contradictory → accumulate into **one** concentrated round of questions. Never drip-feed questions; never ask what the document already answered.

## Step 2: Generate definition files

Using `assets/stage-template.md` as the skeleton, generate `dmaf/workflows/stages/<phase>/<slug>.md`, filling in automatically:

- `retry: {max_attempts: 1, on_exhausted: abort_workflow}` (the defaults, unless the user asks / the document says otherwise)
- `execution: always` (output-format conventions go into the skill's execution instructions; the protocol carries no format field)
- Artifact paths carry business paths (conventional prefix `dmaf-output/{SESSION_ID}/<type dir>/` with session bucketing; work products are committed; stay consistent with the project's existing path style); an inputs path must match the upstream outputs exactly after rendering

Compile constraints that must hold at generation time (violations fail the compile; details in field-reference):

- Producers of hard inputs must be scope members and precede their consumers — **derive members and order from the inputs**; do not miss any;
- Producer-consumer relations form on an **exact match of rendered paths** in inputs/outputs (the same name in a different directory is a different artifact);
- The scope's `include_stages` **order is the execution order** — producers precede consumers. Switching to a soft input (`required: false`) expresses on-demand trimming only when **the producer is not a member** (absence = not produced on demand); when the producer is a member but comes after the consumer, a soft input still fails the compile (dead wiring) — the only fix is reordering.

Scope updates edit the `include_stages` of `workflows/scopes/<name>.md` directly (order is the execution order; the compiler no longer reorders). One scope per end-to-end flow. Before writing, re-check the hard constraints against field-reference one more time (template variables come only from the vocabulary, stage files must not carry scope fields, `collaboration: single` is exactly 1 agent, ...) — getting it right in one pass is cheaper than compile rework.

## Step 3: User confirmation

- Q&A mode: show the user the generated file paths + full contents and ask explicitly "All correct? Anything to adjust?"
- Document mode: show the user the stage table (id / phase / collaboration / approval gates / sequence position / inputs → outputs), the scope (with the ordered `include_stages`), all generated file paths, and the complete assumption list.

Apply any requested changes until confirmed.

## Step 4: Compile validation

Run only after the user confirms:

```bash
dmaf compile
```

- **Success**: report the stage/scope counts and remind the user to commit `dmaf/compiled/` (the artifact diff is the review record of the process change); when assets were flagged "to be created", suggest following up with `dmaf check --harness <name>` (AI asset completeness).
- **Failure**: explain each problem one by one (against the compile error reference table in field-reference), fix it yourself and retry — **fix compile errors yourself first; ask the user only about business-semantic questions**.

## Red lines

- Never hand-edit artifacts under `dmaf/compiled/`;
- Never overwrite the user's existing stage/scope files (same name → ask first);
- Revision loops live in the approval gate, not in the structure — the execution order is a linear sequence; a gate rejection automatically re-dispatches carrying the revision notes;
- Do not deliver until the compile passes; fix until it passes.
