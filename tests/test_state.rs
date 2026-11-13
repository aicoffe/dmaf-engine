//! 状态文件格式与状态机转移测试（对应 test_state_machine.py）。

mod common;

use std::collections::BTreeMap;

use dmaf_engine::state::{
    self, ActionRecord, Failure, ReportEntry, StageState, StateError, WorkflowState,
};

fn make_state() -> WorkflowState {
    let mut vars = BTreeMap::new();
    vars.insert("knowledge_point".to_string(), "大语言模型".to_string());
    vars.insert("constraints".to_string(), String::new());
    vars.insert(
        "course_id".to_string(),
        "course_20260101-000000".to_string(),
    );
    vars.insert("SESSION_ID".to_string(), "20260101-000000".to_string());
    let mut reports = BTreeMap::new();
    reports.insert(
        "act-20260101-000000-research-researcher-001".to_string(),
        ReportEntry {
            status: "success".into(),
            summary: "带\"引号\"和\n换行的摘要".into(),
            error: String::new(),
        },
    );
    reports.insert(
        "approval:outline-review".to_string(),
        ReportEntry {
            status: "rejected".into(),
            summary: "太深了".into(),
            error: String::new(),
        },
    );
    WorkflowState {
        locale: "zh".into(),
        workflow_id: "wf-lesson".into(),
        session_dir: ".dmaf/20260101-000000".into(),
        status: "running".into(),
        scope: String::new(),
        created_at: "2026-10-13T10:00:00+08:00".into(),
        updated_at: String::new(),
        seq: 3,
        vars,
        failure: Some(Failure {
            stage: "video".into(),
            reason: "TTS 失败".into(),
        }),
        reports,
        stages: vec![
            StageState {
                id: "research".into(),
                status: "completed".into(),
                attempt: 1,
                max_attempts: 1,
                actions: vec![ActionRecord {
                    action_id: "act-20260101-000000-research-researcher-001".into(),
                    agent: "lesson-researcher".into(),
                    attempt: 1,
                    status: "success".into(),
                }],
            },
            StageState {
                id: "outline".into(),
                status: "awaiting-approval".into(),
                attempt: 1,
                max_attempts: 1,
                actions: vec![],
            },
            StageState {
                id: "video".into(),
                status: "failed".into(),
                attempt: 2,
                max_attempts: 2,
                actions: vec![
                    ActionRecord {
                        action_id: "act-20260101-000000-video-videoer-001".into(),
                        agent: "lesson-videoer".into(),
                        attempt: 1,
                        status: "failed".into(),
                    },
                    ActionRecord {
                        action_id: "act-20260101-000000-video-videoer-002".into(),
                        agent: "lesson-videoer".into(),
                        attempt: 2,
                        status: "failed".into(),
                    },
                ],
            },
        ],
    }
}

#[test]
fn render_parse_roundtrip() {
    let original = make_state();
    let text = state::render(&original);
    let parsed = state::parse(&text).unwrap();
    assert_eq!(
        state::canonical_hash(&parsed),
        state::canonical_hash(&original)
    );
    // 二次渲染幂等
    assert_eq!(state::render(&parsed), text);
}

#[test]
fn parse_rejects_broken_body() {
    let original = make_state();
    let text = state::render(&original);
    // 复选框与状态不一致
    let broken = text.replace_first("attempt 1 success", "attempt 1 failed");
    assert!(matches!(state::parse(&broken), Err(StateError(_))));
    // 未知行
    let rubbish = text.replace_first("# Stages", "# Stages\nrubbish line");
    assert!(matches!(state::parse(&rubbish), Err(StateError(_))));
    // 缺 front matter
    assert!(matches!(state::parse("no front matter"), Err(StateError(_))));
}

trait ReplaceFirst {
    fn replace_first(&self, from: &str, to: &str) -> String;
}

impl ReplaceFirst for str {
    fn replace_first(&self, from: &str, to: &str) -> String {
        match self.find(from) {
            Some(idx) => {
                let mut s = String::with_capacity(self.len());
                s.push_str(&self[..idx]);
                s.push_str(to);
                s.push_str(&self[idx + from.len()..]);
                s
            }
            None => self.to_string(),
        }
    }
}

#[test]
fn transition_table() {
    let mut stage = StageState::new("s", 1);
    state::transition(&mut stage, "in-progress").unwrap();
    state::transition(&mut stage, "awaiting-approval").unwrap();
    state::transition(&mut stage, "revising").unwrap();
    state::transition(&mut stage, "in-progress").unwrap();
    state::transition(&mut stage, "failed").unwrap();
    state::transition(&mut stage, "cancelled").unwrap();
    assert!(matches!(
        state::transition(&mut stage, "completed"),
        Err(StateError(_))
    ));
}

#[test]
fn pending_to_completed_illegal() {
    let mut stage = StageState::new("s", 1);
    assert!(matches!(
        state::transition(&mut stage, "completed"),
        Err(StateError(_))
    ));
}

fn shard(seq: u64, event: &str, stage_id: Option<&str>, action_id: Option<&str>, payload: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "event_type": event,
        "stage_id": stage_id,
        "workflow_id": "wf-x",
        "action_id": action_id,
        "payload": payload,
    })
}

#[test]
fn replay_rebuilds_from_shards() {
    let shards = vec![
        shard(
            1,
            "workflow.started",
            None,
            None,
            serde_json::json!({
                "session_dir": ".dmaf/s1",
                "vars": {"knowledge_point": "x"},
                "stages": [{"id": "a", "max_attempts": 1}, {"id": "b", "max_attempts": 1}],
            }),
        ),
        shard(
            2,
            "action.dispatched",
            Some("a"),
            None,
            serde_json::json!({
                "from": "pending", "to": "in-progress", "attempt": 1,
                "agents": ["lesson-researcher"],
                "actions": [{"action_id": "act1", "agent": "lesson-researcher", "attempt": 1}],
            }),
        ),
        shard(
            3,
            "action.reported",
            Some("a"),
            Some("act1"),
            serde_json::json!({
                "from": "in-progress", "to": "completed", "action_id": "act1",
                "agent": "lesson-researcher", "attempt": 1, "status": "success",
                "effective_status": "success", "summary": "ok", "error": "",
                "expected_outputs": [], "missing_outputs": [],
            }),
        ),
    ];
    let rebuilt = state::replay(&shards).unwrap();
    assert_eq!(rebuilt.workflow_id, "wf-x");
    assert_eq!(rebuilt.stage("a").unwrap().status, "completed");
    assert_eq!(rebuilt.reports["act1"].summary, "ok");
    assert_eq!(rebuilt.stage("b").unwrap().status, "pending");
}

#[test]
fn replay_detects_inconsistent_from() {
    let shards = vec![
        shard(
            1,
            "workflow.started",
            None,
            None,
            serde_json::json!({
                "session_dir": ".dmaf/s1",
                "vars": {},
                "stages": [{"id": "a", "max_attempts": 1}],
            }),
        ),
        shard(
            2,
            "action.reported",
            Some("a"),
            Some("act1"),
            serde_json::json!({
                "from": "in-progress", "to": "completed", "action_id": "act1",
                "effective_status": "success",
            }),
        ),
    ];
    assert!(matches!(state::replay(&shards), Err(StateError(_))));
}

#[test]
fn canonical_view_stable_across_parse() {
    let original = make_state();
    let parsed = state::parse(&state::render(&original)).unwrap();
    let v1 = state::canonical_view(&original);
    let v2 = state::canonical_view(&parsed);
    assert_eq!(dmaf_engine::json_text(&v1), dmaf_engine::json_text(&v2));
}
