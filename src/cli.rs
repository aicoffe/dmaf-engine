//! dmaf_engine 命令行入口。
//!
//! JSON 输出走 stdout（UTF-8），诊断与错误走 stderr。
//! 退出码：0 成功；2 用法错误；3 schema/定义校验失败（状态不变）；
//! 4 状态冲突；5 文件系统错误。

use std::path::{Path, PathBuf};

use clap::{error::ErrorKind, Parser, Subcommand};
use crate::i18n::t_static;
use serde_json::{json, Value};

use crate::audit::{verify_chain, AuditTrail};
use crate::forking;
use crate::paths;
use crate::registry::{harness_root, load_registry, validate_refs};
use crate::reports::{self, ApplyError, EngineContext};
use crate::router;
use crate::schema_check;
use crate::state::{self, WorkflowState};
use crate::workflow::{check_templates, load_workflow, resolve_vars, Workflow, WorkflowError};
use crate::{py_list, CmdResult, Fail, Outcome};

pub const EXIT_OK: i32 = 0;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_SCHEMA: i32 = 3;
pub const EXIT_CONFLICT: i32 = 4;
pub const EXIT_IO: i32 = 5;

const DEFAULT_WORKFLOW_DIR: &str = "dmaf/workflows/inline";
const DEFAULT_ARTIFACTS: &str = "dmaf/compiled";
const DEFAULT_OUTPUT_ROOT: &str = ".dmaf";

#[derive(Parser, Debug)]
#[command(
    name = "dmaf",
    version,
    about = t_static("help.about"),
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = t_static("help.cmd.start"))]
    Start {
        #[arg(long, value_parser = ["zh", "en"], help = t_static("help.start.locale"))]
        locale: Option<String>,
        #[arg(long, conflicts_with_all = ["scope"])]
        workflow: Option<String>,
        #[arg(long, conflicts_with_all = ["workflow"])]
        scope: Option<String>,
        #[arg(long = "var")]
        vars: Vec<String>,
        #[arg(long, help = t_static("help.start.task"))]
        task: Option<String>,
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        output_root: String,
        #[arg(long, default_value = DEFAULT_WORKFLOW_DIR)]
        workflow_dir: String,
        #[arg(long, default_value = DEFAULT_ARTIFACTS, help = t_static("help.start.artifacts"))]
        artifacts: String,
        #[arg(long, help = t_static("help.no_hooks"))]
        no_hooks: bool,
    },
    #[command(about = t_static("help.cmd.next"))]
    Next {
        #[arg(long)]
        session: String,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long, default_value = DEFAULT_WORKFLOW_DIR)]
        workflow_dir: String,
        #[arg(long, default_value = DEFAULT_ARTIFACTS)]
        artifacts: String,
        #[arg(long, help = t_static("help.no_hooks"))]
        no_hooks: bool,
    },
    #[command(about = t_static("help.cmd.report"))]
    Report {
        #[arg(long)]
        session: String,
        #[arg(long, help = t_static("help.report.payload"))]
        payload: String,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long, default_value = DEFAULT_WORKFLOW_DIR)]
        workflow_dir: String,
        #[arg(long, default_value = DEFAULT_ARTIFACTS)]
        artifacts: String,
        #[arg(long, help = t_static("help.no_hooks"))]
        no_hooks: bool,
    },
    #[command(about = t_static("help.cmd.validate"))]
    Validate {
        #[arg(long)]
        workflow: String,
        #[arg(long, default_value = DEFAULT_WORKFLOW_DIR)]
        workflow_dir: String,
    },
    #[command(about = t_static("help.cmd.inspect"))]
    Inspect {
        #[arg(long)]
        session: String,
    },
    #[command(about = t_static("help.cmd.verify_audit"))]
    VerifyAudit {
        #[arg(long)]
        session: String,
    },
    #[command(about = t_static("help.cmd.cancel"))]
    Cancel {
        #[arg(long)]
        session: String,
        #[arg(long, help = t_static("help.no_hooks"))]
        no_hooks: bool,
    },
    #[command(about = t_static("help.cmd.log"))]
    Log {
        #[arg(long)]
        session: String,
        #[arg(long = "type", help = t_static("help.log.type"))]
        event_type: Option<String>,
        #[arg(long, help = t_static("help.log.stage"))]
        stage: Option<String>,
        #[arg(long, help = t_static("help.log.limit"))]
        limit: Option<i64>,
    },
    #[command(about = t_static("help.cmd.replay"))]
    Replay {
        #[arg(long)]
        session: String,
        #[arg(long, help = t_static("help.replay.overwrite"))]
        overwrite: bool,
    },
    #[command(about = t_static("help.cmd.fork"))]
    Fork {
        #[arg(long)]
        session: String,
        #[arg(long, help = t_static("help.fork.seq"))]
        seq: i64,
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        output_root: String,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long, default_value = DEFAULT_WORKFLOW_DIR)]
        workflow_dir: String,
        #[arg(long, default_value = DEFAULT_ARTIFACTS)]
        artifacts: String,
        #[arg(long, help = t_static("help.no_hooks"))]
        no_hooks: bool,
    },
    #[command(about = t_static("help.cmd.verify_determinism"))]
    VerifyDeterminism {
        #[arg(long)]
        session: String,
        #[arg(long, help = t_static("help.verify_determinism.workflow"))]
        workflow: Option<String>,
        #[arg(long, default_value = DEFAULT_ARTIFACTS)]
        artifacts: String,
        #[arg(long, help = t_static("help.verify_determinism.print_pairs"))]
        print_pairs: bool,
    },
    #[command(about = t_static("help.cmd.new"))]
    New {
        #[arg(long, default_value = "minimal", help = t_static("help.new.template"))]
        template: String,
        #[arg(long, help = t_static("help.new.dir"))]
        dir: Option<String>,
        #[arg(long, help = t_static("help.new.force"))]
        force: bool,
    },
    #[command(about = t_static("help.cmd.doctor"))]
    Doctor {
        #[arg(long, default_value = DEFAULT_WORKFLOW_DIR)]
        workflow_dir: String,
        #[arg(long, default_value = DEFAULT_ARTIFACTS)]
        artifacts: String,
        #[arg(long, help = t_static("help.doctor.harness"))]
        harness: Option<String>,
    },
    #[command(about = t_static("help.cmd.compile"))]
    Compile {
        #[arg(default_value = "dmaf/workflows/stages")]
        stages: String,
        #[arg(default_value = "dmaf/workflows/scopes")]
        scopes: String,
        #[arg(long, default_value = "dmaf/compiled")]
        out: String,
        #[arg(long, help = t_static("help.compile.check"))]
        check: bool,
        #[arg(long, help = t_static("help.compile.harness"))]
        harness: Option<String>,
    },
    #[command(about = t_static("help.cmd.install"))]
    Install {
        #[arg(long, help = t_static("help.install.harness"))]
        harness: Option<String>,
        #[arg(long, help = t_static("help.install.prune"))]
        prune: bool,
        #[arg(long = "check-only", help = t_static("help.install.check_only"))]
        check_only: bool,
        #[arg(long, help = t_static("help.install.force"))]
        force: bool,
    },
    #[command(about = t_static("help.cmd.add"))]
    Add {
        #[arg(value_name = t_static("help.vn.package"), conflicts_with = "from", help = t_static("help.add.package"))]
        package: Option<String>,
        #[arg(long = "from", value_name = t_static("help.vn.from"), help = t_static("help.add.from"))]
        from: Option<String>,
        #[arg(long, help = t_static("help.add.tag"))]
        tag: Option<String>,
        #[arg(long = "no-install", help = t_static("help.no_install"))]
        no_install: bool,
        #[arg(long = "dry-run", help = t_static("help.dry_run"))]
        dry_run: bool,
        #[arg(long, help = t_static("help.force_overwrite"))]
        force: bool,
    },
    #[command(about = t_static("help.cmd.auth"))]
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    #[command(about = t_static("help.cmd.publish"))]
    Publish {
        #[arg(value_name = t_static("help.vn.dmaf_dir"), default_value = "./dmaf", help = t_static("help.publish.dmaf_dir"))]
        dmaf_dir: String,
        #[arg(long, help = t_static("help.publish.project"))]
        project: Option<String>,
        #[arg(long, conflicts_with = "bump", help = t_static("help.publish.version"))]
        version: Option<String>,
        #[arg(long, help = t_static("help.publish.bump"))]
        bump: bool,
        #[arg(long, value_name = t_static("help.vn.intro"), help = t_static("help.publish.intro"))]
        intro: Option<String>,
        #[arg(long, help = t_static("help.publish.github_url"))]
        github_url: Option<String>,
        #[arg(long, help = t_static("help.market_url_long"))]
        market: Option<String>,
        #[arg(long = "dry-run", help = t_static("help.publish.dry_run"))]
        dry_run: bool,
    },
    #[command(about = t_static("help.cmd.push"))]
    Push {
        #[arg(value_name = t_static("help.vn.target"), help = t_static("help.push.target"))]
        target: String,
        #[arg(long = "no-install", help = t_static("help.push.no_install"))]
        no_install: bool,
        #[arg(long = "dry-run", help = t_static("help.dry_run"))]
        dry_run: bool,
        #[arg(long, help = t_static("help.force_overwrite"))]
        force: bool,
    },
    #[command(about = t_static("help.cmd.check"))]
    Check {
        #[arg(long, help = t_static("help.check.harness"))]
        harness: String,
        #[arg(long, conflicts_with_all = ["session", "stages", "scopes"])]
        workflow: Option<String>,
        #[arg(long, default_value = DEFAULT_WORKFLOW_DIR)]
        workflow_dir: String,
        #[arg(long, conflicts_with_all = ["workflow", "stages", "scopes"])]
        session: Option<String>,
        #[arg(long, requires = "scopes", conflicts_with_all = ["workflow", "session"])]
        stages: Option<String>,
        #[arg(long, requires = "stages", conflicts_with_all = ["workflow", "session"])]
        scopes: Option<String>,
        #[arg(long, default_value = DEFAULT_ARTIFACTS)]
        artifacts: String,
    },
}

#[derive(Subcommand, Debug)]
enum AuthAction {
    #[command(about = t_static("help.cmd.auth_login"))]
    Login {
        #[arg(long, help = t_static("help.auth_login.token"))]
        token: String,
        #[arg(long, help = t_static("help.market_url_long"))]
        market: Option<String>,
    },
    #[command(about = t_static("help.cmd.auth_logout"))]
    Logout {
        #[arg(long, help = t_static("help.market_address"))]
        market: Option<String>,
    },
    #[command(about = t_static("help.cmd.auth_status"))]
    Status {
        #[arg(long, help = t_static("help.market_address"))]
        market: Option<String>,
    },
}

// ---------- 输出助手 ----------

pub fn fail_outcome(code: i32, message: &str, problems: &[String]) -> Outcome {
    let mut body = serde_json::Map::new();
    body.insert("ok".into(), json!(false));
    body.insert("error".into(), json!(message));
    if !problems.is_empty() {
        body.insert("problems".into(), json!(problems));
    }
    let text = serde_json::to_string_pretty(&Value::Object(body)).expect("json 序列化失败");
    Outcome { code, stdout: String::new(), stderr: format!("{text}\n") }
}

fn ok_json(v: &Value) -> Outcome {
    let text = serde_json::to_string_pretty(v).expect("json 序列化失败");
    Outcome { code: EXIT_OK, stdout: format!("{text}\n"), stderr: String::new() }
}

fn ok_text(text: String) -> Outcome {
    Outcome { code: EXIT_OK, stdout: format!("{text}\n"), stderr: String::new() }
}

fn pretty(v: &Value) -> String {
    let text = serde_json::to_string_pretty(v).expect("json 序列化失败");
    format!("{text}\n")
}

// ---------- 公共装载 ----------

/// 返回 (会话目录, 状态文件, 审计目录)。会话目录即 .dmaf 下的引擎独占目录，
/// 状态与审计直接放在会话目录根；session 可传目录或状态文件路径。
fn session_paths(session: &str) -> (PathBuf, PathBuf, PathBuf) {
    let mut p = PathBuf::from(session);
    if p.is_file() {
        p = p.parent().map(|x| x.to_path_buf()).unwrap_or(p);
    }
    let state = p.join("dmaf-state.md");
    let audit = p.join("audit");
    (p, state, audit)
}

fn load_state(session: &str) -> Result<(PathBuf, PathBuf, PathBuf, WorkflowState), Fail> {
    let (session_dir, state_path, audit_dir) = session_paths(session);
    if !state_path.is_file() {
        return Err(Fail::Conflict(crate::i18n::msg(
            "cli.invalid_session_missing_state",
            &[&paths::to_posix(&state_path)],
        )));
    }
    let state = state::load(&state_path).map_err(|e| Fail::Conflict(e.0))?;
    Ok((session_dir, state_path, audit_dir, state))
}

fn resolve_workflow_path(spec: &str, workflow_dir: &str) -> PathBuf {
    if spec.ends_with(".yaml") || spec.ends_with(".yml") || spec.contains('/') || spec.contains('\\') {
        PathBuf::from(spec)
    } else {
        Path::new(workflow_dir).join(format!("{spec}.yaml"))
    }
}

/// 会话的工作流定义解析：显式 override（内联）→ scope 会话（编译产物）→ 内联默认目录。
fn load_workflow_for(
    state: &WorkflowState,
    override_spec: Option<&str>,
    workflow_dir: &str,
    artifacts_dir: &str,
) -> Result<Workflow, Fail> {
    if let Some(spec) = override_spec {
        return load_workflow(&resolve_workflow_path(spec, workflow_dir)).map_err(
            |e: WorkflowError| Fail::Schema {
                message: crate::i18n::t("cli.workflow_invalid"),
                problems: e.problems,
            },
        );
    }
    if !state.scope.is_empty() {
        return load_workflow_from_artifacts(Path::new(artifacts_dir), &state.scope);
    }
    load_workflow(&resolve_workflow_path(&state.workflow_id, workflow_dir)).map_err(
        |e: WorkflowError| Fail::Schema {
            message: crate::i18n::t("cli.workflow_invalid"),
            problems: e.problems,
        },
    )
}

fn load_json_file(path: &Path) -> Result<Value, Fail> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        Fail::Schema {
            message: crate::i18n::msg("cli.read_artifact_failed", &[&paths::to_posix(path)]),
            problems: vec![e.to_string()],
        }
    })?;
    serde_json::from_str(&text)
        .map_err(|e| Fail::Schema {
            message: crate::i18n::msg("cli.artifact_not_json", &[&paths::to_posix(path)]),
            problems: vec![e.to_string()],
        })
}

pub fn load_workflow_from_artifacts(artifacts_dir: &Path, scope: &str) -> Result<Workflow, Fail> {
    let graph = load_json_file(&artifacts_dir.join(crate::compile::graph_file_name()))?;
    let grid = load_json_file(&artifacts_dir.join(crate::compile::grid_file_name()))?;
    crate::compile::workflow_from_artifacts(&graph, &grid, scope).map_err(|e: WorkflowError| {
        Fail::Schema {
            message: crate::i18n::t("cli.build_workflow_from_artifacts_failed"),
            problems: e.problems,
        }
    })
}

fn io_fail(e: std::io::Error) -> Fail {
    Fail::Io(e.to_string())
}

// ---------- start ----------

fn cmd_start(
    workflow_spec: Option<&str>,
    scope: Option<&str>,
    vars: &[String],
    task: Option<&str>,
    output_root: &str,
    workflow_dir: &str,
    artifacts_dir: &str,
    locale: &str,
) -> CmdResult {
    let (workflow, scope_name) = if let Some(s) = scope {
        let wf = load_workflow_from_artifacts(Path::new(artifacts_dir), s)?;
        (wf, s.to_string())
    } else {
        let spec = workflow_spec.expect("clap 保证 workflow/scope 二选一");
        let wf_path = resolve_workflow_path(spec, workflow_dir);
        let wf = load_workflow(&wf_path).map_err(|e: WorkflowError| Fail::Schema {
            message: crate::i18n::t("cli.workflow_invalid"),
            problems: e.problems,
        })?;
        (wf, String::new())
    };

    let mut provided = std::collections::BTreeMap::new();
    for pair in vars {
        let Some((k, v)) = pair.split_once('=') else {
            return Err(Fail::Usage(crate::i18n::msg("cli.var_kv_form", &[&pair])));
        };
        provided.insert(k.to_string(), v.to_string());
    }

    let output_root = PathBuf::from(output_root);
    let session_dir = paths::session_dir_for(&output_root, &paths::new_session_id(None));
    let session_id = session_dir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let vars_ =
        resolve_vars(&workflow, &provided, &session_id, task.unwrap_or_default()).map_err(|e| Fail::Schema {
        message: crate::i18n::t("cli.vars_validation_failed"),
        problems: e.problems,
    })?;
    let problems = check_templates(&workflow, &vars_);
    if !problems.is_empty() {
        return Err(Fail::Schema {
            message: crate::i18n::t("cli.template_vars_unclosed"),
            problems,
        });
    }

    // init 点位（hooks.yaml）：建会话目录之前调用，结果不影响启动
    crate::hooks::run_init(&json!({
        "kind": "init",
        "workflow_id": workflow.id,
        "scope": if scope_name.is_empty() { Value::Null } else { json!(scope_name) },
        "session_dir": paths::to_posix(&session_dir),
        "session_id": session_id,
        "vars": vars_,
        "stages": workflow.stages.iter().map(|s| json!(s.id)).collect::<Vec<_>>(),
    }));

    // 产物目录（stage 定义的 dir，项目根相对）由 agent 写文件时自建；引擎只建会话独占区
    std::fs::create_dir_all(session_dir.join("audit")).map_err(io_fail)?;

    let mut state = WorkflowState {
        locale: locale.to_string(),
        workflow_id: workflow.id.clone(),
        session_dir: paths::to_posix(&session_dir),
        scope: scope_name.clone(),
        created_at: paths::format_ts(&paths::now_cst()),
        vars: vars_.clone(),
        stages: workflow
            .stages
            .iter()
            .map(|s| state::StageState::new(&s.id, s.retry.max_attempts))
            .collect(),
        ..Default::default()
    };
    state.status = "running".into();
    let audit = AuditTrail::new(&session_dir.join("audit")).map_err(io_fail)?;
    let before = state::canonical_hash(&state);
    let stages_payload: Vec<Value> = workflow
        .stages
        .iter()
        .map(|s| json!({"id": s.id, "max_attempts": s.retry.max_attempts}))
        .collect();
    let shard = audit
        .append(
            &workflow.id,
            "workflow.started",
            None,
            None,
            json!({
                "session_dir": state.session_dir,
                "vars": vars_,
                "stages": stages_payload,
            }),
            &before,
            &state::canonical_hash(&state),
            &paths::format_ts(&paths::now_cst()),
        )
        .expect("内置事件类型必合法");
    state.seq = shard["seq"].as_u64().unwrap_or(0);
    state::save(
        &mut state,
        &session_dir.join("dmaf-state.md"),
        &paths::format_ts(&paths::now_cst()),
    )
    .map_err(io_fail)?;

    Ok(ok_json(&json!({
        "workflow_id": workflow.id,
        "session_dir": state.session_dir,
        "state_path": paths::to_posix(&session_dir.join("dmaf-state.md")),
    })))
}

// ---------- next ----------

fn cmd_next(
    session: &str,
    workflow_override: Option<&str>,
    workflow_dir: &str,
    artifacts_dir: &str,
) -> CmdResult {
    let (_session_dir, state_path, audit_dir, mut state) = load_state(session)?;
    let workflow = load_workflow_for(&state, workflow_override, workflow_dir, artifacts_dir)?;
    let (instruction, effects) =
        router::compute_next(&workflow, &state).map_err(Fail::Conflict)?;

    if let Err(e) = schema_check::validate_instruction(&instruction) {
        return Err(Fail::Schema {
            message: crate::i18n::msg("cli.instruction_schema_violation", &[&e.to_string()]),
            problems: vec![],
        });
    }

    if !effects.is_empty() {
        let audit = AuditTrail::new(&audit_dir).map_err(io_fail)?;
        let now = paths::format_ts(&paths::now_cst());
        let session_dir_posix = state.session_dir.clone();
        let mut prev_hash = state::canonical_hash(&state);
        for effect in &effects {
            let wf_stage = workflow
                .stage(&effect.stage_id)
                .expect("effect 的 stage 必在定义内");
            let from_status = state.stage(&effect.stage_id).unwrap().status.clone();
            if let Some(to) = &effect.transition_to {
                state::transition(state.stage_mut(&effect.stage_id).unwrap(), to)
                    .expect("effect 转移必合法");
            }
            let mut actions_audit: Vec<Value> = Vec::new();
            {
                let st = state.stage_mut(&effect.stage_id).unwrap();
                if !effect.new_actions.is_empty() {
                    let max_attempt = effect
                        .new_actions
                        .iter()
                        .map(|a| a["attempt"].as_u64().unwrap_or(1))
                        .max()
                        .unwrap_or(1);
                    st.attempt = st.attempt.max(max_attempt);
                    for a in &effect.new_actions {
                        st.actions.push(state::ActionRecord {
                            action_id: a["action_id"].as_str().unwrap_or_default().into(),
                            agent: a["agent"].as_str().unwrap_or_default().into(),
                            attempt: a["attempt"].as_u64().unwrap_or(1),
                            status: "dispatched".into(),
                        });
                    }
                }
                for a in &effect.new_actions {
                    let mut obj = a.as_object().cloned().unwrap_or_default();
                    obj.remove("skill_brief");
                    actions_audit.push(Value::Object(obj));
                }
            }
            let st = state.stage(&effect.stage_id).unwrap();
            let (to_status, attempt, agents) =
                (st.status.clone(), st.attempt, effect.new_actions.iter().map(|a| a["agent"].clone()).collect::<Vec<_>>());
            let after_hash = state::canonical_hash(&state);
            let shard = audit
                .append(
                    &workflow.id,
                    "action.dispatched",
                    Some(&effect.stage_id),
                    None,
                    json!({
                        "from": from_status,
                        "to": to_status,
                        "attempt": attempt,
                        "collaboration": wf_stage.collaboration,
                        "agents": agents,
                        "actions": actions_audit,
                    }),
                    &prev_hash,
                    &after_hash,
                    &now,
                )
                .expect("内置事件类型必合法");
            prev_hash = after_hash;
            state.seq = shard["seq"].as_u64().unwrap_or(0);
            crate::hooks::notify(&shard, &session_dir_posix);
        }
        state::save(&mut state, &state_path, &now).map_err(io_fail)?;
    }
    Ok(ok_json(&instruction))
}

// ---------- report ----------

fn cmd_report(
    session: &str,
    payload_spec: &str,
    workflow_override: Option<&str>,
    workflow_dir: &str,
    artifacts_dir: &str,
) -> CmdResult {
    let raw = if payload_spec == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| Fail::Io(crate::i18n::msg("cli.read_payload_failed", &[&e.to_string()])))?;
        buf
    } else {
        std::fs::read_to_string(payload_spec)
            .map_err(|e| Fail::Io(crate::i18n::msg("cli.read_payload_failed", &[&e.to_string()])))?
    };
    let payload: Value = serde_json::from_str(&raw).map_err(|e| {
        Fail::Schema { message: crate::i18n::msg("cli.payload_not_json", &[&e.to_string()]), problems: vec![] }
    })?;

    if let Err(reports::ReportRejected(problems)) = reports::validate_payload_schema(&payload) {
        return Err(Fail::Schema {
            message: crate::i18n::t("cli.report_rejected_schema"),
            problems,
        });
    }

    let (session_dir, state_path, audit_dir, mut state) = load_state(session)?;
    let workflow = load_workflow_for(&state, workflow_override, workflow_dir, artifacts_dir)?;

    let result = {
        let mut ctx = EngineContext {
            workflow: &workflow,
            state: &mut state,
            session_path: &session_dir,
            state_path: &state_path,
            audit: AuditTrail::new(&audit_dir).map_err(io_fail)?,
            now: paths::now_cst(),
        };
        let is_approval = payload.get("kind").and_then(|v| v.as_str()) == Some("approval");
        if is_approval {
            reports::apply_approval_report(&mut ctx, &payload)
        } else {
            reports::apply_action_report(&mut ctx, &payload)
        }
    };
    let result = match result {
        Ok(v) => v,
        Err(ApplyError::Rejected(problems)) => {
            return Err(Fail::Schema { message: crate::i18n::t("cli.report_rejected"), problems });
        }
        Err(ApplyError::Conflict(msg)) => return Err(Fail::Conflict(msg)),
    };

    state::save(&mut state, &state_path, &paths::format_ts(&paths::now_cst()))
        .map_err(io_fail)?;
    Ok(ok_json(&result))
}

// ---------- validate / inspect / verify-audit / cancel ----------

fn cmd_validate(workflow: &str, workflow_dir: &str) -> CmdResult {
    let wf_path = resolve_workflow_path(workflow, workflow_dir);
    let workflow = match load_workflow(&wf_path) {
        Ok(w) => w,
        Err(e) => {
            return Ok(Outcome {
                code: EXIT_SCHEMA,
                stdout: pretty(&json!({"ok": false, "problems": e.problems})),
                stderr: String::new(),
            });
        }
    };
    // harness 绑定校验归 `check --harness`；此处只做结构/语义校验
    Ok(ok_json(&json!({
        "ok": true,
        "workflow_id": workflow.id,
        "stages": workflow.stage_ids(),
    })))
}

fn cmd_inspect(session: &str) -> CmdResult {
    let (session_dir, _state_path, audit_dir, state) = load_state(session)?;
    let mut lines = vec![
        format!("workflow : {}", state.workflow_id),
        format!("session  : {}", state.session_dir),
        format!("status   : {}", state.status),
        format!(
            "vars     : {}",
            serde_json::to_string(&router::effective_vars(&state)).unwrap_or_default()
        ),
        format!("seq      : {}", state.seq),
        String::new(),
    ];
    for s in &state.stages {
        lines.push(format!(
            "  {} {}  attempts {}/{}",
            state::status_mark(&s.status),
            s.id,
            s.attempt,
            s.max_attempts
        ));
        for a in &s.actions {
            let entry = state.reports.get(&a.action_id);
            let extra = entry
                .map(|e| {
                    if !e.summary.is_empty() {
                        e.summary.clone()
                    } else {
                        e.error.clone()
                    }
                })
                .unwrap_or_default();
            let suffix = if extra.is_empty() { String::new() } else { format!("  — {extra}") };
            lines.push(format!(
                "      {} {} attempt {} {}{}",
                state::action_mark(&a.status),
                a.agent,
                a.attempt,
                a.status,
                suffix
            ));
        }
    }
    if let Some(failure) = &state.failure {
        lines.push(String::new());
        lines.push(format!(
            "failure   : {} — {}",
            failure.stage, failure.reason
        ));
    }
    let shards = AuditTrail::new(&audit_dir).map_err(io_fail)?.load_all();
    lines.push(String::new());
    lines.push(format!("audit     : {} shards, last events:", shards.len()));
    for shard in shards.iter().rev().take(5).rev() {
        let stage_id = match shard.get("stage_id") {
            None | Some(Value::Null) => "None".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        };
        lines.push(format!(
            "  #{:06} {} [{}]",
            shard["seq"].as_u64().unwrap_or(0),
            shard["event_type"].as_str().unwrap_or_default(),
            stage_id
        ));
    }
    let _ = session_dir;
    Ok(ok_text(lines.join("\n")))
}

fn cmd_verify_audit(session: &str) -> CmdResult {
    let (_, _, audit_dir, state) = load_state(session)?;
    let shards = AuditTrail::new(&audit_dir).map_err(io_fail)?.load_all();
    let chain_problems = verify_chain(&shards);
    let mut problems = chain_problems.clone();
    let mut replay_matches = false;
    if chain_problems.is_empty() {
        match state::replay(&shards) {
            Ok(replayed) => {
                replay_matches =
                    state::canonical_hash(&replayed) == state::canonical_hash(&state);
                if !replay_matches {
                    problems.push(crate::i18n::t("cli.replay_state_mismatch"));
                }
            }
            Err(e) => problems.push(crate::i18n::msg("cli.replay_failed", &[&e.to_string()])),
        }
    }
    let body = json!({
        "ok": problems.is_empty(),
        "shards": shards.len(),
        "replay_matches": replay_matches,
        "problems": problems,
    });
    Ok(Outcome {
        code: if problems.is_empty() { EXIT_OK } else { EXIT_CONFLICT },
        stdout: pretty(&body),
        stderr: String::new(),
    })
}

fn cmd_cancel(session: &str) -> CmdResult {
    let (_, state_path, audit_dir, mut state) = load_state(session)?;
    if state.status != "running" {
        return Err(Fail::Conflict(crate::i18n::msg(
            "cli.workflow_terminal_not_cancellable",
            &[&state.status],
        )));
    }
    let before_statuses: serde_json::Map<String, Value> = state
        .stages
        .iter()
        .map(|s| (s.id.clone(), json!(s.status)))
        .collect();
    let hash_before = state::canonical_hash(&state);
    for s in &mut state.stages {
        if s.status != "completed" && s.status != "failed" {
            state::transition(s, "cancelled").expect("取消转移必合法");
        }
    }
    state.status = "cancelled".into();
    let audit = AuditTrail::new(&audit_dir).map_err(io_fail)?;
    let now = paths::format_ts(&paths::now_cst());
    let shard = audit
        .append(
            &state.workflow_id,
            "workflow.cancelled",
            None,
            None,
            json!({"from_stage_statuses": before_statuses}),
            &hash_before,
            &state::canonical_hash(&state),
            &now,
        )
        .expect("内置事件类型必合法");
    state.seq = shard["seq"].as_u64().unwrap_or(0);
    crate::hooks::notify(&shard, &state.session_dir);
    state::save(&mut state, &state_path, &now).map_err(io_fail)?;
    Ok(ok_json(&json!({"ok": true, "status": "cancelled"})))
}

fn cmd_replay(session: &str, overwrite: bool) -> CmdResult {
    let (session_dir, state_path, audit_dir) = session_paths(session);
    let shards = AuditTrail::new(&audit_dir).map_err(io_fail)?.load_all();
    if shards.is_empty() {
        return Err(Fail::Conflict(crate::i18n::msg(
            "cli.no_shards_for_replay",
            &[&paths::to_posix(&audit_dir)],
        )));
    }
    let problems = verify_chain(&shards);
    if !problems.is_empty() {
        return Ok(fail_outcome(
            EXIT_CONFLICT,
            &crate::i18n::t("cli.chain_invalid_replay_refused"),
            &problems,
        ));
    }
    let mut rebuilt = state::replay(&shards)
        .map_err(|e| Fail::Conflict(crate::i18n::msg("cli.replay_failed", &[&e.to_string()])))?;

    let matches: Option<bool> = if state_path.is_file() {
        Some(match state::load(&state_path) {
            Ok(current) => {
                state::canonical_hash(&current) == state::canonical_hash(&rebuilt)
            }
            Err(_) => false, // 当前文件不可解析——这正是需要恢复的场景
        })
    } else {
        None
    };
    let target = if state_path.is_file() {
        if overwrite {
            state_path.clone()
        } else {
            session_dir.join("dmaf-state.replay.md")
        }
    } else {
        // 恢复场景：状态文件缺失时直接重建
        state_path.clone()
    };
    let updated_at = rebuilt.updated_at.clone();
    let status = rebuilt.status.clone();
    state::save(&mut rebuilt, &target, &updated_at).map_err(io_fail)?;
    Ok(ok_json(&json!({
        "ok": true,
        "shards": shards.len(),
        "written_to": paths::to_posix(&target),
        "matches_current_state": matches,
        "status": status,
    })))
}

fn cmd_fork(
    session: &str,
    seq: i64,
    output_root: &str,
    workflow_override: Option<&str>,
    workflow_dir: &str,
    artifacts_dir: &str,
) -> CmdResult {
    let (session_dir, _state_path, audit_dir, state) = load_state(session)?;
    let workflow = load_workflow_for(&state, workflow_override, workflow_dir, artifacts_dir)?;
    let result = forking::fork_session(
        &workflow,
        &session_dir,
        &audit_dir,
        seq.max(0) as u64,
        Path::new(output_root),
        paths::now_cst(),
        None,
        &state.scope,
    )
    .map_err(|e: forking::ForkConflict| Fail::Conflict(e.0))?;
    Ok(ok_json(&result))
}

// ---------- log ----------

fn describe_shard(shard: &Value) -> String {
    let event = shard["event_type"].as_str().unwrap_or_default();
    let p = shard.get("payload").cloned().unwrap_or(json!({}));
    let seq = shard["seq"].as_u64().unwrap_or(0);
    let ts = shard["timestamp"].as_str().unwrap_or_default();
    let stage = match shard.get("stage_id") {
        None | Some(Value::Null) => "-".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    };
    let head = format!("#{seq:06} {ts} {event} [{stage}]");
    let get = |k: &str| p.get(k).cloned().unwrap_or(Value::Null);
    let s_of = |k: &str| match get(k) {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => other.to_string(),
    };
    match event {
        "workflow.started" => {
            // 人读显示优先业务主变量，task 驱动的工作流回退 TASK 原文
            let kp = get("vars")
                .get("knowledge_point")
                .cloned()
                .or_else(|| get("vars").get("TASK").cloned())
                .unwrap_or(json!(""))
                .as_str()
                .unwrap_or("")
                .to_string();
            let stages: Vec<String> = get("stages")
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|s| s["id"].as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default();
            let stages_j = stages.join(" → ");
            crate::i18n::msg("cli.log_workflow_started", &[&head, &kp, &stages_j])
        }
        "action.dispatched" => {
            let agents: Vec<String> = get("actions")
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|a| a["agent"].as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default();
            let attempt = match get("attempt") {
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            crate::i18n::msg(
                "cli.log_action_dispatched",
                &[&head, &attempt, &s_of("collaboration"), &agents.join("、")],
            )
        }
        "action.reported" => {
            let status = if s_of("effective_status") == "success" {
                crate::i18n::t("cli.log_status_success")
            } else {
                crate::i18n::t("cli.log_status_failed")
            };
            let mut detail = s_of("summary");
            if detail.is_empty() {
                detail = s_of("error");
            }
            let suffix = if detail.is_empty() { String::new() } else { format!(" — {detail}") };
            let attempt = match get("attempt") {
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            crate::i18n::msg(
                "cli.log_action_reported",
                &[&head, &s_of("agent"), &attempt, &status, &suffix],
            )
        }
        "retry.scheduled" => {
            let attempt = match get("attempt") {
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            crate::i18n::msg("cli.log_retry_scheduled", &[&head, &s_of("new_action_id"), &attempt])
        }
        "approval.granted" | "approval.rejected" => {
            let verdict = if event == "approval.granted" {
                crate::i18n::t("cli.log_verdict_granted")
            } else {
                crate::i18n::t("cli.log_verdict_rejected")
            };
            let note = s_of("note");
            let note_part = if note.is_empty() { String::new() } else { format!("：{note}") };
            crate::i18n::msg("cli.log_approval", &[&head, &verdict, &s_of("gate"), &note_part])
        }
        "workflow.completed" => crate::i18n::msg("cli.log_workflow_completed", &[&head]),
        "workflow.failed" => crate::i18n::msg("cli.log_workflow_failed", &[&head, &s_of("reason")]),
        "workflow.cancelled" => crate::i18n::msg("cli.log_workflow_cancelled", &[&head]),
        "workflow.forked" => {
            let from_seq = match get("from_seq") {
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            crate::i18n::msg("cli.log_workflow_forked", &[&head, &s_of("from_session"), &from_seq])
        }
        _ => head,
    }
}

fn cmd_log(session: &str, event_type: Option<&str>, stage: Option<&str>, limit: Option<i64>) -> CmdResult {
    let (_, _, audit_dir) = session_paths(session);
    let shards = AuditTrail::new(&audit_dir).map_err(io_fail)?.load_all();
    if shards.is_empty() {
        return Err(Fail::Conflict(crate::i18n::msg(
            "cli.no_audit_shards",
            &[&paths::to_posix(&audit_dir)],
        )));
    }

    let types: Option<Vec<String>> = match event_type {
        Some(t) => {
            let list: Vec<String> = t
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let unknown: Vec<String> = list
                .iter()
                .filter(|t| !crate::audit::is_valid_event_type(t))
                .cloned()
                .collect();
            if !unknown.is_empty() {
                return Err(Fail::Usage(crate::i18n::msg(
                    "cli.unknown_event_types",
                    &[&py_list(&unknown)],
                )));
            }
            Some(list)
        }
        None => None,
    };

    let mut lines: Vec<String> = Vec::new();
    let chain_problems = verify_chain(&shards);
    if !chain_problems.is_empty() {
        let chain_n = chain_problems.len().to_string();
        lines.push(crate::i18n::msg("cli.log_chain_problems_warning", &[&chain_n]));
        lines.push(String::new());
    }
    let mut shown: Vec<&Value> = shards
        .iter()
        .filter(|s| {
            let et_ok = types
                .as_ref()
                .map(|ts| {
                    ts.contains(&s["event_type"].as_str().unwrap_or_default().to_string())
                })
                .unwrap_or(true);
            let stage_ok = match stage {
                Some(st) => {
                    let sid = match s.get("stage_id") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(v)) => Some(v.as_str()),
                        _ => None,
                    };
                    sid == Some(st)
                }
                None => true,
            };
            et_ok && stage_ok
        })
        .collect();
    // Python shown[-limit:] 语义：limit>0 取末尾 N 条；==0 取全部；<0 丢前 |limit| 条
    if let Some(limit) = limit {
        let n = shown.len() as i64;
        let start: usize = if limit > 0 {
            (n - limit).max(0) as usize
        } else if limit == 0 {
            0
        } else {
            ((-limit).min(n)) as usize
        };
        shown = shown.split_off(start); // 注意：split_off 返回后半段
    }
    if shown.is_empty() {
        lines.push(crate::i18n::t("cli.no_matching_events"));
    }
    for s in &shown {
        lines.push(describe_shard(s));
    }
    Ok(ok_text(lines.join("\n")))
}

// ---------- verify-determinism ----------

fn actions_fingerprint(actions: &Value) -> String {
    crate::json_text(actions)
}

fn instruction_from_shard(workflow_id: &str, shard: &Value) -> Value {
    let payload = shard.get("payload").cloned().unwrap_or(json!({}));
    let seq = shard["seq"].as_u64().unwrap_or(0);
    json!({
        "instruction_id": format!("ins-{workflow_id}-{:06}", seq.saturating_sub(1)),
        "type": crate::i18n::t("cli.instruction_type_dispatch"),
        "workflow_id": workflow_id,
        "stage": shard.get("stage_id").cloned().unwrap_or(Value::Null),
        "stage_state": payload.get("to").cloned().unwrap_or(Value::Null),
        "payload": payload,
    })
}

fn cmd_verify_determinism(
    session: &str,
    workflow: Option<&str>,
    artifacts_dir: &str,
    print_pairs: bool,
) -> CmdResult {
    let session_dir = PathBuf::from(session);
    let audit_dir = session_dir.join("audit");
    let shards = AuditTrail::new(&audit_dir).map_err(io_fail)?.load_all();
    if shards.is_empty() {
        return Ok(Outcome {
            code: EXIT_CONFLICT,
            stdout: String::new(),
            stderr: format!(
                "{}\n",
                crate::i18n::msg("cli.no_audit_shards", &[&paths::to_posix(&audit_dir)])
            ),
        });
    }
    let chain_problems = verify_chain(&shards);
    if !chain_problems.is_empty() {
        let mut err = format!("{}\n", crate::i18n::t("cli.chain_invalid_determinism_refused"));
        for p in &chain_problems {
            err.push_str(&format!("  {p}\n"));
        }
        return Ok(Outcome { code: EXIT_CONFLICT, stdout: String::new(), stderr: err });
    }

    let state_path = session_dir.join("dmaf-state.md");
    let (workflow_id, scope) = if state_path.is_file() {
        state::load(&state_path)
            .map(|s| (s.workflow_id.clone(), s.scope.clone()))
            .unwrap_or_default()
    } else {
        (
            shards[0]["workflow_id"].as_str().unwrap_or_default().to_string(),
            String::new(),
        )
    };
    // 定义来源：显式 --workflow（内联）→ scope 会话走编译产物 → 内联默认目录
    let (wf, workflow_path) = if let Some(w) = workflow {
        let p = PathBuf::from(w);
        match load_workflow(&p) {
            Ok(w) => (w, p),
            Err(e) => return Ok(verify_det_schema_err(&p, e)),
        }
    } else if !scope.is_empty() {
        let dir = PathBuf::from(artifacts_dir);
        match load_workflow_from_artifacts(&dir, &scope) {
            Ok(w) => (w, dir),
            Err(f) => {
                let Fail::Schema { message, problems } = f else {
                    unreachable!("load_workflow_from_artifacts 只返回 Schema")
                };
                let mut err = format!("{message}\n");
                for p in &problems {
                    err.push_str(&format!("  {p}\n"));
                }
                return Ok(Outcome { code: EXIT_SCHEMA, stdout: String::new(), stderr: err });
            }
        }
    } else {
        let p = Path::new(DEFAULT_WORKFLOW_DIR).join(format!("{workflow_id}.yaml"));
        match load_workflow(&p) {
            Ok(w) => (w, p),
            Err(e) => return Ok(verify_det_schema_err(&p, e)),
        }
    };

    let mut mismatches: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for (i, shard) in shards.iter().enumerate() {
        if shard["event_type"].as_str() != Some("action.dispatched") {
            continue;
        }
        let state_i = state::replay(&shards[..i]).map_err(|e| Fail::Conflict(e.0))?;
        let (instruction, _) = router::compute_next(&wf, &state_i).map_err(Fail::Conflict)?;
        checked += 1;
        let seq = shard["seq"].as_u64().unwrap_or(0);
        let recorded_instruction = instruction_from_shard(&wf.id, shard);
        let mut shard_diffs: Vec<String> = Vec::new();

        let expect_id = format!("ins-{}-{:06}", wf.id, state_i.seq);
        if instruction["instruction_id"].as_str() != Some(expect_id.as_str()) {
            shard_diffs.push(crate::i18n::msg(
                "cli.det_instruction_id_mismatch",
                &[&expect_id, &instruction["instruction_id"].as_str().unwrap_or_default()],
            ));
        }
        if instruction["stage"] != shard["stage_id"] {
            shard_diffs.push(crate::i18n::msg(
                "cli.det_stage_mismatch",
                &[&crate::json_text(&shard["stage_id"]), &crate::json_text(&instruction["stage"])],
            ));
        } else {
            let recorded = &shard["payload"];
            let computed = &instruction["payload"];
            if computed["collaboration"] != recorded["collaboration"] {
                shard_diffs.push(crate::i18n::msg(
                    "cli.det_collaboration_mismatch",
                    &[
                        &crate::json_text(&recorded["collaboration"]),
                        &crate::json_text(&computed["collaboration"]),
                    ],
                ));
            }
            let recorded_agents = recorded
                .get("agents")
                .cloned()
                .unwrap_or(json!([]));
            let computed_agents: Vec<String> = computed
                .get("actions")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|a| a["agent"].as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default();
            if crate::json_text(&recorded_agents)
                != crate::json_text(&json!(computed_agents))
            {
                shard_diffs.push(crate::i18n::msg(
                    "cli.det_agents_mismatch",
                    &[&crate::json_text(&recorded_agents), &crate::json_text(&json!(computed_agents))],
                ));
            }
            let recorded_actions = recorded.get("actions").cloned().unwrap_or(json!([]));
            let computed_actions: Vec<Value> = computed
                .get("actions")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|a| {
                            let mut obj = a.as_object().cloned().unwrap_or_default();
                            obj.remove("skill_brief");
                            Value::Object(obj)
                        })
                        .collect()
                })
                .unwrap_or_default();
            if actions_fingerprint(&recorded_actions)
                != actions_fingerprint(&json!(computed_actions))
            {
                shard_diffs.push(crate::i18n::msg(
                    "cli.det_actions_mismatch",
                    &[
                        &actions_fingerprint(&recorded_actions),
                        &actions_fingerprint(&json!(computed_actions)),
                    ],
                ));
            }
        }

        let seq6 = format!("{seq:06}");
        for d in &shard_diffs {
            mismatches.push(crate::i18n::msg("cli.det_shard_mismatch_line", &[&seq6, d.as_str()]));
        }

        if print_pairs {
            let rp = &recorded_instruction["payload"];
            let cp = &instruction["payload"];
            let verdict = if shard_diffs.is_empty() {
                crate::i18n::t("cli.det_pair_match")
            } else {
                crate::i18n::t("cli.det_pair_mismatch")
            };
            println!(
                "{}",
                crate::i18n::msg(
                    "cli.det_pair_header",
                    &[&instruction["stage"].to_string(), &seq6, &verdict],
                )
            );
            println!(
                "{}",
                crate::i18n::msg(
                    "cli.det_pair_recomputed",
                    &[
                        &instruction["instruction_id"].as_str().unwrap_or_default(),
                        &instruction["type"].as_str().unwrap_or_default(),
                        &crate::json_text(&cp["collaboration"]),
                    ],
                )
            );
            if let Some(actions) = cp.get("actions").and_then(|a| a.as_array()) {
                for a in actions {
                    println!("    {} ({}) attempt {}", a["agent"], a["action_id"], a["attempt"]);
                    println!("      inputs : {}", fmt_opt_list(&a["inputs"]));
                    println!("      outputs: {}", fmt_opt_list(&a["outputs"]));
                    println!("      task   : {}", a["task_description"]);
                }
            }
            println!(
                "{}",
                crate::i18n::msg(
                    "cli.det_pair_ledger",
                    &[
                        &recorded_instruction["instruction_id"].to_string(),
                        &crate::json_text(&rp["from"]),
                        &crate::json_text(&rp["to"]),
                        &crate::json_text(&rp["collaboration"]),
                    ],
                )
            );
            if let Some(actions) = rp.get("actions").and_then(|a| a.as_array()) {
                for a in actions {
                    println!("    {} ({}) attempt {}", a["agent"], a["action_id"], a["attempt"]);
                    println!("      inputs : {}", fmt_opt_list(&a["inputs"]));
                    println!("      outputs: {}", fmt_opt_list(&a["outputs"]));
                    println!("      task   : {}", a.get("task_description").map(|v| crate::json_text(v)).unwrap_or_default());
                }
            }
            for d in &shard_diffs {
                println!("  ⚠ {d}");
            }
        }
    }

    let shards_n = shards.len().to_string();
    let mut out = format!(
        "{}\n",
        crate::i18n::msg(
            "cli.det_summary_header",
            &[&shards_n, &paths::to_posix(&workflow_path)],
        )
    );
    let checked_n = checked.to_string();
    out.push_str(&format!(
        "{}\n",
        crate::i18n::msg("cli.det_summary_checked", &[&checked_n])
    ));
    if !mismatches.is_empty() {
        let mismatches_n = mismatches.len().to_string();
        let mut err = format!(
            "{}\n",
            crate::i18n::msg("cli.det_mismatch_header", &[&mismatches_n])
        );
        for m in &mismatches {
            err.push_str(&format!("  {m}\n"));
        }
        return Ok(Outcome { code: 1, stdout: out, stderr: err });
    }
    out.push_str(&format!("{}\n", crate::i18n::t("cli.det_all_match")));
    Ok(Outcome { code: EXIT_OK, stdout: out, stderr: String::new() })
}

fn verify_det_schema_err(path: &Path, e: WorkflowError) -> Outcome {
    let mut err = format!(
        "{}\n",
        crate::i18n::msg("cli.workflow_invalid_at_path", &[&paths::to_posix(path)])
    );
    for p in &e.problems {
        err.push_str(&format!("  {p}
"));
    }
    Outcome { code: EXIT_SCHEMA, stdout: String::new(), stderr: err }
}

fn fmt_opt_list(v: &Value) -> String {
    let empty: Vec<Value> = Vec::new();
    let arr = v.as_array().unwrap_or(&empty);
    if arr.is_empty() {
        "—".to_string()
    } else {
        crate::json_text(v)
    }
}

// ---------- doctor ----------

/// .dmaf/ 下的会话目录名：YYYYMMDD-HHMMSS（冲突时追加 -NN）。
fn is_session_dir_name(n: &str) -> bool {
    let b = n.as_bytes();
    let digits = |s: &[u8]| s.iter().all(|c| c.is_ascii_digit());
    match b.len() {
        15 => b[8] == b'-' && digits(&b[0..8]) && digits(&b[9..15]),
        18 => b[8] == b'-' && b[14] == b'-' && digits(&b[0..8]) && digits(&b[9..14]) && digits(&b[15..18]),
        _ => false,
    }
}

// ---------- doctor ----------

/// doctor 单项检查（publish 门禁按 severity 复用，E-3：仅 error 级拒绝）。
pub(crate) struct DoctorCheck {
    pub name: &'static str,
    pub ok: bool,
    pub detail: Value,
}

fn under_root(root: &Path, p: &str) -> PathBuf {
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        pb
    } else {
        root.join(pb)
    }
}

/// doctor 核心检查（二进制 / 标准布局 / .dmaf 卫生 / 产物漂移 / 清单兼容），
/// 路径相对 project_root 解析（cmd_doctor 传 "." 与现行行为一致）。
/// harness 相关检查（harness_refs / assets_sync）与 hooks 检查仅 `dmaf doctor` 自身执行。
pub(crate) fn doctor_core_checks(project_root: &Path, artifacts_dir: &str) -> Vec<DoctorCheck> {
    let mut checks: Vec<DoctorCheck> = Vec::new();
    let artifacts = under_root(project_root, artifacts_dir);

    // 1. 二进制与版本
    let exe = std::env::current_exe()
        .map(|p| paths::to_posix(&p))
        .unwrap_or_else(|_| crate::i18n::t("cli.unknown"));
    checks.push(DoctorCheck {
        name: "binary",
        ok: true,
        detail: json!({"path": exe, "version": env!("CARGO_PKG_VERSION")}),
    });

    // 2. 标准布局（dmaf/ 定义层；固定路径，与 --workflow-dir 的内联解析无关）
    let layout: Vec<(&str, bool)> = [
        ("dmaf/workflows/stages", project_root.join("dmaf/workflows/stages").is_dir()),
        ("dmaf/workflows/scopes", project_root.join("dmaf/workflows/scopes").is_dir()),
        ("dmaf/agents", project_root.join("dmaf/agents").is_dir()),
        ("dmaf/skills", project_root.join("dmaf/skills").is_dir()),
        ("dmaf/compiled", artifacts.is_dir()),
    ]
    .iter()
    .map(|(n, ok)| (*n, *ok))
    .collect();
    let missing: Vec<&str> = layout.iter().filter(|(_, ok)| !ok).map(|(n, _)| *n).collect();
    checks.push(DoctorCheck {
        name: "layout",
        ok: missing.is_empty(),
        detail: json!({"missing": missing, "project_root": paths::to_posix(project_root)}),
    });

    // 2b. .dmaf/ 卫生：其下只应有时间戳会话目录（引擎独占，外部勿放文件）
    let dmaf_root = project_root.join(".dmaf");
    if dmaf_root.is_dir() {
        let strangers: Vec<String> = std::fs::read_dir(&dmaf_root)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| !is_session_dir_name(n))
                    .collect()
            })
            .unwrap_or_default();
        checks.push(DoctorCheck {
            name: "dmaf_hygiene",
            ok: strangers.is_empty(),
            detail: json!({"strangers": strangers,
                   "hint": crate::i18n::t("cli.doctor_hint_dmaf_hygiene")}),
        });
    }

    // 3. 产物漂移（stages/scopes 存在时重编译比对）
    let stages_dir = project_root.join("dmaf/workflows/stages");
    let scopes_dir = project_root.join("dmaf/workflows/scopes");
    if stages_dir.is_dir() && scopes_dir.is_dir() {
        match crate::compile::compile(&stages_dir, &scopes_dir, None) {
            Ok(compiled) => {
                // 产物格式版本预检：产物与二进制的版本字段不一致时给出可行动结论，跳过字节比对
                let artifact_version = |file: &str| -> Option<u64> {
                    std::fs::read_to_string(artifacts.join(file))
                        .ok()
                        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                        .and_then(|v| v["version"].as_u64())
                };
                let gv = artifact_version(crate::compile::graph_file_name());
                let sv = artifact_version(crate::compile::grid_file_name());
                if gv != Some(crate::compile::GRAPH_VERSION)
                    || sv != Some(crate::compile::GRID_VERSION)
                {
                    checks.push(DoctorCheck {
                        name: "artifacts_drift",
                        ok: false,
                        detail: json!({
                            "reason": crate::i18n::t("cli.doctor_reason_artifact_version"),
                            "artifact": {"stage_graph": gv, "scope_grid": sv},
                            "binary": {
                                "stage_graph": crate::compile::GRAPH_VERSION,
                                "scope_grid": crate::compile::GRID_VERSION,
                            },
                            "hint": crate::i18n::t("cli.doctor_hint_artifact_version"),
                        }),
                    });
                } else {
                    let g_text = crate::compile::serialize_artifact(&compiled.stage_graph);
                    let s_text = crate::compile::serialize_artifact(&compiled.scope_grid);
                    let g_ok = std::fs::read_to_string(artifacts.join(crate::compile::graph_file_name()))
                        .map(|t| t == g_text)
                        .unwrap_or(false);
                    let s_ok = std::fs::read_to_string(artifacts.join(crate::compile::grid_file_name()))
                        .map(|t| t == s_text)
                        .unwrap_or(false);
                    checks.push(DoctorCheck {
                        name: "artifacts_drift",
                        ok: g_ok && s_ok,
                        detail: json!({"stage_graph": g_ok, "scope_grid": s_ok,
                               "stages": compiled.stage_graph["stages"].as_array().map(|a| a.len()).unwrap_or(0),
                               "scopes": compiled.scope_grid["scopes"].as_object().map(|m| m.len()).unwrap_or(0)}),
                    });
                }
            }
            Err(e) => checks.push(DoctorCheck {
                name: "artifacts_drift",
                ok: false,
                detail: json!({"problems": e.problems}),
            }),
        }
    } else {
        checks.push(DoctorCheck {
            name: "artifacts_drift",
            ok: true,
            detail: json!({"skipped": crate::i18n::t("cli.doctor_skipped_no_stage_lib")}),
        });
    }

    // 4. 项目清单 dmaf/manifest.yaml（可选）：engine 版本范围兼容判定（缺失 = 正常态）
    match crate::manifest::load(project_root) {
        Ok(None) => checks.push(DoctorCheck {
            name: "manifest",
            ok: true,
            detail: json!({"configured": false,
                   "hint": crate::i18n::t("cli.doctor_hint_manifest_optional")}),
        }),
        Ok(Some(m)) => {
            match crate::manifest::engine_matches(&m.engine, env!("CARGO_PKG_VERSION")) {
                Ok(true) => checks.push(DoctorCheck {
                    name: "manifest",
                    ok: true,
                    detail: json!({"configured": true, "name": m.name, "version": m.version,
                           "engine": m.engine, "current": env!("CARGO_PKG_VERSION")}),
                }),
                Ok(false) => checks.push(DoctorCheck {
                    name: "manifest",
                    ok: false,
                    detail: json!({"configured": true, "name": m.name, "engine": m.engine,
                           "current": env!("CARGO_PKG_VERSION"),
                           "hint": crate::i18n::t("cli.doctor_hint_manifest_engine")}),
                }),
                Err(e) => checks.push(DoctorCheck {
                    name: "manifest",
                    ok: false,
                    detail: json!({"configured": true, "error": e}),
                }),
            }
        }
        Err(e) => checks.push(DoctorCheck {
            name: "manifest",
            ok: false,
            detail: json!({"error": e}),
        }),
    }

    checks
}

fn cmd_doctor(workflow_dir: &str, artifacts_dir: &str, harness: Option<&str>) -> CmdResult {
    /// inline/ 目录下唯一的 .yaml；零个或多个时返回 None（调用方降级为跳过）
    fn single_inline_yaml(dir: &Path) -> Option<PathBuf> {
        let mut found: Option<PathBuf> = None;
        for entry in std::fs::read_dir(dir).ok()? {
            let p = entry.ok()?.path();
            if p.extension().and_then(|e| e.to_str()) == Some("yaml") {
                if found.is_some() {
                    return None;
                }
                found = Some(p);
            }
        }
        found
    }

    let mut checks: Vec<Value> = Vec::new();
    let mut all_ok = true;
    let mut add = |name: &str, ok: bool, detail: Value| {
        if !ok {
            all_ok = false;
        }
        checks.push(json!({"check": name, "ok": ok, "detail": detail}));
    };

    // 1–4. 核心检查（publish 门禁复用同一实现）
    for c in doctor_core_checks(Path::new("."), artifacts_dir) {
        add(c.name, c.ok, c.detail);
    }

    // 5. harness 齐备性（指定 --harness 且定义存在时）
    if let Some(h) = harness {
        let rh = harness_root(h).map_err(Fail::Usage)?;
        // 工作流来源：stage 库优先（编译）；否则回退 inline/ 下唯一 .yaml（自动发现）
        let wf_src = if Path::new("dmaf/workflows/stages").is_dir() && Path::new("dmaf/workflows/scopes").is_dir() {
            Some(
                crate::compile::compile(Path::new("dmaf/workflows/stages"), Path::new("dmaf/workflows/scopes"), None)
                    .map(|c| c.workflow)
                    .map_err(|e: crate::compile::CompileError| {
                        Fail::Schema {
                            message: crate::i18n::t("cli.workflow_compile_failed"),
                            problems: e.problems,
                        }
                    })?,
            )
        } else {
            match single_inline_yaml(Path::new(workflow_dir)) {
                Some(p) => Some(load_workflow(&p).map_err(|e: WorkflowError| Fail::Schema {
                    message: crate::i18n::t("cli.inline_workflow_invalid"),
                    problems: e.problems,
                })?),
                None => None,
            }
        };
        match wf_src {
            Some(wf) => {
                let registry = load_registry(&rh.dir, rh.layout);
                let problems = validate_refs(&wf, &registry);
                add(
                    "harness_refs",
                    problems.is_empty(),
                    json!({"harness": paths::to_posix(&rh.dir), "problems": problems}),
                );
            }
            None => add(
                "harness_refs",
                true,
                json!({"harness": paths::to_posix(&rh.dir),
                       "skipped": crate::i18n::t("cli.doctor_skipped_harness_refs")}),
            ),
        }

        // 5b. 资产同步漂移：定义层 vs harness 已装资产的逐文件比对（复用 install --check-only）。
        //     账本哈希门下：缺失/未改动漂移 → 重跑 install 自动同步（would_change）；
        //     用户改过的目标 → 跳过，需 --force 显式覆盖（modified）。
        match crate::install::install(&rh.dir, rh.layout, Path::new("dmaf"), false, true, rh.skills_nested, false) {
            Ok(report) => {
                let list_of = |key: &str| -> Vec<String> {
                    report[key]
                        .as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default()
                };
                let mut would = list_of("installed");
                would.extend(list_of("updated"));
                let modified = list_of("skipped");
                add(
                    "assets_sync",
                    would.is_empty() && modified.is_empty(),
                    json!({
                        "harness": paths::to_posix(&rh.dir),
                        "would_change": would,
                        "modified": modified,
                        "hint": crate::i18n::t("cli.doctor_hint_assets_sync"),
                    }),
                );
            }
            Err(crate::install::InstallError::Invalid(problems)) => {
                add("assets_sync", false, json!({"problems": problems}))
            }
            Err(crate::install::InstallError::Io(e)) => {
                add("assets_sync", false, json!({"error": e}))
            }
        }
    }

    // 6. hooks.yaml 健康度（配置缺失 = 正常态；坏配置 / 未知事件键 → ok=false 带明细）
    let hooks_report = crate::hooks::doctor_report();
    let hooks_ok = !hooks_report["configured"].as_bool().unwrap_or(false)
        || hooks_report["warnings"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(true);
    add("hooks", hooks_ok, hooks_report);

    Ok(ok_json(&json!({"ok": all_ok, "checks": checks})))
}

// ---------- compile ----------

fn cmd_compile(
    stages_dir: &str,
    scopes_dir: &str,
    out: &str,
    check: bool,
    harness: Option<&str>,
) -> CmdResult {
    let rh = match harness {
        Some(h) => Some(harness_root(h).map_err(Fail::Usage)?),
        None => None,
    };
    let compiled = crate::compile::compile(
        Path::new(stages_dir),
        Path::new(scopes_dir),
        rh.as_ref(),
    )
    .map_err(|e: crate::compile::CompileError| Fail::Schema {
        message: crate::i18n::t("cli.workflow_compile_failed"),
        problems: e.problems,
    })?;
    let graph_text = crate::compile::serialize_artifact(&compiled.stage_graph);
    let grid_text = crate::compile::serialize_artifact(&compiled.scope_grid);
    let out_dir = PathBuf::from(out);
    let graph_path = out_dir.join(crate::compile::graph_file_name());
    let grid_path = out_dir.join(crate::compile::grid_file_name());

    if check {
        let mut diffs: Vec<String> = Vec::new();
        for (path, text) in [(&graph_path, &graph_text), (&grid_path, &grid_text)] {
            match std::fs::read_to_string(path) {
                Ok(existing) if existing == *text => {}
                Ok(_) => diffs.push(crate::i18n::msg(
                    "cli.compile_drift_content",
                    &[&crate::paths::to_posix(path)],
                )),
                Err(_) => diffs.push(crate::i18n::msg(
                    "cli.compile_drift_missing",
                    &[&crate::paths::to_posix(path)],
                )),
            }
        }
        if !diffs.is_empty() {
            return Ok(fail_outcome(
                EXIT_SCHEMA,
                &crate::i18n::t("cli.compile_drift_detected"),
                &diffs,
            ));
        }
        return Ok(ok_json(&json!({
            "ok": true,
            "check": true,
            "out": crate::paths::to_posix(&out_dir),
            "stages": compiled.stage_graph["stages"].as_array().map(|a| a.len()).unwrap_or(0),
            "scopes": compiled.scope_grid["scopes"].as_object().map(|m| m.len()).unwrap_or(0),
        })));
    }

    std::fs::create_dir_all(&out_dir).map_err(io_fail)?;
    crate::paths::atomic_write_text(&graph_path, &graph_text).map_err(io_fail)?;
    crate::paths::atomic_write_text(&grid_path, &grid_text).map_err(io_fail)?;
    Ok(ok_json(&json!({
        "ok": true,
        "out": crate::paths::to_posix(&out_dir),
        "written": [crate::paths::to_posix(&graph_path), crate::paths::to_posix(&grid_path)],
        "stages": compiled.stage_graph["stages"].as_array().map(|a| a.len()).unwrap_or(0),
        "scopes": compiled.scope_grid["scopes"].as_object().map(|m| m.len()).unwrap_or(0),
    })))
}

// ---------- install ----------

fn entry_config_for(name: &str, check_only: bool, force: bool) -> Result<Value, Fail> {
    crate::install::install_entry_config(name, Path::new("."), check_only, force).map_err(|e| match e {
        crate::install::InstallError::Io(msg) => Fail::Io(msg),
        crate::install::InstallError::Invalid(problems) => Fail::Schema {
            message: crate::i18n::t("cli.entry_config_install_failed"),
            problems,
        },
    })
}

fn cmd_install(harness: Option<&str>, prune: bool, check_only: bool, force: bool) -> CmdResult {
    // --harness all：安装到项目根下全部已检测的 harness（多 harness 共存时的"全选"）
    if harness == Some("all") {
        let found = crate::registry::detect_harnesses(Path::new("."));
        if found.is_empty() {
            return Err(Fail::Usage(crate::i18n::t("cli.no_harness_detected")));
        }
        let source_root = PathBuf::from("dmaf");
        let mut results = Vec::new();
        let (mut installed_total, mut unchanged_total, mut updated_total, mut skipped_total, mut overwritten_total, mut pruned_total) =
            (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        for name in &found {
            let rh = harness_root(name).expect("探测到的名称必为已知 harness");
            let mut report =
                crate::install::install(&rh.dir, rh.layout, &source_root, prune, check_only, rh.skills_nested, force)
                    .map_err(|e| match e {
                        crate::install::InstallError::Invalid(problems) => Fail::Schema {
                            message: crate::i18n::t("cli.install_invalid_assets"),
                            problems,
                        },
                        crate::install::InstallError::Io(msg) => Fail::Io(msg),
                    })?;
            if let Some(n) = rh.name {
                report["entry_config"] = entry_config_for(n, check_only, force)?;
            }
            report["harness_name"] = json!(rh.name);
            installed_total += report["installed"].as_array().map(|a| a.len()).unwrap_or(0);
            unchanged_total += report["unchanged"].as_array().map(|a| a.len()).unwrap_or(0);
            updated_total += report["updated"].as_array().map(|a| a.len()).unwrap_or(0);
            skipped_total += report["skipped"].as_array().map(|a| a.len()).unwrap_or(0);
            overwritten_total += report["overwritten"].as_array().map(|a| a.len()).unwrap_or(0);
            pruned_total += report["pruned"].as_array().map(|a| a.len()).unwrap_or(0);
            results.push(report);
        }
        return Ok(ok_json(&json!({
            "ok": true,
            "harness": "all",
            "auto_detected": false,
            "check_only": check_only,
            "prune": prune,
            "force": force,
            "harnesses": found,
            "results": results,
            "installed_total": installed_total,
            "unchanged_total": unchanged_total,
            "updated_total": updated_total,
            "skipped_total": skipped_total,
            "overwritten_total": overwritten_total,
            "pruned_total": pruned_total,
        })));
    }

    // 缺省 --harness 时自动探测项目根下已安装的 harness 目录：
    // 唯一 → 直接安装；多个 → 列出让用户显式指定（或 --harness all 全装）；无 → 给出可行动的指引。
    let (rh, auto_detected) = match harness {
        Some(h) => (harness_root(h).map_err(Fail::Usage)?, false),
        None => {
            let found = crate::registry::detect_harnesses(Path::new("."));
            match found.as_slice() {
                [] => return Err(Fail::Usage(crate::i18n::t("cli.no_harness_detected"))),
                [only] => (harness_root(only).expect("探测到的名称必为已知 harness"), true),
                many => {
                    return Err(Fail::Usage(crate::i18n::msg(
                        "cli.multiple_harnesses_detected",
                        &[&many.join("、")],
                    )));
                }
            }
        }
    };
    let source_root = PathBuf::from("dmaf");
    let result = crate::install::install(&rh.dir, rh.layout, &source_root, prune, check_only, rh.skills_nested, force);
    let mut report = match result {
        Ok(report) => report,
        Err(crate::install::InstallError::Invalid(problems)) => {
            return Err(Fail::Schema {
                message: crate::i18n::t("cli.install_invalid_assets"),
                problems,
            });
        }
        Err(crate::install::InstallError::Io(e)) => return Err(Fail::Io(e)),
    };
    report["harness_name"] = json!(rh.name);
    report["auto_detected"] = json!(auto_detected);
    // 入口配置落位（仅裸名 harness；路径透传不适用）。逐文件落位到 harness 目录，已存在者跳过。
    if let Some(name) = rh.name {
        report["entry_config"] = entry_config_for(name, check_only, force)?;
    }
    Ok(ok_json(&report))
}

// ---------- add / push ----------

fn fail_from_package(e: crate::package::PackageError) -> Fail {
    match e {
        crate::package::PackageError::Usage(m) => Fail::Usage(m),
        crate::package::PackageError::Schema { message, problems } => Fail::Schema { message, problems },
        crate::package::PackageError::Conflict(list) => Fail::Conflict(crate::i18n::msg(
            "cli.package_merge_conflict",
            &[&list.len().to_string(), &crate::py_list(&list)],
        )),
        crate::package::PackageError::Compile(problems) => Fail::Schema {
            message: crate::i18n::t("cli.package_compile_failed"),
            problems,
        },
        crate::package::PackageError::Install(m) => Fail::Io(m),
        crate::package::PackageError::Io(m) => Fail::Io(m),
    }
}

fn cmd_add(
    package: Option<&str>,
    from: Option<&str>,
    tag: Option<&str>,
    no_install: bool,
    dry_run: bool,
    force: bool,
) -> CmdResult {
    let opts = crate::package::MergeOptions {
        check_only: dry_run,
        chain_install: !no_install,
        source_label: String::new(),
        tag: tag.map(String::from),
        digest: None,
        command: "add",
        force,
    };
    // 位置参数 = market 条目 URL（条目页安装命令的形态，与 --from 天然分流）
    if let Some(url) = package {
        if !url.contains("://") {
            return Err(Fail::Usage(crate::i18n::t("cli.add_shortname_unsupported")));
        }
        let report = crate::market::add_entry(Path::new("."), url, tag, &opts)
            .map_err(fail_from_package)?;
        return Ok(outcome_with_compat_warning(&report));
    }
    let Some(from) = from else {
        return Err(Fail::Usage(crate::i18n::t("cli.add_usage")));
    };
    // --from 本地归档（离线安装）：同一套安全解包与合入，不出现手工拷贝通道
    let path = Path::new(from);
    if path.is_file() && (from.ends_with(".tar.gz") || from.ends_with(".tgz")) {
        let mut archive_opts = opts;
        archive_opts.source_label = crate::paths::to_posix(path);
        let report = crate::market::add_archive(Path::new("."), path, &archive_opts)
            .map_err(fail_from_package)?;
        return Ok(outcome_with_compat_warning(&report));
    }
    let mut dir_opts = opts;
    dir_opts.source_label = from.to_string();
    let report =
        crate::package::add_package(Path::new("."), from, tag, &dir_opts).map_err(fail_from_package)?;
    Ok(ok_json(&report))
}

/// E-7：engine 范围不兼容时告警走 stderr（报告 JSON 在 stdout，含 compatibility_warning 字段）。
fn outcome_with_compat_warning(report: &Value) -> Outcome {
    let mut outcome = ok_json(report);
    if let Some(w) = report.get("compatibility_warning").and_then(|v| v.as_str()) {
        outcome.stderr = format!("{}\n", crate::i18n::msg("cli.warning_prefix", &[&w]));
    }
    outcome
}

fn cmd_push(target: &str, no_install: bool, dry_run: bool, force: bool) -> CmdResult {
    let source_label =
        crate::paths::to_posix(&std::env::current_dir().map_err(|e| Fail::Io(e.to_string()))?);
    let opts = crate::package::MergeOptions {
        check_only: dry_run,
        chain_install: !no_install,
        source_label,
        tag: None,
        digest: None,
        command: "push",
        force,
    };
    let report = crate::package::push_package(Path::new("."), Path::new(target), &opts)
        .map_err(fail_from_package)?;
    Ok(ok_json(&report))
}

fn cmd_auth_login(token: &str, market: Option<&str>) -> CmdResult {
    crate::market::auth_login(token, market).map(|v| ok_json(&v))
}

fn cmd_auth_logout(market: Option<&str>) -> CmdResult {
    crate::market::auth_logout(market).map(|v| ok_json(&v))
}

fn cmd_auth_status(market: Option<&str>) -> CmdResult {
    crate::market::auth_status(market).map(|v| ok_json(&v))
}

fn cmd_publish(
    dmaf_dir: &str,
    project: Option<&str>,
    version: Option<&str>,
    bump: bool,
    intro: Option<&str>,
    github_url: Option<&str>,
    market: Option<&str>,
    dry_run: bool,
) -> CmdResult {
    let args = crate::market::PublishArgs {
        dmaf_dir,
        project,
        version,
        bump,
        intro_file: intro,
        github_url,
        market,
        dry_run,
    };
    crate::market::publish(args).map(|v| ok_json(&v))
}

// ---------- new / init ----------

fn cmd_new(template: &str, dir: Option<&str>, force: bool) -> CmdResult {
    let target = PathBuf::from(dir.unwrap_or(template));
    let report = crate::scaffold::new_project(template, &target, force).map_err(|e| match e {
        crate::scaffold::ScaffoldError::Usage(m) => Fail::Usage(m),
        crate::scaffold::ScaffoldError::Io(m) => Fail::Io(m),
    })?;
    Ok(ok_json(&report))
}

// ---------- check ----------

/// 汇总一个 workflow 定义对 harness 的全部引用需求，逐条校验。
fn cmd_check(
    harness: &str,
    workflow: Option<&str>,
    workflow_dir: &str,
    session: Option<&str>,
    stages: Option<&str>,
    scopes: Option<&str>,
    artifacts_dir: &str,
) -> CmdResult {
    let rh = harness_root(harness).map_err(Fail::Usage)?;
    // 无任何输入选择器时默认标准布局（dmaf/workflows/{stages,scopes}）
    let (stages, scopes) = if workflow.is_none()
        && session.is_none()
        && stages.is_none()
        && scopes.is_none()
        && Path::new("dmaf/workflows/stages").is_dir()
        && Path::new("dmaf/workflows/scopes").is_dir()
    {
        (Some("dmaf/workflows/stages"), Some("dmaf/workflows/scopes"))
    } else {
        (stages, scopes)
    };
    // 输入源：--stages/--scopes（stage 库编译路径）、显式 --workflow，或 --session 自举
    let wf = if let (Some(stages_dir), Some(scopes_dir)) = (stages, scopes) {
        crate::compile::compile(Path::new(&stages_dir), Path::new(&scopes_dir), None)
            .map_err(|e: crate::compile::CompileError| Fail::Schema {
                message: crate::i18n::t("cli.stage_scope_compile_failed"),
                problems: e.problems,
            })?
            .workflow
    } else {
        match (workflow, session) {
            (Some(w), _) => load_workflow(&resolve_workflow_path(w, workflow_dir))
                .map_err(|e: WorkflowError| Fail::Schema {
                    message: crate::i18n::t("cli.workflow_invalid"),
                    problems: e.problems,
                })?,
            (None, Some(s)) => {
                let (_, _, _, state) = load_state(s)?;
                load_workflow_for(&state, None, workflow_dir, artifacts_dir)?
            }
            _ => {
                return Err(Fail::Usage(crate::i18n::t("cli.check_input_required")));
            }
        }
    };
    let registry = load_registry(&rh.dir, rh.layout);
    let problems = validate_refs(&wf, &registry);
    if !problems.is_empty() {
        return Ok(fail_outcome(EXIT_SCHEMA, &crate::i18n::t("cli.check_refs_failed"), &problems));
    }
    Ok(ok_json(&json!({
        "ok": true,
        "harness": paths::to_posix(&rh.dir),
        "workflow_id": wf.id,
        "agents": wf.agents_referenced(),
        "skills": wf.skills_referenced(),
    })))
}

// ---------- 入口 ----------

pub fn run(argv: &[String]) -> Outcome {
    // locale 两阶段之第一阶段提前到 parse 之前：help/--version 与用法文案按 DMAF_LOCALE 取语；
    // start --locale 与 state::load 随后覆盖（第二阶段不变）。
    crate::i18n::preset();
    // clap 约定 argv[0] 为程序名，调用方传入的是纯参数，这里补一个占位名
    let cli = match Cli::try_parse_from(
        std::iter::once("dmaf".to_string()).chain(argv.iter().cloned()),
    ) {
        Ok(cli) => cli,
        Err(e) => {
            return match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Outcome {
                    code: EXIT_OK,
                    stdout: e.to_string(),
                    stderr: String::new(),
                },
                _ => Outcome { code: EXIT_USAGE, stdout: String::new(), stderr: format!("{e}\n") },
            };
        }
    };
    let result: CmdResult = match cli.command {
        Commands::Start { workflow, scope, vars, task, locale, output_root, workflow_dir, artifacts, no_hooks } => {
            crate::hooks::set_disabled(no_hooks);
            // locale 两阶段之第二阶段：--locale > env（run() 已预设）；烘焙值 = 生效 locale。
            if let Some(l) = locale.as_deref() {
                crate::i18n::set(crate::i18n::Locale::from_str(l));
            }
            let baked_locale = crate::i18n::current().as_str().to_string();
            cmd_start(
                workflow.as_deref(),
                scope.as_deref(),
                &vars,
                task.as_deref(),
                &output_root,
                &workflow_dir,
                &artifacts,
                &baked_locale,
            )
        }
        Commands::Next { session, workflow, workflow_dir, artifacts, no_hooks } => {
            crate::hooks::set_disabled(no_hooks);
            cmd_next(&session, workflow.as_deref(), &workflow_dir, &artifacts)
        }
        Commands::Report { session, payload, workflow, workflow_dir, artifacts, no_hooks } => {
            crate::hooks::set_disabled(no_hooks);
            cmd_report(&session, &payload, workflow.as_deref(), &workflow_dir, &artifacts)
        }
        Commands::Validate { workflow, workflow_dir } => {
            cmd_validate(&workflow, &workflow_dir)
        }
        Commands::Inspect { session } => cmd_inspect(&session),
        Commands::VerifyAudit { session } => cmd_verify_audit(&session),
        Commands::Cancel { session, no_hooks } => {
            crate::hooks::set_disabled(no_hooks);
            cmd_cancel(&session)
        }
        Commands::Log { session, event_type, stage, limit } => {
            cmd_log(&session, event_type.as_deref(), stage.as_deref(), limit)
        }
        Commands::Replay { session, overwrite } => cmd_replay(&session, overwrite),
        Commands::Fork { session, seq, output_root, workflow, workflow_dir, artifacts, no_hooks } => {
            crate::hooks::set_disabled(no_hooks);
            cmd_fork(
                &session,
                seq,
                &output_root,
                workflow.as_deref(),
                &workflow_dir,
                &artifacts,
            )
        }
        Commands::VerifyDeterminism { session, workflow, artifacts, print_pairs } => {
            cmd_verify_determinism(&session, workflow.as_deref(), &artifacts, print_pairs)
        }
        Commands::Check { harness, workflow, workflow_dir, session, stages, scopes, artifacts } => {
            cmd_check(
                &harness,
                workflow.as_deref(),
                &workflow_dir,
                session.as_deref(),
                stages.as_deref(),
                scopes.as_deref(),
                &artifacts,
            )
        }
        Commands::New { template, dir, force } => cmd_new(&template, dir.as_deref(), force),
        Commands::Doctor { workflow_dir, artifacts, harness } => {
            cmd_doctor(&workflow_dir, &artifacts, harness.as_deref())
        }
        Commands::Compile { stages, scopes, out, check, harness } => {
            cmd_compile(&stages, &scopes, &out, check, harness.as_deref())
        }
        Commands::Install { harness, prune, check_only, force } => {
            cmd_install(harness.as_deref(), prune, check_only, force)
        }
        Commands::Add { package, from, tag, no_install, dry_run, force } => {
            cmd_add(package.as_deref(), from.as_deref(), tag.as_deref(), no_install, dry_run, force)
        }
        Commands::Push { target, no_install, dry_run, force } => {
            cmd_push(&target, no_install, dry_run, force)
        }
        Commands::Auth { action } => match action {
            AuthAction::Login { token, market } => cmd_auth_login(&token, market.as_deref()),
            AuthAction::Logout { market } => cmd_auth_logout(market.as_deref()),
            AuthAction::Status { market } => cmd_auth_status(market.as_deref()),
        },
        Commands::Publish { dmaf_dir, project, version, bump, intro, github_url, market, dry_run } => {
            cmd_publish(
                &dmaf_dir,
                project.as_deref(),
                version.as_deref(),
                bump,
                intro.as_deref(),
                github_url.as_deref(),
                market.as_deref(),
                dry_run,
            )
        }
    };
    match result {
        Ok(outcome) => outcome,
        Err(fail) => fail.to_outcome(),
    }
}
