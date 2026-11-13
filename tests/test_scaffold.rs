//! dmaf new 脚手架测试（greenfield 建新项目 / brownfield 补齐已有仓库）。
//!
//! 新布局（v0.4）：定义层收拢在 dmaf/ 下；.dmaf/ 为引擎运行独占区（gitignore）。

mod common;

use dmaf_engine::scaffold;
use serde_json::json;

fn new_ok(dir: &std::path::Path, template: &str) -> serde_json::Value {
    scaffold::new_project(template, dir, false).unwrap()
}

#[test]
fn unknown_template_rejected() {
    let root = common::unique_tmp("new-unknown");
    let err = scaffold::new_project("ghost", &root, false).unwrap_err();
    match err {
        scaffold::ScaffoldError::Usage(m) => {
            assert!(m.contains("未知模板") && m.contains("minimal"))
        }
        other => panic!("期望 Usage，得到 {other:?}"),
    }
}

#[test]
fn new_merges_into_existing_repo_never_overwrites() {
    // brownfield：已有仓库（含自己的代码、agent 与 .gitignore）→ 只补缺失，用户内容不动
    let root = common::unique_tmp("new-merge");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
    std::fs::create_dir_all(root.join("dmaf")).unwrap();
    std::fs::write(root.join("dmaf/hooks.yaml"), "# user-owned hooks").unwrap();

    let r = scaffold::new_project("minimal", &root, false).unwrap();
    // 已存在的：hooks.yaml unchanged 且内容不动；.gitignore 走追加
    assert!(
        r["unchanged"]
            .as_array()
            .unwrap()
            .contains(&json!("dmaf/hooks.yaml"))
    );
    assert_eq!(
        std::fs::read_to_string(root.join("dmaf/hooks.yaml")).unwrap(),
        "# user-owned hooks"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("main.rs")).unwrap(),
        "fn main() {}"
    );
    // .gitignore：用户条目保留在前，dmaf 条目追加在后
    let gi = std::fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(gi.starts_with("target/\n"), "{gi}");
    assert!(gi.contains(".dmaf/"));
    // 缺失的模板文件全部创建
    assert!(root.join("dmaf/manifest.yaml").is_file());
    assert_eq!(r["created"].as_array().unwrap().len(), 15);
}

#[test]
fn new_is_idempotent_on_second_run() {
    let root = common::unique_tmp("new-idempotent");
    let dir = root.join("proj");
    scaffold::new_project("minimal", &dir, false).unwrap();
    let r2 = scaffold::new_project("minimal", &dir, false).unwrap();
    assert_eq!(r2["created"].as_array().unwrap().len(), 0);
    assert_eq!(r2["unchanged"].as_array().unwrap().len(), 16);
    assert_eq!(r2["gitignore_appended"].as_array().unwrap().len(), 0);
}

#[test]
fn new_creates_minimal_project_layout() {
    let root = common::unique_tmp("new-minimal");
    let dir = root.join("proj");
    let report = new_ok(&dir, "minimal");
    // 6 模板文件（.gitkeep 占位 + hooks.yaml + dmaf/README）+ 指南 + 使用者 README + manifest
    // + 内嵌定义技能 7 文件（create-workflow：问答式 + 文档式双模式）
    assert_eq!(report["created"].as_array().unwrap().len(), 16);
    assert!(dir.join("README.md").is_file());
    assert!(dir.join("dmaf/manifest.yaml").is_file());
    assert!(dir.join("dmaf/docs/流程定义指南.md").is_file());
    assert!(dir.join("dmaf/hooks.yaml").is_file());
}

#[test]
fn new_delivers_authoring_skills_into_dmaf_skills() {
    let root = common::unique_tmp("new-skills");
    let dir = root.join("proj");
    let report = new_ok(&dir, "minimal");
    let created = report["created"].as_array().unwrap();
    // 定义技能整目录落盘（dmaf/skills/ 直属目录，install 可直接部署到 harness）
    let expected: Vec<String> = [
        "assets/scope-template.md",
        "assets/stage-template.md",
        "references/doc-example.md",
        "references/examples.md",
        "references/field-reference.md",
        "references/mapping-guide.md",
        "SKILL.md",
    ]
    .iter()
    .map(|f| format!("dmaf/skills/create-workflow/{f}"))
    .collect();
    for rel in &expected {
        assert!(created.contains(&json!(rel)), "缺 {rel}: {created:?}");
        assert!(dir.join(rel).is_file(), "未落盘 {rel}");
    }
    // 内容来自内嵌源（与 tools/skills 一致）：SKILL.md frontmatter 可解析、双模式说明在位
    let skill = std::fs::read_to_string(dir.join("dmaf/skills/create-workflow/SKILL.md")).unwrap();
    assert!(skill.starts_with("---\nname: create-workflow\n"), "{skill}");
    assert!(skill.contains("Document mode"), "SKILL.md 应含文档式模式说明");
    // 用户手改的技能文件 dmaf new 不覆盖（brownfield 合并语义）
    std::fs::write(dir.join("dmaf/skills/create-workflow/SKILL.md"), "用户自持").unwrap();
    let r2 = scaffold::new_project("minimal", &dir, false).unwrap();
    assert!(r2["unchanged"].as_array().unwrap().contains(&json!(
        "dmaf/skills/create-workflow/SKILL.md"
    )));
    assert_eq!(
        std::fs::read_to_string(dir.join("dmaf/skills/create-workflow/SKILL.md")).unwrap(),
        "用户自持"
    );
}

#[test]
fn new_force_overwrites_template_sourced_files() {
    let root = common::unique_tmp("new-force");
    let dir = root.join("proj");
    scaffold::new_project("minimal", &dir, false).unwrap();
    std::fs::write(dir.join("dmaf/README.md"), "被改坏的占位").unwrap();

    let r = scaffold::new_project("minimal", &dir, true).unwrap();
    assert!(r["overwritten"].as_array().unwrap().contains(&json!("dmaf/README.md")));
    assert!(!std::fs::read_to_string(dir.join("dmaf/README.md")).unwrap().contains("被改坏"));
}

#[test]
fn templates_registry_has_only_minimal() {
    let mut names = scaffold::available_templates();
    names.sort();
    assert_eq!(names, vec!["minimal"]);
}

#[test]
fn manifest_engine_range_matches_current_engine_and_lists_all_harnesses() {
    let root = common::unique_tmp("new-manifest");
    let dir = root.join("proj");
    scaffold::new_project("minimal", &dir, false).unwrap();
    let text = std::fs::read_to_string(dir.join("dmaf/manifest.yaml")).unwrap();
    // engine 范围占位符已按当前二进制渲染（doctor 兼容性判定真实路径验证）
    assert!(!text.contains("@ENGINE_RANGE@"), "{text}");
    let m = dmaf_engine::manifest::load(&dir)
        .unwrap()
        .expect("manifest 已创建");
    assert_eq!(
        dmaf_engine::manifest::engine_matches(&m.engine, env!("CARGO_PKG_VERSION")),
        Ok(true),
        "engine 范围 '{}' 须兼容当前二进制 {}",
        m.engine,
        env!("CARGO_PKG_VERSION")
    );
    // harnesses：引擎支持的全部 harness 规范名（registry.rs KNOWN_HARNESSES 同名单）
    assert_eq!(
        m.harnesses,
        vec!["opencode", "claude", "zcode", "cursor", "gemini", "codex", "dsh"]
    );
}
