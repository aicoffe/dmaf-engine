# Field quick reference

Every legal field of stage/scope. Kept in sync with the engine's Flow Definition Guide (docs/flow-definition-guide.md); where conflicting, the latest guide shipped with the engine wins.

## Stage frontmatter (dmaf/workflows/stages/<phase>/<slug>.md)

| Field | Required | Description |
|---|---|---|
| `id` | ✓ | unique stage identifier, **must equal the file-name stem** |
| `phase` | ✓ | **must equal the enclosing directory name** (a pure classification label, takes no part in routing) |
| `collaboration` | ✓ | `single` (exactly 1 agent) \| `sequential` \| `parallel` (≥2 agents) |
| `agents` | ✓ | the worker list, see the table below |
| `retry.max_attempts` | — | the retry cap **after** the first attempt (`1` = 2 attempts in total), default 1 |
| `retry.on_exhausted` | — | `abort_workflow` (default) \| `continue` (failure does not block the flow; missing items are not removed from downstream inputs) |
| `approval.required` | — | when `true`, the stage halts at `awaiting-approval` on completion awaiting human disposition; rejection enters the revision loop (automatic re-dispatch carrying revision_feedback, unlimited rounds) |
| `approval.gate` | required when approval.required | the gate name, unique within the flow (e.g. `outline-review`) |
| `approval.reviewer` | — | optional: hand the disposition to a designated agent for pre-review and file the approval report with its conclusion; omitted = human disposition (audit shards record the reviewer's provenance) |
| `execution` | ✓ | `always` \| `conditional` (for conditional's skip semantics see the engine manual; a producer of hard inputs may not be conditional-only — intercepted at compile time) |
| `on_skip` | — | `skip` (default) \| `run`: when a hard-input producer is skipped, whether this stage propagates the skip or is dispatched as usual |

**A stage file must not carry scope fields** — execution order and membership live only in the scope file. The `depends_on` field has been removed (residue reports a migration error): order goes into the scope's `include_stages` sequence (order is the execution order), and data dependencies are expressed by hard-input full paths (producers derived automatically).

### agents entries

| Field | Required | Description |
|---|---|---|
| `agent` | ✓ | the agent name; must be deployed to the harness (an agents/ asset, mode including subagent/all) |
| `skills` | — | the list of skill names; may be `[]` |
| `inputs` | — | **full path templates** of input artifacts (project-root-relative, `{SESSION_ID}` session bucketing allowed): string = hard dependency; the mapping `{artifact: …, required: false}` = soft input (tolerates absence) |
| `watches` | — | the soft-dependency (reading interest) list; parsed into the instruction payload, tolerates absence |
| `outputs` | — | **full path templates** of output artifacts; a rendered path equal to some inputs entry's rendered result makes them producer-consumer; at report time the Engine verifies the files exist and treats any missing one as failure — **promise only artifacts that are necessarily produced**. The output format is agreed in the skill's execution instructions; the protocol carries no format field |
| `task_description` | — | the task description sent to the agent; supports `{variable}` templates; the user's raw input is carried only by the first stage via `{TASK}` — downstream stages consume only upstream artifacts |
| `vars` | — | optional: explicitly pick a subset from the session variable set (constants included), flattened into the action's `vars` field (e.g. `[product_line]`) |

## Scope frontmatter (dmaf/workflows/scopes/<name>.md)

| Field | Required | Description |
|---|---|---|
| `name` | ✓ | **must equal the file-name stem** |
| `description` | — | a one-sentence summary of the flow |
| `vars` | — | business variable declarations, **mapping form**: empty value = required (must be provided via `start --var`; missing refuses startup), literal value = optional inline default; may be omitted — a purely task-driven flow takes the natural-language original text of `start --task` |
| `constants` | — | constant declarations (mapping): literal values are constant, referencable from templates, always take the defined value, rejected by `--var`; names are mutually exclusive with `vars` and must not occupy reserved names |
| `completion_template` | — | the optional delivery-text template, see the next section |
| `include_stages` | ✓ | member claims + **the execution sequence** (the single source of truth; **order is the execution order**; the engine never reorders); producers of hard inputs must be members and precede their consumers; referencing a nonexistent stage and duplicate entries are both compile errors |

Removed fields (misuse reports a migration error): `defaults` (→ vars inline defaults), `derived_vars` (→ constants or composite references from templates), `description_from_stage` (→ description).

### completion_template syntax (a dedicated renderer, not shared with artifact templates)

| Placeholder | Renders as |
|---|---|
| `{vars.<name>}` | a session variable (constants included); the name must be declared |
| `{outputs.<stage_id>}` | the output paths of all agents of that stage, joined with "、"; stage_id must be in `include_stages` |
| `{{` / `}}` | literal braces |

Conditionals and loops are deliberately unsupported; a placeholder referencing an undeclared variable or a non-member stage is a compile error. Omitting the field = the complete instruction carries no `completion_text`, and the Orchestrator falls back to relaying the summary.

## Template-variable vocabulary (the only names usable in artifact templates)

Variables ∈ **the scope declaration set** (names of `vars` ∪ `constants`) ∪ **engine-reserved variables**; compile time gates against the union of all scope declarations, and an undeclared variable is an immediate compile error.

**Reserved variables** (no declaration needed, injected by the mechanism):

| Variable | Description |
|---|---|
| `SESSION_ID` | session ID (a timestamp string, live-bound on every render to the current session directory name; used for deriving unique identifiers and for session bucketing of artifact paths) |
| `TASK` | the user's original requirement text (injected when provided via `start --task`; carried only by the first stage's `task_description` — downstream stages consume only upstream artifacts and never reference `{TASK}`) |

**Declared variables**: business names (`topic`, `feature`, `constraints`, etc.) are declared in scope `vars` (mapping: empty value = required, literal value = inline default) and provided via `start --var` or fall back to the inline default; fixed literal values go into scope `constants`. At `start` the Engine composes **one** session variable set in the order constants → vars inline defaults → `--var` → reserved variables, shared alike by artifact templates, `task_description`, and completion_template's `{vars.*}`.

## Compile error reference table

| Error message (excerpt) | Fix |
|---|---|
| `工件模板 '…' 引用未知变量 '{x}'` | Undeclared variable: add it to scope `vars` (or it is a constant/reserved variable) |
| `… vars 须为 mapping（名 → 内联默认值，空值 = 必填）；列表形式已移除` | Switch to mapping form: leave required names with an empty value, give optional names a literal default |
| `… defaults 已移除：…` | Optional variables use vars inline defaults (`name: "default value"`); constants go into constants |
| `… derived_vars 已移除：…` | Fixed literal values use constants; composite values are referenced directly by templates (e.g. `"{product_line}-{SESSION_ID}"`) |
| `变量名冲突：'…' 同时声明于 vars 与 constants` | The two declaration categories are mutually exclusive by name; pick one |
| `…含 dir 字段（已移除）` | Write full paths directly in inputs/outputs, e.g. `dmaf-output/{SESSION_ID}/reports/x.md` |
| `…的硬输入 '…' 没有任何 stage 的 outputs 能提供（外部既有文件请改软输入 required: false 引用）` | Add a producing stage and place it in the sequence, or confirm it is an external pre-existing file → switch to a soft input (`required: false`) |
| `stage 含 depends_on 字段（已移除）` | Write the execution order into the scope's include_stages sequence; data dependencies are expressed by hard inputs (producers derived automatically) |
| `…的硬输入 '…' 的生产者 'Y' 不在成员序列中` | Bring Y into the members ahead of the consumer, or switch to a soft input (`required: false`) as an on-demand reference |
| `…的生产者 'Y' 均排在 stage 'X' 之后` | Adjust the include_stages order: producers first (order is the execution order) |
| `…的软输入 '…' 的生产者 'Y' 排在其后…（死接线）` | Under the linear execution order this input would be absent forever in this session: adjust the order, or have it provided by a non-member / externally |
| `…由本 stage 自己生产（疑似定义错误）` | Input and output share the same name; change it |
| `stage id 'X' 与文件名 stem 'Y' 不一致` | Fix the id or the file name |
| `phase 'X' 与所在目录 'Y' 不一致` | Move to the matching directory or fix phase |
| `stage 'X' 声明了 scopes` | Delete it; membership goes into the scope file's include_stages |
| `completion_template 占位符 '…' 引用未声明的变量/stage '…'` | Add the variable to scope vars/constants, or the stage to include_stages |
| `scope 'X' include_stages 条目 'Y' 在 stage 库中不存在` | Fix the slug or create the stage file first |
| `stage id 重复` | Change the id (the sequence has no cycles: execution order = the written order of include_stages) |
| `collaboration=single 时 agents 必须恰好 1 个` | Trim to 1 or change collaboration |

## Appendix: the delegation payload is pre-built by the Engine

Every action of a dispatch instruction carries a ready-made `delegation` JSON, which the Orchestrator forwards verbatim. The path fields share the stage definition's names (`inputs`/`outputs`; the values are the full paths obtained by resolving the templates), and the remaining protocol fields are `role`/`task_description`/`revision_feedback` (only in revision loops) — what the definition declares is exactly what the agent receives, with no second vocabulary to memorize.
