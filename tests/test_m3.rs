//! M3 语义增强测试：execution:conditional 显式跳过与传递跳过、
//! inputs required:false 软输入、watches 软依赖。

mod common;

use dmaf_engine::{compile, router};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 夹具：s1（always）→ mid（conditional，出风险评审记录）→ s3（依赖 mid 产出 + 可选软输入）
/// 返回 (守卫, stages 目录, scopes 目录, 产物目录)。
#[allow(clippy::type_complexity)]
fn build_fixture(tag: &str) -> (common::TmpGuard, PathBuf, PathBuf, PathBuf) {
    let root = common::unique_tmp(tag);
    std::fs::create_dir_all(root.join("stages/a")).unwrap();
    std::fs::create_dir_all(root.join("scopes")).unwrap();

    let s1 = "---\nid: s1\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    inputs: []\n    outputs: [\"out/{topic}-s1.md\"]\n    task_description: \"s1\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\n---\n\n# s1\n";
    let mid = "---\nid: mid\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: [\"out/{topic}-s1.md\"]\n    outputs: [\"out/{topic}-mid.md\"]\n    task_description: \"风险评审（条件执行）\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\non_skip: {on_skip}\n---\n\n# mid\n";
    let s3_hard = "---\nid: s3\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: [\"out/{topic}-mid.md\"]\n    outputs: [\"out/{topic}-s3.md\"]\n    task_description: \"s3 硬依赖 mid\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\n---\n\n# s3\n";
    let s3_soft = "---\nid: s3\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs:\n      - artifact: \"out/{topic}-s1.md\"\n        required: true\n      - artifact: \"out/{topic}-mid.md\"\n        required: false\n    watches: [\"out/{topic}-s1.md\"]\n    outputs: [\"out/{topic}-s3.md\"]\n    task_description: \"s3 软输入 mid\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\n---\n\n# s3\n";
    // softdep：s3 以软输入消费先行的 mid 产物（mid 排在其前，派发顺序由 scope 序列保证）
    let s3_softdep = "---\nid: s3\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs:\n      - artifact: \"out/{topic}-s1.md\"\n        required: true\n      - artifact: \"out/{topic}-mid.md\"\n        required: false\n    outputs: [\"out/{topic}-s3.md\"]\n    task_description: \"s3 依赖 mid 且软输入 mid\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\n---\n\n# s3\n";

    std::fs::write(root.join("stages/a/s1.md"), s1).unwrap();
    std::fs::write(
        root.join("stages/a/mid.md"),
        &mid.replace("{on_skip}", "skip"),
    )
    .unwrap();
    // 三个变体暂存在 stages 外（避免 id=stem 校验误扫），选中版写入 stages/a/s3.md
    std::fs::write(root.join("s3-hard.variant.md"), s3_hard).unwrap();
    std::fs::write(root.join("s3-soft.variant.md"), s3_soft).unwrap();
    std::fs::write(root.join("s3-softdep.variant.md"), s3_softdep).unwrap();
    std::fs::write(
        root.join("scopes/main.md"),
        "---\nname: main\ndescription: t\nvars:
  topic:\ninclude_stages: [s1, mid, s3]\n---\n\n# main\n",
    )
    .unwrap();
    // s3 按变体三选一（hard=默认传递跳过 / soft=required:false + watches / softdep=仅软输入）
    let variant_file = if tag.contains("softdep") {
        "s3-softdep.variant.md"
    } else if tag.contains("soft") {
        "s3-soft.variant.md"
    } else {
        "s3-hard.variant.md"
    };
    std::fs::copy(root.join(variant_file), root.join("stages/a/s3.md")).unwrap();

    let stages = root.join("stages");
    let scopes = root.join("scopes");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&artifacts).unwrap();
    let compiled = compile::compile(&stages, &scopes, None).unwrap();
    dmaf_engine::paths::atomic_write_json(
        &artifacts.join(compile::graph_file_name()),
        &compiled.stage_graph,
    )
    .unwrap();
    dmaf_engine::paths::atomic_write_json(
        &artifacts.join(compile::grid_file_name()),
        &compiled.scope_grid,
    )
    .unwrap();
    (common::TmpGuard(root), stages, scopes, artifacts)
}

fn start_main(driver: &common::Driver, artifacts: &Path) -> String {
    let (rc, out, err) = driver.run(&common::svec(&[
        "start",
        "--scope",
        "main",
        "--var",
        "topic=T",
        "--artifacts",
        artifacts.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    serde_json::from_str::<Value>(&out).unwrap()["session_dir"]
        .as_str()
        .unwrap()
        .to_string()
}

fn next_scope(driver: &common::Driver, session: &str, artifacts: &Path) -> Value {
    let (rc, out, err) = driver.run(&common::svec(&[
        "next",
        "--session",
        session,
        "--artifacts",
        artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    serde_json::from_str(&out).unwrap()
}

fn report_scope(driver: &common::Driver, session: &str, artifacts: &Path, payload: &Value) -> (i32, Value) {
    let pf = driver.tmp.join("_p.json");
    std::fs::write(&pf, serde_json::to_string(payload).unwrap()).unwrap();
    let (rc, out, err) = driver.run(&common::svec(&[
        "report",
        "--session",
        session,
        "--payload",
        pf.to_str().unwrap(),
        "--artifacts",
        artifacts.to_str().unwrap(),
    ]));
    let body: Value = serde_json::from_str(if rc == 0 { &out } else { &err }).unwrap();
    (rc, body)
}

fn report_success(driver: &common::Driver, session: &str, artifacts: &Path, instr: &Value) -> (i32, Value) {
    let a = &instr["payload"]["actions"][0];
    for o in a["outputs"].as_array().unwrap() {
        let p = PathBuf::from(o.as_str().unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "mock").unwrap();
    }
    report_scope(
        driver,
        session,
        artifacts,
        &json!({
            "action_id": a["action_id"], "status": "success", "summary": "ok",
            "error_message": null, "agent_outputs": a["outputs"],
        }),
    )
}

#[test]
fn m3_skipped_and_transitive_cascade() {
    let driver = common::Driver::with_tag("m3-skip");
    let (_g, _s, _sc, artifacts) = build_fixture("m3-skip");
    let session = start_main(&driver, &artifacts);

    // s1 完成
    let (rc, _) = report_success(&driver, &session, &artifacts, &next_scope(&driver, &session, &artifacts));
    assert_eq!(rc, 0);

    // mid（conditional）派发后 agent 回报 skipped，缺 reason → 拒绝（rc=3）
    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["stage"], "mid");
    let a = &instr["payload"]["actions"][0];
    let (rc, body) = report_scope(
        &driver,
        &session,
        &artifacts,
        &json!({"action_id": a["action_id"], "status": "skipped", "reason": "", "error_message": null, "agent_outputs": []}),
    );
    assert_eq!(rc, 3);
    assert!(serde_json::to_string(&body).unwrap().contains("reason"), "{body}");

    // 带 reason → mid 跳过；s3（硬输入来自 mid，默认 on_skip）传递跳过；直达 complete
    let (rc, body) = report_scope(
        &driver,
        &session,
        &artifacts,
        &json!({"action_id": a["action_id"], "status": "skipped", "reason": "本流程无风险项", "error_message": null, "agent_outputs": []}),
    );
    assert_eq!(rc, 0, "{body}");
    assert_eq!(body["stage_state"], "skipped");
    assert_eq!(body["action_status"], "skipped");

    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["type"], "complete", "{instr}");

    // 状态文件：mid/s3 计划行 [S]；s3 状态 skipped
    let state = driver.state(&session);
    let s3 = state.stage("s3").unwrap();
    assert_eq!(s3.status, "skipped");
    let state_text = std::fs::read_to_string(PathBuf::from(&session).join("dmaf-state.md")).unwrap();
    assert!(state_text.contains("[S] mid"));
    assert!(state_text.contains("[S] s3"));

    // 审计与确定性（含级联跳过的回放分支）
    let (rc, out, err) = driver.run(&common::svec(&["verify-audit", "--session", &session]));
    assert_eq!(rc, 0, "{err}{out}");
    let (rc, out, err) = driver.run(&common::svec(&[
        "verify-determinism", "--session", &session,
        "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}{out}");
    assert!(out.contains("全部一致"));
}

#[test]
fn m3_on_skip_run_overrides_cascade() {
    let driver = common::Driver::with_tag("m3-onskip-run");
    let (_g, _s, _sc, artifacts) = build_fixture("m3-onskip-run-soft");
    let session = start_main(&driver, &artifacts);

    let (rc, _) = report_success(&driver, &session, &artifacts, &next_scope(&driver, &session, &artifacts));
    assert_eq!(rc, 0);

    // mid（软输入变体里 mid 仍为 conditional）跳过
    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["stage"], "mid");
    let a = &instr["payload"]["actions"][0];
    let (rc, body) = report_scope(
        &driver,
        &session,
        &artifacts,
        &json!({"action_id": a["action_id"], "status": "skipped", "reason": "不适用", "error_message": null, "agent_outputs": []}),
    );
    assert_eq!(rc, 0, "{body}");

    // s3（软输入来自 s1，watches mid 产物）：mid 跳过不级联到 s3（s3 硬输入只依赖 s1）
    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["type"], "dispatch", "{instr}");
    let action = &instr["payload"]["actions"][0];
    // 软输入（required:false，生产者 mid 已跳过）→ missing_inputs advisory
    assert_eq!(action["inputs"].as_array().unwrap().len(), 2);
    // watches 解析进 payload
    assert!(action["watches"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().ends_with("T-s1.md")));

    let (rc, _) = report_success(&driver, &session, &artifacts, &instr);
    assert_eq!(rc, 0);
    assert_eq!(next_scope(&driver, &session, &artifacts)["type"], "complete");
    let (rc, out, err) = driver.run(&common::svec(&["verify-audit", "--session", &session]));
    assert_eq!(rc, 0, "{err}{out}");
}

/// 成员以软输入消费先行的 conditional stage 产物：生产者 skipped 是已消解终态，
/// 下游照常派发（缺席进 missing_inputs 提示），而不是"工作流停滞"。
#[test]
fn m3_skipped_dependency_does_not_stall_soft_consumer() {
    let driver = common::Driver::with_tag("m3-softdep");
    let (_g, _s, _sc, artifacts) = build_fixture("m3-softdep");
    let session = start_main(&driver, &artifacts);

    let (rc, _) = report_success(&driver, &session, &artifacts, &next_scope(&driver, &session, &artifacts));
    assert_eq!(rc, 0);

    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["stage"], "mid");
    let a = &instr["payload"]["actions"][0];
    let (rc, body) = report_scope(
        &driver,
        &session,
        &artifacts,
        &json!({"action_id": a["action_id"], "status": "skipped", "reason": "不适用", "error_message": null, "agent_outputs": []}),
    );
    assert_eq!(rc, 0, "{body}");

    // mid 跳过后 s3 派发（序列已过 mid 位），软输入 mid 产物进 missing_inputs
    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["type"], "dispatch", "{instr}");
    assert_eq!(instr["stage"], "s3");
    let action = &instr["payload"]["actions"][0];
    assert!(
        action["missing_inputs"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().ends_with("T-mid.md")),
        "{action}"
    );

    let (rc, _) = report_success(&driver, &session, &artifacts, &instr);
    assert_eq!(rc, 0);
    assert_eq!(next_scope(&driver, &session, &artifacts)["type"], "complete");
    let (rc, out, err) = driver.run(&common::svec(&["verify-audit", "--session", &session]));
    assert_eq!(rc, 0, "{err}{out}");
    let (rc, out, err) = driver.run(&common::svec(&[
        "verify-determinism", "--session", &session,
        "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}{out}");
    assert!(out.contains("全部一致"));
}

#[test]
fn m3_watches_resolved_in_payload() {
    let driver = common::Driver::with_tag("m3-watches");
    let (_g, _s, _sc, artifacts) = build_fixture("m3-watches");
    let session = start_main(&driver, &artifacts);

    let (rc, _) = report_success(&driver, &session, &artifacts, &next_scope(&driver, &session, &artifacts));
    assert_eq!(rc, 0);
    let instr = next_scope(&driver, &session, &artifacts);
    let action = &instr["payload"]["actions"][0];
    // s1 的 payload：无 watches/missing_inputs（无声明且无缺失）
    assert_eq!(action["watches"].as_array().unwrap().len(), 0);
    assert_eq!(action["missing_inputs"].as_array().unwrap().len(), 0);

    // mid 派发后：mid 的 payload watches 为空、inputs 为硬输入
    let instr = next_scope(&driver, &session, &artifacts);
    let action = &instr["payload"]["actions"][0];
    assert_eq!(action["inputs"][0], json!("out/T-s1.md"));
    let _ = action;
}

#[test]
fn m3_sequence_and_dead_wiring_rules() {
    // 序列规则四象限：硬输入生产者必须是成员且排在消费者之前；软输入成员生产者
    // 排在其后 = 死接线拒绝；非成员生产者 = 按需裁剪合法（轻流程裁剪设计链的形态）
    let root = common::unique_tmp("m3-seq-rules");
    std::fs::create_dir_all(root.join("stages/a")).unwrap();
    std::fs::create_dir_all(root.join("scopes")).unwrap();
    std::fs::write(
        root.join("stages/a/s1.md"),
        "---\nid: s1\nphase: a\ncollaboration: single\nexecution: always\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    inputs: []\n    outputs: [\"out/{topic}-s1.md\"]\n    task_description: \"s1\"\n---\n\n# s1\n",
    ).unwrap();
    let write_scope = |members: &str| {
        std::fs::write(
            root.join("scopes/main.md"),
            format!("---\nname: main\ndescription: t\nvars:\n  topic:\ninclude_stages: {members}\n---\n\n# main\n"),
        ).unwrap();
    };
    let compile_err_problems = || {
        compile::compile(&root.join("stages"), &root.join("scopes"), None)
            .unwrap_err()
            .problems
    };

    // 1) 软输入 + 生产者非成员 → 合法（按需裁剪）
    std::fs::write(
        root.join("stages/a/s2.md"),
        "---\nid: s2\nphase: a\ncollaboration: single\nexecution: always\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs:\n      - artifact: \"out/{topic}-s1.md\"\n        required: false\n    outputs: [\"out/{topic}-s2.md\"]\n    task_description: \"s2\"\n---\n\n# s2\n",
    ).unwrap();
    write_scope("[s2]");
    assert!(compile::compile(&root.join("stages"), &root.join("scopes"), None).is_ok());

    // 2) 软输入 + 成员生产者排在其后 → 死接线拒绝
    write_scope("[s2, s1]");
    let problems = compile_err_problems();
    assert!(problems.iter().any(|p| p.contains("死接线")), "{:?}", problems);

    // 3) 硬输入 + 生产者非成员 → 拒绝（纳入成员或改软输入）
    std::fs::write(
        root.join("stages/a/s2.md"),
        "---\nid: s2\nphase: a\ncollaboration: single\nexecution: always\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: [\"out/{topic}-s1.md\"]\n    outputs: [\"out/{topic}-s2.md\"]\n    task_description: \"s2\"\n---\n\n# s2\n",
    ).unwrap();
    write_scope("[s2]");
    let problems = compile_err_problems();
    assert!(
        problems.iter().any(|p| p.contains("不在成员序列中") && p.contains("'s1'")),
        "{:?}",
        problems
    );

    // 4) 硬输入 + 成员生产者排在其后 → 拒绝（顺序即执行序）
    write_scope("[s2, s1]");
    let problems = compile_err_problems();
    assert!(
        problems.iter().any(|p| p.contains("排在 stage 's2' 之后")),
        "{:?}",
        problems
    );
}

#[test]
fn m3_conditional_producer_requires_soft_downstream() {
    // conditional stage 是某硬输入的唯一生产者 → 编译错（规则 10）
    let root = common::unique_tmp("m3-cond-prod");
    std::fs::create_dir_all(root.join("stages/a")).unwrap();
    std::fs::create_dir_all(root.join("scopes")).unwrap();
    std::fs::write(
        root.join("stages/a/s1.md"),
        "---\nid: s1\nphase: a\ncollaboration: single\nexecution: conditional\n\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    inputs: []\n    outputs: [\"out/{topic}-s1.md\"]\n    task_description: \"s1\"\n---\n\n# s1\n",
    ).unwrap();
    std::fs::write(
        root.join("stages/a/s2.md"),
        "---\nid: s2\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: [\"out/{topic}-s1.md\"]\n    outputs: [\"out/{topic}-s2.md\"]\n    task_description: \"s2\"\n---\n\n# s2\n",
    ).unwrap();
    std::fs::write(
        root.join("scopes/main.md"),
        "---\nname: main\ndescription: t\nvars:
  topic:\ninclude_stages: [s1, s2]\n---\n\n# main\n",
    ).unwrap();

    let err = compile::compile(&root.join("stages"), &root.join("scopes"), None);
    // 编译层实现 conditional-producer 规则（方案 §2.4 规则 10）
    match err {
        Err(e) => assert!(
            e.problems.iter().any(|p| p.contains("conditional") && p.contains("required: false")),
            "{:?}",
            e.problems
        ),
        Ok(_) => panic!("conditional 唯一生产者的硬输入应被拒绝"),
    }
}

#[test]
fn m3_router_determinism_unaffected() {
    // 确定性回归：带 watches/missing_inputs 的 payload 两次计算逐字节一致
    let driver = common::Driver::with_tag("m3-det");
    let (_g, _s, _sc, artifacts) = build_fixture("m3-det");
    let session = start_main(&driver, &artifacts);
    let (rc, _) = report_success(&driver, &session, &artifacts, &next_scope(&driver, &session, &artifacts));
    assert_eq!(rc, 0);

    let state = driver.state(&session);
    let graph: Value = serde_json::from_str(
        &std::fs::read_to_string(artifacts.join(compile::graph_file_name())).unwrap(),
    ).unwrap();
    let grid: Value = serde_json::from_str(
        &std::fs::read_to_string(artifacts.join(compile::grid_file_name())).unwrap(),
    ).unwrap();
    let wf = compile::workflow_from_artifacts(&graph, &grid, "main").unwrap();
    let (i1, _) = router::compute_next(&wf, &state).unwrap();
    let (i2, _) = router::compute_next(&wf, &state).unwrap();
    assert_eq!(dmaf_engine::json_text(&i1), dmaf_engine::json_text(&i2));
}
