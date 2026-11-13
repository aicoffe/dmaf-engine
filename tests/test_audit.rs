//! 审计分片哈希链测试（对应 test_audit_chain.py）。

mod common;

use dmaf_engine::audit::{genesis_hash, sha256_of, verify_chain, AuditTrail};
use serde_json::json;

fn trail(tag: &str) -> (std::path::PathBuf, AuditTrail) {
    let tmp = common::unique_tmp(tag);
    let t = AuditTrail::new(&tmp.join("audit")).unwrap();
    (tmp, t)
}

#[test]
fn append_builds_chain() {
    let (_tmp, t) = trail("chain");
    let s1 = t
        .append("wf-x", "workflow.started", None, None, json!({}), &genesis_hash(),
                &format!("sha256:{}", "a".repeat(64)), "2026-10-13T10:00:00+08:00")
        .unwrap();
    let s2 = t
        .append("wf-x", "action.dispatched", Some("a"), Some("act1"), json!({}),
                s1["shard_hash"].as_str().unwrap(),
                &format!("sha256:{}", "b".repeat(64)), "2026-10-13T10:00:01+08:00")
        .unwrap();
    assert_eq!(s1["prev_shard_hash"].as_str().unwrap(), genesis_hash());
    assert_eq!(s2["prev_shard_hash"].as_str().unwrap(), s1["shard_hash"].as_str().unwrap());
    assert_eq!(s2["seq"].as_u64().unwrap(), 2);
    let shards = t.load_all();
    assert!(verify_chain(&shards).is_empty());
    assert_eq!(shards.len(), 2);
}

#[test]
fn unknown_event_type_rejected() {
    let (_tmp, t) = trail("bad-event");
    assert!(t
        .append("wf-x", "state.transitioned", None, None, json!({}), &genesis_hash(),
                &genesis_hash(), "t")
        .is_err());
}

#[test]
fn tamper_detected() {
    let (tmp, t) = trail("tamper");
    let s1 = t
        .append("wf-x", "workflow.started", None, None, json!({"a": 1}), &genesis_hash(),
                &format!("sha256:{}", "a".repeat(64)), "t1")
        .unwrap();
    t.append("wf-x", "action.dispatched", Some("a"), Some("act1"), json!({}),
             s1["shard_hash"].as_str().unwrap(),
             &format!("sha256:{}", "b".repeat(64)), "t2")
        .unwrap();
    let audit_dir = tmp.join("audit");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&audit_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|f| f.to_string_lossy().starts_with("shard-"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    let mut tampered: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&files[0]).unwrap()).unwrap();
    tampered["payload"]["a"] = json!(999); // 篡改历史
    std::fs::write(&files[0], serde_json::to_string(&tampered).unwrap()).unwrap();
    let problems = verify_chain(&t.load_all());
    assert!(problems.iter().any(|p| p.contains("shard_hash") || p.contains("篡改")));
}

#[test]
fn removed_shard_breaks_seq() {
    let (tmp, t) = trail("removed");
    let s1 = t
        .append("wf-x", "workflow.started", None, None, json!({}), &genesis_hash(),
                &format!("sha256:{}", "a".repeat(64)), "t1")
        .unwrap();
    t.append("wf-x", "action.dispatched", Some("a"), Some("act1"), json!({}),
             s1["shard_hash"].as_str().unwrap(),
             &format!("sha256:{}", "b".repeat(64)), "t2")
        .unwrap();
    let audit_dir = tmp.join("audit");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&audit_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|f| f.to_string_lossy().starts_with("shard-"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    std::fs::remove_file(&files[0]).unwrap();
    let problems = verify_chain(&t.load_all());
    assert!(!problems.is_empty()); // seq 断裂 + prev 不衔接
}

#[test]
fn shard_matches_schema() {
    let (_tmp, t) = trail("schema");
    let shard = t
        .append("wf-x", "workflow.started", None, None, json!({"k": "v"}), &genesis_hash(),
                &format!("sha256:{}", "a".repeat(64)), "2026-10-13T10:00:00+08:00")
        .unwrap();
    dmaf_engine::schema_check::validate_audit_shard(&shard).unwrap();
}

#[test]
fn canonical_json_deterministic() {
    let a = sha256_of(&json!({"b": 1, "a": "中文"}));
    let b = sha256_of(&json!({"a": "中文", "b": 1}));
    assert_eq!(a, b);
}
