//! 通用 Conductor 第一步测试：
//! A. action.delegation 预构造（conductor 原样转发；vars 平铺；路径字段与定义同名 outputs/inputs；仅 revision_feedback 条件出现）
//! B. completion_template（引擎渲染 completion_text；无模板省略字段；占位符定义期校验）

mod common;

use dmaf_engine::{compile, workflow};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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

/// scope 会话的 report：带 --artifacts、不带 --workflow（定义来自编译产物）。
fn report_scope(
    driver: &common::Driver,
    session: &str,
    artifacts: &Path,
    payload: &Value,
) -> (i32, Value) {
    let pf = driver.tmp.join("_p_scope.json");
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
    let _ = std::fs::remove_file(&pf);
    let body_source = if rc == 0 { &out } else { &err };
    let body: Value = serde_json::from_str(body_source)
        .unwrap_or_else(|_| panic!("report 输出不是 JSON: {body_source}"));
    (rc, body)
}

fn report_scope_success(
    driver: &common::Driver,
    session: &str,
    artifacts: &Path,
    instruction: &Value,
    index: usize,
) -> (i32, Value) {
    let action = &instruction["payload"]["actions"][index];
    for out in action["outputs"].as_array().unwrap() {
        let p = PathBuf::from(out.as_str().unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "mock").unwrap();
    }
    report_scope(
        driver,
        session,
        artifacts,
        &json!({
            "action_id": action["action_id"], "status": "success", "summary": "ok",
            "error_message": null, "agent_outputs": action["outputs"],
        }),
    )
}

fn report_scope_approval(
    driver: &common::Driver,
    session: &str,
    artifacts: &Path,
    gate: &str,
    decision: &str,
    note: &str,
) -> (i32, Value) {
    report_scope(
        driver,
        session,
        artifacts,
        &json!({"kind": "approval", "gate": gate, "decision": decision, "note": note}),
    )
}

fn start_scope(driver: &common::Driver, artifacts: &Path) -> String {
    let (rc, out, err) = driver.run(&common::svec(&[
        "start",
        "--scope",
        "main",
        "--var",
        "topic=T1",
        "--artifacts",
        artifacts.to_str().unwrap(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "start 失败: {err}");
    serde_json::from_str::<Value>(&out).unwrap()["session_dir"]
        .as_str()
        .unwrap()
        .to_string()
}

/// 写 artifacts 并启动 scope 会话，返回 (session, artifacts)。
fn setup_scope(driver: &common::Driver, scope_extra: &str) -> (String, std::path::PathBuf) {
    std::fs::create_dir_all(driver.tmp.join("stages/a")).unwrap();
    std::fs::create_dir_all(driver.tmp.join("scopes")).unwrap();
    std::fs::write(
        driver.tmp.join("stages/a/s1.md"),
        "---\nid: s1\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    inputs: []\n    outputs: [\"out/{topic}-s1.md\"]\n    task_description: \"s1 任务 {topic}\"\n---\n\n# s1\n",
    )
    .unwrap();
    std::fs::write(
        driver.tmp.join("scopes/main.md"),
        format!(
            "---\nname: main\ndescription: t\nvars:
  topic:\n{}include_stages: [s1]\n---\n\n# main\n",
            scope_extra
        ),
    )
    .unwrap();
    let compiled = compile::compile(
        &driver.tmp.join("stages"),
        &driver.tmp.join("scopes"),
        None,
    )
    .unwrap();
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
    (start_scope(driver, &artifacts), artifacts)
}

// ---------- A. delegation 预构造 ----------

#[test]
fn delegation_fields_and_vars_flatten() {
    let driver = common::Driver::with_tag("cond-del");
    let (session, artifacts) = setup_scope(&driver, "");
    let instruction = next_scope(&driver, &session, &artifacts);
    let action = &instruction["payload"]["actions"][0];
    let delegation = &action["delegation"];

    // 协议字段（路径字段与 stage 定义同名，值为模板解析后的完整路径；恒在）
    assert_eq!(delegation["role"], "task_delegation");
    assert_eq!(delegation["task_description"], action["task_description"]);
    assert_eq!(delegation["inputs"], action["inputs"]);
    assert_eq!(delegation["outputs"], action["outputs"]);

    // vars 平铺：会话 vars（topic）在；description 未声明不在；无 revision_feedback
    assert_eq!(delegation["topic"], "T1");
    assert!(delegation.get("description").is_none());
    assert!(delegation.get("revision_feedback").is_none());

    // action 顶层不再携带业务字段 knowledge_point
    assert!(action.get("knowledge_point").is_none());
    // action.vars 子集保留
    assert_eq!(action["vars"], json!({}));
}

#[test]
fn delegation_outputs_empty_array_when_no_outputs_and_flattens_declared_vars() {
    let driver = common::Driver::with_tag("cond-nout");
    std::fs::create_dir_all(driver.tmp.join("stages/a")).unwrap();
    std::fs::create_dir_all(driver.tmp.join("scopes")).unwrap();
    std::fs::write(
        driver.tmp.join("stages/a/fin.md"),
        "---\nid: fin\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-submiter\n    skills: [lesson-submit]\n    inputs: []\n    outputs: []\n    task_description: \"提交 {topic}\"\n    vars: [product_line]\n---\n\n# fin\n",
    )
    .unwrap();
    std::fs::write(
        driver.tmp.join("scopes/main.md"),
        "---\nname: main\ndescription: t\nvars:
  topic:\nconstants:\n  product_line: \"课件\"\ninclude_stages: [fin]\n---\n\n# main\n",
    )
    .unwrap();
    let compiled = compile::compile(
        &driver.tmp.join("stages"),
        &driver.tmp.join("scopes"),
        None,
    )
    .unwrap();
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
    let session = start_scope(&driver, &artifacts);
    let instruction = next_scope(&driver, &session, &artifacts);
    let action = &instruction["payload"]["actions"][0];
    let delegation = &action["delegation"];

    // outputs 为空 → outputs 恒在、为空数组（submit 形态；写入目标约定不适用）
    assert_eq!(delegation["outputs"], json!([]));
    // AgentRef.vars 声明集从会话变量集（含常量）挑选进 action.vars；description 非协议字段
    assert_eq!(action["vars"]["product_line"], json!("课件"));
    assert!(delegation.get("description").is_none());
}

#[test]
fn delegation_carries_revision_feedback_on_revising_round() {
    let driver = common::Driver::with_tag("cond-rev");
    std::fs::create_dir_all(driver.tmp.join("stages/a")).unwrap();
    std::fs::create_dir_all(driver.tmp.join("scopes")).unwrap();
    std::fs::write(
        driver.tmp.join("stages/a/mid.md"),
        "---\nid: mid\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: []\n    outputs: [\"out/{topic}-mid.md\"]\n    task_description: \"mid\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: true\n  gate: review-x\n---\n\n# mid\n",
    )
    .unwrap();
    std::fs::write(
        driver.tmp.join("scopes/main.md"),
        "---\nname: main\ndescription: t\nvars:
  topic:\ninclude_stages: [mid]\n---\n\n# main\n",
    )
    .unwrap();
    let compiled = compile::compile(
        &driver.tmp.join("stages"),
        &driver.tmp.join("scopes"),
        None,
    )
    .unwrap();
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
    let session = start_scope(&driver, &artifacts);

    // 第一轮派发：无 revision_feedback
    let instruction = next_scope(&driver, &session, &artifacts);
    assert!(instruction["payload"]["actions"][0]["delegation"]
        .get("revision_feedback")
        .is_none());
    let (rc, _) = report_scope_success(&driver, &session, &artifacts, &instruction, 0);
    assert_eq!(rc, 0);

    // 拒绝 → 修订轮：delegation 与 action 顶层都携带 revision_feedback
    let (rc, _) = report_scope_approval(&driver, &session, &artifacts, "review-x", "rejected", "重写第二章");
    assert_eq!(rc, 0);
    let instruction = next_scope(&driver, &session, &artifacts);
    let action = &instruction["payload"]["actions"][0];
    assert_eq!(action["revision_feedback"], "重写第二章");
    assert_eq!(action["delegation"]["revision_feedback"], "重写第二章");
}

// ---------- B. completion_template ----------


#[test]
fn completion_text_rendered_and_summary_has_no_var_leak() {
    let driver = common::Driver::with_tag("cond-ct");
    let (session, artifacts) = setup_scope(&driver, "completion_template: |\n  完成 {vars.topic}！产出：{outputs.s1}\n");
    let dispatch = next_scope(&driver, &session, &artifacts);
    let expected_output = dispatch["payload"]["actions"][0]["outputs"][0]
        .as_str()
        .unwrap()
        .to_string();
    let (rc, _) = report_scope_success(&driver, &session, &artifacts, &dispatch, 0);
    assert_eq!(rc, 0);

    let complete = next_scope(&driver, &session, &artifacts);
    assert_eq!(complete["type"], "complete");
    let text = complete["payload"]["completion_text"].as_str().unwrap();
    assert_eq!(text, format!("完成 T1！产出：{expected_output}\n"));
    // summary 顶层不泄漏变量（会话变量只经 summary.vars 结构化承载）
    assert!(complete["payload"]["summary"].get("course_id").is_none());
    assert!(complete["payload"]["summary"]["vars"].get("topic").is_some());
}

#[test]
fn completion_text_omitted_without_template() {
    let driver = common::Driver::with_tag("cond-noct");
    let (session, artifacts) = setup_scope(&driver, "");
    let instruction = next_scope(&driver, &session, &artifacts);
    let (rc, _) = report_scope_success(&driver, &session, &artifacts, &instruction, 0);
    assert_eq!(rc, 0);
    let complete = next_scope(&driver, &session, &artifacts);
    assert_eq!(complete["type"], "complete");
    assert!(complete["payload"].get("completion_text").is_none());
}

#[test]
fn completion_template_brace_escape() {
    let lookup = |_kind: &str, key: &str| {
        if key == "t" {
            Some("V".to_string())
        } else {
            None
        }
    };
    let rendered =
        workflow::render_completion("a {vars.t} {{}} }}{{ {outputs.x-y} 哦", &lookup);
    // {{}} → 字面 {}；}} {{ → 字面 } {；outputs.x-y 缺键报错（渲染器不做定义期校验）
    assert!(rendered.is_err());

    let rendered = workflow::render_completion("a {vars.t} {{原样}}", &lookup).unwrap();
    assert_eq!(rendered, "a V {原样}");
}

#[test]
fn completion_template_placeholder_extraction_and_validation() {
    // 词法校验：无前缀 / 空 key / 未闭合 / 孤立 }
    assert!(workflow::completion_placeholders("x {foo}").is_err());
    assert!(workflow::completion_placeholders("x {vars.}").is_err());
    assert!(workflow::completion_placeholders("x {vars.a").is_err());
    assert!(workflow::completion_placeholders("x a}").is_err());
    assert_eq!(
        workflow::completion_placeholders("{vars.a} {outputs.quiz-and-video} {vars.a}").unwrap(),
        vec![
            ("vars".to_string(), "a".to_string()),
            ("outputs".to_string(), "quiz-and-video".to_string()),
            ("vars".to_string(), "a".to_string()),
        ]
    );

    // 定义期校验：未知 var / 非成员 stage
    let declared: std::collections::BTreeSet<String> = ["a".to_string()].into_iter().collect();
    let ids: std::collections::BTreeSet<String> = ["s1".to_string()].into_iter().collect();
    let problems = workflow::validate_completion_placeholders(
        "{vars.b} {outputs.s2}",
        &declared,
        &ids,
        "scope 'x'",
    );
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("引用未声明的变量 'b'"));
    assert!(problems[1].contains("引用未声明的 stage 's2'"));
}

#[test]
fn compile_rejects_bad_completion_template() {
    let driver = common::Driver::with_tag("cond-badct");
    std::fs::create_dir_all(driver.tmp.join("stages/a")).unwrap();
    std::fs::create_dir_all(driver.tmp.join("scopes")).unwrap();
    std::fs::write(
        driver.tmp.join("stages/a/s1.md"),
        "---\nid: s1\nphase: a\ncollaboration: single\nexecution: always\n\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    inputs: []\n    outputs: [\"out/{topic}-s1.md\"]\n    task_description: \"s1\"\n---\n\n# s1\n",
    )
    .unwrap();
    std::fs::write(
        driver.tmp.join("scopes/main.md"),
        "---\nname: main\ndescription: t\nvars:
  topic:\ncompletion_template: \"{vars.nope} {outputs.elsewhere}\"\ninclude_stages: [s1]\n---\n\n# main\n",
    )
    .unwrap();
    let err = compile::compile(
        &driver.tmp.join("stages"),
        &driver.tmp.join("scopes"),
        None,
    )
    .unwrap_err();
    let text = format!("{err:?}");
    assert!(text.contains("引用未声明的变量 'nope'"), "{text}");
    assert!(text.contains("引用未声明的 stage 'elsewhere'"), "{text}");
}

#[test]
fn inline_workflow_validates_completion_template() {
    // 坏占位符 → load_workflow 报错
    let bad = driver_tmp_yaml(
        "bad",
        Some("completion_template: \"{vars.nope}\""),
    );
    let err = workflow::load_workflow(&bad).unwrap_err();
    assert!(format!("{err:?}").contains("引用未声明的变量 'nope'"));

    // 剔除自带模板 → 解析通过且 completion_template 为 None
    let none = driver_tmp_yaml("none", None);
    let wf = workflow::load_workflow(&none).unwrap();
    assert!(wf.completion_template.is_none());

    // 仓库自带定义（与 scope 黄金对照同源）解析通过且模板在
    let wf = workflow::load_workflow(&common::workflow_file()).unwrap();
    assert!(wf
        .completion_template
        .as_deref()
        .unwrap()
        .contains("{outputs.research}"));
}

/// 基于仓库内联 wf-lesson.yaml 构造临时定义：tpl 为 Some(行) 时替换自带模板，None 时剔除自带模板。
fn driver_tmp_yaml(tag: &str, tpl: Option<&str>) -> std::path::PathBuf {
    let src = std::fs::read_to_string(common::workflow_file()).unwrap();
    let stripped = src
        .split('\n')
        .filter(|line| {
            !line.contains("completion_template:")
                && !line.starts_with("    课程设计完成")
                && !line.starts_with("    产出物清单")
                && !line.starts_with("    📄")
                && !line.starts_with("    📋")
                && !line.starts_with("    📊")
                && !line.starts_with("    📝")
                && !line.starts_with("    ✅")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let replaced = match tpl {
        Some(line) => stripped.replace(
            "  stages:\n",
            &format!("  {line}\n  stages:\n"),
        ),
        None => stripped,
    };
    let p = common::unique_tmp(&format!("cond-yaml-{tag}")).join("wf.yaml");
    std::fs::write(&p, replaced).unwrap();
    p
}
