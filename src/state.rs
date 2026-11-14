//! dmaf-state.md —— Engine 独占的工作流状态文件。
//!
//! 格式：YAML front matter（标量与映射）+ 复选框正文（Stage/Action 清单）。
//! 正文是 Stage/Action 的唯一真相；front matter 承载 vars/reports/failure。
//! 本模块同时实现状态转移表与规范视图（canonical view，供哈希与回放比对）。

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::audit;
use crate::paths;

pub fn status_mark(status: &str) -> &'static str {
    match status {
        "pending" => "[ ]",
        "in-progress" => "[-]",
        "awaiting-approval" => "[?]",
        "revising" => "[R]",
        "completed" => "[x]",
        "failed" => "[F]",
        "cancelled" => "[C]",
        "skipped" => "[S]",
        _ => panic!("未知 stage 状态: {status}"),
    }
}

pub fn mark_status(mark: &str) -> Option<&'static str> {
    Some(match mark {
        "[ ]" => "pending",
        "[-]" => "in-progress",
        "[?]" => "awaiting-approval",
        "[R]" => "revising",
        "[x]" => "completed",
        "[F]" => "failed",
        "[C]" => "cancelled",
        "[S]" => "skipped",
        _ => return None,
    })
}

pub fn action_mark(status: &str) -> &'static str {
    match status {
        "dispatched" => "[-]",
        "success" => "[x]",
        "failed" => "[F]",
        "skipped" => "[S]",
        _ => panic!("未知 action 状态: {status}"),
    }
}

pub fn action_mark_status(mark: &str) -> Option<&'static str> {
    Some(match mark {
        "[-]" => "dispatched",
        "[x]" => "success",
        "[F]" => "failed",
        "[S]" => "skipped",
        _ => return None,
    })
}

/// Stage 状态转移表（《设计》第五节/第八节）。
fn allowed_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("pending", "in-progress")
            | ("revising", "in-progress")
            | ("in-progress", "awaiting-approval")
            | ("in-progress", "completed")
            | ("in-progress", "failed")
            | ("pending", "skipped")
            | ("in-progress", "skipped")
            | ("awaiting-approval", "completed")
            | ("awaiting-approval", "revising")
            | ("revising", "awaiting-approval")
            | ("pending", "cancelled")
            | ("in-progress", "cancelled")
            | ("awaiting-approval", "cancelled")
            | ("revising", "cancelled")
            | ("failed", "cancelled")
            | ("completed", "cancelled")
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionRecord {
    pub action_id: String,
    pub agent: String,
    pub attempt: u64,
    pub status: String, // dispatched | success | failed
}

#[derive(Debug, Clone, PartialEq)]
pub struct StageState {
    pub id: String,
    pub status: String,
    pub attempt: u64,
    pub max_attempts: u64,
    pub actions: Vec<ActionRecord>,
}

impl StageState {
    pub fn new(id: &str, max_attempts: u64) -> Self {
        StageState {
            id: id.to_string(),
            status: "pending".into(),
            attempt: 0,
            max_attempts,
            actions: Vec::new(),
        }
    }

    pub fn action(&self, action_id: &str) -> Option<&ActionRecord> {
        self.actions.iter().find(|a| a.action_id == action_id)
    }

    pub fn action_mut(&mut self, action_id: &str) -> Option<&mut ActionRecord> {
        self.actions.iter_mut().find(|a| a.action_id == action_id)
    }

    pub fn unresolved(&self) -> Vec<&ActionRecord> {
        self.actions.iter().filter(|a| a.status == "dispatched").collect()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Failure {
    pub stage: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReportEntry {
    pub status: String,
    pub summary: String,
    pub error: String,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowState {
    pub locale: String,     // 会话事实（zh|en）：start 按 --locale > env 烘焙，会话命令覆盖进程 locale；缺省/旧会话 = zh
    pub workflow_id: String,
    pub session_dir: String, // posix 路径，如 output/20261013-154500
    pub status: String,      // running | completed | failed | cancelled
    pub scope: String,       // scope 模式会话记录 scope 名；内联模式为空
    pub created_at: String,
    pub updated_at: String,
    pub seq: u64,
    pub vars: BTreeMap<String, String>,
    pub failure: Option<Failure>,
    pub reports: BTreeMap<String, ReportEntry>,
    pub stages: Vec<StageState>,
}

impl WorkflowState {
    pub fn stage(&self, stage_id: &str) -> Option<&StageState> {
        self.stages.iter().find(|s| s.id == stage_id)
    }

    pub fn stage_mut(&mut self, stage_id: &str) -> Option<&mut StageState> {
        self.stages.iter_mut().find(|s| s.id == stage_id)
    }

    pub fn find_action(&self, action_id: &str) -> Option<(&StageState, &ActionRecord)> {
        for s in &self.stages {
            if let Some(a) = s.action(action_id) {
                return Some((s, a));
            }
        }
        None
    }

    pub fn awaiting_stage(&self) -> Option<&StageState> {
        self.stages.iter().find(|s| s.status == "awaiting-approval")
    }

    pub fn all_terminal(&self) -> bool {
        self.stages
            .iter()
            .all(|s| matches!(s.status.as_str(), "completed" | "failed" | "skipped"))
    }
}

#[derive(Debug, Clone)]
pub struct StateError(pub String);

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn transition(stage: &mut StageState, to: &str) -> Result<(), StateError> {
    if !allowed_transition(&stage.status, to) {
        return Err(StateError(crate::i18n::msg(
            "state.illegal_transition",
            &[&stage.id, &stage.status, to],
        )));
    }
    stage.status = to.to_string();
    Ok(())
}

// ---------- 序列化 ----------

/// 以 YAML 兼容的带引号标量渲染字符串（JSON 双引号是合法的 YAML flow scalar）。
pub fn json_scalar(value: &str) -> String {
    serde_json::to_string(value).expect("字符串序列化失败")
}

pub fn render(state: &WorkflowState) -> String {
    let mut lines = vec![
        "---".to_string(),
        format!("workflow_id: {}", state.workflow_id),
        format!("session_dir: {}", state.session_dir),
        format!("status: {}", state.status),
        format!("locale: {}", state.locale),
    ];
    if !state.scope.is_empty() {
        lines.push(format!("scope: {}", state.scope));
    }
    lines.push(format!("created_at: {}", json_scalar(&state.created_at)));
    lines.push(format!("updated_at: {}", json_scalar(&state.updated_at)));
    lines.push(format!("seq: {}", state.seq));
    lines.push("vars:".to_string());
    for (k, v) in &state.vars {
        lines.push(format!("  {k}: {}", json_scalar(v)));
    }
    if let Some(failure) = &state.failure {
        lines.push("failure:".to_string());
        lines.push(format!("  stage: {}", failure.stage));
        lines.push(format!("  reason: {}", json_scalar(&failure.reason)));
    }
    if !state.reports.is_empty() {
        lines.push("reports:".to_string());
        for (key, entry) in &state.reports {
            lines.push(format!("  {}:", json_scalar(key)));
            lines.push(format!("    status: {}", entry.status));
            lines.push(format!("    summary: {}", json_scalar(&entry.summary)));
            if !entry.error.is_empty() {
                lines.push(format!("    error: {}", json_scalar(&entry.error)));
            }
        }
    }
    lines.push("---".to_string());
    lines.push(String::new());
    lines.push("# Stages".to_string());
    lines.push(String::new());
    for s in &state.stages {
        lines.push(format!(
            "- {} {} — attempts: {}/{}",
            status_mark(&s.status),
            s.id,
            s.attempt,
            s.max_attempts
        ));
        for a in &s.actions {
            lines.push(format!(
                "  - {} {}: {} attempt {} {}",
                action_mark(&a.status),
                a.action_id,
                a.agent,
                a.attempt,
                a.status
            ));
        }
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

// ---------- 解析 ----------

/// YAML 标量 → 字符串（对应 Python str() 的常用路径）。
fn ystr(v: Option<&serde_yaml::Value>) -> String {
    match v {
        None | Some(serde_yaml::Value::Null) => String::new(),
        Some(serde_yaml::Value::String(s)) => s.clone(),
        Some(serde_yaml::Value::Bool(b)) => b.to_string(),
        Some(serde_yaml::Value::Number(n)) => n.to_string(),
        Some(other) => {
            serde_yaml::to_string(other).unwrap_or_default().trim().to_string()
        }
    }
}

/// 解析复选框行首：`<indent>- [M] <rest>`，返回 (标记字符, 余下内容)。
fn split_marked_line<'a>(line: &'a str, indent: &str) -> Option<(char, &'a str)> {
    let rest = line.strip_prefix(indent)?;
    let rest = rest.strip_prefix("- [")?;
    let c = rest.chars().next()?;
    if !matches!(c, ' ' | '-' | '?' | 'R' | 'x' | 'F' | 'C' | 'S') {
        return None;
    }
    let rest = rest.strip_prefix(c)?;
    let rest = rest.strip_prefix("] ")?;
    Some((c, rest))
}

pub fn parse(text: &str) -> Result<WorkflowState, StateError> {
    let (front_raw, body) = text
        .split_once("\n---\n")
        .ok_or_else(|| StateError(crate::i18n::t("state.frontmatter_separator_missing")))?;
    let front = front_raw
        .strip_prefix("---\n")
        .ok_or_else(|| StateError(crate::i18n::t("state.frontmatter_start_missing")))?;
    let meta: serde_yaml::Value = serde_yaml::from_str(front)
        .map_err(|e| StateError(crate::i18n::msg("state.frontmatter_yaml_parse_failed", &[&e.to_string()])))?;
    let meta: serde_yaml::Mapping = meta.as_mapping().cloned().unwrap_or_default();

    let get = |k: &str| meta.get(serde_yaml::Value::String(k.into()));

    let mut vars = BTreeMap::new();
    if let Some(m) = get("vars").and_then(|v| v.as_mapping()) {
        for (k, v) in m {
            vars.insert(ystr(Some(k)), ystr(Some(v)));
        }
    }
    let mut reports = BTreeMap::new();
    if let Some(m) = get("reports").and_then(|v| v.as_mapping()) {
        for (k, v) in m {
            let entry = v.as_mapping();
            let g = |kk: &str| entry.and_then(|e| e.get(serde_yaml::Value::String(kk.into())));
            reports.insert(
                ystr(Some(k)),
                ReportEntry {
                    status: ystr(g("status")),
                    summary: ystr(g("summary")),
                    error: ystr(g("error")),
                },
            );
        }
    }
    let failure = get("failure").and_then(|v| v.as_mapping()).map(|f| {
        let g = |k: &str| f.get(serde_yaml::Value::String(k.into()));
        Failure { stage: ystr(g("stage")), reason: ystr(g("reason")) }
    });

    let mut state = WorkflowState {
        locale: {
            let s = ystr(get("locale"));
            if s.is_empty() { "zh".to_string() } else { s }
        },
        workflow_id: ystr(get("workflow_id")),
        session_dir: ystr(get("session_dir")),
        status: {
            let s = ystr(get("status"));
            if s.is_empty() { "running".to_string() } else { s }
        },
        scope: ystr(get("scope")),
        created_at: ystr(get("created_at")),
        updated_at: ystr(get("updated_at")),
        seq: get("seq").and_then(|v| v.as_u64()).unwrap_or(0),
        vars,
        failure,
        reports,
        stages: Vec::new(),
    };

    let mut cur: Option<usize> = None;
    for line in body.split('\n') {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        // Stage 行：`- [M] <id> — attempts: <n>/<m>`
        if let Some((mark_char, rest)) = split_marked_line(line, "") {
            let mark = format!("[{mark_char}]");
            let status = mark_status(&mark)
                .ok_or_else(|| StateError(crate::i18n::msg("state.unparseable_state_line", &[&format!("{line:?}")])))?;
            let (sid, counts) = rest
                .split_once(" — attempts: ")
                .ok_or_else(|| StateError(crate::i18n::msg("state.unparseable_state_line", &[&format!("{line:?}")])))?;
            if sid.is_empty() || sid.chars().any(char::is_whitespace) {
                return Err(StateError(crate::i18n::msg(
                    "state.unparseable_state_line",
                    &[&format!("{line:?}")],
                )));
            }
            let (att, max) = counts
                .split_once('/')
                .ok_or_else(|| StateError(crate::i18n::msg("state.unparseable_state_line", &[&format!("{line:?}")])))?;
            let attempt: u64 = att
                .parse()
                .map_err(|_| StateError(crate::i18n::msg("state.unparseable_state_line", &[&format!("{line:?}")])))?;
            let max_attempts: u64 = max
                .parse()
                .map_err(|_| StateError(crate::i18n::msg("state.unparseable_state_line", &[&format!("{line:?}")])))?;
            state.stages.push(StageState {
                id: sid.to_string(),
                status: status.to_string(),
                attempt,
                max_attempts,
                actions: Vec::new(),
            });
            cur = Some(state.stages.len() - 1);
            continue;
        }
        // Action 行：`  - [M] <action_id>: <agent> attempt <n> <status>`
        if let Some((mark_char, rest)) = split_marked_line(line, "  ") {
            let bad = || StateError(crate::i18n::msg("state.unparseable_state_line", &[&format!("{line:?}")]));
            let mark = format!("[{mark_char}]");
            let (action_id, tail) = rest.split_once(": ").ok_or_else(bad)?;
            if action_id.is_empty() || action_id.chars().any(char::is_whitespace) {
                return Err(bad());
            }
            let parts: Vec<&str> = tail.split_whitespace().collect();
            if parts.len() != 4 || parts[1] != "attempt" {
                return Err(bad());
            }
            let attempt: u64 = parts[2].parse().map_err(|_| bad())?;
            let status = parts[3];
            if !matches!(status, "dispatched" | "success" | "failed" | "skipped") {
                return Err(bad());
            }
            let expected = action_mark_status(&mark).ok_or_else(bad)?;
            if expected != status {
                return Err(StateError(crate::i18n::msg(
                    "state.action_mark_status_mismatch",
                    &[action_id],
                )));
            }
            let idx = cur.ok_or_else(bad)?;
            state.stages[idx].actions.push(ActionRecord {
                action_id: action_id.to_string(),
                agent: parts[0].to_string(),
                attempt,
                status: status.to_string(),
            });
            continue;
        }
        return Err(StateError(crate::i18n::msg(
            "state.unparseable_state_line",
            &[&format!("{line:?}")],
        )));
    }
    Ok(state)
}

pub fn load(path: &Path) -> Result<WorkflowState, StateError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| StateError(crate::i18n::msg("state.state_file_read_failed", &[&e.to_string()])))?;
    let state = parse(&text)?;
    // 会话命令的 locale 覆盖点：加载成功即以会话 locale 取代进程预设（env）——确定性红线。
    crate::i18n::set(crate::i18n::Locale::from_str(&state.locale));
    Ok(state)
}

pub fn save(state: &mut WorkflowState, path: &Path, updated_at: &str) -> std::io::Result<()> {
    state.updated_at = updated_at.to_string();
    paths::atomic_write_text(path, &render(state))
}

// ---------- 规范视图与回放 ----------

pub fn canonical_view(state: &WorkflowState) -> Value {
    let vars: Map<String, Value> = state
        .vars
        .iter()
        .map(|(k, v)| (k.clone(), json!(v)))
        .collect();
    let reports: Map<String, Value> = state
        .reports
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                json!({"status": v.status, "summary": v.summary, "error": v.error}),
            )
        })
        .collect();
    let stages: Vec<Value> = state
        .stages
        .iter()
        .map(|s| {
            let actions: Vec<Value> = s
                .actions
                .iter()
                .map(|a| {
                    json!({"action_id": a.action_id, "agent": a.agent,
                           "attempt": a.attempt, "status": a.status})
                })
                .collect();
            json!({"id": s.id, "status": s.status, "attempt": s.attempt,
                   "max_attempts": s.max_attempts, "actions": actions})
        })
        .collect();
    json!({
        "status": state.status,
        "vars": vars,
        "failure": state.failure.as_ref().map(|f| json!({"stage": f.stage, "reason": f.reason})).unwrap_or(Value::Null),
        "reports": reports,
        "stages": stages,
    })
}

pub fn canonical_hash(state: &WorkflowState) -> String {
    audit::sha256_of(&canonical_view(state))
}

fn payload_str(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).map(|v| match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    })
}

fn payload_u64(payload: &Value, key: &str, default: u64) -> u64 {
    payload.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

/// 按审计分片顺序重建状态（verify-audit / replay / fork 用）。链或语义不一致即报错。
pub fn replay(shards: &[Value]) -> Result<WorkflowState, StateError> {
    let mut state = WorkflowState {
        locale: crate::i18n::current().as_str().to_string(),
        workflow_id: String::new(),
        session_dir: String::new(),
        status: "running".into(),
        ..Default::default()
    };
    for shard in shards {
        let event = shard
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let payload = shard.get("payload").cloned().unwrap_or(Value::Null);
        let empty = Value::Object(Default::default());
        let payload = if payload.is_null() { &empty } else { &payload };
        let stage_id = shard.get("stage_id").and_then(|v| v.as_str()).unwrap_or_default();
        let seq = shard.get("seq").and_then(|v| v.as_u64()).unwrap_or(0);
        match event {
            "workflow.started" => {
                state.workflow_id = shard
                    .get("workflow_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                state.session_dir = payload_str(payload, "session_dir").unwrap_or_default();
                state.vars.clear();
                if let Some(vars) = payload.get("vars").and_then(|v| v.as_object()) {
                    for (k, v) in vars {
                        let s = match v {
                            Value::String(s) => s.clone(),
                            Value::Null => String::new(),
                            other => other.to_string(),
                        };
                        state.vars.insert(k.clone(), s);
                    }
                }
                state.stages = payload
                    .get("stages")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|s| {
                                StageState::new(
                                    s.get("id").and_then(|v| v.as_str()).unwrap_or_default(),
                                    s.get("max_attempts").and_then(|v| v.as_u64()).unwrap_or(1),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
            }
            "action.dispatched" => {
                let from = payload_str(payload, "from").unwrap_or_default();
                let stage = state.stage_mut(stage_id);
                let stage = match stage {
                    Some(s) if s.status == from => s,
                    Some(s) => {
                        return Err(StateError(crate::i18n::msg(
                            "state.shard_dispatch_status_mismatch",
                            &[&seq.to_string(), &s.status, &from],
                        )))
                    }
                    None => {
                        return Err(StateError(crate::i18n::msg(
                            "state.shard_dispatch_status_mismatch",
                            &[&seq.to_string(), "None", &from],
                        )))
                    }
                };
                transition(stage, "in-progress")?;
                let attempt = payload_u64(payload, "attempt", 1);
                stage.attempt = stage.attempt.max(attempt);
                if let Some(actions) = payload.get("actions").and_then(|v| v.as_array()) {
                    for arec in actions {
                        stage.actions.push(ActionRecord {
                            action_id: arec
                                .get("action_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            agent: arec
                                .get("agent")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            attempt: arec.get("attempt").and_then(|v| v.as_u64()).unwrap_or(1),
                            status: "dispatched".into(),
                        });
                    }
                }
            }
            "retry.scheduled" => {
                let stage = state
                    .stage_mut(stage_id)
                    .ok_or_else(|| StateError(crate::i18n::msg("state.shard_retry_unknown_stage", &[&seq.to_string(), stage_id])))?;
                let attempt = payload_u64(payload, "attempt", 1);
                stage.attempt = stage.attempt.max(attempt);
                stage.actions.push(ActionRecord {
                    action_id: payload_str(payload, "new_action_id").unwrap_or_default(),
                    agent: payload_str(payload, "agent").unwrap_or_default(),
                    attempt,
                    status: "dispatched".into(),
                });
            }
            "action.reported" => {
                let from = payload_str(payload, "from").unwrap_or_default();
                let to = payload_str(payload, "to").unwrap_or_default();
                let action_id = payload_str(payload, "action_id").unwrap_or_default();
                let effective = payload_str(payload, "effective_status").unwrap_or_default();
                // 级联跳过分片：action_id = null，仅做 stage 级转移（无 action 可标记）
                if action_id.is_empty() {
                    let stage = state.stage_mut(stage_id);
                    let stage = match stage {
                        Some(s) if s.status == from => s,
                        Some(s) => {
                            return Err(StateError(crate::i18n::msg(
                                "state.shard_report_status_mismatch",
                                &[&seq.to_string(), &s.status, &from],
                            )))
                        }
                        None => {
                            return Err(StateError(crate::i18n::msg(
                                "state.shard_report_status_mismatch",
                                &[&seq.to_string(), "None", &from],
                            )))
                        }
                    };
                    if to != stage.status {
                        transition(stage, &to)?;
                    }
                    continue;
                }
                let stage = state.stage_mut(stage_id);
                let stage = match stage {
                    Some(s) if s.status == from => s,
                    Some(s) => {
                        return Err(StateError(crate::i18n::msg(
                            "state.shard_report_status_mismatch",
                            &[&seq.to_string(), &s.status, &from],
                        )))
                    }
                    None => {
                        return Err(StateError(crate::i18n::msg(
                            "state.shard_report_status_mismatch",
                            &[&seq.to_string(), "None", &from],
                        )))
                    }
                };
                let action = stage
                    .action_mut(&action_id)
                    .ok_or_else(|| StateError(crate::i18n::msg("state.shard_report_unknown_action", &[&seq.to_string()])))?;
                action.status = effective.clone();
                if to != stage.status {
                    transition(stage, &to)?;
                }
                state.reports.insert(
                    action_id.clone(),
                    ReportEntry {
                        status: effective,
                        summary: payload_str(payload, "summary").unwrap_or_default(),
                        error: payload_str(payload, "error").unwrap_or_default(),
                    },
                );
            }
            "approval.granted" | "approval.rejected" => {
                let from = payload_str(payload, "from").unwrap_or_default();
                let to = payload_str(payload, "to").unwrap_or_default();
                let stage = state.stage_mut(stage_id);
                let stage = match stage {
                    Some(s) if s.status == from => s,
                    Some(s) => {
                        return Err(StateError(crate::i18n::msg(
                            "state.shard_approval_status_mismatch",
                            &[&seq.to_string(), &s.status, &from],
                        )))
                    }
                    None => {
                        return Err(StateError(crate::i18n::msg(
                            "state.shard_approval_status_mismatch",
                            &[&seq.to_string(), "None", &from],
                        )))
                    }
                };
                transition(stage, &to)?;
                let gate = payload_str(payload, "gate").unwrap_or_default();
                state.reports.insert(
                    format!("approval:{gate}"),
                    ReportEntry {
                        status: if event == "approval.granted" { "granted" } else { "rejected" }.into(),
                        summary: payload_str(payload, "note").unwrap_or_default(),
                        error: String::new(),
                    },
                );
            }
            "workflow.completed" => {
                state.status = "completed".into();
            }
            "workflow.failed" => {
                state.status = "failed".into();
                state.failure = Some(Failure {
                    stage: payload_str(payload, "stage").unwrap_or_default(),
                    reason: payload_str(payload, "reason").unwrap_or_default(),
                });
            }
            "workflow.cancelled" => {
                for stage in state.stages.iter_mut() {
                    if stage.status != "completed" && stage.status != "failed" {
                        transition(stage, "cancelled")?;
                    }
                }
                state.status = "cancelled".into();
            }
            "workflow.forked" => {
                // fork 收尾分片：源会话分片前缀里的 session_dir 由此校正为新会话
                if let Some(to_dir) = payload_str(payload, "to_session_dir") {
                    if !to_dir.is_empty() {
                        state.session_dir = to_dir;
                    }
                }
            }
            other => {
                return Err(StateError(crate::i18n::msg("state.unknown_event_type", &[other])));
            }
        }
    }
    if let (Some(first), Some(last)) = (shards.first(), shards.last()) {
        state.seq = last.get("seq").and_then(|v| v.as_u64()).unwrap_or(0);
        state.created_at = first
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        state.updated_at = last
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
    }
    Ok(state)
}
