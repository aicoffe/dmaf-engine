//! M2 scope 模式测试：start --scope、产物加载、状态记录 scope、fork 保留 scope、确定性。

mod common;

use dmaf_engine::compile;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 构造一个最小 scope 库（s1 → s2 链，topic 变量）并编译出产物。
/// 返回 (目录守卫, stages 目录, 产物目录)。
fn build_fixture(tag: &str) -> (common::TmpGuard, PathBuf, PathBuf) {
    let root = common::unique_tmp(tag);
    std::fs::create_dir_all(root.join("stages/a")).unwrap();
    std::fs::create_dir_all(root.join("scopes")).unwrap();
    let stage = |id: &str, input: &str| {
        let inp_line = if input.is_empty() {
            "inputs: []".to_string()
        } else {
            format!("inputs: [\"{input}\"]")
        };
        format!(
            "---\nid: {id}\nphase: a\ncollaboration: single\nexecution: always\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    {inp_line}\n    outputs: [\"out/{{topic}}-{id}.md\"]\n    task_description: \"处理 {id}\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\n---\n\n# {id}\n"
        )
    };
    std::fs::write(root.join("stages/a/s1.md"), stage("s1", "")).unwrap();
    std::fs::write(
        root.join("stages/a/s2.md"),
        stage("s2", "out/{topic}-s1.md"),
    )
    .unwrap();
    std::fs::write(
        root.join("scopes/main.md"),
        "---\nname: main\ndescription: 测试流程\nvars:
  topic:\ninclude_stages: [s1, s2]\n---\n\n# main\n",
    )
    .unwrap();
    let stages_dir = root.join("stages");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&artifacts).unwrap();
    let compiled = compile::compile(&stages_dir, &root.join("scopes"), None).unwrap();
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
    (common::TmpGuard(root), stages_dir, artifacts)
}

fn start_scope(driver: &common::Driver, artifacts: &Path, scope: &str, topic: &str) -> Value {
    let (rc, out, err) = driver.run(&common::svec(&[
        "start",
        "--scope",
        scope,
        "--var",
        &format!("topic={topic}"),
        "--artifacts",
        artifacts.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "start 失败: {err}");
    serde_json::from_str(&out).unwrap()
}

fn next_scope(driver: &common::Driver, session: &str, artifacts: &Path) -> Value {
    let (rc, out, err) = driver.run(&common::svec(&[
        "next",
        "--session",
        session,
        "--artifacts",
        artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "next 失败: {err}");
    serde_json::from_str(&out).unwrap()
}

fn report_scope(
    driver: &common::Driver,
    session: &str,
    artifacts: &Path,
    payload: &Value,
) -> (i32, Value) {
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

/// 产出 action 的全部输出文件（mock），并回报 success。
fn report_success(
    driver: &common::Driver,
    session: &str,
    artifacts: &Path,
    instr: &Value,
) -> (i32, Value) {
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
            "action_id": a["action_id"],
            "status": "success",
            "summary": "ok",
            "error_message": null,
            "agent_outputs": a["outputs"],
        }),
    )
}

/// 进程内确定性断言：同状态两次 compute_next 必得相同指令（逐字节）与相同 effects。
fn assert_scope_deterministic(driver: &common::Driver, session: &str, artifacts: &Path) {
    let state = driver.state(session);
    let graph: Value = serde_json::from_str(
        &std::fs::read_to_string(artifacts.join(compile::graph_file_name())).unwrap(),
    )
    .unwrap();
    let grid: Value = serde_json::from_str(
        &std::fs::read_to_string(artifacts.join(compile::grid_file_name())).unwrap(),
    )
    .unwrap();
    let wf = compile::workflow_from_artifacts(&graph, &grid, "main").unwrap();
    let (i1, e1) = dmaf_engine::router::compute_next(&wf, &state).unwrap();
    let (i2, e2) = dmaf_engine::router::compute_next(&wf, &state).unwrap();
    assert_eq!(
        dmaf_engine::json_text(&i1),
        dmaf_engine::json_text(&i2),
        "scope 模式指令必须确定"
    );
    assert_eq!(e1, e2);
}

#[test]
fn scope_mode_end_to_end() {
    let driver = common::Driver::with_tag("scope-e2e");
    let (_g, _stages, artifacts) = build_fixture("scope-e2e-fixture");

    // start --scope：workflow_id = scope 名，状态文件记录 scope
    let start = start_scope(&driver, &artifacts, "main", "登录");
    let session = start["session_dir"].as_str().unwrap().to_string();
    assert_eq!(start["workflow_id"], json!("main"));
    let state_text =
        std::fs::read_to_string(PathBuf::from(&session).join("dmaf-state.md")).unwrap();
    assert!(state_text.contains("workflow_id: main"));
    assert!(state_text.contains("scope: main"));

    // next 不带 --workflow，凭 state 的 scope 走产物
    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["type"], "dispatch");
    assert_eq!(instr["stage"], "s1");
    assert_eq!(
        instr["payload"]["actions"][0]["outputs"][0],
        json!("out/登录-s1.md")
    );
    assert_scope_deterministic(&driver, &session, &artifacts);

    // 跑完全链
    let (rc, _) = report_success(&driver, &session, &artifacts, &instr);
    assert_eq!(rc, 0);
    assert_scope_deterministic(&driver, &session, &artifacts);
    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["stage"], "s2");
    let (rc, _) = report_success(&driver, &session, &artifacts, &instr);
    assert_eq!(rc, 0);
    let instr = next_scope(&driver, &session, &artifacts);
    assert_eq!(instr["type"], "complete", "{instr}");

    // verify-audit + verify-determinism（scope 会话默认走产物）
    let (rc, out, err) = driver.run(&common::svec(&["verify-audit", "--session", &session]));
    assert_eq!(rc, 0, "{err}{out}");
    let (rc, out, err) = driver.run(&common::svec(&[
        "verify-determinism",
        "--session",
        &session,
        "--artifacts",
        artifacts.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}{out}");
    assert!(out.contains("全部一致"));
}

#[test]
fn fork_preserves_scope() {
    let driver = common::Driver::with_tag("scope-fork");
    let (_g, _stages, artifacts) = build_fixture("scope-fork-fixture");

    let start = start_scope(&driver, &artifacts, "main", "登录");
    let session = start["session_dir"].as_str().unwrap().to_string();
    let instr = next_scope(&driver, &session, &artifacts);
    let (rc, _) = report_success(&driver, &session, &artifacts, &instr);
    assert_eq!(rc, 0);
    let seq = driver.state(&session).seq;

    // fork：源 scope 必须延续到新会话
    let (rc, out, err) = driver.run(&common::svec(&[
        "fork",
        "--session",
        &session,
        "--seq",
        &seq.to_string(),
        "--artifacts",
        artifacts.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}");
    let result: Value = serde_json::from_str(&out).unwrap();
    let forked = result["session_dir"].as_str().unwrap().to_string();
    // 产物路径不含 {SESSION_ID}（out/{topic}-s1.md）：两侧同址，fork 只做存在性确认、不会自复制
    assert_eq!(result["copied_artifacts"], json!(["out/登录-s1.md"]));
    assert_eq!(result["missing_artifacts"], json!([]));
    assert_eq!(std::fs::read_to_string("out/登录-s1.md").unwrap(), "mock");
    let forked_state =
        std::fs::read_to_string(PathBuf::from(&forked).join("dmaf-state.md")).unwrap();
    assert!(forked_state.contains("scope: main"), "{forked_state}");

    // 分叉会话凭 scope 继续跑
    let instr = next_scope(&driver, &forked, &artifacts);
    assert_eq!(instr["stage"], "s2");
    let (rc, _) = report_success(&driver, &forked, &artifacts, &instr);
    assert_eq!(rc, 0);
    assert_eq!(next_scope(&driver, &forked, &artifacts)["type"], "complete");
}

#[test]
fn scope_mode_determinism_with_retry() {
    // 覆盖失败→重试→成功的 scope 模式轨迹，每步断言进程内确定性
    let driver = common::Driver::with_tag("scope-det");
    let (_g, _stages, artifacts) = build_fixture("scope-det-fixture");

    let start = start_scope(&driver, &artifacts, "main", "回归");
    let session = start["session_dir"].as_str().unwrap().to_string();

    let mut saw_retry = false;
    let mut s1_done = false;
    let mut s2_done = false;
    let mut terminal = String::new();
    for _ in 0..12 {
        assert_scope_deterministic(&driver, &session, &artifacts);
        let instr = next_scope(&driver, &session, &artifacts);
        let kind = instr["type"].as_str().unwrap().to_string();
        match kind.as_str() {
            "complete" | "fail" => {
                terminal = kind;
                break;
            }
            "await_approval" => panic!("夹具无审批门"),
            "dispatch" | "retry" => {
                if kind == "retry" {
                    saw_retry = true;
                }
                // s1 第一次失败（触发重试），其余全部成功
                let create = if instr["stage"] == "s1" && !s1_done && !saw_retry {
                    s1_done = true;
                    false
                } else {
                    if instr["stage"] == "s2" {
                        s2_done = true;
                    }
                    true
                };
                let a = &instr["payload"]["actions"][0];
                if create {
                    for o in a["outputs"].as_array().unwrap() {
                        let p = PathBuf::from(o.as_str().unwrap());
                        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                        std::fs::write(&p, "mock").unwrap();
                    }
                }
                let payload = json!({
                    "action_id": a["action_id"],
                    "status": if create { "success" } else { "failed" },
                    "summary": "s",
                    "error_message": null,
                    "agent_outputs": a["outputs"],
                });
                let (rc, body) = report_scope(&driver, &session, &artifacts, &payload);
                assert_eq!(rc, 0, "{body}");
                assert_scope_deterministic(&driver, &session, &artifacts);
            }
            other => panic!("未知指令类型 {other}"),
        }
    }
    assert_eq!(terminal, "complete");
    assert!(saw_retry, "轨迹应覆盖失败→重试");
    assert!(s2_done, "轨迹应到达 s2");
    let _ = s1_done;
}
