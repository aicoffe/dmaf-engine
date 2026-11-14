//! next 指令计算：纯函数 (workflow, state, registry) -> instruction。
//!
//! 不读时钟、不做 IO —— 相同输入必得逐字节相同的指令（确定性回归测试的锚点）。
//! 副作用（建 action 记录、状态转移、审计分片）以 effects 返回，由 cli 落盘。
//!
//! 一致性规则：
//! - 重试记录在 report 失败时即创建并由 retry.scheduled 审计（含 new_action_id），
//!   next 只是无副作用地重发；因此审计回放可完整重建 action 清单。
//! - 修订轮（审批拒绝后）走 revising 分支，产生新的 dispatch effect 与
//!   action.dispatched 事件（from: revising）。

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::state::{ActionRecord, StageState, WorkflowState};
use crate::workflow::{
    format_or_panic, artifact_index, render_completion, resolve_agent_paths, Stage, Workflow,
};

/// depends_on 的"已终结"判定：completed / failed / skipped。
/// skipped 是已消解终态（与 `WorkflowState::all_terminal` 一致）——依赖它的下游照常派发，
/// 其产物按编译期约束只能以软输入引用（缺席进 missing_inputs 提示），硬输入链由传递跳过处理。
pub const TERMINAL_STAGE: &[&str] = &["completed", "failed", "skipped"];

#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub op: String, // dispatch_stage
    pub stage_id: String,
    pub new_actions: Vec<Value>,
    pub transition_to: Option<String>, // None = 状态不变
}

pub fn session_id_of(state: &WorkflowState) -> String {
    state
        .session_dir
        .rsplit('/')
        .next()
        .unwrap_or(&state.session_dir)
        .to_string()
}

/// 渲染用变量全集：会话 vars ∪ 活绑定的 `SESSION_ID`。
/// `SESSION_ID` 恒等于当前会话 ID（session_dir 基名），不取 start 时烘焙进 vars 的存量值——
/// fork 校正 session_dir 后产物桶随之切到新会话，分支之间不再互写同一路径。
/// session_dir 是状态的一部分，纯函数性质不变。
pub fn effective_vars(state: &WorkflowState) -> BTreeMap<String, String> {
    let mut vars = state.vars.clone();
    vars.insert("SESSION_ID".to_string(), session_id_of(state));
    vars
}

pub fn action_id_for(session_id: &str, stage: &Stage, agent: &str, attempt: u64) -> String {
    format!("act-{session_id}-{}-{}-{attempt:03}", stage.id, agent)
}

fn is_terminal(status: &str) -> bool {
    TERMINAL_STAGE.contains(&status)
}

fn deps_terminal(_workflow: &Workflow, state: &WorkflowState, stage: &Stage) -> bool {
    stage.depends_on.iter().all(|dep| {
        state
            .stage(dep)
            .map(|s| is_terminal(&s.status))
            .unwrap_or(false)
    })
}

/// 派发断言（纯防御）：硬输入的成员生产者必已终结。正常流程由 deps_terminal
/// （数据依赖边 = 硬输入生产者，编译期推导）保证；违反说明产物与状态不一致
/// （如产物被手改），引擎显式报错而非静默派发缺料 stage。
fn assert_hard_inputs_ready(
    workflow: &Workflow,
    state: &WorkflowState,
    stage: &Stage,
) -> Result<(), String> {
    for r in &stage.agents {
        for inp in &r.inputs {
            if !inp.required {
                continue;
            }
            for ws in &workflow.stages {
                if ws.id == stage.id {
                    continue;
                }
                let produces = ws
                    .agents
                    .iter()
                    .any(|a| a.outputs.iter().any(|o| o == &inp.artifact));
                if !produces {
                    continue;
                }
                let ready = state
                    .stage(&ws.id)
                    .map(|s| is_terminal(&s.status))
                    .unwrap_or(false);
                if !ready {
                    return Err(crate::i18n::msg(
                        "router.hard_input_producer_not_terminal",
                        &[&stage.id, &inp.artifact, &ws.id],
                    ));
                }
            }
        }
    }
    Ok(())
}

fn last_rejection_note(state: &WorkflowState, stage: &Stage) -> String {
    state
        .reports
        .get(&format!("approval:{}", stage.approval.gate))
        .map(|e| e.summary.clone())
        .unwrap_or_default()
}

/// 构造完整 action payload。action_id 由 (session, stage, agent, attempt) 决定，可重现。
pub fn build_action_payload(
    workflow: &Workflow,
    state: &WorkflowState,
    stage: &Stage,
    r: &crate::workflow::AgentRef,
    attempt: u64,
) -> Result<Value, String> {
    let vars: BTreeMap<String, String> = effective_vars(state);
    for name in &r.vars {
        if !vars.contains_key(name) {
            return Err(crate::i18n::msg("router.action_var_unresolvable", &[name]));
        }
    }
    let artifacts = artifact_index(workflow, &vars);
    let resolved =
        resolve_agent_paths(
                workflow,
                stage, r, &vars, &artifacts);
    let inputs: Vec<String> = resolved.inputs.iter().map(|i| i.path.clone()).collect();
    // M3：缺失输入 advisory——生产者 stage 已跳过/失败的输入（确定性，源自状态）
    let mut missing_inputs: Vec<String> = Vec::new();
    for i in &resolved.inputs {
        // 生产者按渲染后的完整路径匹配（outputs 模板渲染后与输入路径相同）
        let producers: Vec<&crate::workflow::Stage> = workflow
            .stages
            .iter()
            .filter(|ws| {
                ws.agents.iter().any(|a| {
                    a.outputs.iter().any(|o| crate::workflow::render_path(o, &vars) == i.artifact)
                })
            })
            .collect();
        if producers.is_empty() {
            continue; // 外部产物，无法从状态判断
        }
        let all_missing = producers.iter().all(|ps| {
            state
                .stage(&ps.id)
                .map(|ss| ss.status == "skipped" || ss.status == "failed")
                .unwrap_or(false)
        });
        if all_missing {
            missing_inputs.push(i.path.clone());
        }
    }
    let subset: BTreeMap<String, String> = r
        .vars
        .iter()
        .filter_map(|k| vars.get(k).map(|v| (k.clone(), v.clone())))
        .collect();
    let task_rendered = format_or_panic(&r.task_description, &vars);
    // 委托报文预构造：conductor 原样转发给 subagent，不再自行拼装。
    // vars 平铺 = 会话 vars（常量与保留变量注入后的全集）∪ AgentRef.vars 已解析集；
    // 协议字段后插入，同名 var 冲突时以协议字段为准。
    let mut delegation = serde_json::Map::new();
    for (k, v) in &vars {
        delegation.insert(k.clone(), json!(v));
    }
    delegation.insert("role".into(), json!("task_delegation"));
    delegation.insert("task_description".into(), json!(task_rendered));
    // 路径字段与 stage 定义同名（inputs/outputs），值为名称模板解析后的完整路径；
    // 恒在（无产出/输入为空数组），协议中唯一条件字段是 revision_feedback。
    delegation.insert("inputs".into(), json!(inputs));
    delegation.insert("outputs".into(), json!(resolved.outputs));
    let mut payload = json!({
        "action_id": action_id_for(&session_id_of(state), stage, &r.agent, attempt),
        "agent": r.agent,
        "skills": &r.skills,
        "inputs": inputs,
        "watches": resolved.watches,
        "missing_inputs": missing_inputs,
        "outputs": resolved.outputs,
        "task_description": task_rendered,
        "attempt": attempt,
        "max_attempts": stage.retry.max_attempts,
        "vars": subset,
    });
    let st_status = state
        .stage(&stage.id)
        .map(|s| s.status.as_str())
        .unwrap_or_default();
    if st_status == "revising" {
        let note = last_rejection_note(state, stage);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("revision_feedback".into(), json!(note.clone()));
        }
        delegation.insert("revision_feedback".into(), json!(note));
    }
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("delegation".into(), Value::Object(delegation));
    }
    Ok(payload)
}

/// complete 指令的 payload.summary：最终交付报告的全部数据。
pub fn build_summary(workflow: &Workflow, state: &WorkflowState) -> Value {
    let vars: BTreeMap<String, String> = effective_vars(state);
    let artifacts = artifact_index(workflow, &vars);
    let mut stages = Vec::new();
    for (st, wf_stage) in state.stages.iter().zip(workflow.stages.iter()) {
        let mut actions = Vec::new();
        for a in &st.actions {
            let entry = state.reports.get(&a.action_id);
            actions.push(json!({
                "agent": a.agent,
                "status": a.status,
                "attempt": a.attempt,
                "summary": entry.map(|e| e.summary.clone()).unwrap_or_default(),
                "error": entry.map(|e| e.error.clone()).unwrap_or_default(),
            }));
        }
        let mut outputs: Vec<String> = Vec::new();
        for r in &wf_stage.agents {
            outputs.extend(
                resolve_agent_paths(workflow, wf_stage, r, &vars, &artifacts).outputs,
            );
        }
        stages.push(json!({
            "id": st.id, "status": st.status, "actions": actions, "outputs": outputs,
        }));
    }
    // 按 (stage, agent) 取最后一次尝试；仅保留最终失败的 agent（保持首次出现顺序）
    let mut latest_keys: Vec<(String, String)> = Vec::new();
    let mut latest: BTreeMap<(String, String), Value> = BTreeMap::new();
    for st in &state.stages {
        for a in &st.actions {
            let key = (st.id.clone(), a.agent.clone());
            if !latest_keys.contains(&key) {
                latest_keys.push(key.clone());
            }
            let error = state
                .reports
                .get(&a.action_id)
                .map(|e| e.error.clone())
                .unwrap_or_default();
            latest.insert(
                key,
                json!({"stage": st.id, "agent": a.agent, "status": a.status, "error": error}),
            );
        }
    }
    let failed_actions: Vec<Value> = latest_keys
        .iter()
        .filter_map(|k| latest.get(k))
        .filter(|v| v["status"] == "failed")
        .map(|v| {
            json!({"stage": v["stage"], "agent": v["agent"], "error": v["error"]})
        })
        .collect();
    json!({
        "workflow_id": workflow.id,
        "session_dir": state.session_dir,
        "vars": vars,
        "stages": stages,
        "failed_actions": failed_actions,
    })
}

/// complete 指令的 payload.completion_text：定义声明 completion_template 时由引擎渲染。
/// 占位符已在解析/编译期校验闭合，此处缺键/坏语法属引擎内部错误。
/// 无模板返回 None——指令省略该字段，conductor 退回展示 summary 结构化数据。
fn completion_text(workflow: &Workflow, state: &WorkflowState) -> Option<String> {
    let tpl = workflow.completion_template.as_deref()?;
    let vars: BTreeMap<String, String> = effective_vars(state);
    let artifacts = artifact_index(workflow, &vars);
    let lookup = |kind: &str, key: &str| -> Option<String> {
        match kind {
            "vars" => vars.get(key).cloned(),
            "outputs" => workflow.stage(key).map(|wf_stage| {
                let outs: Vec<String> = wf_stage
                    .agents
                    .iter()
                    .flat_map(|r| {
                        resolve_agent_paths(workflow, wf_stage, r, &vars, &artifacts).outputs
                    })
                    .collect();
                if outs.is_empty() {
                    String::new()
                } else {
                    outs.join(&crate::i18n::t("router.outputs_joiner"))
                }
            }),
            _ => None,
        }
    };
    Some(render_completion(tpl, &lookup).expect("completion_template 占位符已在定义期校验闭合"))
}

fn complete_instruction(workflow: &Workflow, state: &WorkflowState) -> Value {
    let mut payload = json!({"summary": build_summary(workflow, state)});
    if let Some(text) = completion_text(workflow, state) {
        payload["completion_text"] = json!(text);
    }
    json!({
        "workflow_id": workflow.id, "session_dir": state.session_dir,
        "instruction_id": iid(state), "type": "complete",
        "stage": Value::Null, "stage_state": Value::Null,
        "payload": payload,
    })
}

fn stage_dispatch_payload(wf_stage: &Stage, actions: Vec<Value>) -> Value {
    json!({
        "collaboration": wf_stage.collaboration,
        "actions": actions,
        "retry": {
            "max_attempts": wf_stage.retry.max_attempts,
            "on_exhausted": wf_stage.retry.on_exhausted,
        },
        "approval": {
            "required": wf_stage.approval.required,
            "gate": wf_stage.approval.gate,
        },
    })
}

fn iid(state: &WorkflowState) -> String {
    format!("ins-{}-{:06}", state.workflow_id, state.seq)
}

/// 返回 (instruction, effects)。effects 为空表示纯查询（resume/终态/重发）。
pub fn compute_next(
    workflow: &Workflow,
    state: &WorkflowState,
) -> Result<(Value, Vec<Effect>), String> {
    if state.status == "completed" {
        return Ok((complete_instruction(workflow, state), Vec::new()));
    }
    if state.status == "failed" {
        let failure = state.failure.clone().unwrap_or_default();
        return Ok((
            json!({
                "workflow_id": workflow.id, "session_dir": state.session_dir,
                "instruction_id": iid(state), "type": "fail",
                "stage": state.failure.as_ref().map(|f| json!(f.stage)).unwrap_or(Value::Null),
                "stage_state": "failed",
                "payload": {"reason": failure.reason},
            }),
            Vec::new(),
        ));
    }
    if state.status == "cancelled" {
        return Ok((
            json!({
                "workflow_id": workflow.id, "session_dir": state.session_dir,
                "instruction_id": iid(state), "type": "fail",
                "stage": Value::Null, "stage_state": "cancelled",
                "payload": {"reason": crate::i18n::t("router.cancelled_reason")},
            }),
            Vec::new(),
        ));
    }

    // 1. 审批门：有 stage 停在 awaiting-approval，就只等批复
    if let Some(awaiting) = state.awaiting_stage() {
        let wf_stage = workflow.stage(&awaiting.id).expect("状态中的 stage 必在定义内");
        let vars: BTreeMap<String, String> = effective_vars(state);
        let artifacts = artifact_index(workflow, &vars);
        let mut outputs: Vec<String> = Vec::new();
        for r in &wf_stage.agents {
            outputs.extend(
                resolve_agent_paths(workflow, wf_stage, r, &vars, &artifacts).outputs,
            );
        }
        let summaries: Vec<Value> = awaiting
            .actions
            .iter()
            .map(|a| {
                let summary = state
                    .reports
                    .get(&a.action_id)
                    .map(|e| e.summary.clone())
                    .unwrap_or_default();
                json!({"agent": a.agent, "summary": summary})
            })
            .collect();
        // M4a：一等评审者——派发信息随门指令下发（结论仍以 approval report 落审计）
        let reviewer_section = wf_stage.approval.reviewer.as_ref().map(|r| {
            json!({
                "agent": r,
                "gate": wf_stage.approval.gate,
                "inputs": outputs,
            })
        });
        let mut payload = json!({
            "gate": wf_stage.approval.gate,
            "outputs": outputs,
            "summaries": summaries,
        });
        if let Some(rv) = reviewer_section {
            payload["reviewer"] = rv;
        }
        return Ok((
            json!({
                "workflow_id": workflow.id, "session_dir": state.session_dir,
                "instruction_id": iid(state), "type": "await_approval",
                "stage": awaiting.id, "stage_state": "awaiting-approval",
                "payload": payload,
            }),
            Vec::new(),
        ));
    }

    // 2. 依序列找第一个可推进的 stage（stages 顺序 = scope include_stages 序列，顺序即执行序）
    for wf_stage in &workflow.stages {
        let Some(st) = state.stage(&wf_stage.id) else {
            continue;
        };

        if st.status == "pending" && deps_terminal(workflow, state, wf_stage) {
            assert_hard_inputs_ready(workflow, state, wf_stage)?;
            let mut actions = Vec::new();
            for r in &wf_stage.agents {
                actions.push(build_action_payload(workflow, state, wf_stage, r, 1)?);
            }
            let instruction = json!({
                "workflow_id": workflow.id, "session_dir": state.session_dir,
                "instruction_id": iid(state), "type": "dispatch",
                "stage": wf_stage.id, "stage_state": "in-progress",
                "payload": stage_dispatch_payload(wf_stage, actions.clone()),
            });
            let effect = Effect {
                op: "dispatch_stage".into(),
                stage_id: wf_stage.id.clone(),
                new_actions: actions,
                transition_to: Some("in-progress".into()),
            };
            return Ok((instruction, vec![effect]));
        }

        if st.status == "revising" {
            assert_hard_inputs_ready(workflow, state, wf_stage)?;
            let mut actions = Vec::new();
            for r in &wf_stage.agents {
                let prev = st
                    .actions
                    .iter()
                    .filter(|a| a.agent == r.agent)
                    .map(|a| a.attempt)
                    .max()
                    .unwrap_or(0);
                actions.push(build_action_payload(
                    workflow, state, wf_stage, r, prev + 1,
                )?);
            }
            let instruction = json!({
                "workflow_id": workflow.id, "session_dir": state.session_dir,
                "instruction_id": iid(state), "type": "dispatch",
                "stage": wf_stage.id, "stage_state": "in-progress",
                "payload": stage_dispatch_payload(wf_stage, actions.clone()),
            });
            let effect = Effect {
                op: "dispatch_stage".into(),
                stage_id: wf_stage.id.clone(),
                new_actions: actions,
                transition_to: Some("in-progress".into()),
            };
            return Ok((instruction, vec![effect]));
        }

        if st.status == "in-progress" && !st.unresolved().is_empty() {
            let mut payload_actions = Vec::new();
            for a in st.unresolved() {
                let r = wf_stage
                    .agents
                    .iter()
                    .find(|r| r.agent == a.agent)
                    .expect("在途 action 的 agent 必在定义内");
                payload_actions.push(build_action_payload(
                    workflow, state, wf_stage, r, a.attempt,
                )?);
            }
            let is_retry = st.unresolved().iter().any(|a| a.attempt > 1);
            let instruction = json!({
                "workflow_id": workflow.id, "session_dir": state.session_dir,
                "instruction_id": iid(state),
                "type": if is_retry { "retry" } else { "dispatch" },
                "stage": wf_stage.id, "stage_state": "in-progress",
                "payload": stage_dispatch_payload(wf_stage, payload_actions),
            });
            return Ok((instruction, Vec::new())); // 重发/resume：无副作用
        }
    }

    // 3. 无可派发且无在途
    if state.all_terminal() {
        return Ok((complete_instruction(workflow, state), Vec::new()));
    }
    Ok((
        json!({
            "workflow_id": workflow.id, "session_dir": state.session_dir,
            "instruction_id": iid(state), "type": "fail",
            "stage": Value::Null, "stage_state": state.status,
            "payload": {"reason": crate::i18n::t("router.stalled_reason")},
        }),
        Vec::new(),
    ))
}

/// 从 effect 构造要追加的 ActionRecord（cli 落盘用）。
pub fn effect_action_records(effect: &Effect) -> Vec<ActionRecord> {
    effect
        .new_actions
        .iter()
        .map(|a| ActionRecord {
            action_id: a["action_id"].as_str().unwrap_or_default().to_string(),
            agent: a["agent"].as_str().unwrap_or_default().to_string(),
            attempt: a["attempt"].as_u64().unwrap_or(1),
            status: "dispatched".into(),
        })
        .collect()
}

/// 供测试/外部驱动：StageState 便捷构造。
pub fn stage_state_of(id: &str, max_attempts: u64) -> StageState {
    StageState::new(id, max_attempts)
}
