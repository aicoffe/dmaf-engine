//! M1 测试：compile 产物黄金对照与确定性、编译校验失败形态、install 场景、check 命令。

mod common;

use std::path::PathBuf;

use dmaf_engine::compile;
use dmaf_engine::registry::HarnessLayout;
use dmaf_engine::workflow::load_workflow;
use serde_json::json;

fn compile_repo() -> compile::CompiledOutput {
    compile::compile(&common::stages_dir(), &common::scopes_dir(), None).unwrap()
}

// ---------- 双轨等价守卫：scope 编译产物 ≡ 内联解析（合成夹具；样例已无内联孪生） ----------

#[test]
fn golden_compiled_matches_inline_workflow() {
    let root = common::unique_tmp("golden-twin");
    let _guard = common::TmpGuard(root.clone());
    let stages_dir = root.join("stages");
    let scopes_dir = root.join("scopes");
    std::fs::create_dir_all(stages_dir.join("p")).unwrap();
    std::fs::create_dir_all(&scopes_dir).unwrap();

    let w = |rel: &str, text: &str| std::fs::write(root.join(rel), text).unwrap();
    w(
        "stages/p/a.md",
        r#"---
id: a
phase: p
collaboration: single
execution: always
agents:
  - agent: agent-a
    skills: [skill-a]
    inputs: []
    outputs: ["out/{SESSION_ID}/a.md"]
    task_description: "做 a：{TASK}"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: false
---

# a
"#,
    );
    w(
        "stages/p/b.md",
        r#"---
id: b
phase: p
collaboration: single
execution: always

agents:
  - agent: agent-b
    skills: [skill-b]
    inputs: ["out/{SESSION_ID}/a.md"]
    outputs: ["out/{SESSION_ID}/b.md"]
    task_description: "做 b"
    vars: [SESSION_ID]
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: true
  gate: twin-gate
---

# b
"#,
    );
    w(
        "stages/p/c.md",
        r#"---
id: c
phase: p
collaboration: parallel
execution: always

agents:
  - agent: agent-c1
    skills: [skill-c]
    inputs: ["out/{SESSION_ID}/b.md"]
    outputs: ["out/{SESSION_ID}/c1.md"]
    task_description: "做 c1"
  - agent: agent-c2
    skills: [skill-c]
    inputs: ["out/{SESSION_ID}/b.md"]
    outputs: ["out/{SESSION_ID}/c2.md"]
    task_description: "做 c2"
retry:
  max_attempts: 1
  on_exhausted: continue
approval:
  required: false
---

# c
"#,
    );
    w(
        "scopes/twin.md",
        r#"---
name: twin
description: 双轨等价夹具
completion_template: |
  完成：{outputs.a}
include_stages: [a, b, c]
---

# twin
"#,
    );
    // 内联侧：与 scope 同一张图（含 gate / parallel / continue / stage vars）
    w(
        "wf-twin.yaml",
        r#"workflow:
  id: wf-twin
  name: 双轨等价夹具
  completion_template: |
    完成：{outputs.a}
  stages:
    - id: a
      collaboration: single
      agents:
        - agent: agent-a
          skills: [skill-a]
          inputs: []
          outputs: ["out/{SESSION_ID}/a.md"]
          task_description: "做 a：{TASK}"
      retry:
        max_attempts: 1
        on_exhausted: abort_workflow
      approval:
        required: false
    - id: b
      
      collaboration: single
      agents:
        - agent: agent-b
          skills: [skill-b]
          inputs: ["out/{SESSION_ID}/a.md"]
          outputs: ["out/{SESSION_ID}/b.md"]
          task_description: "做 b"
          vars: [SESSION_ID]
      retry:
        max_attempts: 1
        on_exhausted: abort_workflow
      approval:
        required: true
        gate: twin-gate
    - id: c
      
      collaboration: parallel
      agents:
        - agent: agent-c1
          skills: [skill-c]
          inputs: ["out/{SESSION_ID}/b.md"]
          outputs: ["out/{SESSION_ID}/c1.md"]
          task_description: "做 c1"
        - agent: agent-c2
          skills: [skill-c]
          inputs: ["out/{SESSION_ID}/b.md"]
          outputs: ["out/{SESSION_ID}/c2.md"]
          task_description: "做 c2"
      retry:
        max_attempts: 1
        on_exhausted: continue
      approval:
        required: false
"#,
    );

    let inline = load_workflow(&root.join("wf-twin.yaml")).unwrap();
    let compiled = compile::compile(&stages_dir, &scopes_dir, None).unwrap();
    // scope 视图（含编译期派生的数据依赖边）是与内联解析对照的真源
    let compiled_scope =
        compile::workflow_from_artifacts(&compiled.stage_graph, &compiled.scope_grid, "twin")
            .unwrap();

    // 逐 stage 逐字段：编译与内联解析必须同构
    for wf_stage in &inline.stages {
        let c = compiled_scope
            .stage(&wf_stage.id)
            .unwrap_or_else(|| panic!("编译产物缺 stage {}", wf_stage.id));
        assert_eq!(wf_stage.collaboration, c.collaboration, "{}", wf_stage.id);
        assert_eq!(wf_stage.depends_on, c.depends_on, "{}", wf_stage.id);
        assert_eq!(wf_stage.retry.max_attempts, c.retry.max_attempts, "{}", wf_stage.id);
        assert_eq!(wf_stage.retry.on_exhausted, c.retry.on_exhausted, "{}", wf_stage.id);
        assert_eq!(wf_stage.approval.required, c.approval.required, "{}", wf_stage.id);
        assert_eq!(wf_stage.approval.gate, c.approval.gate, "{}", wf_stage.id);
        assert_eq!(wf_stage.agents.len(), c.agents.len(), "{}", wf_stage.id);
        for (a, ca) in wf_stage.agents.iter().zip(c.agents.iter()) {
            assert_eq!(a.agent, ca.agent);
            assert_eq!(a.skills, ca.skills, "{}", wf_stage.id);
            assert_eq!(a.inputs, ca.inputs, "{}", wf_stage.id);
            assert_eq!(a.outputs, ca.outputs, "{}", wf_stage.id);
            assert_eq!(a.task_description, ca.task_description, "{}", wf_stage.id);
            assert_eq!(a.vars, ca.vars, "{}", wf_stage.id);
        }
    }

    // scope 元数据
    let grid = &compiled.scope_grid;
    let twin = &grid["scopes"]["twin"];
    assert_eq!(twin["vars"], json!({}));
    assert_eq!(twin["constants"], json!({}));
    assert_eq!(
        twin["completion_template"].as_str(),
        inline.completion_template.as_deref()
    );
    assert_eq!(twin["stages"], json!(inline.stage_ids()));

    // stage-graph 内 stage 顺序 = 库文件读序（确定性）；执行顺序由 scope 序列决定
    let graph = &compiled.stage_graph;
    let order: Vec<&str> = graph["stages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(order, vec!["a", "b", "c"]);
    for s in graph["stages"].as_array().unwrap() {
        assert!(
            s["scopes"].as_array().unwrap().iter().any(|v| v == "twin"),
            "{}",
            s["id"]
        );
    }
}

// ---------- 确定性与 --check ----------

#[test]
fn compile_is_byte_deterministic() {
    let a = compile_repo();
    let b = compile_repo();
    assert_eq!(
        compile::serialize_artifact(&a.stage_graph),
        compile::serialize_artifact(&b.stage_graph)
    );
    assert_eq!(
        compile::serialize_artifact(&a.scope_grid),
        compile::serialize_artifact(&b.scope_grid)
    );
}

#[test]
fn compile_check_passes_on_committed_artifacts() {
    let driver = common::Driver::with_tag("check-artifacts");
    let (rc, out, err) = driver.run(&common::svec(&["compile", "--check"]));
    assert_eq!(rc, 0, "{err}{out}");
    let body: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(body["ok"], json!(true));
    assert_eq!(body["stages"], json!(12));
    assert_eq!(body["scopes"], json!(2));
}

// ---------- 编译校验失败形态 ----------

fn write_temp_stage(dir: &std::path::Path, phase: &str, stem: &str, frontmatter: &str) {
    let d = dir.join("stages").join(phase);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join(format!("{stem}.md")),
        format!("---\n{frontmatter}\n---\n\n# body\n"),
    )
    .unwrap();
}

fn temp_workflow_root(tag: &str) -> PathBuf {
    let root = common::unique_tmp(tag);
    std::fs::create_dir_all(root.join("stages/a")).unwrap();
    std::fs::create_dir_all(root.join("scopes")).unwrap();
    write_temp_stage(
        &root,
        "a",
        "s1",
        "id: s1\nphase: a\ncollaboration: single\nexecution: always\nagents:\n  - agent: lesson-researcher\n    skills: [lesson-research]\n    inputs: []\n    outputs: [\"out/{knowledge_point}-s1.md\"]\n",
    );
    std::fs::write(
        root.join("scopes/main.md"),
        "---\nname: main\ndescription: t\ninclude_stages: [s1]\n---\n\n# body\n",
    )
    .unwrap();
    root
}

#[test]
fn compile_rejects_stage_scope_field() {
    let root = temp_workflow_root("stage-scope-field");
    write_temp_stage(
        &root,
        "a",
        "s2",
        "id: s2\nphase: a\ncollaboration: single\nexecution: always\nscopes: [main]\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: []\n    outputs: [\"out/x.md\"]\n",
    );
    let err = compile::compile(&root.join("stages"), &root.join("scopes"), None).unwrap_err();
    assert!(
        err.problems.iter().any(|p| p.contains("不得携带 scope")),
        "{:?}",
        err.problems
    );
}

#[test]
fn compile_rejects_bad_include_and_duplicates() {
    let root = temp_workflow_root("bad-include");
    std::fs::write(
        root.join("scopes/main.md"),
        "---\nname: main\ndescription: t\ninclude_stages: [s1, s1, ghost]\n---\n\n# body\n",
    )
    .unwrap();
    let err = compile::compile(&root.join("stages"), &root.join("scopes"), None).unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("重复")), "{:?}", err.problems);
    assert!(err.problems.iter().any(|p| p.contains("ghost")), "{:?}", err.problems);
}

#[test]
fn compile_rejects_phase_mismatch() {
    let root = temp_workflow_root("phase-mismatch");
    write_temp_stage(
        &root,
        "a",
        "s2",
        "id: s2\nphase: b\ncollaboration: single\nexecution: always\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: []\n    outputs: [\"out/x.md\"]\n",
    );
    let err = compile::compile(&root.join("stages"), &root.join("scopes"), None).unwrap_err();
    eprintln!("problems: {:?}", err.problems);
    assert!(err.problems.iter().any(|p| p.contains("phase")), "{:?}", err.problems);
}

#[test]
fn compile_rejects_hard_producer_not_in_scope() {
    // 硬输入的生产者不在 scope 成员中 → 编译期拒绝（裁剪成员必须连带软化输入）
    let root = temp_workflow_root("scope-closure");
    write_temp_stage(
        &root,
        "a",
        "s2",
        "id: s2
phase: a
collaboration: single
execution: always

agents:
  - agent: lesson-outliner
    skills: [lesson-outline]
    inputs: [\"out/{knowledge_point}-s1.md\"]
    outputs: [\"out/{knowledge_point}-s2.md\"]
",
    );
    std::fs::write(
        root.join("scopes/main.md"),
        "---
name: main
description: t
include_stages: [s2]
---

# body
",
    )
    .unwrap();
    let err = compile::compile(&root.join("stages"), &root.join("scopes"), None).unwrap_err();
    assert!(
        err.problems
            .iter()
            .any(|p| p.contains("不在成员序列中") && p.contains("'s1'")),
        "{:?}",
        err.problems
    );
}

#[test]
fn compile_rejects_id_stem_mismatch() {
    let root = temp_workflow_root("id-stem");
    write_temp_stage(
        &root,
        "a",
        "s2",
        "id: other\nphase: a\ncollaboration: single\nexecution: always\nagents:\n  - agent: lesson-outliner\n    skills: [lesson-outline]\n    inputs: []\n    outputs: [\"out/x.md\"]\n",
    );
    let err = compile::compile(&root.join("stages"), &root.join("scopes"), None).unwrap_err();
    eprintln!("problems: {:?}", err.problems);
    assert!(err.problems.iter().any(|p| p.contains("stem")), "{:?}", err.problems);
}

// ---------- check 命令 ----------

#[test]
fn check_ok_against_driver_harness() {
    let driver = common::Driver::with_tag("check-ok");
    let (rc, out, err) = driver.run(&common::svec(&[
        "check",
        "--harness",
        driver.registry.to_str().unwrap(),
        "--workflow",
        driver.workflow.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}{out}");
    let body: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(body["ok"], json!(true));
    assert_eq!(body["workflow_id"], json!("wf-lesson"));
}

#[test]
fn check_reports_missing_agent_skill_and_primary_mode() {
    let driver = common::Driver::with_tag("check-bad");

    // 缺 agent
    let mut stages = json!([
        {"id": "a", "collaboration": "single", "execution": "always",
         "agents": [{"agent": "lesson-ghost", "skills": ["lesson-research"],
                     "inputs": [], "outputs": ["out/{knowledge_point}-a.md"]}]},
    ]);
    let doc = json!({"workflow": {"id": "wf-x", "vars": {"knowledge_point": null}, "stages": stages}});
    let path = driver.tmp.join("wf-ghost.yaml");
    std::fs::write(&path, serde_yaml::to_string(&doc).unwrap()).unwrap();
    let (rc, _, err) = driver.run(&common::svec(&[
        "check",
        "--harness",
        driver.registry.to_str().unwrap(),
        "--workflow",
        path.to_str().unwrap(),
    ]));
    assert_eq!(rc, 3);
    assert!(err.contains("lesson-ghost"), "{err}");

    // 缺 skill
    stages[0]["agents"][0] = json!({"agent": "lesson-researcher", "skills": ["lesson-ghost"],
        "inputs": [], "outputs": ["out/{knowledge_point}-a.md"]});
    let doc = json!({"workflow": {"id": "wf-x", "vars": {"knowledge_point": null}, "stages": stages}});
    std::fs::write(&path, serde_yaml::to_string(&doc).unwrap()).unwrap();
    let (rc, _, err) = driver.run(&common::svec(&[
        "check",
        "--harness",
        driver.registry.to_str().unwrap(),
        "--workflow",
        path.to_str().unwrap(),
    ]));
    assert_eq!(rc, 3);
    assert!(err.contains("lesson-ghost"), "{err}");

    // primary 模式不可派发
    stages[0]["agents"][0] = json!({"agent": "dmaf-conductor", "skills": ["lesson-research"],
        "inputs": [], "outputs": ["out/{knowledge_point}-a.md"]});
    let doc = json!({"workflow": {"id": "wf-x", "vars": {"knowledge_point": null}, "stages": stages}});
    std::fs::write(&path, serde_yaml::to_string(&doc).unwrap()).unwrap();
    let (rc, _, err) = driver.run(&common::svec(&[
        "check",
        "--harness",
        driver.registry.to_str().unwrap(),
        "--workflow",
        path.to_str().unwrap(),
    ]));
    assert_eq!(rc, 3);
    assert!(err.contains("subagent"), "{err}");
}

#[test]
fn check_stage_library_via_compile_path() {
    // 自包含夹具：合成 stage 库的 agent/skill 与 Driver 注册表对齐（不依赖真实 dmaf/ 样例）
    let lib = common::unique_tmp("check-lib");
    let _guard = common::TmpGuard(lib.clone());
    let stages_dir = lib.join("stages");
    let scopes_dir = lib.join("scopes");
    std::fs::create_dir_all(stages_dir.join("p")).unwrap();
    std::fs::create_dir_all(&scopes_dir).unwrap();
    let w = |rel: &str, text: &str| std::fs::write(lib.join(rel), text).unwrap();
    w(
        "stages/p/s1.md",
        r#"---
id: s1
phase: p
collaboration: single
execution: always
agents:
  - agent: lesson-intent
    skills: [lesson-intent]
    inputs: []
    outputs: ["out/{SESSION_ID}/s1.md"]
    task_description: "做 s1"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: false
---

# s1
"#,
    );
    w(
        "stages/p/s2.md",
        r#"---
id: s2
phase: p
collaboration: single
execution: always

agents:
  - agent: lesson-researcher
    skills: [lesson-research]
    inputs: ["out/{SESSION_ID}/s1.md"]
    outputs: ["out/{SESSION_ID}/s2.md"]
    task_description: "做 s2"
retry:
  max_attempts: 1
  on_exhausted: abort_workflow
approval:
  required: false
---

# s2
"#,
    );
    w(
        "stages/p/s3.md",
        r#"---
id: s3
phase: p
collaboration: parallel
execution: always

agents:
  - agent: lesson-quizzer
    skills: [lesson-quiz]
    inputs: ["out/{SESSION_ID}/s2.md"]
    outputs: ["out/{SESSION_ID}/s3a.md"]
    task_description: "做 s3a"
  - agent: lesson-videoer
    skills: [lesson-video]
    inputs: ["out/{SESSION_ID}/s2.md"]
    outputs: ["out/{SESSION_ID}/s3b.md"]
    task_description: "做 s3b"
retry:
  max_attempts: 1
  on_exhausted: continue
approval:
  required: false
---

# s3
"#,
    );
    w(
        "scopes/lib.md",
        "---\nname: lib\ndescription: 校验夹具\ninclude_stages: [s1, s2, s3]\n---\n\n# lib\n",
    );

    let driver = common::Driver::with_tag("check-lib");
    let (rc, out, err) = driver.run(&common::svec(&[
        "check",
        "--harness",
        driver.registry.to_str().unwrap(),
        "--stages",
        stages_dir.to_str().unwrap(),
        "--scopes",
        scopes_dir.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "{err}{out}");
    let body: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(body["ok"], json!(true));
}

// ---------- install ----------

struct SourceFixture {
    root: PathBuf,
}

impl SourceFixture {
    fn new(tag: &str) -> Self {
        let root = common::unique_tmp(tag);
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::create_dir_all(root.join("skills/demo")).unwrap();
        std::fs::write(
            root.join("agents").join("a-one.md"),
            "---\nname: a-one\nmode: subagent\ndescription: 第一个\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            root.join("agents").join("a-two.md"),
            "---\nname: a-two\nmode: primary\ndescription: 二号\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            root.join("skills").join("demo").join("SKILL.md"),
            "---\nname: demo\ndescription: 示例技能\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(root.join("skills").join("demo").join("run.py"), "print(1)").unwrap();
        SourceFixture { root }
    }

    fn harness(&self) -> PathBuf {
        self.root.join("harness")
    }
}

impl Drop for SourceFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn install_ledger_gate_upgrades_and_protects() {
    let fx = SourceFixture::new("install-basic");
    let harness = fx.harness();
    let ledger = harness.join(dmaf_engine::install::INSTALL_LEDGER_FILE);

    // 首次：全部 installed，账本落盘
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();
    assert_eq!(r["installed"].as_array().unwrap().len(), 3);
    assert!(harness.join("agents/a-one.md").is_file());
    assert!(harness.join("skills/demo/SKILL.md").is_file());
    assert!(harness.join("skills/demo/run.py").is_file());
    assert!(ledger.is_file(), "安装账本写在 harness 目录根");

    // 第二次：全部 unchanged
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();
    assert_eq!(r["unchanged"].as_array().unwrap().len(), 3);
    assert_eq!(r["installed"].as_array().unwrap().len(), 0);

    // 改源、目标未改动 → updated：升级自动传播（agent 逐文件；skill 整目录替换，旧版残留文件清除）
    std::fs::write(
        fx.root.join("agents").join("a-one.md"),
        "---\nname: a-one\nmode: subagent\ndescription: 改过\n---\nbody2\n",
    )
    .unwrap();
    std::fs::write(
        fx.root.join("skills").join("demo").join("SKILL.md"),
        "---\nname: demo\ndescription: 改过的技能\n---\nbody2\n",
    )
    .unwrap();
    std::fs::remove_file(fx.root.join("skills").join("demo").join("run.py")).unwrap();
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();
    assert_eq!(r["updated"], json!(["agents/a-one.md", "skills/demo"]));
    assert!(std::fs::read_to_string(harness.join("agents/a-one.md")).unwrap().contains("改过"));
    assert!(std::fs::read_to_string(harness.join("skills/demo/SKILL.md")).unwrap().contains("改过的技能"));
    assert!(!harness.join("skills/demo/run.py").exists(), "整目录替换清除旧版残留文件");

    // 用户改过目标 → skipped（保护定制）；--force → overwritten
    std::fs::write(harness.join("agents/a-one.md"), "用户定制的 agent\n").unwrap();
    std::fs::write(
        fx.root.join("agents").join("a-one.md"),
        "---\nname: a-one\nmode: subagent\ndescription: 第三版\n---\nbody3\n",
    )
    .unwrap();
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();
    assert_eq!(r["skipped"], json!(["agents/a-one.md"]));
    assert!(std::fs::read_to_string(harness.join("agents/a-one.md")).unwrap().contains("用户定制"));
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, true).unwrap();
    assert_eq!(r["overwritten"], json!(["agents/a-one.md"]));
    assert!(std::fs::read_to_string(harness.join("agents/a-one.md")).unwrap().contains("第三版"));

    // 无账本记录的既有文件（账本缺失/用户自建同名资产）→ skipped；--force 后纳入账本，此后随源自动更新
    std::fs::remove_file(&ledger).unwrap();
    std::fs::write(harness.join("agents/a-two.md"), "用户自建的同名 agent\n").unwrap();
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();
    assert!(r["skipped"].as_array().unwrap().contains(&json!("agents/a-two.md")));
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, true).unwrap();
    assert!(r["overwritten"].as_array().unwrap().contains(&json!("agents/a-two.md")));
    std::fs::write(
        fx.root.join("agents").join("a-two.md"),
        "---\nname: a-two\nmode: primary\ndescription: 二号第二版\n---\nbody2\n",
    )
    .unwrap();
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();
    assert_eq!(r["updated"], json!(["agents/a-two.md"]));
}

#[test]
fn install_supports_one_category_level() {
    // 源：skills/<分类>/<技能>/SKILL.md → 安装按技能名拍平为 skills/<技能名>
    let root = common::unique_tmp("install-nested");
    let _g = common::TmpGuard(root.clone());
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::write(
        root.join("agents").join("a-one.md"),
        "---\nname: a-one\nmode: subagent\ndescription: 一个\n---\nbody\n",
    )
    .unwrap();
    let skill = |p: &std::path::Path, name: &str| {
        std::fs::create_dir_all(p).unwrap();
        std::fs::write(
            p.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {name} 技能\n---\nbody\n"),
        )
        .unwrap();
    };
    skill(&root.join("skills/backend/api-x"), "api-x");
    skill(&root.join("skills/frontend/ui-x"), "ui-x");
    // 分类下再嵌套（缺 SKILL.md）→ 拒装并指出分类路径
    std::fs::create_dir_all(root.join("skills/frontend/deep/inner")).unwrap();
    let harness = root.join("harness");
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &root, false, false, false, false);
    match r {
        Ok(_) => panic!("深层缺 SKILL.md 应拒装"),
        Err(dmaf_engine::install::InstallError::Invalid(problems)) => assert!(
            problems.iter().any(|p| p.contains("skills/frontend/deep")),
            "{problems:?}"
        ),
        Err(e) => panic!("unexpected: {e:?}"),
    }

    // 移除深层后：nested harness（如 opencode/cursor）保留分类子目录部署，且 registry 可发现
    std::fs::remove_dir_all(root.join("skills/frontend/deep")).unwrap();
    let harness = root.join("harness-nested");
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &root, false, false, true, false).unwrap();
    assert_eq!(r["installed"].as_array().unwrap().len(), 3);
    assert!(harness.join("skills/backend/api-x/SKILL.md").is_file());
    assert!(harness.join("skills/frontend/ui-x/SKILL.md").is_file());
    assert!(!harness.join("skills/api-x").exists(), "nested 不拍平");
    let reg = dmaf_engine::registry::load_registry(&harness, HarnessLayout::FlatMd);
    assert!(reg.skills.contains_key("api-x") && reg.skills.contains_key("ui-x"));

    // nested 孤儿清理：分类下的外来技能被 prune，报告带分类路径；清空的分类目录一并移除
    skill(&harness.join("skills/backend/orphan-x"), "orphan-x");
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &root, true, false, true, false).unwrap();
    assert_eq!(r["pruned"].as_array().unwrap(), &["skills/backend/orphan-x"]);
    assert!(harness.join("skills/backend/api-x/SKILL.md").is_file());

    // flat harness（如 claude/zcode）→ 拍平到 skills/<名>
    let harness = root.join("harness-flat");
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &root, false, false, false, false).unwrap();
    assert_eq!(r["installed"].as_array().unwrap().len(), 3);
    assert!(harness.join("skills/api-x/SKILL.md").is_file());
    assert!(harness.join("skills/ui-x/SKILL.md").is_file());
    assert!(!harness.join("skills/backend").exists() && !harness.join("skills/frontend").exists());

    // 跨分类重名 → 拒装
    skill(&root.join("skills/backend/dup"), "dup");
    skill(&root.join("skills/frontend/dup"), "dup");
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &root, false, false, false, false);
    match r {
        Ok(_) => panic!("跨分类重名应拒装"),
        Err(dmaf_engine::install::InstallError::Invalid(problems)) => assert!(
            problems.iter().any(|p| p.contains("技能名 'dup' 重复")),
            "{problems:?}"
        ),
        Err(e) => panic!("unexpected: {e:?}"),
    }
}

#[test]
fn install_prune_removes_orphans() {
    let fx = SourceFixture::new("install-prune");
    let harness = fx.harness();
    dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();

    // 源里删掉 a-two → 默认保留；--prune 清理
    std::fs::remove_file(fx.root.join("agents").join("a-two.md")).unwrap();
    dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap();
    assert!(harness.join("agents/a-two.md").is_file());
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, true, false, false, false).unwrap();
    assert_eq!(r["pruned"], json!(["agents/a-two.md"]));
    assert!(!harness.join("agents/a-two.md").is_file());
}

#[test]
fn install_check_only_writes_nothing() {
    let fx = SourceFixture::new("install-checkonly");
    let harness = fx.harness();
    let r = dmaf_engine::install::install(&harness, HarnessLayout::FlatMd, &fx.root, false, true, false, false).unwrap();
    assert_eq!(r["installed"].as_array().unwrap().len(), 3);
    assert!(!harness.join("agents").is_dir());
    assert!(!harness.join(dmaf_engine::install::INSTALL_LEDGER_FILE).exists(), "check-only 不写账本");
}

#[test]
fn install_rejects_invalid_assets() {
    let fx = SourceFixture::new("install-invalid");
    // agent 缺 name
    std::fs::write(
        fx.root.join("agents").join("bad.md"),
        "---\nmode: subagent\ndescription: x\n---\nbody\n",
    )
    .unwrap();
    let err = dmaf_engine::install::install(&fx.harness(), HarnessLayout::FlatMd, &fx.root, false, false, false, false).unwrap_err();
    match err {
        dmaf_engine::install::InstallError::Invalid(problems) => {
            assert!(problems.iter().any(|p| p.contains("name")), "{problems:?}")
        }
        other => panic!("期望 Invalid，得到 {other:?}"),
    }

    // skill name 与目录不一致
    let fx2 = SourceFixture::new("install-invalid2");
    std::fs::write(
        fx2.root.join("skills").join("demo").join("SKILL.md"),
        "---\nname: other\ndescription: x\n---\nbody\n",
    )
    .unwrap();
    let err = dmaf_engine::install::install(&fx2.harness(), HarnessLayout::FlatMd, &fx2.root, false, false, false, false).unwrap_err();
    match err {
        dmaf_engine::install::InstallError::Invalid(problems) => {
            assert!(problems.iter().any(|p| p.contains("不一致")))
        }
        other => panic!("期望 Invalid，得到 {other:?}"),
    }

    // 非法 mode
    let fx3 = SourceFixture::new("install-invalid3");
    std::fs::write(
        fx3.root.join("agents").join("a-one.md"),
        "---\nname: a-one\nmode: boss\ndescription: x\n---\nbody\n",
    )
    .unwrap();
    let err = dmaf_engine::install::install(&fx3.harness(), HarnessLayout::FlatMd, &fx3.root, false, false, false, false).unwrap_err();
    match err {
        dmaf_engine::install::InstallError::Invalid(problems) => {
            assert!(problems.iter().any(|p| p.contains("mode")))
        }
        other => panic!("期望 Invalid，得到 {other:?}"),
    }
}

#[test]
fn compile_rejects_undeclared_template_var() {
    // 词表纯化：模板变量必须 ∈ scope 声明集 ∪ 引擎保留变量；样例语义不再被引擎词表放行
    let root = temp_workflow_root("undeclared-var");
    write_temp_stage(
        &root,
        "a",
        "s2",
        "id: s2
phase: a
collaboration: single
execution: always

agents:
  - agent: lesson-outliner
    skills: [lesson-outline]
    inputs: [\"out/{knowledge_point}-s1.md\"]
    outputs: [\"out/{undeclared}-s2.md\"]
",
    );
    let err = compile::compile(&root.join("stages"), &root.join("scopes"), None).unwrap_err();
    assert!(
        err.problems
            .iter()
            .any(|p| p.contains("undeclared") && p.contains("未知变量")),
        "{:?}",
        err.problems
    );
}

// ---------- harness 名称解析 ----------

#[test]
fn harness_name_resolves_to_project_dir() {
    use dmaf_engine::registry::harness_root;
    let dir = |n: &str| harness_root(n).unwrap().dir;
    let layout = |n: &str| harness_root(n).unwrap().layout;
    assert_eq!(dir("opencode"), PathBuf::from(".opencode"));
    assert_eq!(dir("claude"), PathBuf::from(".claude"));
    assert_eq!(dir("claude-code"), PathBuf::from(".claude"));
    assert_eq!(dir("zcode"), PathBuf::from(".zcode"));
    assert_eq!(dir("cursor"), PathBuf::from(".cursor"));
    assert_eq!(dir("gemini"), PathBuf::from(".gemini"));
    assert_eq!(dir("gemini-cli"), PathBuf::from(".gemini"));
    assert_eq!(layout("opencode"), HarnessLayout::FlatMd);
    // codex/dsh：TOML 布局
    assert_eq!(dir("codex"), PathBuf::from(".codex"));
    assert_eq!(dir("dsh"), PathBuf::from(".dsh"));
    assert_eq!(layout("codex"), HarnessLayout::TomlAgents);
    assert_eq!(layout("dsh"), HarnessLayout::TomlAgents);
    // 路径形态透传（含分隔符或以 . 开头），按 flat-md
    assert_eq!(dir(".opencode"), PathBuf::from(".opencode"));
    assert_eq!(layout(".opencode"), HarnessLayout::FlatMd);
    assert_eq!(dir("tmp/h"), PathBuf::from("tmp/h"));
    assert_eq!(dir("C:\\x\\y"), PathBuf::from("C:\\x\\y"));
    // 未知裸名报错（防笔误静默新建陌生目录）
    let e = harness_root("claud").unwrap_err();
    assert!(e.contains("未知 harness") && e.contains("codex"), "{e}");
}

#[test]
fn install_and_check_reject_unknown_bare_harness_name() {
    let driver = common::Driver::with_tag("harness-bad-name");
    let (rc, _, err) = driver.run(&common::svec(&["install", "--harness", "claud"]));
    assert_eq!(rc, 2, "{err}");
    assert!(err.contains("未知 harness 'claud'"), "{err}");

    let (rc, _, err) = driver.run(&common::svec(&[
        "check",
        "--harness",
        "claud",
        "--workflow",
        driver.workflow.to_str().unwrap(),
    ]));
    assert_eq!(rc, 2, "{err}");
    assert!(err.contains("未知 harness 'claud'"), "{err}");
}

#[test]
fn named_harness_resolves_relative_to_cwd() {
    // 名称解析为 CWD（项目根）下的隐藏目录：子进程指定临时 CWD，不扰动测试进程的工作目录
    let root = common::unique_tmp("harness-cwd");
    std::fs::create_dir_all(root.join("dmaf/agents")).unwrap();
    std::fs::create_dir_all(root.join("dmaf/skills/demo")).unwrap();
    std::fs::write(
        root.join("dmaf/agents/a-one.md"),
        "---\nname: a-one\nmode: subagent\ndescription: 第一个\n---\nbody\n",
    )
    .unwrap();
    std::fs::write(
        root.join("dmaf/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: 示例技能\n---\nbody\n",
    )
    .unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_dmaf"))
        .args(["install", "--harness", "cursor"])
        .current_dir(&root)
        .output()
        .expect("启动 dmaf 子进程");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let body: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("install 输出不是 JSON");
    assert_eq!(body["harness"], json!(".cursor"));
    // 资产确实复制到了名称映射的 .cursor（相对 CWD）
    assert!(root.join(".cursor/agents/a-one.md").is_file());
    assert!(root.join(".cursor/skills/demo/SKILL.md").is_file());
    let _ = std::fs::remove_dir_all(&root);
}

// ---------- toml-agents 布局（codex / dsh） ----------

#[test]
fn render_agent_toml_deterministic_and_lossless() {
    use dmaf_engine::install::render_agent_toml;
    let body = "你是资深评审。\n\n## 规则\n1. 引用 \"文件:行号\"。\n2. 路径 C:\\x 不需转义。\n";
    let a = render_agent_toml("rev-1", "subagent", "评审 \"严格\"", body).unwrap();
    let b = render_agent_toml("rev-1", "subagent", "评审 \"严格\"", body).unwrap();
    assert_eq!(a, b, "同输入必须逐字节相同（确定性）");
    assert!(a.starts_with("# dmaf-mode: subagent\n"));
    // TOML 可解析且正文无损（literal 多行字符串零转义）
    let v: toml::Value = a.parse().expect("渲染结果必须是合法 TOML");
    assert_eq!(v["name"].as_str().unwrap(), "rev-1");
    assert_eq!(v["description"].as_str().unwrap(), "评审 \"严格\"");
    assert_eq!(v["developer_instructions"].as_str().unwrap(), body);
    // 正文含 ''' 定界符 → 拒绝转换（由调用方拒装）
    assert!(render_agent_toml("x", "subagent", "d", "含 ''' 序列").is_err());
    // 正文无尾换行时补齐，闭合定界符独占一行
    let c = render_agent_toml("x", "subagent", "d", "无尾换行").unwrap();
    assert!(c.ends_with("无尾换行\n'''\n"), "{c}");
}

#[test]
fn install_toml_layout_converts_and_is_idempotent() {
    let fx = SourceFixture::new("install-toml");
    let harness = fx.harness();

    // 首次：agent 转 TOML，skill 原样目录复制
    let r =
        dmaf_engine::install::install(&harness, HarnessLayout::TomlAgents, &fx.root, false, false, false, false)
            .unwrap();
    assert_eq!(r["layout"], json!("toml-agents"));
    assert_eq!(r["installed"].as_array().unwrap().len(), 3);
    let a_one = std::fs::read_to_string(harness.join("agents/a-one.toml")).unwrap();
    let expected =
        dmaf_engine::install::render_agent_toml("a-one", "subagent", "第一个", "body\n").unwrap();
    assert_eq!(a_one, expected);
    assert!(!harness.join("agents/a-one.md").exists());
    assert!(harness.join("skills/demo/SKILL.md").is_file());

    // 第二次：渲染结果逐字节一致 → 全 unchanged
    let r =
        dmaf_engine::install::install(&harness, HarnessLayout::TomlAgents, &fx.root, false, false, false, false)
            .unwrap();
    assert_eq!(r["unchanged"].as_array().unwrap().len(), 3);
    assert_eq!(r["installed"].as_array().unwrap().len(), 0);

    // 改源、目标未改动 → updated（账本哈希门：已存在 toml 随源升级自动重渲染）
    std::fs::write(
        fx.root.join("agents").join("a-one.md"),
        "---\nname: a-one\nmode: subagent\ndescription: 改过\n---\nbody2\n",
    )
    .unwrap();
    let r =
        dmaf_engine::install::install(&harness, HarnessLayout::TomlAgents, &fx.root, false, false, false, false)
            .unwrap();
    assert_eq!(r["updated"].as_array().unwrap(), &["agents/a-one.toml"]);
    assert!(std::fs::read_to_string(harness.join("agents/a-one.toml")).unwrap().contains("改过"));

    // prune：源删 a-two → 只清孤儿 toml，用户自建文件不动
    std::fs::remove_file(fx.root.join("agents").join("a-two.md")).unwrap();
    std::fs::write(harness.join("agents").join("user-note.txt"), "用户的").unwrap();
    let r =
        dmaf_engine::install::install(&harness, HarnessLayout::TomlAgents, &fx.root, true, false, false, false)
            .unwrap();
    assert_eq!(r["pruned"], json!(["agents/a-two.toml"]));
    assert!(!harness.join("agents/a-two.toml").exists());
    assert!(harness.join("agents/user-note.txt").is_file());
}

#[test]
fn toml_registry_readback_restores_mode() {
    use dmaf_engine::registry::{load_registry, validate_refs};
    let fx = SourceFixture::new("toml-registry");
    let harness = fx.harness();
    dmaf_engine::install::install(&harness, HarnessLayout::TomlAgents, &fx.root, false, false, false, false)
        .unwrap();

    let reg = load_registry(&harness, HarnessLayout::TomlAgents);
    assert_eq!(reg.agents.len(), 2);
    // mode 从 # dmaf-mode: 注释恢复
    for (name, want) in [("a-one", "subagent"), ("a-two", "primary")] {
        let meta = serde_yaml::to_string(&reg.agents[name].meta).unwrap();
        assert!(meta.contains(&format!("mode: {want}")), "{name}: {meta}");
    }

    // 手写 toml（无 dmaf-mode 注释）→ 默认 subagent 放行
    std::fs::write(
        harness.join("agents").join("hand-written.toml"),
        "name = \"hand-written\"\ndescription = \"手写\"\n",
    )
    .unwrap();
    let reg = load_registry(&harness, HarnessLayout::TomlAgents);
    let meta = serde_yaml::to_string(&reg.agents["hand-written"].meta).unwrap();
    assert!(meta.contains("mode: subagent"), "{meta}");

    // validate_refs：primary agent 被引用 → 只能派发 subagent
    let doc = json!({"workflow": {"id": "wf-toml", "vars": {}, "stages": [
        {"id": "s1", "collaboration": "single", "execution": "always",
         "agents": [{"agent": "a-two", "skills": [], "inputs": [], "outputs": ["out/x.md"]}]}
    ]}});
    let path = fx.root.join("wf-toml.yaml");
    std::fs::write(&path, serde_yaml::to_string(&doc).unwrap()).unwrap();
    let wf = load_workflow(&path).unwrap();
    let problems = validate_refs(&wf, &reg);
    assert!(problems.iter().any(|p| p.contains("subagent")), "{problems:?}");
}

#[test]
fn dsh_end_to_end_install_and_check() {
    // dsh 命名 harness 端到端：安装（转 TOML）→ 按名称 check（读回 TOML 注册表）
    let root = common::unique_tmp("dsh-e2e");
    std::fs::create_dir_all(root.join("dmaf/agents")).unwrap();
    std::fs::create_dir_all(root.join("dmaf/skills/demo")).unwrap();
    std::fs::write(
        root.join("dmaf/agents/a-one.md"),
        "---\nname: a-one\nmode: subagent\ndescription: 第一个\n---\nbody\n",
    )
    .unwrap();
    std::fs::write(
        root.join("dmaf/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: 示例技能\n---\nbody\n",
    )
    .unwrap();
    let bin = env!("CARGO_BIN_EXE_dmaf");
    let run = |args: &[&str]| {
        let o = std::process::Command::new(bin)
            .args(args)
            .current_dir(&root)
            .output()
            .expect("启动 dmaf 子进程");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (rc, out, err) = run(&["install", "--harness", "dsh"]);
    assert_eq!(rc, 0, "{err}{out}");
    assert!(root.join(".dsh/agents/a-one.toml").is_file());
    assert!(root.join(".dsh/skills/demo/SKILL.md").is_file());

    std::fs::write(
        root.join("wf.yaml"),
        "workflow:\n  id: wf-dsh\n  stages:\n    - id: s1\n      collaboration: single\n      execution: always\n      agents:\n        - agent: a-one\n          skills: [demo]\n          inputs: []\n          outputs: [\"out/x.md\"]\n",
    )
    .unwrap();
    let (rc, out, err) = run(&["check", "--harness", "dsh", "--workflow", "wf.yaml"]);
    assert_eq!(rc, 0, "{err}{out}");
    let body: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(body["harness"], json!(".dsh"));
    let _ = std::fs::remove_dir_all(&root);
}
