//! 审计分片：追加式、SHA-256 哈希链、Audit-first 原子性。
//!
//! 规则：
//! 1. Engine 在持久化状态变更前先写审计分片；分片写失败则异常、状态不落盘；
//! 2. 每个状态转换有且仅有一个发射器（engine 在各业务动作内集中发事件）；
//! 3. 分片一经写入不可变更；verify_chain 可发现任何篡改。

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::paths;

pub fn genesis_hash() -> String {
    format!("sha256:{}", "0".repeat(64))
}

pub const ACTOR: &str = "tool:dmaf-engine";

pub const EVENT_TYPES: &[&str] = &[
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

pub fn is_valid_event_type(t: &str) -> bool {
    EVENT_TYPES.contains(&t)
}

/// 排序键 + 紧凑分隔符的规范化 JSON（哈希输入）。
pub fn canonical_json(obj: &Value) -> String {
    serde_json::to_string(obj).expect("json 序列化失败")
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn sha256_of(obj: &Value) -> String {
    format!("sha256:{}", sha256_hex(canonical_json(obj).as_bytes()))
}

pub struct AuditTrail {
    pub audit_dir: PathBuf,
}

impl AuditTrail {
    pub fn new(audit_dir: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(audit_dir)?;
        Ok(AuditTrail { audit_dir: audit_dir.to_path_buf() })
    }

    fn shard_path(&self, seq: u64, workflow_id: &str) -> PathBuf {
        self.audit_dir.join(format!("shard-{workflow_id}-{seq:06}.json"))
    }

    fn shard_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.audit_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .map(|f| {
                        let s = f.to_string_lossy();
                        s.starts_with("shard-") && s.ends_with(".json")
                    })
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        files
    }

    /// (最新 seq, 最新分片哈希)；无分片时 (0, GENESIS)。
    pub fn last_seq_and_hash(&self) -> (u64, String) {
        let files = self.shard_files();
        if files.is_empty() {
            return (0, genesis_hash());
        }
        let last: Value = serde_json::from_str(
            &std::fs::read_to_string(files.last().unwrap()).expect("读取分片失败"),
        )
        .expect("分片不是合法 JSON");
        (
            last.get("seq").and_then(|v| v.as_u64()).unwrap_or(0),
            last.get("shard_hash")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        )
    }

    /// 追加一个分片，返回写入的完整分片 JSON。event_type 非法即报错。
    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &self,
        workflow_id: &str,
        event_type: &str,
        stage_id: Option<&str>,
        action_id: Option<&str>,
        payload: Value,
        state_before_hash: &str,
        state_after_hash: &str,
        timestamp: &str,
    ) -> Result<Value, String> {
        if !is_valid_event_type(event_type) {
            return Err(format!("未知审计事件类型: {event_type}"));
        }
        let (prev_seq, prev_hash) = self.last_seq_and_hash();
        let seq = prev_seq + 1;
        let mut shard = Map::new();
        shard.insert("shard_id".into(), json!(format!("shard-{workflow_id}-{seq:06}")));
        shard.insert("workflow_id".into(), json!(workflow_id));
        shard.insert("action_id".into(), json!(action_id));
        shard.insert("stage_id".into(), json!(stage_id));
        shard.insert("seq".into(), json!(seq));
        shard.insert("event_type".into(), json!(event_type));
        shard.insert("prev_shard_hash".into(), json!(prev_hash));
        shard.insert("state_before_hash".into(), json!(state_before_hash));
        shard.insert("state_after_hash".into(), json!(state_after_hash));
        shard.insert("actor".into(), json!(ACTOR));
        shard.insert("payload".into(), payload);
        shard.insert("timestamp".into(), json!(timestamp));
        let hash = sha256_of(&Value::Object(shard.clone()));
        shard.insert("shard_hash".into(), json!(hash));
        let shard = Value::Object(shard);
        let path = self.shard_path(seq, workflow_id);
        paths::atomic_write_json(&path, &shard)
            .map_err(|e| format!("写审计分片失败: {e}"))?;
        Ok(shard)
    }

    /// 按 seq 升序加载全部分片。
    pub fn load_all(&self) -> Vec<Value> {
        let mut shards: Vec<Value> = self
            .shard_files()
            .iter()
            .map(|p| {
                serde_json::from_str(&std::fs::read_to_string(p).expect("读取分片失败"))
                    .expect("分片不是合法 JSON")
            })
            .collect();
        shards.sort_by_key(|s| s.get("seq").and_then(|v| v.as_u64()).unwrap_or(0));
        shards
    }
}

/// 校验分片序列的哈希链完整性，返回问题清单（空 = 完整）。
pub fn verify_chain(shards: &[Value]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut prev_hash = genesis_hash();
    for (i, shard) in shards.iter().enumerate() {
        let i = i + 1;
        let seq = shard.get("seq");
        if seq != Some(&json!(i as u64)) {
            problems.push(format!("分片 {i} seq 不连续: {}", display_of(seq)));
        }
        if shard.get("prev_shard_hash").and_then(|v| v.as_str()) != Some(prev_hash.as_str()) {
            problems.push(format!("分片 {i} prev_shard_hash 与前片不衔接"));
        }
        let without_hash: Map<String, Value> = shard
            .as_object()
            .map(|m| {
                m.iter()
                    .filter(|(k, _)| k.as_str() != "shard_hash")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let expected = sha256_of(&Value::Object(without_hash));
        if shard.get("shard_hash").and_then(|v| v.as_str()) != Some(expected.as_str()) {
            problems.push(format!("分片 {i} shard_hash 不匹配（内容被篡改或损坏）"));
        }
        prev_hash = shard
            .get("shard_hash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(prev_hash);
        let et = shard.get("event_type").and_then(|v| v.as_str()).unwrap_or("");
        if !is_valid_event_type(et) {
            problems.push(format!("分片 {i} 未知事件类型: {}", display_of(shard.get("event_type"))));
        }
    }
    problems
}

/// Python .get(...) 缺失值在错误消息里显示为 None；这里保持相近的可读形态。
fn display_of(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "None".to_string(),
        Some(other) => other.to_string(),
    }
}
