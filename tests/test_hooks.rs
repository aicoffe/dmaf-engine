//! hooks.yaml 项目级 hook 的验收测试（契约见 src/hooks.rs 模块注释）。
//!
//! hook 可执行体 = rustc 现场编译的 tests/fixtures/hook-helper.rs（离线、跨平台）。
//! dmaf/hooks.yaml 按进程 CWD 解析——chdir 是进程全局状态，
//! 用 common::lock_cwd 跨测试文件互斥 + common::CwdGuard 承载。

mod common;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::Value;

use common::{svec, Driver};

fn y(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// hook 辅助可执行体：rustc 编译一次，进程内复用。
fn helper_exe() -> PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let src = common::repo_root()
            .join("tests")
            .join("fixtures")
            .join("hook-helper.rs");
        let out =
            std::env::temp_dir().join(format!("dmaf-hook-helper-{}.exe", std::process::id()));
        let status = std::process::Command::new("rustc")
            .args(["--edition", "2021"])
            .arg(&src)
            .arg("-o")
            .arg(&out)
            .status()
            .expect("rustc 编译 hook 辅助程序失败（工具链应随 cargo test 可用）");
        assert!(status.success(), "hook 辅助程序编译失败");
        out
    })
    .clone()
}

/// 本文件场景串行（chdir 进程全局）+ chdir 到项目根（tmp）。
fn lock_and_enter(driver: &Driver) -> (std::sync::MutexGuard<'static, ()>, common::CwdGuard) {
    (common::lock_cwd(), common::CwdGuard::enter(&driver.tmp))
}

fn read_dump(p: &Path) -> Value {
    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("dump 不存在 {p:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("dump 不是合法 JSON: {e}"))
}

fn write_config(driver: &Driver, text: &str) {
    std::fs::create_dir_all(driver.tmp.join("dmaf")).unwrap();
    std::fs::write(driver.tmp.join("dmaf").join("hooks.yaml"), text).unwrap();
}

/// YAML 列表形式的 argv 文本（全部元素双引号包裹，posix 路径无转义问题）。
fn argv_yaml(items: &[String]) -> String {
    format!(
        "[{}]",
        items.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ")
    )
}

fn next_argv(driver: &Driver, session: &str) -> Vec<String> {
    svec(&[
        "next",
        "--session",
        session,
        "--workflow",
        driver.workflow.to_str().unwrap(),
    ])
}

fn start_next(driver: &Driver) -> String {
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    driver.next(&session);
    session
}

// ---------- init 点位 ----------

#[test]
fn init_payload_contract_and_failure_irrelevance() {
    let driver = Driver::with_tag("hook-init");
    let _env = lock_and_enter(&driver);
    let dump = driver.tmp.join("dump-init.json");
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!("init:\n  - {}\n", argv_yaml(&[helper, "ok".into(), y(&dump)])),
    );
    let started = driver.start_default();
    let envelope = read_dump(&dump);
    assert_eq!(envelope["kind"], "init");
    assert_eq!(envelope["workflow_id"], started["workflow_id"]);
    assert_eq!(envelope["session_dir"], started["session_dir"]);
    assert_eq!(envelope["session_dir"], envelope["session_dir"]);
    assert_eq!(envelope["vars"]["TASK"], "讲解「大语言模型」");
    assert!(envelope["stages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.as_str() == Some("research")));

    // init 失败 / 崩溃：start 照常成功，会话完好
    let helper = y(&helper_exe());
    write_config(&driver, &format!("init:\n  - {}\n", argv_yaml(&[helper, "fail".into()])));
    let started = driver.start("init失败不挡启动", None);
    assert!(Path::new(started["state_path"].as_str().unwrap()).is_file());
    write_config(
        &driver,
        &format!("init:\n  - {}\n", argv_yaml(&[y(&helper_exe()), "panic".into()])),
    );
    let started = driver.start("init崩溃不挡启动", None);
    assert!(Path::new(started["state_path"].as_str().unwrap()).is_file());
}

// ---------- on-event 点位 ----------

#[test]
fn on_event_reported_golden_and_unconfigured_silent() {
    let driver = Driver::with_tag("hook-on-event");
    let _env = lock_and_enter(&driver);
    let dump = driver.tmp.join("dump-reported.json");
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!(
            "on-event:\n  action.reported:\n    - {}\n",
            argv_yaml(&[helper, "ok".into(), y(&dump)])
        ),
    );
    let session = start_next(&driver);
    assert!(!dump.exists(), "未配置的事件类型（action.dispatched）不应触发 hook");
    let (rc, _) = driver.report_success(&session, &driver.next(&session));
    assert_eq!(rc, 0);

    let envelope = read_dump(&dump);
    assert_eq!(envelope["session_dir"].as_str().unwrap(), session);
    assert_eq!(envelope["shard"]["event_type"], "action.reported");
    // golden：stdin 的 shard 与落盘分片 JSON 内容一致
    let seq = envelope["shard"]["seq"].as_u64().unwrap();
    let shard_file = PathBuf::from(&session)
        .join("audit")
        .join(format!("shard-wf-lesson-{seq:06}.json"));
    let file_shard: Value =
        serde_json::from_str(&std::fs::read_to_string(&shard_file).unwrap()).unwrap();
    assert_eq!(envelope["shard"], file_shard, "stdin.shard 与落盘分片内容不一致");
}

#[test]
fn hook_stdout_does_not_pollute_instruction_stream() {
    let driver = Driver::with_tag("hook-loud");
    let _env = lock_and_enter(&driver);
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!(
            "on-event:\n  action.dispatched:\n    - {}\n",
            argv_yaml(&[helper, "loud".into()])
        ),
    );
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    let (rc, out, _) = driver.run(&next_argv(&driver, &session));
    assert_eq!(rc, 0);
    assert!(!out.contains("xxxxx"), "hook stdout 泄漏进引擎 stdout");
    let instruction: Value =
        serde_json::from_str(&out).expect("引擎 stdout 不再是纯 JSON（被 hook 污染）");
    assert_eq!(instruction["type"], "dispatch");
    assert_eq!(instruction["stage"], "intent");
}

#[test]
fn failure_does_not_block_rest_of_list() {
    let driver = Driver::with_tag("hook-fail");
    let _env = lock_and_enter(&driver);
    let log = driver.tmp.join("order.log");
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!(
            "on-event:\n  action.dispatched:\n    - {}\n    - {}\n",
            argv_yaml(&[helper.clone(), "fail".into(), "".into(), y(&log)]),
            argv_yaml(&[helper, "ok".into(), "".into(), y(&log)])
        ),
    );
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    let (rc, _, _) = driver.run(&next_argv(&driver, &session));
    assert_eq!(rc, 0);
    let order_text = std::fs::read_to_string(&log).unwrap();
    let order: Vec<&str> = order_text.lines().collect();
    assert_eq!(order, vec!["fail", "ok"], "失败后应继续执行列表下一项");
}

#[test]
fn timeout_kills_hook_and_continues() {
    let driver = Driver::with_tag("hook-timeout");
    let _env = lock_and_enter(&driver);
    let log2 = driver.tmp.join("after-timeout.log");
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!(
            "timeout_secs: 1\non-event:\n  action.dispatched:\n    - {}\n    - {}\n",
            argv_yaml(&[helper.clone(), "hang".into()]),
            argv_yaml(&[helper, "ok".into(), "".into(), y(&log2)])
        ),
    );
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    let t0 = std::time::Instant::now();
    let (rc, _, _) = driver.run(&next_argv(&driver, &session));
    assert_eq!(rc, 0);
    assert!(
        t0.elapsed() < std::time::Duration::from_secs(30),
        "挂起 hook 未被超时杀掉"
    );
    let after_text = std::fs::read_to_string(&log2).unwrap();
    let after: Vec<&str> = after_text.lines().collect();
    assert_eq!(after, vec!["ok"], "超时后应继续执行列表下一项");
}

// ---------- 命令写法 ----------

#[test]
fn command_string_split_and_list_with_spaces() {
    let driver = Driver::with_tag("hook-forms");
    let _env = lock_and_enter(&driver);
    let helper = helper_exe();
    let plain = driver.tmp.join("helper-plain.exe");
    std::fs::copy(&helper, &plain).unwrap();
    let spaced_dir = driver.tmp.join("my tools");
    std::fs::create_dir_all(&spaced_dir).unwrap();
    let spaced = spaced_dir.join("helper-spaced.exe");
    std::fs::copy(&helper, &spaced).unwrap();
    let dump1 = driver.tmp.join("dump-str.json");
    let dump2 = driver.tmp.join("dump-list.json");
    write_config(
        &driver,
        &format!(
            "on-event:\n  action.reported:\n    - \"{} ok {}\"\n    - {}\n",
            y(&plain),
            y(&dump1),
            argv_yaml(&[y(&spaced), "ok".into(), y(&dump2)])
        ),
    );
    let session = start_next(&driver);
    let (rc, _) = driver.report_success(&session, &driver.next(&session));
    assert_eq!(rc, 0);
    assert!(dump1.exists(), "字符串形式（空白切分）未生效");
    assert!(dump2.exists(), "列表形式（含空格路径）未生效");

    // Windows：.cmd/.bat 包装器须显式过 cmd —— 引擎直接 spawn 的对照验证
    #[cfg(windows)]
    {
        let dump3 = driver.tmp.join("dump-cmd.json");
        write_config(
            &driver,
            &format!(
                "on-event:\n  action.reported:\n    - {}\n",
                argv_yaml(&[
                    "cmd".into(),
                    "/c".into(),
                    y(&spaced),
                    "ok".into(),
                    y(&dump3)
                ])
            ),
        );
        let session = start_next(&driver);
        let (rc, _) = driver.report_success(&session, &driver.next(&session));
        assert_eq!(rc, 0);
        assert!(dump3.exists(), "[cmd, /c, ...] 包装形式未生效");
    }
}

// ---------- 配置降级与开关 ----------

#[test]
fn broken_config_degrades_and_unknown_key_ignored() {
    let driver = Driver::with_tag("hook-bad");
    let _env = lock_and_enter(&driver);
    let dump = driver.tmp.join("dump-bad.json");
    // 坏 YAML：整份忽略，命令不受阻
    write_config(&driver, "init: [unclosed\n");
    let session = start_next(&driver);
    assert_eq!(
        driver.run(&next_argv(&driver, &session)).0,
        0,
        "坏配置不应使命令失败"
    );
    assert!(!dump.exists());

    // 未知事件键：忽略；合法键照常生效；doctor 报 warnings
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!(
            "on-event:\n  foo.bar:\n    - {}\n  action.reported:\n    - {}\n",
            argv_yaml(&[helper.clone(), "ok".into()]),
            argv_yaml(&[helper, "ok".into(), y(&dump)])
        ),
    );
    let (rc, _) = driver.report_success(&session, &driver.next(&session));
    assert_eq!(rc, 0);
    assert!(dump.exists(), "合法事件键应照常生效");
    let (_, out, _) = driver.run(&svec(&["doctor"]));
    let body: Value = serde_json::from_str(&out).unwrap();
    let hooks = body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "hooks")
        .expect("doctor 缺 hooks 检查项");
    assert_eq!(hooks["ok"], false, "坏配置应使 doctor hooks 项失败");
    assert!(!hooks["detail"]["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn doctor_reports_missing_config_as_healthy() {
    let driver = Driver::with_tag("hook-doctor");
    let _env = lock_and_enter(&driver);
    let (_, out, _) = driver.run(&svec(&["doctor"]));
    let body: Value = serde_json::from_str(&out).unwrap();
    let hooks = body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "hooks")
        .expect("doctor 缺 hooks 检查项");
    assert_eq!(hooks["ok"], true, "未配置 hook 是正常态");
    assert_eq!(hooks["detail"]["configured"], false);
}

#[test]
fn no_hooks_flag_and_env_disable() {
    let driver = Driver::with_tag("hook-switch");
    let _env = lock_and_enter(&driver);
    let log = driver.tmp.join("switch.log");
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!(
            "on-event:\n  action.dispatched:\n    - {}\n",
            argv_yaml(&[helper, "ok".into(), "".into(), y(&log)])
        ),
    );
    let count = || std::fs::read_to_string(&log).unwrap().lines().count();

    let s1 = start_next(&driver);
    let _ = s1;
    assert_eq!(count(), 1);

    let s2 = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    let mut argv = next_argv(&driver, &s2);
    argv.push("--no-hooks".into());
    driver.run(&argv);
    assert_eq!(count(), 1, "--no-hooks 应禁用 hook");

    std::env::set_var("DMAF_NO_HOOKS", "1");
    let s3 = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    driver.run(&next_argv(&driver, &s3));
    std::env::remove_var("DMAF_NO_HOOKS");
    assert_eq!(count(), 1, "DMAF_NO_HOOKS=1 应禁用 hook");
}

// ---------- 确定性不受扰 ----------

#[test]
fn ledger_unaffected_by_hooks() {
    let plain = Driver::with_tag("hook-ledger-plain");
    let s = start_next(&plain);
    let (rc, _) = plain.report_success(&s, &plain.next(&s));
    assert_eq!(rc, 0);

    let hooked = Driver::with_tag("hook-ledger-hooked");
    let _env = lock_and_enter(&hooked);
    let helper = y(&helper_exe());
    write_config(
        &hooked,
        &format!(
            "init:\n  - {}\non-event:\n  action.dispatched:\n    - {}\n  action.reported:\n    - {}\n",
            argv_yaml(&[helper.clone(), "ok".into()]),
            argv_yaml(&[helper.clone(), "loud".into()]),
            argv_yaml(&[helper, "ok".into()])
        ),
    );
    let s2 = start_next(&hooked);
    let (rc, _) = hooked.report_success(&s2, &hooked.next(&s2));
    assert_eq!(rc, 0);

    let count = |dir: &str| std::fs::read_dir(PathBuf::from(dir).join("audit")).unwrap().count();
    assert_eq!(count(&s), count(&s2), "hook 不应增删审计分片");
    let v = hooked.verify_audit(&s2);
    assert_eq!(v["replay_matches"], true);
}

// ---------- fork / cancel 点位 ----------

#[test]
fn fork_and_cancel_events_fire_hooks() {
    let driver = Driver::with_tag("hook-fc");
    let _env = lock_and_enter(&driver);
    let dump_fork = driver.tmp.join("dump-fork.json");
    let dump_cancel = driver.tmp.join("dump-cancel.json");
    let helper = y(&helper_exe());
    write_config(
        &driver,
        &format!(
            "on-event:\n  workflow.forked:\n    - {}\n  workflow.cancelled:\n    - {}\n",
            argv_yaml(&[helper.clone(), "ok".into(), y(&dump_fork)]),
            argv_yaml(&[helper, "ok".into(), y(&dump_cancel)])
        ),
    );
    // fork：research 成功后的 checkpoint
    let s1 = start_next(&driver);
    let (rc, _) = driver.report_success(&s1, &driver.next(&s1));
    assert_eq!(rc, 0);
    let seq = driver.state(&s1).seq;
    let (rc, _, err) = driver.run(&svec(&[
        "fork",
        "--session",
        &s1,
        "--seq",
        &seq.to_string(),
        "--output-root",
        driver.output_root.to_str().unwrap(),
        "--workflow",
        driver.workflow.to_str().unwrap(),
    ]));
    assert_eq!(rc, 0, "fork 失败: {err}");
    assert_eq!(read_dump(&dump_fork)["shard"]["event_type"], "workflow.forked");

    // cancel：派发后取消
    let s2 = driver.start("取消触发hook", None)["session_dir"].as_str().unwrap().to_string();
    let (rc, _, _) = driver.run(&svec(&["cancel", "--session", &s2]));
    assert_eq!(rc, 0);
    let envelope = read_dump(&dump_cancel);
    assert_eq!(envelope["shard"]["event_type"], "workflow.cancelled");
    assert_eq!(envelope["session_dir"].as_str().unwrap(), s2);
}
