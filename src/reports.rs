//! report 处理：schema 校验、产出物确定性校验、状态转移与审计事件生成。
//!
//! Audit-first：每个事件的分片先落盘，状态文件最后统一保存；
//! 分片携带 from/to 与全部回放所需字段，哈希链逐事件衔接。

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};

use crate::audit::AuditTrail;
use crate::paths;
use crate::router;
use crate::state::{self, ActionRecord, ReportEntry, StageState, WorkflowState};
use crate::workflow::{artifact_index, resolve_agent_paths, Workflow};

pub const ACTION_STATUS: &[&str] = &["success", "failed", "skipped"];

/// report 处理错误：被拒（状态不变）或状态冲突。
#[derive(Debug, Clone)]
pub enum ApplyError {
    Rejected(Vec<String>),
    Conflict(String),
}

/// payload 不合法（schema/引用），状态不变，可修正后重报。→ 退出码 3
#[derive(Debug, Clone)]
pub struct ReportRejected(pub Vec<String>);

pub struct EngineContext<'a> {
    pub workflow: &'a Workflow,
    pub state: &'a mut WorkflowState,
    /// 会话目录的文件系统路径（相对 cwd 或绝对）
    pub session_path: &'a Path,
    pub state_path: &'a Path,
    pub audit: AuditTrail,
    pub now: DateTime<FixedOffset>,
}

pub fn validate_payload_schema(payload: &Value) -> Result<(), ReportRejected> {
    crate::schema_check::validate_report(payload)
        .map_err(|msg| ReportRejected(vec![crate::i18n::msg("reports.payload_schema_fail", &[&msg])]))
}

/// 逐事件衔接哈希链：before=上一事件后的状态哈希，after=本次变更后的状态哈希。
struct Chain {
    prev_hash: String,
}

impl Chain {
    fn new(state: &WorkflowState) -> Self {
        Chain { prev_hash: state::canonical_hash(state) }
    }
}

fn emit(
    ctx: &mut EngineContext,
    chain: &mut Chain,
    event_type: &str,
    stage_id: Option<&str>,
    action_id: Option<&str>,
    payload: Value,
) -> Result<(), String> {
    let after = state::canonical_hash(ctx.state);
    let shard = ctx.audit.append(
        &ctx.state.workflow_id,
        event_type,
        stage_id,
        action_id,
        payload,
        &chain.prev_hash,
        &after,
        &paths::format_ts(&ctx.now),
    )?;
    chain.prev_hash = after;
    ctx.state.seq = shard["seq"].as_u64().unwrap_or(0);
    crate::hooks::notify(&shard, &ctx.state.session_dir);
    Ok(())
}

fn expected_outputs(
    workflow: &Workflow,
    st: &StageState,
    agent: &str,
    vars: &BTreeMap<String, String>,
) -> Vec<String> {
    let wf_stage = workflow.stage(&st.id).expect("状态中的 stage 必在定义内");
    let r = wf_stage
        .agents
        .iter()
        .find(|r| r.agent == agent)
        .expect("action 的 agent 必在定义内");
    let artifacts = artifact_index(workflow, vars);
    resolve_agent_paths(workflow, wf_stage, r, vars, &artifacts).outputs
}

fn json_str_or_empty(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

pub fn apply_action_report(
    ctx: &mut EngineContext,
    payload: &Value,
) -> Result<Value, ApplyError> {
    let action_id = payload["action_id"].as_str().unwrap_or_default().to_string();
    let found = ctx
        .state
        .find_action(&action_id)
        .map(|(s, a)| (s.id.clone(), a.clone()));
    let Some((stage_id, action)) = found else {
        return Err(ApplyError::Conflict(crate::i18n::msg(
            "reports.action_not_found",
            &[&action_id],
        )));
    };
    {
        let st = ctx.state.stage(&stage_id).unwrap();
        let act = st.action(&action_id).unwrap();
        if act.status != "dispatched" {
            return Err(ApplyError::Conflict(crate::i18n::msg(
                "reports.action_already_final",
                &[&act.status, &action_id],
            )));
        }
    }
    let wf_stage = ctx
        .workflow
        .stage(&stage_id)
        .expect("状态中的 stage 必在定义内")
        .clone();
    let r = wf_stage
        .agents
        .iter()
        .find(|r| r.agent == action.agent)
        .expect("action 的 agent 必在定义内")
        .clone();

    let status = match payload.get("status") {
        None => "success".to_string(), // Python: payload.get("status", "success")
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => String::new(),
        Some(other) => other.to_string(),
    };
    if !ACTION_STATUS.contains(&status.as_str()) {
        return Err(ApplyError::Rejected(vec![crate::i18n::msg(
            "reports.status_invalid",
            &[&crate::py_list(&["failed".to_string(), "success".to_string()])],
        )]));
    }

    let vars: BTreeMap<String, String> = router::effective_vars(ctx.state);
    let expected = expected_outputs(
        ctx.workflow,
        ctx.state.stage(&stage_id).unwrap(),
        &action.agent,
        &vars,
    );
    let missing: Vec<String> = expected
        .iter()
        .filter(|p| !Path::new(p.as_str()).is_file())
        .cloned()
        .collect();
    let reason = json_str_or_empty(payload.get("reason"));
    if status == "skipped" && reason.trim().is_empty() {
        return Err(ApplyError::Rejected(vec![
            crate::i18n::t("reports.skipped_needs_reason"),
        ]));
    }

    // skipped 与产出物校验无关：agent 判定不适用即跳过
    let effective = if status == "skipped" {
        "skipped"
    } else if status == "failed" || !missing.is_empty() {
        "failed"
    } else {
        "success"
    }
    .to_string();

    let summary = if status == "skipped" {
        reason.clone()
    } else {
        json_str_or_empty(payload.get("summary"))
    };
    let mut error = json_str_or_empty(payload.get("error_message"));
    if !missing.is_empty() {
        if !error.is_empty() {
            error.push_str("; ");
        }
        error.push_str(&crate::i18n::msg("reports.missing_outputs", &[&missing.join(", ")]));
    }

    let mut chain = Chain::new(ctx.state);
    let status_before = ctx.state.stage(&stage_id).unwrap().status.clone();
    let max_attempts = ctx.state.stage(&stage_id).unwrap().max_attempts;

    // 变更：记录 report 结果
    ctx.state.reports.insert(
        action_id.clone(),
        ReportEntry {
            status: effective.clone(),
            summary: summary.clone(),
            error: error.clone(),
        },
    );
    ctx.state
        .stage_mut(&stage_id)
        .unwrap()
        .action_mut(&action_id)
        .unwrap()
        .status = effective.clone();

    macro_rules! emit_reported {
        ($stage_to:expr) => {
            emit(
                ctx,
                &mut chain,
                "action.reported",
                Some(&stage_id),
                Some(&action_id),
                json!({
                    "from": status_before,
                    "to": $stage_to,
                    "action_id": action_id,
                    "agent": action.agent,
                    "attempt": action.attempt,
                    "status": status,
                    "effective_status": effective,
                    "summary": summary,
                    "error": error,
                    "expected_outputs": expected,
                    "missing_outputs": missing,
                }),
            )
            .map_err(ApplyError::Conflict)
        };
    }

    let mut scheduled_retry: Option<String> = None;
    if effective == "failed" {
        // max_attempts 是初始尝试之后的重试上限（max_attempts: 1 = 最多重试 1 次，共 2 次尝试）
        if action.attempt <= max_attempts {
            emit_reported!("in-progress")?;
            let retry_payload = router::build_action_payload(
                ctx.workflow,
                ctx.state,
                &wf_stage,
                &r,
                action.attempt + 1,
            )
            .map_err(ApplyError::Conflict)?;
            let new_action_id =
                retry_payload["action_id"].as_str().unwrap_or_default().to_string();
            {
                let st = ctx.state.stage_mut(&stage_id).unwrap();
                st.actions.push(ActionRecord {
                    action_id: new_action_id.clone(),
                    agent: r.agent.clone(),
                    attempt: action.attempt + 1,
                    status: "dispatched".into(),
                });
                st.attempt = action.attempt + 1;
            }
            scheduled_retry = Some(new_action_id.clone());
            let attempt = ctx.state.stage(&stage_id).unwrap().attempt;
            emit(
                ctx,
                &mut chain,
                "retry.scheduled",
                Some(&stage_id),
                Some(&action_id),
                json!({
                    "from_action_id": action_id,
                    "new_action_id": new_action_id,
                    "agent": r.agent,
                    "attempt": attempt,
                    "max_attempts": max_attempts,
                }),
            )
            .map_err(ApplyError::Conflict)?;
        } else {
            // 耗尽：abort 立即终态；continue 时若兄弟 action 仍在途，
            // stage 保持 in-progress，推迟到最后一个 action 聚合
            let abort = wf_stage.retry.on_exhausted == "abort_workflow";
            {
                let st = ctx.state.stage_mut(&stage_id).unwrap();
                if abort || st.unresolved().is_empty() {
                    if st.status != "failed" {
                        state::transition(st, "failed")
                            .map_err(|e| ApplyError::Conflict(e.0))?;
                    }
                }
            }
            let cur_status = ctx.state.stage(&stage_id).unwrap().status.clone();
            emit_reported!(cur_status)?;
            if abort && ctx.state.status == "running" {
                let reason = if !error.is_empty() {
                    error.clone()
                } else {
                    crate::i18n::msg("reports.retry_exhausted", &[&stage_id])
                };
                ctx.state.status = "failed".into();
                ctx.state.failure = Some(state::Failure {
                    stage: stage_id.clone(),
                    reason: reason.clone(),
                });
                emit(
                    ctx,
                    &mut chain,
                    "workflow.failed",
                    Some(&stage_id),
                    Some(&action_id),
                    json!({"stage": stage_id, "reason": reason}),
                )
                .map_err(ApplyError::Conflict)?;
            }
        }
    } else {
        let stage_to = {
            let st = ctx.state.stage(&stage_id).unwrap();
            if !st.unresolved().is_empty() {
                "in-progress".to_string()
            } else {
                // 聚合以每个 agent 最后一次尝试为准：重试成功即视为成功
                let mut latest: BTreeMap<String, &ActionRecord> = BTreeMap::new();
                for a in &st.actions {
                    latest.insert(a.agent.clone(), a);
                }
                // skipped 视为已消解（不适用≠失败）；仅 failed 聚合为失败
                if latest.values().any(|a| a.status == "failed") {
                    "failed".to_string() // on_exhausted=continue：带失败聚合
                } else if latest.values().all(|a| a.status == "skipped") {
                    "skipped".to_string() // 全部工作者判定不适用 → stage 跳过
                } else if wf_stage.approval.required {
                    "awaiting-approval".to_string()
                } else {
                    "completed".to_string()
                }
            }
        };
        {
            let st = ctx.state.stage_mut(&stage_id).unwrap();
            if st.status != stage_to {
                state::transition(st, &stage_to).map_err(|e| ApplyError::Conflict(e.0))?;
            }
        }
        let cur_status = ctx.state.stage(&stage_id).unwrap().status.clone();
        emit_reported!(cur_status)?;
        if cur_status == "completed" {
            finish(ctx, &mut chain).map_err(ApplyError::Conflict)?;
        }
        // M3 传递跳过：硬输入生产者已跳过的 pending stage 逐级跳过（确定性拓扑序，逐级审计）
        if cur_status == "skipped" {
            cascade_transitive_skip(ctx, &mut chain).map_err(ApplyError::Conflict)?;
        }
    }

    Ok(json!({
        "accepted": true,
        "action_id": action_id,
        "stage": stage_id,
        "stage_state": ctx.state.stage(&stage_id).unwrap().status,
        "action_status": effective,
        "attempt": action.attempt,
        "missing_outputs": missing,
        "scheduled_retry": scheduled_retry,
    }))
}

pub fn apply_approval_report(
    ctx: &mut EngineContext,
    payload: &Value,
) -> Result<Value, ApplyError> {
    let gate = payload["gate"].as_str().unwrap_or_default().to_string();
    let decision = payload["decision"].as_str().unwrap_or_default().to_string();
    let stage_id = {
        let found = ctx
            .state
            .stages
            .iter()
            .find(|s| {
                s.status == "awaiting-approval"
                    && ctx
                        .workflow
                        .stage(&s.id)
                        .map(|ws| ws.approval.gate == gate)
                        .unwrap_or(false)
            })
            .map(|s| s.id.clone());
        let Some(sid) = found else {
            return Err(ApplyError::Conflict(crate::i18n::msg(
                "reports.gate_not_awaiting",
                &[&gate],
            )));
        };
        sid
    };
    if decision != "granted" && decision != "rejected" {
        return Err(ApplyError::Rejected(vec![
            crate::i18n::t("reports.decision_invalid"),
        ]));
    }

    let note = json_str_or_empty(payload.get("note"));
    let mut chain = Chain::new(ctx.state);

    ctx.state.reports.insert(
        format!("approval:{gate}"),
        ReportEntry {
            status: decision.clone(),
            summary: note.clone(),
            error: String::new(),
        },
    );

    // M4a：决策来源可追溯——分片记录评审者（人批时为 null）
    let reviewer = ctx
        .workflow
        .stage(&stage_id)
        .and_then(|ws| ws.approval.reviewer.clone());
    let reviewer_value = reviewer
        .as_ref()
        .map(|r| json!(r))
        .unwrap_or(Value::Null);

    if decision == "granted" {
        {
            let st = ctx.state.stage_mut(&stage_id).unwrap();
            state::transition(st, "completed").map_err(|e| ApplyError::Conflict(e.0))?;
        }
        emit(
            ctx,
            &mut chain,
            "approval.granted",
            Some(&stage_id),
            None,
            json!({
                "gate": gate,
                "from": "awaiting-approval",
                "to": "completed",
                "note": note,
                "reviewer": reviewer_value,
            }),
        )
        .map_err(ApplyError::Conflict)?;
        finish(ctx, &mut chain).map_err(ApplyError::Conflict)?;
    } else {
        {
            let st = ctx.state.stage_mut(&stage_id).unwrap();
            state::transition(st, "revising").map_err(|e| ApplyError::Conflict(e.0))?;
        }
        emit(
            ctx,
            &mut chain,
            "approval.rejected",
            Some(&stage_id),
            None,
            json!({
                "gate": gate,
                "from": "awaiting-approval",
                "to": "revising",
                "note": note,
                "reviewer": reviewer_value,
            }),
        )
        .map_err(ApplyError::Conflict)?;
    }

    Ok(json!({
        "accepted": true,
        "stage": stage_id,
        "stage_state": ctx.state.stage(&stage_id).unwrap().status,
        "decision": decision,
    }))
}

/// 传递跳过级联：按拓扑序反复扫描 pending stage，
/// 硬输入的全部生产者均已 skipped 且自身 on_skip != run → 跳过并逐级审计。
/// 每个级联跳过复用 action.reported 事件（action_id = null，回放加一支）。
fn cascade_transitive_skip(ctx: &mut EngineContext, chain: &mut Chain) -> Result<(), String> {
    loop {
        let mut skipped_any = false;
        let stage_ids: Vec<String> = ctx.state.stages.iter().map(|s| s.id.clone()).collect();
        for sid in &stage_ids {
            let (st_status, st_on_skip) = {
                let st = ctx.state.stage(sid).unwrap();
                (st.status.clone(), ctx.workflow.stage(sid).map(|w| w.on_skip.clone()).unwrap_or_default())
            };
            if st_status != "pending" {
                continue;
            }
            let wf_stage = match ctx.workflow.stage(sid) {
                Some(w) => w,
                None => continue,
            };
            if st_on_skip == "run" {
                continue;
            }
            // 硬输入的生产者全部已跳过才级联（软输入不阻塞；无硬输入不级联）
            let hard_inputs: Vec<&crate::workflow::InputRef> = wf_stage
                .agents
                .iter()
                .flat_map(|a| a.inputs.iter())
                .filter(|i| i.required)
                .collect();
            if hard_inputs.is_empty() {
                continue;
            }
            let all_skipped = hard_inputs.iter().all(|i| {
                let producers: Vec<String> = ctx
                    .workflow
                    .stages
                    .iter()
                    .filter(|ws| {
                        ws.agents.iter().any(|a| {
                            a.outputs.iter().any(|o| o == &i.artifact)
                        })
                    })
                    .map(|ws| ws.id.clone())
                    .collect();
                !producers.is_empty()
                    && producers.iter().all(|pid| {
                        ctx.state
                            .stage(pid)
                            .map(|ss| ss.status == "skipped")
                            .unwrap_or(false)
                    })
            });
            if !all_skipped {
                continue;
            }
            let st = ctx.state.stage_mut(sid).unwrap();
            state::transition(st, "skipped").map_err(|e| e.0)?;
            emit(
                ctx,
                chain,
                "action.reported",
                Some(sid),
                None,
                json!({
                    "from": "pending",
                    "to": "skipped",
                    "effective_status": "skipped",
                    "reason": crate::i18n::msg("reports.transitive_skip_reason", &[&st_on_skip]),
                }),
            )?;
            skipped_any = true;
        }
        if !skipped_any {
            return Ok(());
        }
    }
}

fn finish(ctx: &mut EngineContext, chain: &mut Chain) -> Result<(), String> {
    if ctx.state.all_terminal() && ctx.state.status == "running" {
        ctx.state.status = "completed".into();
        let stages: serde_json::Map<String, Value> = ctx
            .state
            .stages
            .iter()
            .map(|s| (s.id.clone(), json!(s.status)))
            .collect();
        emit(
            ctx,
            chain,
            "workflow.completed",
            None,
            None,
            json!({"stages": stages}),
        )?;
    }
    Ok(())
}

