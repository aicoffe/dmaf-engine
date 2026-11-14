//! i18n 机制验收：目录 key 对齐（强制）、locale 烘焙（start --locale 写入会话）、
//! 旧会话回退（无 locale 字段 = zh）、en 冒烟（错误文案随 locale）。
//! 注意：i18n locale 是进程级全局（CLI 单命令语义），locale 相关测试共用一把
//! lock_cwd 串行化，避免与并行测试互踩。

mod common;

use common::svec;
use serde_json::Value;

/// 目录 key 唯一且 zh/en 非空且两语不同（缺项/漏译在编译后的这条断言拦截）。
#[test]
fn catalog_keys_unique_and_bilingual() {
    let mut seen = std::collections::BTreeSet::new();
    for (key, zh, en) in dmaf_engine::i18n::all_entries() {
        assert!(!key.is_empty(), "空 key");
        assert!(!zh.is_empty(), "zh 缺失: {key}");
        assert!(!en.is_empty(), "en 缺失（漏译）: {key}");
        assert!(zh != en, "zh 与 en 相同（疑似漏译）: {key}");
        assert!(zh.contains("{}") == en.contains("{}"), "占位符数量不一致: {key}");
        assert!(seen.insert(key), "key 重复: {key}");
    }
    // 样板条目在位（迁移模式的锚点）
    assert!(
        dmaf_engine::i18n::all_entries().any(|(k, _, _)| k == "cli.var_kv_form"),
        "样板条目 cli.var_kv_form 缺失"
    );
}

/// t/msg：未知 key 回落原样 key（不 panic、不空串）。
#[test]
fn t_unknown_key_falls_back() {
    let _guard = common::lock_cwd();
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
    assert_eq!(dmaf_engine::i18n::t("no.such.key"), "no.such.key");
    assert_eq!(dmaf_engine::i18n::msg("no.such.key", &["x"]), "no.such.key");
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
}

/// clap help 全量走 catalog：cli.rs 源码里每个 t_static("key") 的 key 必须在目录中
/// （防调用点与目录漂移；缺项运行期表现为哨兵串，这里在测试期拦截）。
#[test]
fn help_static_keys_resolved_in_catalog() {
    let src = include_str!("../src/cli.rs");
    let mut keys = std::collections::BTreeSet::new();
    let mut rest = src;
    while let Some(pos) = rest.find("t_static(\"") {
        let after = &rest[pos + "t_static(\"".len()..];
        let end = after.find('"').expect("t_static 调用未闭合");
        keys.insert(after[..end].to_string());
        rest = &after[end..];
    }
    assert!(keys.len() >= 70, "t_static 调用点异常减少: {}", keys.len());
    let catalog: std::collections::BTreeSet<&str> =
        dmaf_engine::i18n::all_entries().map(|(k, _, _)| k).collect();
    let missing: Vec<&String> = keys.iter().filter(|k| !catalog.contains(k.as_str())).collect();
    assert!(missing.is_empty(), "t_static 引用了目录缺失的 key: {missing:?}");
}

/// help 跟随 locale：显式 set(En) 后 -h 输出英文 about；复位 zh 后输出中文 about
/// （显式设置使 preset 跳过，宿主系统语言不参与）。
#[test]
fn help_follows_locale() {
    let _guard = common::lock_cwd();
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::En);
    let out = dmaf_engine::run(&common::svec(&["-h"]));
    assert_eq!(out.code, 0);
    assert!(
        out.stdout.contains("D-MAF deterministic routing engine"),
        "en locale 下 help 应为英文: {}",
        out.stdout
    );
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
    let out2 = dmaf_engine::run(&common::svec(&["-h"]));
    assert!(
        out2.stdout.contains("D-MAF 确定性路由 Engine"),
        "zh locale 下 help 应为中文: {}",
        out2.stdout
    );
}

/// start --locale en：会话状态文件烘焙 locale: en；同 locale 下错误文案为英文。
#[test]
fn start_bakes_locale_and_en_error_smoke() {
    let _guard = common::lock_cwd();
    let d = common::Driver::with_tag("i18n-bake");
    // en 会话 + 非法 --var：错误文案走英文目录
    let argv = svec(&[
        "start",
        "--workflow",
        d.workflow.to_str().unwrap(),
        "--task",
        "t",
        "--output-root",
        d.output_root.to_str().unwrap(),
        "--locale",
        "en",
        "--var",
        "notakv",
    ]);
    // zh 断言先行（进程 locale 复位为缺省），随后 en——进程内全局 locale 按序生效
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
    let argv_zh = svec(&[
        "start",
        "--workflow",
        d.workflow.to_str().unwrap(),
        "--task",
        "t",
        "--output-root",
        d.output_root.to_str().unwrap(),
        "--var",
        "notakv",
    ]);
    let (rc_zh, _out, err_zh) = d.run(&argv_zh);
    assert_eq!(rc_zh, 2);
    assert!(err_zh.contains("k=v 形式"), "缺省 zh 文案回归: {err_zh}");
    let (rc, _out, err) = d.run(&argv);
    assert_eq!(rc, 2, "非法 --var 应退出码 2: {err}");
    assert!(
        err.contains("must be in k=v form"),
        "en locale 下错误文案应为英文: {err}"
    );
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
}

/// start 缺省烘焙 zh；剥离 locale 行模拟旧会话（0.6.x 文件）→ 加载回退 zh 不报错，
/// 后续会话命令（verify-audit）正常。
#[test]
fn legacy_state_without_locale_falls_back_to_zh() {
    let _guard = common::lock_cwd();
    let d = common::Driver::with_tag("i18n-legacy");
    let v = d.start_default();
    let session = v["session_dir"].as_str().unwrap().to_string();
    let state_path = d.tmp.join(session.strip_prefix("./").unwrap_or(&session)).join("dmaf-state.md");
    let text = std::fs::read_to_string(&state_path).unwrap();
    assert!(text.contains("locale: zh"), "缺省烘焙应写 locale: zh: {text}");
    // 模拟 0.6.x 旧会话：删除 locale 行
    let stripped: String = text
        .lines()
        .filter(|l| !l.starts_with("locale:"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&state_path, &stripped).unwrap();
    let (rc, out, err) = d.run(&svec(&["verify-audit", "--session", &session]));
    assert_eq!(rc, 0, "旧会话（无 locale 字段）verify-audit 应通过: {out} {err}");
    // 加载旧会话（无 locale 字段）已把进程 locale 回退为 zh：行为证明 = 同参数错误文案为中文
    let (rc2, _o, err2) = d.run(&svec(&[
        "start",
        "--workflow",
        d.workflow.to_str().unwrap(),
        "--task",
        "t",
        "--output-root",
        d.output_root.to_str().unwrap(),
        "--var",
        "notakv",
    ]));
    assert_eq!(rc2, 2);
    assert!(err2.contains("k=v 形式"), "旧会话回退后进程 locale 应为 zh: {err2}");
}

/// start --locale en 烘焙后的会话：state 文件 locale: en 可被再次加载（session 命令读会话）。
#[test]
fn en_session_state_roundtrip() {
    let _guard = common::lock_cwd();
    let d = common::Driver::with_tag("i18n-roundtrip");
    let argv = svec(&[
        "start",
        "--workflow",
        d.workflow.to_str().unwrap(),
        "--task",
        "t",
        "--output-root",
        d.output_root.to_str().unwrap(),
        "--locale",
        "en",
    ]);
    let (rc, out, err) = d.run(&argv);
    assert_eq!(rc, 0, "en 会话启动失败: {err}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let session = v["session_dir"].as_str().unwrap().to_string();
    let (rc2, _o, e2) = d.run(&svec(&["verify-audit", "--session", &session]));
    assert_eq!(rc2, 0, "en 会话 verify-audit 应通过: {e2}");
    let state_path =
        d.tmp.join(session.strip_prefix("./").unwrap_or(&session)).join("dmaf-state.md");
    let text = std::fs::read_to_string(&state_path).unwrap();
    assert!(text.contains("locale: en"), "en 烘焙应落盘: {text}");
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
}
