//! M4a 评审者（approval.reviewer）测试：
//! await_approval 指令携带评审者派发信息；审批分片记录决策来源；编译校验。

mod common;

use dmaf_engine::compile;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 夹具：s1（always）→ mid（always，审批门 outline-review，评审者 lesson-architect）
fn write_fixture(root: &Path, reviewer: Option<&str>) {
    std::fs::create_dir_all(root.join("stages/a")).unwrap();
    std::fs::create_dir_all(root.join("scopes")).unwrap();
    let reviewer_line = match reviewer {
        Some(r) => format!("  reviewer: {r}\n"),
        None => String::new(),
    };
    std::fs::write(
        root.join("stages/a/s1.md"),
        "---\nid: s1\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    inputs: []\n    outputs: [\"out/{topic}-s1.md\"]\n    task_description: \"s1\"\n---\n\n# s1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("stages/a/mid.md"),
        format!(
            "---\nid: mid\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: [\"out/{{topic}}-s1.md\"]\n    outputs: [\"out/{{topic}}-mid.md\"]\n    task_description: \"mid\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: true\n  gate: outline-review\n{reviewer_line}---\n\n# mid\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("scopes/main.md"),
        "---\nname: main\ndescription: t\nvars:
  topic:\ninclude_stages: [s1, mid]\n---\n\n# main\n",
    )
    .unwrap();
}

fn compile_ok(root: &Path) -> compile::CompiledOutput {
    compile::compile(&root.join("stages"), &root.join("scopes"), None).unwrap()
}

#[test]
fn m4a_reviewer_in_instruction_payload_and_audit() {
    let driver = common::Driver::with_tag("m4a-e2e");
    write_fixture(&driver.tmp, Some("lesson-architect"));
    let compiled = compile_ok(&driver.tmp);
    let artifacts = driver.tmp.join("artifacts");
    std::fs::create_dir_all(&artifacts).unwrap();
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

    // scope 模式启动并推进到审批门
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
    let session = serde_json::from_str::<Value>(&out).unwrap()["session_dir"]
        .as_str()
        .unwrap()
        .to_string();

    let (rc, out, err) = driver.run(&common::svec(&[
        "next", "--session", &session, "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    let instr: Value = serde_json::from_str(&out).unwrap();
    let a = &instr["payload"]["actions"][0];
    for o in a["outputs"].as_array().unwrap() {
        let p = PathBuf::from(o.as_str().unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "mock").unwrap();
    }
    let pf = driver.tmp.join("_p.json");
    std::fs::write(
        &pf,
        serde_json::to_string(&json!({
            "action_id": a["action_id"], "status": "success", "summary": "ok",
            "error_message": null, "agent_outputs": a["outputs"],
        }))
        .unwrap(),
    )
    .unwrap();
    let (rc, _, err) = driver.run(&common::svec(&[
        "report", "--session", &session, "--payload", pf.to_str().unwrap(),
        "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");

    // mid 派发并完成，才进入审批门
    let (rc, out, err) = driver.run(&common::svec(&[
        "next", "--session", &session, "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    let instr: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(instr["type"], "dispatch");
    assert_eq!(instr["stage"], "mid");
    let a = &instr["payload"]["actions"][0];
    for o in a["outputs"].as_array().unwrap() {
        let p = PathBuf::from(o.as_str().unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "mock").unwrap();
    }
    let pf = driver.tmp.join("_p3.json");
    std::fs::write(
        &pf,
        serde_json::to_string(&json!({
            "action_id": a["action_id"], "status": "success", "summary": "ok",
            "error_message": null, "agent_outputs": a["outputs"],
        }))
        .unwrap(),
    )
    .unwrap();
    let (rc, _, err) = driver.run(&common::svec(&[
        "report", "--session", &session, "--payload", pf.to_str().unwrap(),
        "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");

    // 审批门：payload.reviewer 携带评审者派发信息（agent + 被审产出物路径）
    let (rc, out, err) = driver.run(&common::svec(&[
        "next", "--session", &session, "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    let instr: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(instr["type"], "await_approval");
    assert_eq!(instr["payload"]["gate"], json!("outline-review"));
    assert_eq!(instr["payload"]["reviewer"]["agent"], json!("lesson-architect"));
    let reviewer_inputs = instr["payload"]["reviewer"]["inputs"].as_array().unwrap();
    assert!(reviewer_inputs
        .iter()
        .any(|v| v.as_str().unwrap().ends_with("T-mid.md")));

    // 批准：审计分片记录 reviewer
    let pf = driver.tmp.join("_p2.json");
    std::fs::write(
        &pf,
        serde_json::to_string(&json!({
            "kind": "approval", "gate": "outline-review", "decision": "granted", "note": "通过",
        }))
        .unwrap(),
    )
    .unwrap();
    let (rc, _, err) = driver.run(&common::svec(&[
        "report", "--session", &session, "--payload", pf.to_str().unwrap(),
        "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");

    let shards = dmaf_engine::audit::AuditTrail::new(
        &PathBuf::from(&session).join("audit"),
    )
    .unwrap()
    .load_all();
    let approval_shard = shards
        .iter()
        .find(|s| s["event_type"] == json!("approval.granted"))
        .unwrap();
    assert_eq!(
        approval_shard["payload"]["reviewer"],
        json!("lesson-architect")
    );

    // verify-audit / verify-determinism
    let (rc, out, err) = driver.run(&common::svec(&["verify-audit", "--session", &session]));
    assert_eq!(rc, 0, "{err}{out}");
    let (rc, out, err) = driver.run(&common::svec(&[
        "verify-determinism", "--session", &session,
        "--artifacts", artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}{out}");
}

#[test]
fn m4a_compile_rejects_reviewer_without_required() {
    let driver = common::Driver::with_tag("m4a-compile");
    write_fixture(&driver.tmp, Some("lesson-architect"));
    // approval.required 去掉 → reviewer 无意义，编译报错
    let mid = driver.tmp.join("stages/a/mid.md");
    let text = std::fs::read_to_string(&mid).unwrap().replace(
        "approval:\n  required: true\n  gate: outline-review\n  reviewer: lesson-architect\n",
        "approval:\n  required: false\n  gate: outline-review\n  reviewer: lesson-architect\n",
    );
    assert_ne!(text, std::fs::read_to_string(&mid).unwrap(), "替换应生效");
    std::fs::write(&mid, text).unwrap();
    let err = compile::compile(&driver.tmp.join("stages"), &driver.tmp.join("scopes"), None)
        .unwrap_err();
    assert!(err
        .problems
        .iter()
        .any(|p| p.contains("reviewer") && p.contains("required")), "{:?}", err.problems);
}

#[test]
fn m4a_check_lists_unregistered_reviewer() {
    let driver = common::Driver::with_tag("m4a-check");
    write_fixture(&driver.tmp, Some("lesson-architect"));
    // driver 注册表没有 lesson-architect → check 列出
    let wf = dmaf_engine::workflow::load_workflow(&driver.tmp.join("stages").join("a").join("mid.md"));
    let _ = wf; // stage 文件不可直接 load_workflow（顶层无 workflow 键），改走编译+手写 workflow 引用校验：
    let compiled = compile_ok(&driver.tmp);
    let problems = dmaf_engine::registry::validate_refs(&compiled.workflow, &driver.registry_obj());
    eprintln!("DEBUG stages={:?}", compiled.workflow.stages.iter().map(|s| (s.id.clone(), s.approval.reviewer.clone())).collect::<Vec<_>>());
    eprintln!("DEBUG registry agents={:?}", driver.registry_obj().agents.keys().collect::<Vec<_>>());
    eprintln!("DEBUG mid.md=
{}", std::fs::read_to_string(driver.tmp.join("stages/a/mid.md")).unwrap());
    eprintln!("DEBUG graph approval={}", compiled.stage_graph["stages"][1]["approval"]);
    assert!(problems.iter().any(|p| p.contains("lesson-architect")), "{problems:?}");
}

#[test]
fn m4a_no_reviewer_graph_field_null() {
    // 回归：无 reviewer 的门，产物 approval.reviewer 为 null（行为与历史版本一致）
    let driver = common::Driver::with_tag("m4a-noreviewer");
    write_fixture(&driver.tmp, None);
    let compiled = compile_ok(&driver.tmp);
    for s in compiled.stage_graph["stages"].as_array().unwrap() {
        assert!(s["approval"]["reviewer"].is_null());
    }
}
