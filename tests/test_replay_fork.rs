//! replay / fork 测试：审计重建恢复与 checkpoint 分叉。
//! （对应 test_replay_fork.py）

mod common;

use std::path::PathBuf;

use common::Driver;
use dmaf_engine::audit::AuditTrail;
use dmaf_engine::state;
use serde_json::json;

fn session_of(v: &serde_json::Value) -> String {
    v["session_dir"].as_str().unwrap().to_string()
}

/// 会话目录基名 = 会话 ID（活绑定的 SESSION_ID）。
fn basename(p: &str) -> String {
    std::path::Path::new(p).file_name().unwrap().to_string_lossy().to_string()
}

#[test]
fn replay_recovers_corrupted_state() {
    let driver = Driver::with_tag("replay-corrupt");
    let session = session_of(&driver.start_default());
    let state_path = PathBuf::from(&session).join("dmaf-state.md");
    common::drive_to_after_pptx(&driver, &session);
    let original_hash = state::canonical_hash(&state::load(&state_path).unwrap());
    std::fs::write(&state_path, "CORRUPTED").unwrap();

    let (rc, out, err) = driver.run(&common::svec(&["replay", "--session", &session]));
    assert_eq!(rc, 0, "{err}");
    let result: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(result["matches_current_state"], json!(false)); // 写回前比对的是损坏文件
    assert!(result["written_to"]
        .as_str()
        .unwrap()
        .ends_with("dmaf-state.replay.md")); // 默认不覆盖正式文件
    let recovered =
        state::load(&PathBuf::from(&session).join("dmaf-state.replay.md")).unwrap();
    assert_eq!(state::canonical_hash(&recovered), original_hash);

    let (rc, _, _) = driver.run(&common::svec(&[
        "replay", "--session", &session, "--overwrite",
    ]));
    assert_eq!(rc, 0);
    assert_eq!(
        state::canonical_hash(&state::load(&state_path).unwrap()),
        original_hash
    );
    assert_eq!(driver.verify_audit(&session)["replay_matches"], json!(true));
}

#[test]
fn replay_rebuilds_missing_state() {
    let driver = Driver::with_tag("replay-missing");
    let session = session_of(&driver.start_default());
    let instruction = driver.next(&session);
    driver.report_success(&session, &instruction);
    let state_path = PathBuf::from(&session).join("dmaf-state.md");
    let expected_hash = state::canonical_hash(&state::load(&state_path).unwrap());
    std::fs::remove_file(&state_path).unwrap();

    let (rc, out, err) = driver.run(&common::svec(&["replay", "--session", &session]));
    assert_eq!(rc, 0, "{err}");
    let result: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        result["written_to"].as_str().unwrap(),
        dmaf_engine::paths::to_posix(&state_path)
    ); // 缺失时直接重建正式文件
    assert_eq!(
        state::canonical_hash(&state::load(&state_path).unwrap()),
        expected_hash
    );
}

#[test]
fn replay_rejects_broken_chain() {
    let driver = Driver::with_tag("replay-broken");
    let session = session_of(&driver.start_default());
    driver.next(&session);
    let audit_dir = PathBuf::from(&session).join("audit");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&audit_dir)
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
    tampered["payload"]["session_dir"] = json!("dmaf-output/tampered");
    std::fs::write(&files[0], serde_json::to_string(&tampered).unwrap()).unwrap();

    let (rc, out, err) = driver.run(&common::svec(&["replay", "--session", &session]));
    assert_eq!(rc, 4);
    let body: serde_json::Value = serde_json::from_str(&err).unwrap();
    assert_eq!(body["ok"], json!(false));
    assert!(!body["problems"].as_array().unwrap().is_empty());
    let _ = out;
}

#[test]
fn fork_branches_from_checkpoint() {
    let driver = Driver::with_tag("fork");
    let session = session_of(&driver.start_default());
    let seq = common::drive_to_after_pptx(&driver, &session);
    let source_hash = state::canonical_hash(&driver.state(&session));

    let (rc, out, err) = driver.run(&common::svec(&[
        "fork",
        "--session",
        &session,
        "--seq",
        &seq.to_string(),
        "--workflow",
        driver.workflow.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    let result: serde_json::Value = serde_json::from_str(&out).unwrap();
    let forked = result["session_dir"].as_str().unwrap().to_string();
    assert_ne!(forked, session);
    assert_eq!(result["source_session"].as_str().unwrap(), session);
    assert_eq!(result["from_seq"].as_u64().unwrap(), seq);

    // 源会话未被改动
    assert_eq!(state::canonical_hash(&driver.state(&session)), source_hash);

    // 新会话：分片前缀 + workflow.forked 收尾
    let shards = AuditTrail::new(&PathBuf::from(&forked).join("audit"))
        .unwrap()
        .load_all();
    assert_eq!(shards.len() as u64, seq + 1);
    assert_eq!(shards.last().unwrap()["event_type"], "workflow.forked");
    assert_eq!(shards.last().unwrap()["payload"]["from_session"].as_str().unwrap(), session);
    assert_eq!(shards.last().unwrap()["payload"]["from_seq"].as_u64().unwrap(), seq);
    dmaf_engine::schema_check::validate_audit_shard(shards.last().unwrap()).unwrap();

    // SESSION_ID 活绑定：分叉会话的产物桶切到新会话 ID，分叉点前已完成的产物从源桶复制到新桶，源桶原样保留
    let sid = basename(&session);
    let fid = basename(&forked);
    assert_ne!(sid, fid);
    assert_eq!(result["session_id"].as_str().unwrap(), fid);
    let rels = [
        "intent/意图文档.md",
        "research-reports/研究报告.md",
        "outlines/PPT大纲.md",
        "pptx/课件.pptx",
    ];
    for rel in rels {
        assert!(PathBuf::from(format!("dmaf-output/{sid}/{rel}")).is_file(), "源桶 {rel}");
        assert!(PathBuf::from(format!("dmaf-output/{fid}/{rel}")).is_file(), "新桶 {rel}");
    }
    let copied: Vec<String> = result["copied_artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let expected: Vec<String> = rels.iter().map(|r| format!("dmaf-output/{fid}/{r}")).collect();
    assert_eq!(copied, expected);
    assert!(result["missing_artifacts"].as_array().unwrap().is_empty());

    // 分叉会话的指令按新桶渲染：输入指向复制件，委托报文的 SESSION_ID 与 action_id 同为新会话 ID
    let instruction = driver.next(&forked);
    assert_eq!(instruction["type"], "dispatch");
    assert_eq!(instruction["stage"], "quiz-and-video");
    let action = &instruction["payload"]["actions"][0];
    assert_eq!(
        action["inputs"][0].as_str().unwrap(),
        format!("dmaf-output/{fid}/research-reports/研究报告.md")
    );
    assert_eq!(
        action["outputs"][0].as_str().unwrap(),
        format!("dmaf-output/{fid}/questions/题库.json")
    );
    assert_eq!(action["delegation"]["SESSION_ID"].as_str().unwrap(), fid);
    assert!(action["action_id"].as_str().unwrap().contains(&fid));

    // 分叉会话可独立跑完；新产物落新桶，交付摘要的 SESSION_ID 亦为新会话 ID
    driver.report_action(&forked, &instruction, 0, "success", "ok", true, None);
    driver.report_action(&forked, &instruction, 1, "success", "ok", true, None);
    assert!(PathBuf::from(format!("dmaf-output/{fid}/questions/题库.json")).is_file());
    let instruction = driver.next(&forked);
    assert_eq!(instruction["stage"], "submit");
    assert_eq!(
        instruction["payload"]["actions"][0]["inputs"][1].as_str().unwrap(),
        format!("dmaf-output/{fid}/videos/课程视频.mp4")
    );
    let (rc, _) = driver.report_action(&forked, &instruction, 0, "success", "ok", false, None);
    assert_eq!(rc, 0);
    let instruction = driver.next(&forked);
    assert_eq!(instruction["type"], "complete");
    assert_eq!(
        instruction["payload"]["summary"]["vars"]["SESSION_ID"].as_str().unwrap(),
        fid
    );
    assert_eq!(driver.state(&forked).status, "completed");
    assert_eq!(driver.verify_audit(&forked)["replay_matches"], json!(true));
    // 活绑定不破坏确定性：分叉链路上的派发可从账本逐条复现
    let (rc, out, err) = driver.run(&common::svec(&[
        "verify-determinism",
        "--session",
        &forked,
        "--workflow",
        driver.workflow.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}{out}");
    assert!(out.contains("全部一致"), "{out}");
}

#[test]
fn fork_rejects_in_flight_actions() {
    let driver = Driver::with_tag("fork-inflight");
    let session = session_of(&driver.start_default());
    driver.next(&session); // research 派发后在途
    let seq = driver.state(&session).seq;
    let (rc, _, err) = driver.run(&common::svec(&[
        "fork",
        "--session",
        &session,
        "--seq",
        &seq.to_string(),
        "--workflow",
        driver.workflow.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 4);
    let body: serde_json::Value = serde_json::from_str(&err).unwrap();
    assert!(body["error"].as_str().unwrap().contains("在途"));
}

#[test]
fn fork_rejects_unknown_seq() {
    let driver = Driver::with_tag("fork-seq");
    let session = session_of(&driver.start_default());
    let (rc, _, err) = driver.run(&common::svec(&[
        "fork",
        "--session",
        &session,
        "--seq",
        "999",
        "--workflow",
        driver.workflow.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 4);
    let body: serde_json::Value = serde_json::from_str(&err).unwrap();
    assert!(body["error"].as_str().unwrap().contains("seq=999"));
}

#[test]
fn fork_from_start_reruns_aborted_workflow() {
    // 失败工作流的推荐重试姿势：fork 回该 stage 派发前的 checkpoint 重跑
    let driver = Driver::with_tag("fork-rerun");
    let session = session_of(&driver.start_default());
    let instruction = driver.next(&session);
    driver.report_action(
        &session,
        &instruction,
        0,
        "failed",
        "ok",
        false,
        Some("搜索服务不可用"),
    );
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "retry");
    driver.report_action(&session, &instruction, 0, "failed", "ok", false, None);
    assert_eq!(driver.state(&session).status, "failed");

    let (rc, out, err) = driver.run(&common::svec(&[
        "fork",
        "--session",
        &session,
        "--seq",
        "1",
        "--workflow",
        driver.workflow.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    let forked: String =
        serde_json::from_str::<serde_json::Value>(&out).unwrap()["session_dir"]
            .as_str()
            .unwrap()
            .to_string();

    // 从起点分叉：六 stage 全序列重跑 intent→research→outline→审批→pptx→quiz∥video→submit
    let instruction = driver.next(&forked);
    assert_eq!(instruction["stage"], "intent");
    driver.report_success(&forked, &instruction);
    let instruction = driver.next(&forked);
    assert_eq!(instruction["stage"], "research");
    driver.report_success(&forked, &instruction);
    let instruction = driver.next(&forked);
    assert_eq!(instruction["stage"], "outline");
    driver.report_success(&forked, &instruction);
    driver.report_approval(&forked, "outline-review", "granted", "");
    let instruction = driver.next(&forked);
    assert_eq!(instruction["stage"], "pptx");
    driver.report_success(&forked, &instruction);
    let instruction = driver.next(&forked);
    assert_eq!(instruction["stage"], "quiz-and-video");
    driver.report_action(&forked, &instruction, 0, "success", "ok", true, None);
    driver.report_action(&forked, &instruction, 1, "success", "ok", true, None);
    let instruction = driver.next(&forked);
    assert_eq!(instruction["stage"], "submit");
    driver.report_action(&forked, &instruction, 0, "success", "ok", false, None);
    assert_eq!(driver.next(&forked)["type"], "complete");
    assert_eq!(driver.state(&forked).status, "completed");
    // 源失败会话原样保留
    assert_eq!(driver.state(&session).status, "failed");
}
