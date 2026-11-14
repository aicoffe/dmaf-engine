//! dmaf add / dmaf push（Phase B）验收：合并核心（账本哈希门）、骨架补齐、
//! packages.yaml 账本、合入后重编译、串联 harness 部署、push 验证门、git 克隆。
//! 语义依据 docs/Add命令与工作流分发实施方案.md §4。

mod common;

use std::path::{Path, PathBuf};

use common::{lock_cwd, svec, CwdGuard, Driver};
use dmaf_engine::package::{add_package, merge_package, push_package, MergeOptions, PackageError};
use serde_json::{json, Value};

const AGENT_V1: &str = "---\nname: pkg-agent\nmode: subagent\ndescription: 包 agent\n---\nbody v1\n";
const AGENT_V2: &str = "---\nname: pkg-agent\nmode: subagent\ndescription: 包 agent\n---\nbody v2\n";
const STAGE_MD: &str = "---\nid: p1\nphase: demo\ncollaboration: single\n\nagents:\n  - agent: pkg-agent\n    skills: []\n    inputs: []\n    outputs: [\"out/p1.md\"]\n    task_description: \"包任务\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\nexecution: always\n---\n\n# p1\n";
const SCOPE_MD: &str = "---\nname: pkg-scope\ndescription: 包 scope\ninclude_stages: [p1]\n---\n\n# pkg-scope\n";

fn opts(command: &'static str) -> MergeOptions {
    MergeOptions {
        check_only: false,
        chain_install: false,
        source_label: "test-src".into(),
        tag: None,
        digest: None,
        command,
        force: false,
    }
}

/// 构造一个合法工作流包源（含工程自属槽位与 compiled 垃圾，用于验证排除逻辑）。
fn make_source(tag: &str) -> PathBuf {
    let root = common::unique_tmp(tag);
    let w = |rel: &str, content: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    };
    w("dmaf/manifest.yaml", "name: pkg\nversion: 0.1.0\ndescription: 测试包\nengine: \">=0.1\"\nharnesses: []\n");
    w("dmaf/agents/pkg-agent.md", AGENT_V1);
    w("dmaf/skills/pkg-skill/SKILL.md", "---\nname: pkg-skill\ndescription: 包技能\n---\nbody\n");
    w("dmaf/workflows/stages/demo/p1.md", STAGE_MD);
    w("dmaf/workflows/scopes/pkg-scope.md", SCOPE_MD);
    w("dmaf/hooks.yaml", "hooks: []\n");
    w("dmaf/harness/opencode/opencode.json", "{\"default_agent\": \"dmaf-conductor\"}\n");
    // 工程自属槽位与编译产物：包携带但合入时必须被排除
    w("dmaf/README.md", "包说明——工程自属槽位\n");
    w("dmaf/docs/流程定义指南.md", "指南——工程自属槽位\n");
    w("dmaf/compiled/stage-graph.json", "垃圾——不应被合入\n");
    // 引擎下发的定义技能（目录形态槽位）：包携带也必须被排除；目录边界前缀匹配，
    // 同名前缀的他名技能（create-workflow-from-doc / create-workflow-pro）是普通包技能，照常合入。
    // 槽位文件也要合法 frontmatter——串联 install 会校验 dmaf/skills 全部技能。
    w("dmaf/skills/create-workflow/SKILL.md", "---\nname: create-workflow\ndescription: 槽位技能——不应被合入\n---\nbody\n");
    w("dmaf/skills/create-workflow-from-doc/SKILL.md", "---\nname: create-workflow-from-doc\ndescription: 普通包技能\n---\nbody\n");
    w("dmaf/skills/create-workflow-pro/SKILL.md", "---\nname: create-workflow-pro\ndescription: 普通包技能\n---\nbody\n");
    root
}

/// 在源上执行真实编译（写 dmaf/compiled/），push 验证门需要。
fn compile_source(src: &Path) {
    let compiled = dmaf_engine::compile::compile(
        &src.join("dmaf/workflows/stages"),
        &src.join("dmaf/workflows/scopes"),
        None,
    )
    .unwrap();
    let out = src.join("dmaf/compiled");
    std::fs::create_dir_all(&out).unwrap();
    for (file, value) in [
        (dmaf_engine::compile::graph_file_name(), &compiled.stage_graph),
        (dmaf_engine::compile::grid_file_name(), &compiled.scope_grid),
    ] {
        std::fs::write(out.join(file), dmaf_engine::compile::serialize_artifact(value)).unwrap();
    }
}

fn write_agent(src: &Path, content: &str) {
    std::fs::write(src.join("dmaf/agents/pkg-agent.md"), content).unwrap();
}

#[test]
fn add_greenfield_bootstraps_merges_and_compiles() {
    let src = make_source("pkg-src-a");
    let target = common::unique_tmp("pkg-tgt-a");
    let r = merge_package(&src, &target, &opts("add")).unwrap();

    assert_eq!(r["skeleton"]["bootstrap"], json!(true));
    // created 覆盖包资产；工程自属槽位与 compiled 被排除
    let created = r["created"].as_array().unwrap();
    for must in [
        "dmaf/agents/pkg-agent.md",
        "dmaf/skills/pkg-skill/SKILL.md",
        "dmaf/workflows/stages/demo/p1.md",
        "dmaf/workflows/scopes/pkg-scope.md",
        "dmaf/hooks.yaml",
        "dmaf/harness/opencode/opencode.json",
    ] {
        assert!(created.contains(&json!(must)), "缺 {must}: {created:?}");
    }
    assert!(!created.contains(&json!("dmaf/README.md")));
    assert!(!created.contains(&json!("dmaf/manifest.yaml")));
    assert!(!created.contains(&json!("dmaf/docs/流程定义指南.md")));
    assert!(!created.contains(&json!("dmaf/compiled/stage-graph.json")));
    assert!(!created.contains(&json!("dmaf/skills/create-workflow/SKILL.md")));
    assert!(created.contains(&json!("dmaf/skills/create-workflow-from-doc/SKILL.md")));
    assert!(created.contains(&json!("dmaf/skills/create-workflow-pro/SKILL.md")));
    assert!(!target.join("dmaf/README.md").exists());
    assert!(!target.join("dmaf/manifest.yaml").exists());
    assert!(!target.join("dmaf/skills/create-workflow").exists());
    // 合入后重编译：产物为联合编译结果（版本字段来自当前编译器）
    let graph: Value =
        serde_json::from_str(&std::fs::read_to_string(target.join("dmaf/compiled/stage-graph.json")).unwrap())
            .unwrap();
    assert_eq!(graph["version"], json!(dmaf_engine::compile::GRAPH_VERSION));
    assert_eq!(graph["stages"].as_array().unwrap().len(), 1);
    // 账本：包属文件 + 哈希
    let ledger_text = std::fs::read_to_string(target.join("dmaf/packages.yaml")).unwrap();
    let ledger: Value = serde_yaml::from_str(&ledger_text).unwrap();
    let entry = &ledger["packages"][0];
    assert_eq!(entry["name"], json!("pkg"));
    assert_eq!(entry["source"], json!("test-src"));
    assert!(entry["files"].as_array().unwrap().len() >= 6);
    // 骨架补齐追加 .gitignore（忽略引擎运行区与 harness 派生物；工作成果 output/ 绝不入忽略清单）
    let gi = std::fs::read_to_string(target.join(".gitignore")).unwrap();
    assert!(gi.contains(".dmaf/"));
    assert!(!gi.contains("output"), "output/ 是用户命名空间，不得出现在忽略清单");
    assert!(!gi.contains("dmaf-output"), "dmaf-output/ 是工作成果，不得出现在忽略清单");
    // opts 默认不串联 → skipped 标注旗标（串联行为由专测覆盖）
    assert_eq!(r["install"]["skipped"], json!("--no-install"));
}

#[test]
fn add_rerun_is_idempotent() {
    let src = make_source("pkg-src-idem");
    let target = common::unique_tmp("pkg-tgt-idem");
    merge_package(&src, &target, &opts("add")).unwrap();
    let r2 = merge_package(&src, &target, &opts("add")).unwrap();
    assert_eq!(r2["created"].as_array().unwrap().len(), 0);
    assert_eq!(r2["updated"].as_array().unwrap().len(), 0);
    assert_eq!(r2["skeleton"]["bootstrap"], json!(false));
    assert!(r2["unchanged"].as_array().unwrap().len() >= 6);
}

#[test]
fn add_upgrade_updates_unmodified_files() {
    let src = make_source("pkg-src-upg");
    let target = common::unique_tmp("pkg-tgt-upg");
    merge_package(&src, &target, &opts("add")).unwrap();
    write_agent(&src, AGENT_V2);
    let r = merge_package(&src, &target, &opts("add")).unwrap();
    assert_eq!(r["updated"], json!(["dmaf/agents/pkg-agent.md"]));
    assert!(
        std::fs::read_to_string(target.join("dmaf/agents/pkg-agent.md")).unwrap().contains("body v2"),
        "目标侧未改动的包属文件应被更新"
    );
    // 账本哈希同步刷新
    let ledger: Value =
        serde_yaml::from_str(&std::fs::read_to_string(target.join("dmaf/packages.yaml")).unwrap()).unwrap();
    let files = ledger["packages"][0]["files"].as_array().unwrap();
    let agent = files.iter().find(|f| f["path"] == json!("dmaf/agents/pkg-agent.md")).unwrap();
    let expect = dmaf_engine::audit::sha256_hex(AGENT_V2.as_bytes());
    assert_eq!(agent["hash"], json!(expect));
}

#[test]
fn add_conflict_when_target_modified() {
    let src = make_source("pkg-src-conf");
    let target = common::unique_tmp("pkg-tgt-conf");
    merge_package(&src, &target, &opts("add")).unwrap();
    // 用户改目标 + 包也改同一文件 → 冲突，整体不落盘
    std::fs::write(target.join("dmaf/agents/pkg-agent.md"), "用户的本地修改\n").unwrap();
    write_agent(&src, AGENT_V2);
    let err = merge_package(&src, &target, &opts("add")).unwrap_err();
    match err {
        PackageError::Conflict(list) => {
            assert_eq!(list, vec!["dmaf/agents/pkg-agent.md".to_string()]);
        }
        other => panic!("期望 Conflict，得到 {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(target.join("dmaf/agents/pkg-agent.md")).unwrap(),
        "用户的本地修改\n",
        "冲突时不落盘，用户内容不动"
    );
}

#[test]
fn add_force_overwrites_modified_target_and_rerecords_ledger() {
    let src = make_source("pkg-src-force");
    let target = common::unique_tmp("pkg-tgt-force");
    merge_package(&src, &target, &opts("add")).unwrap();
    // 用户改目标 + 包也改同一文件
    std::fs::write(target.join("dmaf/agents/pkg-agent.md"), "用户的本地修改\n").unwrap();
    write_agent(&src, AGENT_V2);
    // 非 force：仍冲突（dry-run 同样拒绝，不写任何文件）
    let err = merge_package(&src, &target, &opts("add")).unwrap_err();
    assert!(matches!(err, PackageError::Conflict(_)));
    let mut force_opts = opts("add");
    force_opts.force = true;
    let dry = merge_package(&src, &target, &MergeOptions { check_only: true, ..force_opts.clone() }).unwrap();
    assert_eq!(dry["overwritten"], json!(["dmaf/agents/pkg-agent.md"]));
    assert_eq!(dry["force"], json!(true));
    assert_eq!(
        std::fs::read_to_string(target.join("dmaf/agents/pkg-agent.md")).unwrap(),
        "用户的本地修改\n",
        "dry-run 不落盘，用户内容仍在"
    );
    // force：覆盖 + 账本按新哈希重记 + 后续无 force 重跑无冲突（账本已对新哈希）
    let r = merge_package(&src, &target, &force_opts).unwrap();
    assert_eq!(r["overwritten"], json!(["dmaf/agents/pkg-agent.md"]));
    assert_eq!(
        std::fs::read_to_string(target.join("dmaf/agents/pkg-agent.md")).unwrap(),
        AGENT_V2,
        "--force 覆盖为包侧新内容"
    );
    let ledger: Value =
        serde_yaml::from_str(&std::fs::read_to_string(target.join("dmaf/packages.yaml")).unwrap()).unwrap();
    let files = ledger["packages"][0]["files"].as_array().unwrap();
    let agent = files.iter().find(|f| f["path"] == json!("dmaf/agents/pkg-agent.md")).unwrap();
    assert_eq!(agent["hash"], json!(dmaf_engine::audit::sha256_hex(AGENT_V2.as_bytes())));
    let r2 = merge_package(&src, &target, &opts("add")).unwrap();
    assert_eq!(r2["updated"].as_array().unwrap().len(), 0);
    assert_eq!(
        std::fs::read_to_string(target.join("dmaf/agents/pkg-agent.md")).unwrap(),
        AGENT_V2,
        "账本已按新哈希登记，后续重跑走 unchanged/updated 而非冲突"
    );
}

#[test]
fn add_rejects_invalid_sources_and_same_dir() {
    let empty = common::unique_tmp("pkg-src-empty");
    let target = common::unique_tmp("pkg-tgt-x");
    assert!(matches!(
        merge_package(&empty, &target, &opts("add")),
        Err(PackageError::Usage(_))
    ));
    // 有 dmaf/ 但缺 manifest
    let no_manifest = common::unique_tmp("pkg-src-nomf");
    std::fs::create_dir_all(no_manifest.join("dmaf/agents")).unwrap();
    assert!(matches!(
        merge_package(&no_manifest, &target, &opts("add")),
        Err(PackageError::Usage(_))
    ));
    // 源 = 目标
    let src = make_source("pkg-src-self");
    assert!(matches!(
        merge_package(&src, &src, &opts("add")),
        Err(PackageError::Usage(_))
    ));
}

#[test]
fn add_reports_no_longer_shipped_without_deleting() {
    let src = make_source("pkg-src-nls");
    let extra = src.join("dmaf/agents/pkg-extra.md");
    std::fs::write(&extra, "---\nname: pkg-extra\nmode: subagent\ndescription: 将被移除\n---\nx\n").unwrap();
    let target = common::unique_tmp("pkg-tgt-nls");
    merge_package(&src, &target, &opts("add")).unwrap();
    std::fs::remove_file(&extra).unwrap();
    let r = merge_package(&src, &target, &opts("add")).unwrap();
    assert_eq!(r["no_longer_shipped"], json!(["dmaf/agents/pkg-extra.md"]));
    assert!(target.join("dmaf/agents/pkg-extra.md").exists(), "v1 不删除旧包属文件");
}

#[test]
fn chain_install_runs_for_single_detected_harness() {
    let src = make_source("pkg-src-inst");
    let target = common::unique_tmp("pkg-tgt-inst");
    std::fs::create_dir_all(target.join(".opencode")).unwrap();
    let mut o = opts("add");
    o.chain_install = true;
    let r = merge_package(&src, &target, &o).unwrap();
    assert_eq!(r["install"]["harness_name"], json!("opencode"));
    assert!(target.join(".opencode/agents/pkg-agent.md").is_file());
    // 入口配置随串联落位（落在 harness 目录内，不落项目根）
    assert_eq!(
        std::fs::read_to_string(target.join(".opencode/opencode.json")).unwrap(),
        "{\"default_agent\": \"dmaf-conductor\"}\n"
    );
    assert!(!target.join("opencode.json").exists());
}

#[test]
fn chain_install_lists_multiple_without_error() {
    let src = make_source("pkg-src-multi");
    let target = common::unique_tmp("pkg-tgt-multi");
    std::fs::create_dir_all(target.join(".opencode")).unwrap();
    std::fs::create_dir_all(target.join(".claude")).unwrap();
    let mut o = opts("add");
    o.chain_install = true;
    let r = merge_package(&src, &target, &o).unwrap();
    let skipped = r["install"]["skipped"].as_str().unwrap();
    assert!(skipped.contains("多个") && skipped.contains("claude"), "{skipped}");
    assert!(!target.join(".opencode/agents/pkg-agent.md").exists(), "未显式指定时不写入");
}

#[test]
fn no_install_flag_skips_chain() {
    let src = make_source("pkg-src-noinst");
    let target = common::unique_tmp("pkg-tgt-noinst");
    std::fs::create_dir_all(target.join(".opencode")).unwrap();
    let mut o = opts("add");
    o.chain_install = false;
    let r = merge_package(&src, &target, &o).unwrap();
    assert_eq!(r["install"]["skipped"], json!("--no-install"));
}

#[test]
fn check_only_writes_nothing() {
    let src = make_source("pkg-src-dry");
    let target = common::unique_tmp("pkg-tgt-dry");
    let mut o = opts("add");
    o.check_only = true;
    let r = merge_package(&src, &target, &o).unwrap();
    assert_eq!(r["check_only"], json!(true));
    assert!(r["created"].as_array().unwrap().len() >= 6, "报告仍列出将创建项");
    assert!(!target.join("dmaf").exists(), "dry-run 不建任何文件");
}

// ---------- push ----------

#[test]
fn push_requires_manifest_and_existing_target() {
    let src = common::unique_tmp("push-src-nomf");
    std::fs::create_dir_all(src.join("dmaf/agents")).unwrap();
    let target = common::unique_tmp("push-tgt-ok");
    std::fs::create_dir_all(&target).unwrap();
    assert!(matches!(
        push_package(&src, &target, &opts("push")),
        Err(PackageError::Usage(m)) if m.contains("manifest")
    ));
    // 目标不存在（unique_tmp 会建目录，这里手工取一个未创建的路径）
    let good = make_source("push-src-good");
    let absent = good.parent().unwrap().join("push-tgt-truly-absent");
    assert!(matches!(
        push_package(&good, &absent, &opts("push")),
        Err(PackageError::Usage(m)) if m.contains("不存在")
    ));
}

#[test]
fn push_requires_fresh_artifacts_then_delivers() {
    let src = make_source("push-src-gate");
    let target = common::unique_tmp("push-tgt-gate");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("宿主文件.txt"), "宿主已有内容\n").unwrap();

    // 未编译 → 产物漂移拒绝（编译读不到入库产物）
    assert!(matches!(
        push_package(&src, &target, &opts("push")),
        Err(PackageError::Compile(p)) if p[0].contains("漂移")
    ));
    // 编译后通过；合入 + 重编译目标产物
    compile_source(&src);
    let r = push_package(&src, &target, &opts("push")).unwrap();
    assert_eq!(r["command"], json!("push"));
    assert!(target.join("dmaf/agents/pkg-agent.md").is_file());
    assert!(target.join("dmaf/compiled/stage-graph.json").is_file());
    assert!(target.join("dmaf/packages.yaml").is_file());
    assert_eq!(std::fs::read_to_string(target.join("宿主文件.txt")).unwrap(), "宿主已有内容\n");

    // 源演进：stage 定义变更（影响产物）+ agent 正文变更（不影响产物）。
    // 未重编译 → 产物漂移拒绝；重编译 → 再 push 即升级（agent 属包属未改文件 → updated）
    let stage_path = src.join("dmaf/workflows/stages/demo/p1.md");
    std::fs::write(
        &stage_path,
        STAGE_MD.replace("包任务", "包任务 v2"),
    )
    .unwrap();
    write_agent(&src, AGENT_V2);
    assert!(
        matches!(
            push_package(&src, &target, &opts("push")),
            Err(PackageError::Compile(p)) if p[0].contains("漂移")
        ),
        "改 stage 源后必须重编译才能 push"
    );
    compile_source(&src);
    let r2 = push_package(&src, &target, &opts("push")).unwrap();
    let updated = r2["updated"].as_array().unwrap();
    assert!(updated.contains(&json!("dmaf/agents/pkg-agent.md")));
    assert!(updated.contains(&json!("dmaf/workflows/stages/demo/p1.md")));
    assert!(
        std::fs::read_to_string(target.join("dmaf/agents/pkg-agent.md")).unwrap().contains("body v2"),
        "push 重跑交付 agent 新正文"
    );
}

// ---------- git 克隆源 ----------

fn git(args: &[&str], cwd: &Path) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("git 须可用");
    assert!(status.success(), "git {args:?} 失败");
}

#[test]
fn add_clones_git_url_with_tag() {
    let repo = make_source("pkg-git-src");
    git(&["init", "-q", "-b", "main"], &repo);
    git(&["-c", "user.name=t", "-c", "user.email=t@t", "add", "-A"], &repo);
    git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init"], &repo);
    git(&["tag", "v0.1.0"], &repo);

    let target = common::unique_tmp("pkg-git-tgt");
    let url = format!("file:///{}", repo.to_string_lossy().replace('\\', "/"));

    let mut o = opts("add");
    o.tag = Some("v0.1.0".into());
    o.source_label = url.clone();
    let r = add_package(&target, &url, Some("v0.1.0"), &o).unwrap();
    assert_eq!(r["package"]["tag"], json!("v0.1.0"));
    assert!(target.join("dmaf/agents/pkg-agent.md").is_file());
    // 账本记录 tag 与 URL 源
    let ledger: Value =
        serde_yaml::from_str(&std::fs::read_to_string(target.join("dmaf/packages.yaml")).unwrap()).unwrap();
    assert_eq!(ledger["packages"][0]["tag"], json!("v0.1.0"));
    assert_eq!(ledger["packages"][0]["source"], json!(url));

    // 错误 tag → 克隆失败报 IO 错
    let target2 = common::unique_tmp("pkg-git-tgt2");
    let err = add_package(&target2, &url, Some("no-such-tag"), &o).unwrap_err();
    assert!(
        matches!(err, PackageError::Io(ref m) if m.contains("git clone 失败")),
        "{err:?}"
    );
}

// ---------- CLI 装配（需 chdir） ----------

#[test]
fn cli_add_short_name_reports_market_not_ready() {
    let root = common::unique_tmp("cli-add-short");
    let _lock = lock_cwd();
    let _g = CwdGuard::enter(&root);
    let (rc, _, err) = Driver::with_tag("cli-add").run(&svec(&["add", "sdlc"]));
    assert_eq!(rc, 2);
    assert!(err.contains("market"), "{err}");
}

#[test]
fn cli_add_from_path_end_to_end() {
    let src = make_source("cli-add-src");
    let target = common::unique_tmp("cli-add-tgt");
    let _lock = lock_cwd();
    let _g = CwdGuard::enter(&target);
    let from = src.to_string_lossy().replace('\\', "/");
    let (rc, out, err) = Driver::with_tag("cli-add2").run(&svec(&[
        "add",
        "--from",
        &from,
        "--no-install",
    ]));
    assert_eq!(rc, 0, "{err}");
    let body: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(body["command"], json!("add"));
    assert_eq!(body["package"]["name"], json!("pkg"));
    assert!(target.join("dmaf/packages.yaml").is_file());
}

#[test]
fn cli_check_defaults_to_standard_layout() {
    let src = make_source("cli-check-src");
    // 源本身即一个带 stage 库的工程；先安装资产再校验（注册表来自 .opencode）
    std::fs::create_dir_all(src.join(".opencode")).unwrap();
    let _lock = lock_cwd();
    let _g = CwdGuard::enter(&src);
    let (rc, _, err) = Driver::with_tag("cli-check0").run(&svec(&["install", "--harness", "opencode"]));
    assert_eq!(rc, 0, "{err}");
    let (rc, out, err) = Driver::with_tag("cli-check").run(&svec(&["check", "--harness", "opencode"]));
    assert_eq!(rc, 0, "{err}");
    let body: Value = serde_json::from_str(&out).unwrap();
    // 编译工作流的 id 为合成名；校验点在引用清单
    let agents = body["agents"].as_array().unwrap();
    assert!(agents.contains(&json!("pkg-agent")), "{body}");
}

#[test]
fn cli_check_without_inputs_and_layout_still_errors() {
    let root = common::unique_tmp("cli-check-none");
    let _lock = lock_cwd();
    let _g = CwdGuard::enter(&root);
    let (rc, _, err) = Driver::with_tag("cli-check2").run(&svec(&["check", "--harness", "opencode"]));
    assert_eq!(rc, 2, "无标准布局且无输入源仍应报用法错误");
    assert!(err.contains("必须指定"), "{err}");
}

#[test]
fn chain_install_propagates_upgrade_to_harness() {
    // add 升级 → dmaf/ 按包账本更新 → 串联 install 按安装账本把未改动的 harness 副本同步到新版
    let src = make_source("pkg-src-upg-chain");
    let target = common::unique_tmp("pkg-tgt-upg-chain");
    std::fs::create_dir_all(target.join(".opencode")).unwrap();
    let mut o = opts("add");
    o.chain_install = true;
    merge_package(&src, &target, &o).unwrap();
    assert!(std::fs::read_to_string(target.join(".opencode/agents/pkg-agent.md")).unwrap().contains("body v1"));

    write_agent(&src, AGENT_V2);
    let r = merge_package(&src, &target, &o).unwrap();
    assert_eq!(r["updated"], json!(["dmaf/agents/pkg-agent.md"]));
    assert_eq!(r["install"]["updated"], json!(["agents/pkg-agent.md"]));
    assert!(
        std::fs::read_to_string(target.join(".opencode/agents/pkg-agent.md")).unwrap().contains("body v2"),
        "harness 侧未改动的副本随升级自动同步"
    );

    // harness 侧被用户改过 → 串联 install 跳过保护，dmaf/ 仍正常升级
    std::fs::write(target.join(".opencode/agents/pkg-agent.md"), "用户在 harness 里的定制\n").unwrap();
    write_agent(&src, "---\nname: pkg-agent\nmode: subagent\ndescription: 包 agent\n---\nbody v3\n");
    let r = merge_package(&src, &target, &o).unwrap();
    assert_eq!(r["updated"], json!(["dmaf/agents/pkg-agent.md"]));
    assert_eq!(r["install"]["skipped"], json!(["agents/pkg-agent.md"]));
    assert!(std::fs::read_to_string(target.join(".opencode/agents/pkg-agent.md")).unwrap().contains("用户在 harness"));

    // add --force → 串联 install 同步强制覆盖：harness 内用户改动让位包源（overwritten）
    let mut of = opts("add");
    of.chain_install = true;
    of.force = true;
    let r = merge_package(&src, &target, &of).unwrap();
    assert_eq!(r["install"]["overwritten"], json!(["agents/pkg-agent.md"]));
    assert!(
        std::fs::read_to_string(target.join(".opencode/agents/pkg-agent.md")).unwrap().contains("body v3"),
        "--force 下 harness 内用户改动被覆盖为包源新版"
    );
}
