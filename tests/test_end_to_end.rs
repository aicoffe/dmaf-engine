//! 端到端 mock 测试：mock 文件系统驱动 start→next→report 全工作流。
//! （对应 test_end_to_end_mock.py）

mod common;

use std::path::PathBuf;

use serde_json::json;

fn session_of(v: &serde_json::Value) -> String {
    v["session_dir"].as_str().unwrap().to_string()
}

#[test]
fn happy_path_with_approval_revision_and_parallel() {
    let driver = common::Driver::with_tag("e2e");
    let start = driver.start("大语言模型", Some("面向有Python基础的学生"));
    let session = session_of(&start);
    let sid = std::path::Path::new(&session)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    // 会话目录（.dmaf/<ts>）是引擎独占区，只含 audit；产物目录在项目根、agent 写入时自建
    for sub in ["audit"] {
        assert!(PathBuf::from(&session).join(sub).is_dir(), "{sub}");
    }

    // 0. intent：意图判断与提取（TASK 原文承载于 task_description）
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "dispatch");
    assert_eq!(instruction["stage"], "intent");
    let action = &instruction["payload"]["actions"][0];
    assert!(action["task_description"].as_str().unwrap().contains("Python基础"));
    let (rc, _) = driver.report_success(&session, &instruction);
    assert_eq!(rc, 0);

    // 1. research
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "dispatch");
    assert_eq!(instruction["stage"], "research");
    let action = &instruction["payload"]["actions"][0];
    assert_eq!(action["agent"], "lesson-researcher");
    assert_eq!(action["skills"], json!(["lesson-research"]));
    assert_eq!(
        action["outputs"],
        json!([format!("dmaf-output/{sid}/research-reports/研究报告.md")])
    );
    assert!(action["task_description"].as_str().unwrap().contains("Python基础"));
    let (rc, body) = driver.report_action(&session, &instruction, 0, "success", "大语言模型一句话简介", true, None);
    assert_eq!(rc, 0);
    assert_eq!(body["action_status"], "success");
    // description 不再有协议字段/metadata 通道——上游摘要由 submit 从研究报告工件提炼

    // 2. outline → 审批门
    let instruction = driver.next(&session);
    assert_eq!(instruction["stage"], "outline");
    let (rc, body) = driver.report_success(&session, &instruction);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "awaiting-approval");

    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "await_approval");
    assert_eq!(instruction["payload"]["gate"], "outline-review");
    assert_eq!(
        instruction["payload"]["outputs"],
        json!([format!("dmaf-output/{sid}/outlines/PPT大纲.md")])
    );

    // 3. 拒绝 → 修订轮
    let (rc, body) = driver.report_approval(&session, "outline-review", "rejected", "第3页太深了");
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "revising");
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "dispatch");
    assert_eq!(instruction["stage"], "outline");
    let revision = &instruction["payload"]["actions"][0];
    assert_eq!(revision["attempt"], 2);
    assert_eq!(revision["revision_feedback"], "第3页太深了");
    assert!(revision["action_id"].as_str().unwrap().ends_with("-002"));
    let (rc, body) = driver.report_success(&session, &instruction);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "awaiting-approval");

    // 4. 批准 → pptx
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "await_approval");
    driver.report_approval(&session, "outline-review", "granted", "");
    let instruction = driver.next(&session);
    assert_eq!(instruction["stage"], "pptx");
    let (rc, body) = driver.report_success(&session, &instruction);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "completed");

    // 5. quiz-and-video 并行（一条指令两个 action）
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "dispatch");
    assert_eq!(instruction["stage"], "quiz-and-video");
    assert_eq!(instruction["payload"]["collaboration"], "parallel");
    assert_eq!(instruction["payload"]["actions"].as_array().unwrap().len(), 2);
    let agents: Vec<&str> = instruction["payload"]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["agent"].as_str().unwrap())
        .collect();
    assert_eq!(agents, vec!["lesson-quizzer", "lesson-videoer"]);

    let (rc, body) = driver.report_action(&session, &instruction, 0, "success", "ok", true, None);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "in-progress");
    let (rc, body) = driver.report_action(
        &session,
        &instruction,
        1,
        "failed",
        "ok",
        false,
        Some("DASHSCOPE key 无效"),
    );
    assert_eq!(rc, 0);
    assert_eq!(body["action_status"], "failed");
    assert!(body["scheduled_retry"].as_str().unwrap().ends_with("-002"));

    // 6. 重试指令
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "retry");
    assert_eq!(instruction["stage"], "quiz-and-video");
    let retry_actions = instruction["payload"]["actions"].as_array().unwrap();
    assert_eq!(retry_actions.len(), 1);
    assert_eq!(retry_actions[0]["agent"], "lesson-videoer");
    assert_eq!(retry_actions[0]["attempt"], 2);
    let (rc, body) = driver.report_success(&session, &instruction);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "completed");

    // 7. submit：inputs 汇聚三路工件 + AgentRef.vars 声明集
    let instruction = driver.next(&session);
    assert_eq!(instruction["stage"], "submit");
    let action = &instruction["payload"]["actions"][0];
    assert_eq!(
        action["inputs"],
        json!([
            format!("dmaf-output/{sid}/pptx/课件.pptx"),
            format!("dmaf-output/{sid}/videos/课程视频.mp4"),
            format!("dmaf-output/{sid}/questions/题库.json"),
            format!("dmaf-output/{sid}/intent/意图文档.md"),
        ])
    );
    // AgentRef.vars 声明集从会话变量集挑选（SESSION_ID 为保留变量，恒在）
    assert_eq!(action["vars"]["SESSION_ID"], json!(sid));
    assert_eq!(action["outputs"].as_array().unwrap().len(), 0);
    let (rc, _) = driver.report_action(&session, &instruction, 0, "success", "ok", false, None);
    assert_eq!(rc, 0);

    // 8. complete
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "complete");
    let summary = &instruction["payload"]["summary"];
    let stage_ids: Vec<&str> = summary["stages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        stage_ids,
        vec!["intent", "research", "outline", "pptx", "quiz-and-video", "submit"]
    );
    assert_eq!(summary["failed_actions"].as_array().unwrap().len(), 0);
    // summary 顶层不泄漏变量；会话变量经 summary.vars 结构化承载
    assert!(summary.get("course_id").is_none());
    assert_eq!(summary["vars"]["SESSION_ID"], json!(sid));
    // 交付文案由 completion_template 引擎渲染（与 scopes/lesson.md 同源）
    assert!(instruction["payload"]["completion_text"]
        .as_str()
        .unwrap()
        .contains("课程设计完成！"));

    let state = driver.state(&session);
    assert_eq!(state.status, "completed");
    assert!(state.stages.iter().all(|s| s.status == "completed"));

    let result = driver.verify_audit(&session);
    assert_eq!(result["ok"], json!(true));
    assert_eq!(result["replay_matches"], json!(true));
    assert!(result["shards"].as_u64().unwrap() >= 12);

    let (rc, out, _) = driver.run(&common::svec(&["inspect", "--session", &session]));
    assert_eq!(rc, 0);
    assert!(out.contains("completed"));
}

#[test]
fn abort_when_research_exhausted() {
    // max_attempts:1 = 最多重试 1 次；两次失败后 abort_workflow
    let driver = common::Driver::with_tag("abort");
    let session = session_of(&driver.start_default());
    let _sid = std::path::Path::new(&session)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    // 先完成 intent，再让 research 两次失败
    let instruction = driver.next(&session);
    driver.report_success(&session, &instruction);
    let instruction = driver.next(&session);
    assert_eq!(instruction["stage"], "research");
    let (rc, body) = driver.report_action(
        &session,
        &instruction,
        0,
        "failed",
        "ok",
        false,
        Some("搜索服务不可用"),
    );
    assert_eq!(rc, 0);
    assert!(!body["scheduled_retry"].is_null());
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "retry");
    let (rc, _) = driver.report_action(
        &session,
        &instruction,
        0,
        "failed",
        "ok",
        false,
        Some("搜索服务仍不可用"),
    );
    assert_eq!(rc, 0);
    let state = driver.state(&session);
    assert_eq!(state.status, "failed");
    assert_eq!(state.failure.as_ref().unwrap().stage, "research");
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "fail");
    assert!(instruction["payload"]["reason"].as_str().unwrap().contains("搜索服务"));
    let result = driver.verify_audit(&session);
    assert_eq!(result["replay_matches"], json!(true));
}

#[test]
fn continue_semantics_when_video_exhausted() {
    let driver = common::Driver::with_tag("continue");
    let session = session_of(&driver.start_default());
    let sid = std::path::Path::new(&session)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let instruction = driver.next(&session);
    driver.report_success(&session, &instruction); // intent
    let instruction = driver.next(&session);
    driver.report_success(&session, &instruction); // research
    let instruction = driver.next(&session);
    driver.report_success(&session, &instruction); // outline
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "await_approval");
    driver.report_approval(&session, "outline-review", "granted", "");
    let instruction = driver.next(&session);
    assert_eq!(instruction["stage"], "pptx");
    driver.report_success(&session, &instruction);

    let instruction = driver.next(&session);
    assert_eq!(instruction["stage"], "quiz-and-video");
    driver.report_action(&session, &instruction, 0, "success", "ok", true, None);
    let (rc, body) = driver.report_action(&session, &instruction, 1, "failed", "ok", false, None);
    assert_eq!(rc, 0);
    assert!(!body["scheduled_retry"].is_null());
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "retry");
    let (rc, body) = driver.report_action(&session, &instruction, 0, "failed", "ok", false, None);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "failed");

    // on_exhausted=continue：工作流继续，submit 照常派发且输入不剔除缺失项
    let instruction = driver.next(&session);
    assert_eq!(instruction["stage"], "submit");
    let inputs: Vec<String> = instruction["payload"]["actions"][0]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(inputs.contains(&format!("dmaf-output/{sid}/videos/课程视频.mp4")));
    driver.report_action(&session, &instruction, 0, "success", "ok", false, None);
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "complete");
    let summary = &instruction["payload"]["summary"];
    let failed = summary["failed_actions"].as_array().unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["agent"], "lesson-videoer");
    assert_eq!(driver.state(&session).status, "completed");
    assert_eq!(driver.verify_audit(&session)["replay_matches"], json!(true));
}

#[test]
fn duplicate_report_conflict() {
    let driver = common::Driver::with_tag("dup-report");
    let session = session_of(&driver.start_default());
    let _sid = std::path::Path::new(&session)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let instruction = driver.next(&session);
    let (rc, _) = driver.report_success(&session, &instruction);
    assert_eq!(rc, 0);
    let payload = json!({
        "action_id": instruction["payload"]["actions"][0]["action_id"],
        "status": "success",
    });
    let (rc, body, _) = driver.report_payload(&session, &payload);
    assert_eq!(rc, 4);
    assert!(body["error"].as_str().unwrap().contains("不可重复"));
}

#[test]
fn schema_violation_rejected_state_unchanged() {
    let driver = common::Driver::with_tag("schema-reject");
    let session = session_of(&driver.start_default());
    let _sid = std::path::Path::new(&session)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    driver.next(&session);
    let seq_before = driver.state(&session).seq;
    // 缺 action_id
    let (rc, body, _) = driver.report_payload(&session, &json!({"status": "success"}));
    assert_eq!(rc, 3);
    assert_eq!(body["ok"], json!(false));
    assert_eq!(driver.state(&session).seq, seq_before);
}

#[test]
fn cancel_mid_flow() {
    let driver = common::Driver::with_tag("cancel");
    let session = session_of(&driver.start_default());
    let _sid = std::path::Path::new(&session)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    driver.next(&session);
    let (rc, _, _) = driver.run(&common::svec(&["cancel", "--session", &session]));
    assert_eq!(rc, 0);
    let state = driver.state(&session);
    assert_eq!(state.status, "cancelled");
    assert!(state.stages.iter().all(|s| s.status == "cancelled"));
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "fail");
    assert_eq!(driver.verify_audit(&session)["replay_matches"], json!(true));
}

#[test]
fn module_entrypoint_smoke() {
    let out = dmaf_engine::run(&common::svec(&["--help"]));
    assert_eq!(out.code, 0);
    assert!(out.stdout.contains("start"));
}
