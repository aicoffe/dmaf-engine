//! Phase A 验收：install 自动探测（缺省 --harness）、入口配置落位（dmaf/harness/<名>/ → 项目根）、
//! doctor 新检查（资产同步漂移 / 产物版本兼容 / manifest engine 兼容）。
//!
//! cmd_install / cmd_doctor 按 CWD 读 dmaf/，chdir 是进程全局状态——
//! 用 common::lock_cwd 跨测试文件互斥 + common::CwdGuard 承载。

mod common;

use common::{lock_cwd, svec, CwdGuard, Driver};
use dmaf_engine::registry::{detect_harnesses, harness_root};
use serde_json::{json, Value};

/// 在 tmp 建一个最小 dmaf/ 定义层（1 agent + 1 skill + 1 stage/1 scope + compiled 空目录），返回项目根。
fn mini_project(tag: &str) -> std::path::PathBuf {
    let root = common::unique_tmp(tag);
    for d in [
        "dmaf/agents",
        "dmaf/skills/alpha",
        "dmaf/compiled",
        "dmaf/workflows/stages/demo",
        "dmaf/workflows/scopes",
    ] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(
        root.join("dmaf/agents/alpha.md"),
        "---\nname: alpha\nmode: subagent\ndescription: 测试 agent\n---\nbody\n",
    )
    .unwrap();
    std::fs::write(
        root.join("dmaf/skills/alpha/SKILL.md"),
        "---\nname: alpha\ndescription: 测试技能\n---\nbody\n",
    )
    .unwrap();
    // stage 库按 <phase>/<stage>.md 两级组织
    std::fs::write(
        root.join("dmaf/workflows/stages/demo/t1.md"),
        "---\nid: t1\nphase: demo\ncollaboration: single\n\nagents:\n  - agent: alpha\n    skills: []\n    inputs: []\n    outputs: [\"out/result.md\"]\n    task_description: \"做一次演示任务\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\nexecution: always\n---\n\n# t1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("dmaf/workflows/scopes/s1.md"),
        "---\nname: s1\ndescription: 单 stage 演示\ninclude_stages: [t1]\n---\n\n# s1\n",
    )
    .unwrap();
    root
}

fn run_in(dir: &std::path::Path, argv: &[&str]) -> (i32, Value, String) {
    let _guard = CwdGuard::enter(dir);
    let (rc, out, err) = Driver::with_tag("phase-a").run(&svec(argv));
    let body_source = if rc == 0 { &out } else { &err };
    let body = serde_json::from_str(body_source)
        .unwrap_or_else(|_| panic!("输出不是 JSON（rc={rc}）: {body_source}"));
    (rc, body, err)
}

fn find_check<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == name)
        .unwrap_or_else(|| panic!("doctor 缺 {name} 检查项"))
}

// ---------- registry：正向映射与逆向探测一致性 ----------

#[test]
fn detect_and_harness_root_share_one_table() {
    for (dir, name) in [
        (".opencode", "opencode"),
        (".claude", "claude"),
        (".zcode", "zcode"),
        (".cursor", "cursor"),
        (".gemini", "gemini"),
        (".codex", "codex"),
        (".dsh", "dsh"),
    ] {
        let rh = harness_root(name).unwrap();
        assert_eq!(rh.name, Some(name));
        assert_eq!(rh.dir.to_string_lossy(), dir);
    }
    // 别名归到规范名
    assert_eq!(harness_root("claude-code").unwrap().name, Some("claude"));
    assert_eq!(harness_root("gemini-cli").unwrap().name, Some("gemini"));
    // 路径透传无规范名
    assert_eq!(harness_root("./custom-harness").unwrap().name, None);
    // 探测：只认已存在的目录，按规范名字典序
    let root = common::unique_tmp("detect");
    std::fs::create_dir_all(root.join(".codex")).unwrap();
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    assert_eq!(detect_harnesses(&root), vec!["claude", "codex"]);
    assert!(detect_harnesses(&common::unique_tmp("detect-none")).is_empty());
}

// ---------- install 自动探测 ----------

#[test]
fn install_without_harness_auto_detects_single() {
    let root = mini_project("install-auto-single");
    std::fs::create_dir_all(root.join(".opencode")).unwrap();
    let _lock = lock_cwd();
    let (rc, body, err) = run_in(&root, &["install"]);
    assert_eq!(rc, 0, "{err}");
    assert_eq!(body["auto_detected"], serde_json::json!(true));
    assert_eq!(body["harness_name"], serde_json::json!("opencode"));
    assert!(
        root.join(".opencode/agents/alpha.md").is_file(),
        "自动探测目标安装成功"
    );
    // 入口配置目录缺失 → skipped
    assert!(body["entry_config"]["skipped"].is_string());
}

#[test]
fn install_without_harness_lists_multiple_candidates() {
    let root = mini_project("install-auto-multi");
    std::fs::create_dir_all(root.join(".opencode")).unwrap();
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    let _lock = lock_cwd();
    let (rc, _, err) = run_in(&root, &["install"]);
    assert_eq!(rc, 2, "多候选应拒绝并退出码 2");
    assert!(
        err.contains("发现多个")
            && err.contains("claude")
            && err.contains("opencode")
            && err.contains("--harness all"),
        "{err}"
    );
    assert!(!root.join(".opencode/agents").exists(), "未写入任何目标");
}

#[test]
fn install_harness_all_installs_into_every_detected() {
    let root = mini_project("install-all");
    std::fs::create_dir_all(root.join(".opencode")).unwrap();
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    std::fs::create_dir_all(root.join("dmaf/harness/opencode")).unwrap();
    std::fs::write(root.join("dmaf/harness/opencode/opencode.json"), "入口配置\n").unwrap();
    let _lock = lock_cwd();
    let (rc, body, err) = run_in(&root, &["install", "--harness", "all"]);
    assert_eq!(rc, 0, "{err}");
    assert_eq!(body["harness"], json!("all"));
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 2, "两个 harness 各得一份报告");
    assert!(root.join(".opencode/agents/alpha.md").is_file());
    assert!(root.join(".claude/agents/alpha.md").is_file());
    // 入口配置只对声明了 dmaf/harness/<名>/ 的 harness 落位，且落在 harness 目录内
    assert_eq!(
        std::fs::read_to_string(root.join(".opencode/opencode.json")).unwrap(),
        "入口配置\n"
    );
    assert!(!root.join("opencode.json").exists(), "配置不再落项目根");
    let opencode_report = results
        .iter()
        .find(|r| r["harness_name"] == json!("opencode"))
        .unwrap();
    assert_eq!(opencode_report["entry_config"]["created"], json!(["opencode.json"]));
    // 聚合计数：每 harness 2 个资产（agent + skill）
    assert_eq!(body["installed_total"], json!(4));
}

#[test]
fn install_harness_all_without_any_harness_errors() {
    let root = mini_project("install-all-none");
    let _lock = lock_cwd();
    let (rc, _, err) = run_in(&root, &["install", "--harness", "all"]);
    assert_eq!(rc, 2);
    assert!(err.contains("未发现"), "{err}");
}

#[test]
fn install_without_harness_gives_guidance_when_none() {
    let root = mini_project("install-auto-none");
    let _lock = lock_cwd();
    let (rc, _, err) = run_in(&root, &["install"]);
    assert_eq!(rc, 2);
    assert!(err.contains("未发现") && err.contains("--harness"), "{err}");
}

#[test]
fn install_explicit_harness_disables_auto_detect() {
    let root = mini_project("install-explicit");
    std::fs::create_dir_all(root.join(".opencode")).unwrap();
    let _lock = lock_cwd();
    let (rc, body, err) = run_in(&root, &["install", "--harness", "opencode"]);
    assert_eq!(rc, 0, "{err}");
    assert_eq!(body["auto_detected"], serde_json::json!(false));
}

// ---------- 入口配置落位 ----------

#[test]
fn entry_config_per_file_ledger_gate() {
    let root = mini_project("entry-config");
    std::fs::create_dir_all(root.join(".opencode")).unwrap();
    std::fs::create_dir_all(root.join("dmaf/harness/opencode")).unwrap();
    std::fs::write(root.join("dmaf/harness/opencode/opencode.json"), "源配置 A\n").unwrap();

    let _lock = lock_cwd();
    let (rc, body, err) = run_in(&root, &["install", "--harness", "opencode"]);
    assert_eq!(rc, 0, "{err}");
    assert_eq!(
        body["entry_config"]["created"],
        serde_json::json!(["opencode.json"])
    );
    // 落位到 harness 目录内，而非项目根
    assert!(!root.join("opencode.json").exists(), "配置不再落项目根");
    assert_eq!(
        std::fs::read_to_string(root.join(".opencode/opencode.json")).unwrap(),
        "源配置 A\n"
    );

    // 逐文件哈希门：内容一致者 unchanged，新增者照常落位
    std::fs::write(root.join("dmaf/harness/opencode/extra.md"), "说明\n").unwrap();
    let (rc, body, _) = run_in(&root, &["install", "--harness", "opencode"]);
    assert_eq!(rc, 0);
    assert_eq!(body["entry_config"]["created"], serde_json::json!(["extra.md"]));
    assert_eq!(body["entry_config"]["unchanged"], serde_json::json!(["opencode.json"]));

    // 源升级、目标未改动 → updated（升级自动传播）
    std::fs::write(root.join("dmaf/harness/opencode/opencode.json"), "源配置 B\n").unwrap();
    let (rc, body, _) = run_in(&root, &["install", "--harness", "opencode"]);
    assert_eq!(rc, 0);
    assert_eq!(body["entry_config"]["updated"], serde_json::json!(["opencode.json"]));
    assert_eq!(
        std::fs::read_to_string(root.join(".opencode/opencode.json")).unwrap(),
        "源配置 B\n"
    );

    // 用户改过目标 → skipped（不覆盖）；--force → overwritten
    std::fs::write(root.join(".opencode/opencode.json"), "用户定制\n").unwrap();
    std::fs::write(root.join("dmaf/harness/opencode/opencode.json"), "源配置 C\n").unwrap();
    let (rc, body, _) = run_in(&root, &["install", "--harness", "opencode"]);
    assert_eq!(rc, 0);
    assert_eq!(body["entry_config"]["skipped"], serde_json::json!(["opencode.json"]));
    assert_eq!(
        std::fs::read_to_string(root.join(".opencode/opencode.json")).unwrap(),
        "用户定制\n",
        "已改动配置不被覆盖"
    );
    let (rc, body, _) = run_in(&root, &["install", "--harness", "opencode", "--force"]);
    assert_eq!(rc, 0);
    assert_eq!(body["entry_config"]["overwritten"], serde_json::json!(["opencode.json"]));
    assert_eq!(
        std::fs::read_to_string(root.join(".opencode/opencode.json")).unwrap(),
        "源配置 C\n"
    );
}

#[test]
fn entry_config_check_only_writes_nothing() {
    let root = mini_project("entry-checkonly");
    std::fs::create_dir_all(root.join(".opencode")).unwrap();
    std::fs::create_dir_all(root.join("dmaf/harness/opencode")).unwrap();
    std::fs::write(root.join("dmaf/harness/opencode/opencode.json"), "源配置\n").unwrap();
    let _lock = lock_cwd();
    let (rc, body, _) = run_in(&root, &["install", "--harness", "opencode", "--check-only"]);
    assert_eq!(rc, 0);
    assert_eq!(
        body["entry_config"]["created"],
        serde_json::json!(["opencode.json"])
    );
    assert!(
        !root.join("opencode.json").exists() && !root.join(".opencode/opencode.json").exists(),
        "check-only 不落盘"
    );
}

// ---------- doctor 新检查 ----------

#[test]
fn doctor_reports_asset_sync_drift() {
    let root = mini_project("doctor-sync");
    std::fs::create_dir_all(root.join(".opencode")).unwrap();
    let _lock = lock_cwd();
    let (rc, _, err) = run_in(&root, &["install"]);
    assert_eq!(rc, 0, "{err}");

    // 刚装完 → 同步
    let (rc, report, _) = run_in(&root, &["doctor", "--harness", "opencode"]);
    assert_eq!(rc, 0);
    assert_eq!(find_check(&report, "assets_sync")["ok"], serde_json::json!(true));

    // 改源 → 漂移 + 可行动提示
    std::fs::write(
        root.join("dmaf/agents/alpha.md"),
        "---\nname: alpha\nmode: subagent\ndescription: 改过的 agent\n---\nbody v2\n",
    )
    .unwrap();
    let (rc, report, _) = run_in(&root, &["doctor", "--harness", "opencode"]);
    assert_eq!(rc, 0, "doctor 以 JSON ok 字段承载健康度（退出码恒 0）");
    assert_eq!(report["ok"], serde_json::json!(false));
    let sync = find_check(&report, "assets_sync");
    assert_eq!(sync["ok"], serde_json::json!(false));
    assert_eq!(
        sync["detail"]["would_change"],
        serde_json::json!(["agents/alpha.md"])
    );
    assert!(sync["detail"]["hint"].as_str().unwrap().contains("dmaf install"));
}

#[test]
fn doctor_detects_artifact_version_mismatch() {
    let root = mini_project("doctor-version");
    let _lock = lock_cwd();
    // 先真编译出当期产物，再伪造成"未来版本"
    let (rc, _, err) = run_in(&root, &["compile"]);
    assert_eq!(rc, 0, "{err}");
    std::fs::write(
        root.join("dmaf/compiled/stage-graph.json"),
        "{\"version\": 99999, \"stages\": []}",
    )
    .unwrap();
    std::fs::write(
        root.join("dmaf/compiled/scope-grid.json"),
        "{\"version\": 99999, \"scopes\": {}}",
    )
    .unwrap();

    let (_, report, _) = run_in(&root, &["doctor"]);
    let drift = find_check(&report, "artifacts_drift");
    assert_eq!(drift["ok"], serde_json::json!(false));
    assert!(
        drift["detail"]["reason"]
            .as_str()
            .unwrap()
            .contains("版本"),
        "版本不兼容应给出明确原因: {drift}"
    );
}

#[test]
fn doctor_checks_manifest_engine_range() {
    let root = mini_project("doctor-manifest");
    std::fs::write(
        root.join("dmaf/manifest.yaml"),
        "name: t\nversion: 0.1.0\ndescription: d\nengine: \">=999.0\"\nharnesses: []\n",
    )
    .unwrap();
    let _lock = lock_cwd();

    // engine 范围不满足 → ok=false + 可行动提示
    let (_, report, _) = run_in(&root, &["doctor"]);
    let m = find_check(&report, "manifest");
    assert_eq!(m["ok"], serde_json::json!(false));
    assert!(m["detail"]["hint"].as_str().unwrap().contains("engine"));

    // 范围满足 → ok=true
    std::fs::write(
        root.join("dmaf/manifest.yaml"),
        "name: t\nversion: 0.1.0\ndescription: d\nengine: \">=0.1,<99\"\nharnesses: []\n",
    )
    .unwrap();
    let (_, report, _) = run_in(&root, &["doctor"]);
    let m = find_check(&report, "manifest");
    assert_eq!(m["ok"], serde_json::json!(true));
    assert_eq!(m["detail"]["configured"], serde_json::json!(true));
}

#[test]
fn doctor_without_manifest_reports_unconfigured_ok() {
    let root = mini_project("doctor-nomanifest");
    let _lock = lock_cwd();
    let (_, report, _) = run_in(&root, &["doctor"]);
    let m = find_check(&report, "manifest");
    assert_eq!(m["ok"], serde_json::json!(true));
    assert_eq!(m["detail"]["configured"], serde_json::json!(false));
}

#[test]
fn path_like_harness_skips_entry_config() {
    let root = mini_project("entry-passthrough");
    std::fs::create_dir_all(root.join("custom-harness-dir")).unwrap();
    std::fs::create_dir_all(root.join("dmaf/harness/opencode")).unwrap();
    std::fs::write(root.join("dmaf/harness/opencode/opencode.json"), "源配置\n").unwrap();
    let _lock = lock_cwd();
    // 路径透传须含分隔符或以 . 开头
    let (rc, body, _) = run_in(&root, &["install", "--harness", "./custom-harness-dir"]);
    assert_eq!(rc, 0);
    assert!(
        body["entry_config"].is_null(),
        "路径透传不落位入口配置: {}",
        serde_json::to_string(&body).unwrap()
    );
    assert!(!root.join("opencode.json").exists());
}
