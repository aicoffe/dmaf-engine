//! 内置 JSON Schema 的等价校验（instruction / report / audit-shard）。
//!
//! 与 dmaf_engine/schemas/*.json 一一对应；schema 体积小且固定，
//! 为免引入完整 jsonschema 依赖，此处手写等价校验。
//! 规则变更时须同步更新 docs/schemas/ 下的 schema 文件。

use serde_json::Value;

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_f64() {
                "number"
            } else {
                "integer"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn require_keys(v: &Value, required: &[&str]) -> Result<(), String> {
    for k in required {
        if v.get(k).is_none() {
            return Err(format!("'{k}' is a required property"));
        }
    }
    Ok(())
}

fn forbid_extra_keys(v: &Value, allowed: &[&str]) -> Result<(), String> {
    if let Some(obj) = v.as_object() {
        for k in obj.keys() {
            if !allowed.contains(&k.as_str()) {
                return Err(format!("Additional properties are not allowed ('{k}' was unexpected)"));
            }
        }
    }
    Ok(())
}

fn is_str_or_null(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::Null) | Some(Value::String(_)))
}

fn type_err(field: &str, v: Option<&Value>, expect: &str) -> String {
    match v {
        None => format!("'{field}' is a required property"),
        Some(val) => format!("'{field}' must be {expect}, but got {}", type_name(val)),
    }
}

/// instruction.schema.json —— `next` 指令自检。
pub fn validate_instruction(v: &Value) -> Result<(), String> {
    if !v.is_object() {
        return Err(format!("instruction must be object, but got {}", type_name(v)));
    }
    const REQUIRED: &[&str] = &[
        "instruction_id",
        "type",
        "workflow_id",
        "session_dir",
        "stage",
        "stage_state",
        "payload",
    ];
    require_keys(v, REQUIRED)?;
    forbid_extra_keys(v, REQUIRED)?;
    if v["instruction_id"].as_str().is_none() {
        return Err(type_err("instruction_id", v.get("instruction_id"), "string"));
    }
    const TYPES: &[&str] = &["dispatch", "await_approval", "retry", "park", "complete", "fail"];
    let ty = v["type"].as_str().unwrap_or_default();
    if !TYPES.contains(&ty) {
        return Err(format!("'type' must be one of {TYPES:?}, but got '{ty}'"));
    }
    if v["workflow_id"].as_str().is_none() {
        return Err(type_err("workflow_id", v.get("workflow_id"), "string"));
    }
    if v["session_dir"].as_str().is_none() {
        return Err(type_err("session_dir", v.get("session_dir"), "string"));
    }
    if !is_str_or_null(v.get("stage")) {
        return Err(type_err("stage", v.get("stage"), "string or null"));
    }
    if !is_str_or_null(v.get("stage_state")) {
        return Err(type_err("stage_state", v.get("stage_state"), "string or null"));
    }
    if !v["payload"].is_object() {
        return Err(type_err("payload", v.get("payload"), "object"));
    }
    Ok(())
}

/// report.schema.json —— `report` payload 校验（oneOf ActionReport / ApprovalReport）。
pub fn validate_report(v: &Value) -> Result<(), String> {
    if !v.is_object() {
        return Err(format!("report must be object, but got {}", type_name(v)));
    }
    let action_ok = validate_action_report(v).is_ok();
    let approval_ok = validate_approval_report(v).is_ok();
    match (action_ok, approval_ok) {
        (true, false) | (false, true) => Ok(()),
        (true, true) => Err("report is valid against more than one subschema (oneOf)".into()),
        (false, false) => Err(
            "report is not valid against any subschema of oneOf (ActionReport / ApprovalReport)"
                .into(),
        ),
    }
}

fn validate_action_report(v: &Value) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "kind", "action_id", "status", "summary", "error_message", "agent_outputs", "reason",
    ];
    forbid_extra_keys(v, ALLOWED)?;
    require_keys(v, &["action_id", "status"])?;
    if let Some(kind) = v.get("kind") {
        if kind.as_str() != Some("action") {
            return Err("'kind' must be 'action' in ActionReport".into());
        }
    }
    if v["action_id"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
        return Err("'action_id' must be a non-empty string".into());
    }
    let status = v["status"].as_str().unwrap_or_default();
    if status != "success" && status != "failed" && status != "skipped" {
        return Err("'status' must be 'success', 'failed' or 'skipped'".into());
    }
    if let Some(reason) = v.get("reason") {
        if !is_str_or_null(Some(reason)) {
            return Err(type_err("reason", Some(reason), "string or null"));
        }
    }
    if !is_str_or_null(v.get("summary")) {
        return Err(type_err("summary", v.get("summary"), "string or null"));
    }
    if !is_str_or_null(v.get("error_message")) {
        return Err(type_err("error_message", v.get("error_message"), "string or null"));
    }
    if let Some(ao) = v.get("agent_outputs") {
        let Some(arr) = ao.as_array() else {
            return Err("'agent_outputs' must be an array of strings".into());
        };
        if arr.iter().any(|i| !i.is_string()) {
            return Err("'agent_outputs' items must be strings".into());
        }
    }
    Ok(())
}

fn validate_approval_report(v: &Value) -> Result<(), String> {
    const ALLOWED: &[&str] = &["kind", "gate", "decision", "note"];
    forbid_extra_keys(v, ALLOWED)?;
    require_keys(v, &["kind", "gate", "decision"])?;
    if v["kind"].as_str() != Some("approval") {
        return Err("'kind' must be 'approval' in ApprovalReport".into());
    }
    if v["gate"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
        return Err("'gate' must be a non-empty string".into());
    }
    let d = v["decision"].as_str().unwrap_or_default();
    if d != "granted" && d != "rejected" {
        return Err("'decision' must be 'granted' or 'rejected'".into());
    }
    if !is_str_or_null(v.get("note")) {
        return Err(type_err("note", v.get("note"), "string or null"));
    }
    Ok(())
}

fn is_sha256_hash(v: Option<&Value>) -> bool {
    match v {
        Some(Value::String(s)) => {
            let Some(hex) = s.strip_prefix("sha256:") else {
                return false;
            };
            hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        _ => false,
    }
}

/// audit-shard.schema.json —— 分片结构自检（测试用）。
pub fn validate_audit_shard(v: &Value) -> Result<(), String> {
    if !v.is_object() {
        return Err(format!("shard must be object, but got {}", type_name(v)));
    }
    const REQUIRED: &[&str] = &[
        "shard_id",
        "workflow_id",
        "action_id",
        "stage_id",
        "seq",
        "event_type",
        "prev_shard_hash",
        "state_before_hash",
        "state_after_hash",
        "actor",
        "payload",
        "shard_hash",
        "timestamp",
    ];
    require_keys(v, REQUIRED)?;
    if v["shard_id"].as_str().is_none() {
        return Err(type_err("shard_id", v.get("shard_id"), "string"));
    }
    if v["workflow_id"].as_str().is_none() {
        return Err(type_err("workflow_id", v.get("workflow_id"), "string"));
    }
    if !is_str_or_null(v.get("action_id")) {
        return Err(type_err("action_id", v.get("action_id"), "string or null"));
    }
    if !is_str_or_null(v.get("stage_id")) {
        return Err(type_err("stage_id", v.get("stage_id"), "string or null"));
    }
    match v.get("seq").and_then(|s| s.as_u64()) {
        Some(seq) if seq >= 1 => {}
        _ => return Err("'seq' must be an integer >= 1".into()),
    }
    const EVENTS: &[&str] = &[
        "workflow.started",
        "action.dispatched",
        "action.reported",
        "approval.granted",
        "approval.rejected",
        "retry.scheduled",
        "workflow.completed",
        "workflow.failed",
        "workflow.cancelled",
        "workflow.forked",
    ];
    let et = v["event_type"].as_str().unwrap_or_default();
    if !EVENTS.contains(&et) {
        return Err(format!("'event_type' must be one of {EVENTS:?}, but got '{et}'"));
    }
    for field in ["prev_shard_hash", "state_before_hash", "state_after_hash", "shard_hash"] {
        if !is_sha256_hash(v.get(field)) {
            return Err(format!("'{field}' must match ^sha256:[0-9a-f]{{64}}$"));
        }
    }
    if v["actor"].as_str().is_none() {
        return Err(type_err("actor", v.get("actor"), "string"));
    }
    if !v["payload"].is_object() {
        return Err(type_err("payload", v.get("payload"), "object"));
    }
    if v["timestamp"].as_str().is_none() {
        return Err(type_err("timestamp", v.get("timestamp"), "string"));
    }
    Ok(())
}
