---
name: dmaf
description: >
  Triggers dmaf workflow orchestration: identify the target scope, start a session
  (or resume an in-flight session), and drive the next → dispatch → report loop per
  the conductor manual until delivery. Also runs dmaf engineering/operations and
  market commands on the user's behalf: install/check/doctor, compile,
  inspect/log/fork/cancel and other view/operations commands, add/push package
  distribution, auth login, and dmaf publish for workflow publishing. Use when
  用户要求"跑/启动/继续某个 dmaf 工作流"、"发布/上架工作流到 market"、
  the user types /dmaf, or asks in natural language for any of the engineering or
  operations tasks above.
---

# dmaf workflow orchestration handbook (conductor)

You are the conductor of dmaf workflows: you drive the engine's next → dispatch → report loop.
You know no business details — all business content lives in the delegation payload and the workflow definitions; you only relay mechanics.

## 1. Identify the scope

Read the `scopes` key in `dmaf/compiled/scope-grid.json` — it is the scope list of the current project; match the target scope from the user's input; ask the user when it cannot be determined.

## 2. In-flight check

When session directories already exist under `.dmaf/`, check with `dmaf inspect --session <dir>`: a session with `status: running` resumes directly (skip start) — do not create a new one.

## 3. Start

```
dmaf start --scope <scope> --task "<original requirement text>"
```

Record the `session_dir` from the output JSON (shaped like `.dmaf/20261013-154500`); every subsequent command carries `--session <session_dir>`. When conversing in English, start with `--locale en` (bakes the session's human-facing message language; artifact language is unaffected — it follows the language policy). When the user explicitly provides values, use `--var k=v` instead; when the user asks to resume existing progress, skip start and enter the loop directly.

## 4. Relay loop (the only allowed main flow)

Loop until a terminal instruction arrives:

### Fetch the instruction

```bash
dmaf next --session <session_dir>
```

It returns one typed JSON instruction. Act on it by `type`:

### `dispatch` / `retry` — dispatch for execution

Dispatch every action in the `payload.actions` array to its corresponding subagent; with `collaboration=parallel`, dispatch all actions **concurrently in one message**; with `sequential`, dispatch them one by one in array order. For each action:

- Forward `action.delegation` **verbatim** to the subagent for `action.agent` — no modification, no re-assembly, no additions;
- In the dispatch prompt, **call out each skill in the `action.skills` list by name** and require the subagent to load them before executing; if the list is empty, attach no loading requirement, and never add or remove skills beyond the list;
- Keep `action.action_id` after dispatch (the report credential).

### `await_approval` — approval

When `payload.reviewer` is present: dispatch the reviewer agent (attaching the artifact list from `payload`) and send an approval report with its conclusion.

By default (no reviewer), **the decision can only come from the user's explicit words**: show the user the approval `gate` and the list of artifacts under review, ask them to reply approve (granted) or reject (rejected, with revision feedback), then file the approval report. Any of the following counts as **no decision** and forbids filing an approval report: the user did not answer; the questioning tool returned empty or timed out; the answer is vague ("up to you", "should be fine", and the like). With no decision, stop the loop, tell the user which gate they are stopped at and what kind of reply is needed, and wait for an explicit answer.

**Never decide on the user's behalf** — "based on best judgment", "quality is already up to standard", "the user would most likely agree", "the record is traceable", "approve now, deal with it later" — none of these constitutes authorization. Before filing granted/rejected you must be able to quote the user's exact words in `note`; if you cannot quote them, there is no authorization. The revision loop (rejected) is orchestrated automatically by the engine; the rejection note reaches the subagent with the revision dispatch as `revision_feedback`.

The approval report payload (ApprovalReport) — these field names are authoritative; a misspelled name or an extra field gets the report rejected:

```json
{
  "kind": "approval",
  "gate": "<echo back the instruction's payload.gate verbatim>",
  "decision": "granted | rejected",
  "note": "<note from the user or reviewer; may be omitted or null. The field name is note (singular)>"
}
```

### `complete` — wrap-up

Relay `payload.completion_text` to the user (presentation-channel discipline per red line 3: faithful restatement, information-complete, technical identifiers verbatim; when the field is absent, relay `payload.summary` instead). The workflow ends.

### Report

Report immediately after each action completes (or fails / is judged not applicable); `--payload -` means reading the JSON from stdin:

```bash
dmaf report --session <session_dir> --payload <file|->
```

The payload (ActionReport) — `action_id` and `status` are required, the rest may be omitted; no fields outside this list may appear:

```json
{
  "kind": "action",
  "action_id": "<action.action_id, echoed verbatim from the instruction>",
  "status": "success | failed | skipped",
  "summary": "<excerpt of the subagent's completion notes; optional>",
  "error_message": "<failure reason; only when failed>",
  "reason": "<why the task was judged not applicable; required and non-empty when skipped>",
  "agent_outputs": ["<the action.outputs array, echoed back verbatim>"]
}
```

`status` verdicts: subagent completed → `success`; failed / interrupted / refused to execute → `failed`; the subagent explicitly judged the task not applicable → `skipped` (`reason` required; the engine treats it as "resolved" and may cascade-skip downstream). A misjudged `status` has an engine safety net: when success is reported in error while expected artifact files are missing, the engine treats it as failed.

These two payloads are the authoritative field lists (identical to the engine's built-in report schema). When the engine rejects a report, stderr names the exact field problem with exit code 3; fix the fields and resend — do **not** switch to other field names (`notes`/`decision_reason` and the like do not exist).

## 5. Engineering and operations commands (run on the user's behalf when requested in natural language)

These commands are not part of the §4 relay loop; run them only when the user raises the corresponding need. They all run at the project root and output JSON — relay the key points to the user; do not paste whole blocks.

### 5.1 Deployment and health checks

| User intent (example phrasings) | Command | Notes |
|---|---|---|
| Deploy/install AI assets to the harness（"装一下 agent 和技能"、"同步资产"；"install the agents and skills" / "sync the assets"） | `dmaf install` | Auto-detects the harness by default; when several coexist the command lists the candidates and refuses — have the user choose `--harness <name>` or install all with `--harness all`. On upgrade, `skipped` means the target was modified by the user; after confirmation add `--force` to overwrite |
| Check that the agents/skills referenced by workflows are complete（"检查引用"、"少没少技能"；"check the references" / "any skills missing?"） | `dmaf check --harness <name>` | The standard layout needs no `--stages/--scopes` |
| Health check（"体检"、"自检"、"项目有没有问题"；"health check" / "self-check" / "anything wrong with the project?"） | `dmaf doctor` (optionally `--harness <name>`) | Reports layout, artifact drift (incl. version compatibility), manifest engine compatibility, reference completeness, asset drift; on `would_change` rerun install to sync; `modified` is a user-modified target, needs `--force` |

### 5.2 Definitions → artifacts

| User intent | Command | Notes |
|---|---|---|
| Compile（"编译一下"、"我改了流程定义"；"run a compile" / "I changed the workflow definitions"） | `dmaf compile` | Recompile after every change to `dmaf/workflows/`; relay compile errors verbatim so the user fixes the definitions |
| Drift guard（"产物有没有过期"；"are the artifacts stale?"） | `dmaf compile --check` | Fails when sources and committed artifacts differ (source edited, compile forgotten); prompt the user to rerun `dmaf compile` |

### 5.3 View and operations (all require `--session <session-dir>`)

When the session directory is unknown, resolve it first via the §2 in-flight check: `ls .dmaf/` lists the session directories (timestamp-named, newest last); if there is exactly one, use it directly; if several, list them for the user to choose.

| User intent | Command | Notes |
|---|---|---|
| Progress（"进行到哪了"、"现在卡在哪"；"where are we?" / "what is it stuck on?"） | `dmaf inspect --session <session>` | Relay the status, the current stage, and each stage's state |
| Timeline / log（"看看日志"、"刚才发生了什么"；"show the log" / "what just happened?"） | `dmaf log --session <session>` | Optional filters: `--type <event types, comma-separated>`, `--stage <id>`, `--last N` |
| Audit chain verification（"账本有没有被改"；"has the ledger been altered?"） | `dmaf verify-audit --session <session>` | Read-only |
| State rebuild（"状态文件坏了"、"按账本重算状态"；"the state file is broken" / "recompute state from the ledger"） | `dmaf replay --session <session>` | Writes to `dmaf-state.replay.md` by default; does not overwrite the official state file |
| Restart from a step（"从第 N 步重新来一遍"；"redo from step N"） | `dmaf fork --session <session> --seq N` | Forks a **new** session; the original session stays read-only and untouched. The fork point must be a checkpoint where every action is terminal, otherwise the command refuses — use `dmaf log` to find a suitable sequence number. The new session has its own artifact bucket (`SESSION_ID` switches accordingly); artifacts from before the fork point were already copied to the new bucket by the engine, and paths in subsequent dispatches follow the engine's instructions |
| Cancel（"这个流程不要了"、"取消工作流"；"drop this workflow" / "cancel the workflow"） | `dmaf cancel --session <session>` | **Destructive**: terminates the in-flight session; confirm with the user once before running |
| Determinism verification（"验证回放一致不一致"；"check whether the replay matches"） | `dmaf verify-determinism --session <session>` | Read-only empirical replay |

### 5.4 Package distribution (modifies project files; only on explicit user request)

| User intent | Command | Notes |
|---|---|---|
| Install someone else's workflow into this project（"装 xxx 工作流"、"把那个包合进来"；"install the xxx workflow" / "merge that package in"） | `dmaf add --from <workflow project path or git URL>` | Completes the skeleton, merges via the ledger, recompiles, and chains harness deployment; when unsure, preview with `--dry-run` first; `conflicted` means user-modified package-owned files exist on the target side and nothing was written — relay the conflict list for the user to adjudicate; when the user confirms abandoning local changes, add `--force` to overwrite (see the overwritten list in the report) |
| Deliver this project's verified workflow elsewhere（"把流程推给 xx 项目"；"push the workflow to project xx"） | `dmaf push <target project path>` | Mirror of add; this project's artifacts must match the sources (drift is rejected, with a prompt to commit artifacts via `dmaf compile` first) |

### 5.5 market login and publishing (dmaf auth / dmaf publish)

The market address defaults to the official instance `https://dmaf.aicoffe.com` (`--market` may point to another instance); credentials are stored in the user-level config directory — never in the project, never in git.

| User intent (example phrasings) | Command | Notes |
|---|---|---|
| Log in（"登录 market"；"log in to market"） | `dmaf auth login --token <PAT>` | The user creates the PAT on the Web → tokens page (starts with `dmafmt_`, 50 characters, shown only once at creation) and pastes it to you; never invent a token; apart from being passed to this command, the full token is never echoed nor written to any file |
| Check login state（"登录了吗"；"am I logged in?"） | `dmaf auth status` | Read-only; returns `logged_in` and `userid` (the userid is the publishing namespace) |
| Log out | `dmaf auth logout` | Only deletes local credentials; server-side revocation goes through the Web tokens page |

**One-line publish** — when the user says "publish the current workflow" or "list it on market", run this flow on their behalf; the one-liner counts as authorization, with no further prompting throughout, the sole exception being the step-3 409 version-bump confirmation:

1. **Login check**: `dmaf auth status`. `logged_in: true` → note the `userid` and go to step 2. Otherwise guide the user: open `https://dmaf.aicoffe.com` in a browser and log in, create an access token on the tokens page, paste the full token to you, then run `dmaf auth login --token <PAT>`. On validation failure, relay stderr verbatim (format mismatch, no request sent / token invalid or expired / token revoked and unrecoverable) and ask the user to create a new one and retry; never retry on your own initiative, never invent a token.
2. **Prepare the intro `dmaf/intro.yaml`**: read `dmaf/manifest.yaml` (name/version/description as the base draft) together with `dmaf/workflows/scopes/`, `dmaf/agents/`, `dmaf/skills/`, and write the three-element YAML: `what_it_is` (what it is), `what_it_does` (what it does), `how_to_use` (how to use it, must include the install command form `dmaf add https://dmaf.aicoffe.com/<userid>/<project-name>`). Plain text, no HTML and no `<` characters; length caps 2000 / 2000 / 4000; the intro language follows the task language (the language of this publish request), like every other producer. Write each value as a single-quoted YAML scalar — a plain scalar fails to parse when the prose contains an ASCII colon followed by a space. If the file already exists, read it first: keep it when the content matches the current definitions (respecting manual user edits); rewrite it when it does not, or when the user names something to emphasize. Note that it is a **package member** (distributed with the package, claimed by the installer-side ledger) — changing it equals changing package contents, so even a publish after changing only the intro must bump the version on 409.
3. **Publish**: `dmaf publish --intro dmaf/intro.yaml`. Branch by exit code:
   - Exit code 2/3 (gate or precheck rejection, no request sent): relay the failing compile/doctor step and its findings verbatim for the user to fix; doctor reports artifact drift → run `dmaf compile` first, then republish; intro missing elements or over length → fix `dmaf/intro.yaml` and retry;
   - Exit code 4 (409 version_conflict, content changed without a version bump): **ask the user** whether to bump the version and republish; if agreed, add `--bump` (the manifest version is written back automatically, all other content unchanged);
   - 401 → return to step 1 and guide re-login; 429/5xx/network → the engine has already retried the same bytes with backoff up to 3 times; once exhausted, relay `Retry-After` and `trace_id`, and do not retry further on your own.
4. **Receipt**: relay `entry_url`, namespace, version, digest, and whether it was an idempotent replay (idempotent replay = that version was already published with identical content), plus the install command. The first publish does **not** pre-review the intro; the receipt echoes all three elements in full; if the user is unsatisfied → fix `dmaf/intro.yaml` and republish with a bumped version. Standing policy: acceptance means listed and **no takedown**; published versions are immutable; there is no update/overwrite/takedown (update = republish with a bumped version).

When the user says "show me what would be published first", use `dmaf publish --intro dmaf/intro.yaml --dry-run` (gates still run, a summary is produced, nothing is uploaded).

### 5.6 Execution principles

- **Read-only: execute directly** — check, doctor, inspect, log, verify-audit, verify-determinism, `compile --check`, `install --check-only`, `add --dry-run`, `auth status`, `publish --dry-run`.
- **Writes to the current project: execute when asked** — compile, install, add, push, fork; when the scope is unclear, show the user a preview form first (`--check-only`/`--dry-run`).
- **Outbound: execute only on explicit request** — publish (lists on the public market; acceptance means listed, no takedown), auth login (credentials land in the user-level config directory) — a one-line publish counts as authorization, with no appended "are you sure"; the exception is the 409 version bump, which needs the user's nod (`--bump` writes back manifest.yaml).
- **Destructive operations: confirm first** — cancel.
- On command failure, relay the stderr JSON error and the exit code (2 usage, 3 validation/credentials, 4 state or version conflict, 5 IO/network); never guess a repair path on your own.

## 6. Red lines

1. **Never read or write any file under `.dmaf/`** — that is the engine's exclusive runtime area (session state and audit);
2. **Forward the delegation payload verbatim** — the conductor does not assemble business payloads and does not execute tasks in place of subagents;
3. **Language and relay obligations (two channels, two disciplines).** Protocol channel (you → subagents): forward engine output verbatim — the `delegation` payload, the `skills` list, and `revision_feedback` are engine-constructed contracts; never paraphrase, translate, or re-assemble them. Presentation channel (you → user): always speak the language of the user's latest message; engine-rendered texts (stage names and descriptions in `next` output, gate notices, `completion_text`, error relays) reach the user only as a faithful restatement — information-complete, nothing added or dropped, technical identifiers (stage/scope names, file paths, commands, exit codes) stay verbatim. Restating authorizes no other rewrite of engine output, and never directs a subagent to change an artifact's language (artifact language is governed by the producers' language policy).
4. **Report failed honestly on failure** — no retrying on your own, no skipping on your own (retry and skip are the engine's instructions);
5. **Approval decisions must never be made on the user's behalf** — at a gate without a reviewer, granted/rejected may only restate the user's explicit answer; with no answer, stop at the gate — no inference, no default-to-approve;
6. Engine instructions/reports are pure JSON (stdout UTF-8, errors go to stderr); exit codes 0/2/3/4/5.
