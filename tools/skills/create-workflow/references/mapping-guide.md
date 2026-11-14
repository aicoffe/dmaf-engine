# Document → dmaf definition mapping guide

Translate a process definition document (typical shape: a phase overview, Stage tables, Gate tables, role/skill tables, artifact ownership tables — e.g. the engine repo's `idea/00-工作流定义.md`) into dmaf stage/scope definitions. This guide gives the concept-level mappings and pitfalls; field details are in `field-reference.md`, and a complete walkthrough is in `doc-example.md`.

## 1. Concept mapping table

| Document concept | dmaf mapping | Notes |
|---|---|---|
| Phases (concept/requirements/design/development...) | `phase` (directory name + frontmatter field) | English kebab-case (concept/requirement/design/build). phase is a pure classification label and **takes no part in routing** — execution order is expressed by the scope's `include_stages` sequence |
| A row of the Stage table (number/name/description/inputs/outputs) | one `dmaf/workflows/stages/<phase>/<slug>.md` | The number (1.1) stays out of `id`; `id` = an English kebab-case slug (intent capture → `intent-capture`); the number and the original name go into the body title (`# 1.1 Intent Capture`) |
| Input/output artifacts | the agents entry's `inputs` / `outputs` (full path templates) | Write the document's repo paths as project-root-relative full path templates (`dmaf-output/{SESSION_ID}/specs/{feature}-spec.md`); template variables may only come from the variable vocabulary (see field-reference) |
| Gate / review tables (GATE-1..., review stages) | the producing stage's `approval: {required: true, gate: <slug>}` | See the criteria in §2 — most "review stages" are approval gates, not independent stages |
| Roles (Product Owner, Architect...) | the agents entry's `agent` name | When the project's `dmaf/agents/` has no matching asset, write the name anyway and flag "asset to be created" (deployment-time `dmaf check --harness` flags it; a runtime absence is an execution error) |
| Required skills (skill table/skill column) | the agents entry's `skills` list | Same as above; when the document says only "a XX-category skill" without a name, coin a semantically fitting English kebab-case name and flag the assumption |
| Parallel relations ("3.3 and 3.4 can run in parallel") | adjacent placement in the sequence, or merge into one stage | The engine has no stage-level parallelism (one stage dispatched at a time): place them into `include_stages` in document order; if they truly must work simultaneously, merge into one `collaboration: parallel` stage (sharing the approval gate and retry policy) |
| Purely managerial precedence (no data dependency but a strict order) | position within `include_stages` | **Order is the execution order**: just place it before its consumer; no dependency declaration of any kind |
| Intra-stage flow (the TDD loop, internal judgment) | body `## Steps` | Structure expresses only the stage sequence; how the work happens inside a stage lives in the body |
| "On-demand" outputs (the hld updated on demand, an ADR if needed) | see §3 | `outputs` are promises; conditional artifacts do not belong there |
| Revision loop ("rejected → sent back for rework → reviewed again") | built into the approval gate | The sequence has no cycles and needs none: a gate rejection automatically re-dispatches the same agent (carrying `revision_feedback`, unlimited rounds) |
| "Triggers the next phase" ("triggers Build") | no declaration of any kind | The sequence advancing onto it is its execution |
| Metrics tables, principles tables, document responsibility boundaries | no mapping | May be condensed into a sentence or two of stage/scope body background; do not copy whole passages |

## 2. Review stage: approval gate or independent stage?

The criterion in one sentence: **a review that only produces an accept/reject verdict → an approval gate; a "review" that produces new artifacts → an independent stage (which may itself carry an approval gate).**

### Map to an approval gate (most common)

- Document shape: "X.2 has a human review X.1's output, accept or reject; rejection sends it back for rework" (e.g. 1.2 intent review, 2.2 spec review, 3.2 architecture design review, 4.2 Plan Review).
- dmaf shape: delete X.2 and declare `approval: {required: true, gate: <slug>}` on the producing stage X.1. The approval gate has `awaiting-approval → granted → completed / rejected → revising` built in — on rejection the Engine automatically re-dispatches the same agent carrying `revision_feedback` (= the rejection notes), unlimited rounds. The document's "rejected → sent back for rework → reviewed again" is exactly this built-in loop; **do not build a stage for it and do not express the back edge in the structure** (the execution order is a linear sequence; the back-edge semantics are carried by the gate).
- Gate naming: English kebab-case, unique within the flow; the convention is "object + review" (`outline-review`, `spec-review`).
- Optional `approval.reviewer: <agent>`: hand the disposition to a designated agent for pre-review and file the approval report with its conclusion; omitted = human disposition.
- The review-record artifact: an approval gate produces no files. Either the reviewed stage produces it alongside its main artifact (add it to that stage's `outputs`), or omit it from the flow — pick one and record it in the assumption list.

### Keep an independent stage

- Document shape: "review and produce the design.md index / write back spec v2 / the corrected design" (e.g. 3.5 conflict adjudication and design fixes, 3.6 design review finalization).
- dmaf shape: a normal stage — `inputs` = the reviewed material, `outputs` = the corrections/index; if human finalization is still needed afterwards, hang an approval gate on it.

## 3. "On-demand" outputs and the outputs promise

`outputs` are verified for file existence by the Engine at report time, and any missing one is treated as failure — **promise only artifacts that are necessarily produced**. For "update on demand / produce if needed" entries in the document, choose one of three:

1. **Split the promise (default)**: the necessarily-produced artifact goes into `outputs` (e.g. the "impact statement"); the on-demand one does not, and goes into the body Steps ("if the judgment is that architecture is affected, update the hld and add an ADR"). The architecture design stage's standard shape is exactly this: `outputs` = the impact statement.
2. **Agree on a placeholder**: agree in the body that "even with no changes, a 'no-change statement' file is produced", so the artifact always exists.
3. **Split the stage + conditional**: split the conditional work into an `execution: conditional` stage. Note: **a producer of hard inputs may not be conditional-only** (intercepted at compile time); conditional semantics are detailed in the engine manual.

### The semantic difference of service-level long-lived assets

Assets the document marks "service-level, long-term maintained" (`architecture/hld.md`, `adr/`) clash with the dmaf session model: session artifacts live under the business output directory's session bucket (e.g. `dmaf-output/{SESSION_ID}/`) and are isolated per session, while such assets live in the repo across sessions. Mapping conventions:

- "Read the existing hld" → a soft input `{artifact: …, required: false}` (absence tolerated; greenfield projects naturally lack it);
- "Write a new hld" → output it as an in-session review draft (`outputs` promises the review draft); merging it into the repo is executed by a human, stated in the body.

This is a real semantic difference; surface it to the user in the assumption list — never swallow it silently.

## 4. External inputs and soft inputs

- Inputs the document marks as coming from outside (ideas, tickets, alerts, existing system context): **do not write them into `inputs`** — producers of hard inputs must be scope members and precede their consumers (validated at compile time). Put them into `task_description` (passed in via scope `vars`) or the body Steps.
- Inputs that may be missing: a soft input `{artifact: …, required: false}` or `watches` (reading interest); absence only lands in the instruction's `missing_inputs` field (advisory). **Dead-wiring interception**: if none of the **member** producers of a soft input/watches precedes the consumer, that is a compile error (under the linear execution order it would be absent forever in this session); a **non-member** producer (on-demand trimming) or external provision is legal — the lightweight flow trimming away a design chain relies on exactly this.

## 5. Multi-role collaboration stages

- Multiple agents in one stage's `agents` list → `collaboration: sequential` (one by one in the document's order, the default) or `parallel` (simultaneously when independent). `single` must be exactly 1 agent.
- "Multiple role groups within one phase, each reviewing its own part" (e.g. design review split into an lld group / UE group / finalization group): usually **splitting into multiple stages** is clearer — one review per reviewed object, each with its own approval gate; the criterion: different input/output sets → split. Only genuinely same-in-same-out work uses a multi-agent stage.

## 6. Variables and scope assembly

- Find the per-feature/per-topic dimension word in the document → scope `vars` (**mapping form**: empty value = required like `topic:`, literal value = optional inline default like `constraints: ""`); a `{feature}` appearing in the document's artifact paths is a template variable, and the names in both places must match.
- Fixed literal values (product line names, a fixed target system, etc.) → `constants` (literal values are constant, referencable from templates, rejected by `--var`). The legacy fields `defaults`/`derived_vars` have been removed (misuse reports a migration error): optional values use vars inline defaults, and composite values are referenced directly by templates (e.g. `"{product_line}-{SESSION_ID}"`).
- When the documented flow takes **natural-language input from humans** (the user starts it with a requirement paragraph): prefer task-driven — declare no vars; the first stage carries the original requirement text via the reserved variable `{TASK}` for intent/admission judgment (reporting `skipped` when unfit), and the extracted business concepts go into the intent/requirement artifacts for downstream consumption; `vars` is reserved for structured contracts from machines/upstream systems.
- One end-to-end flow = one scope, `name` in English kebab-case (e.g. `sdlc-standard`); `include_stages` **order is the execution order**: producers of hard inputs must be members and precede their consumers (derive members and order from the inputs); precedence without a data dependency is expressed by sequence position; downstream of a trimmed chain switches to soft references (a non-member producer = legal absence).
- `completion_template` (recommended to write): placeholders may only use declared `{vars.<name>}` (constants included) and member stages' `{outputs.<stage_id>}`; conditionals and loops are deliberately unsupported — presentation templates do no logic.

## 7. Pitfall list

| Pitfall | Consequence | Prevention |
|---|---|---|
| Building a cycle or round-trip stages for "reject and send back" | semantic misalignment; the sequence has no cycles | Revision loops live in the approval gate, not in the structure |
| A hard input with no producer / the producer after the consumer | compile error | Derive members and order from the inputs; external pre-existing files become soft inputs |
| No member producer of a soft input/watches precedes the consumer (dead wiring) | compile error: absent forever under the linear order | Adjust the order, or have it provided by a non-member / externally (non-member absence = on-demand trimming, legal) |
| Input and output share the same name (self-consumption) | compile error | Rename |
| Scope fields written in a stage file (`scopes`, `depends_on`, etc.) | compile error (depends_on removed) | Execution order lives in the scope file's `include_stages`; so does membership |
| Template variables outside the vocabulary | compile error: an unknown variable is referenced | A new variable must first enter scope `vars`/`constants` |
| Scope declarations retaining legacy fields (`defaults`/`derived_vars`) or list-form `vars` | compile migration error | `vars` uses mapping (empty = required / literal = default); constants go into `constants` |
| A conditional-only stage is the producer of a hard input | intercepted at compile time | Change to `always` or split the stage |
| `outputs` promises an "on-demand" artifact | the runtime report treats it as failure | The three strategies in §3 |
| The scope references a nonexistent stage / duplicate entries | compile error | Check the sequence item by item and deduplicate when generating the scope |
| `collaboration: single` with multiple agents | compile error | single=1; multiple agents use sequential/parallel |
| `max_attempts` read as the total count | twice as many retries as expected | It is the retry cap **after the first attempt** (`1` = 2 attempts in total) |
| `id` ≠ the file-name stem, `phase` ≠ the directory name | compile error | Check item by item at generation |
| Swallowing the "service-level asset" semantic difference silently | artifacts land in the wrong place at runtime; user confusion | Map per the §3 conventions and record it in the assumption list |
