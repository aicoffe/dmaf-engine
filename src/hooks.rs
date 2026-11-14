//! 项目级 hook（hooks.yaml）：引擎只按配置顺序调用，不读结果。
//!
//! 契约：
//! - init 点位：`dmaf start` 建会话目录之前顺序执行 `init` 列表，结果不影响启动；
//! - on-event 点位：每条审计分片落盘后，按分片 event_type 查 `on-event` 表，
//!   命中则顺序执行；未命中/未配置不动作；
//! - 同步等待每个 hook 退出（timeout_secs 全局上限，超时杀掉、列表继续）；
//!   顺序执行，不做并发；
//! - 退出码与输出一律不读不判：单个 hook 失败不阻断列表，也不影响所在命令的
//!   stdout JSON、退出码、状态与账本；匹配仅到事件类型查表为止，无通配/过滤；
//! - hook stdout 转发到 engine stderr（引擎 stdout 契约是纯 JSON，不可被污染；
//!   必须排空管道防写满互死），hook stderr 直通 engine stderr；
//! - 命令统一为 argv 语义：字符串按空白切分（不解析引号），列表原样；
//!   直接 spawn 不经 shell；hook 继承引擎进程的 env 与 CWD；
//! - hooks.yaml 位于进程 CWD 下的 dmaf/hooks.yaml（定义层）；缺失 = 无 hook（静默，正常态）；
//!   坏配置 → stderr 警告 + 有效部分降级生效（doctor 可体检）。

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::audit;

pub const CONFIG_FILE: &str = "dmaf/hooks.yaml";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
pub const ENV_NO_HOOKS: &str = "DMAF_NO_HOOKS";

static NO_HOOKS: AtomicBool = AtomicBool::new(false);

/// `--no-hooks` 旗标：run() 按当次命令覆写（进程内测试可重复驱动）。
pub fn set_disabled(disabled: bool) {
    NO_HOOKS.store(disabled, Ordering::SeqCst);
}

fn disabled() -> bool {
    NO_HOOKS.load(Ordering::SeqCst)
        || std::env::var(ENV_NO_HOOKS)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
}

/// 一条 hook 命令（统一 argv 语义）。
type Argv = Vec<String>;

#[derive(Debug, Default)]
pub struct HooksConfig {
    pub timeout_secs: u64,
    pub init: Vec<Argv>,
    pub on_event: BTreeMap<String, Vec<Argv>>,
    /// 加载期降级说明（stderr 已实时输出；doctor 复用全量清单）。
    pub warnings: Vec<String>,
}

fn warn(msg: &str) {
    eprintln!("⚠ {msg}");
}

/// 警告每次加载配置都会重新产生，进程内只打印一次，避免一次 report 多事件刷屏。
fn emit_warnings_once(warnings: &[String]) {
    static EMITTED: std::sync::Mutex<BTreeSet<String>> = std::sync::Mutex::new(BTreeSet::new());
    if let Ok(mut seen) = EMITTED.lock() {
        for w in warnings {
            if seen.insert(w.clone()) {
                warn(w);
            }
        }
    }
}

// ---------- 配置加载 ----------

fn parse_command(v: &serde_yaml::Value, where_: &str, warnings: &mut Vec<String>) -> Option<Argv> {
    match v {
        serde_yaml::Value::String(s) => {
            let argv: Vec<String> = s.split_whitespace().map(str::to_string).collect();
            if argv.is_empty() {
                warnings.push(crate::i18n::msg("hooks.empty_command", &[where_]));
                None
            } else {
                Some(argv)
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            let mut argv: Vec<String> = Vec::new();
            for item in seq {
                match item.as_str() {
                    Some(s) => argv.push(s.to_string()),
                    None => warnings.push(crate::i18n::msg("hooks.item_not_string", &[where_])),
                }
            }
            if argv.is_empty() {
                warnings.push(crate::i18n::msg("hooks.empty_command", &[where_]));
                None
            } else {
                Some(argv)
            }
        }
        other => {
            warnings.push(crate::i18n::msg(
                "hooks.command_type_invalid",
                &[where_, &format!("{other:?}")],
            ));
            None
        }
    }
}

fn parse_command_list(
    v: &serde_yaml::Value,
    where_: &str,
    warnings: &mut Vec<String>,
) -> Vec<Argv> {
    match v.as_sequence() {
        Some(seq) => seq
            .iter()
            .filter_map(|c| parse_command(c, where_, warnings))
            .collect(),
        None => {
            warnings.push(crate::i18n::msg("hooks.command_list_required", &[where_]));
            Vec::new()
        }
    }
}

/// 读 CWD 下 dmaf/hooks.yaml；文件缺失返回 None（静默）。
/// 坏配置 → stderr 警告，有效部分降级生效。
pub fn load() -> Option<HooksConfig> {
    load_impl(false)
}

/// doctor 用：无视开关与文件缺失的静默语义，报告配置健康度。
pub fn doctor_report() -> Value {
    match load_impl(true) {
        Some(cfg) => json!({
            "configured": true,
            "timeout_secs": cfg.timeout_secs,
            "init": cfg.init.len(),
            "on_event_keys": cfg.on_event.keys().collect::<Vec<_>>(),
            "warnings": cfg.warnings,
        }),
        None => json!({"configured": false}),
    }
}

fn load_impl(ignore_disabled: bool) -> Option<HooksConfig> {
    if !ignore_disabled && disabled() {
        return None;
    }
    let text = match std::fs::read_to_string(CONFIG_FILE) {
        Ok(t) => t,
        Err(_) => return None, // 缺失 = 无 hook（静默）
    };
    let yaml: serde_yaml::Value = match serde_yaml::from_str(&text) {
        Ok(y) => y,
        Err(e) => {
            let msg = crate::i18n::msg("hooks.yaml_parse_fail", &[&e.to_string()]);
            emit_warnings_once(std::slice::from_ref(&msg));
            return Some(HooksConfig { warnings: vec![msg], ..Default::default() });
        }
    };
    let mut warnings = Vec::new();
    let null = serde_yaml::Value::Null;
    let get = |key: &str| -> &serde_yaml::Value {
        yaml.as_mapping()
            .and_then(|m| m.get(serde_yaml::Value::String(key.into())))
            .unwrap_or(&null)
    };

    let timeout_secs = match get("timeout_secs") {
        serde_yaml::Value::Null => DEFAULT_TIMEOUT_SECS,
        v => match v.as_u64() {
            Some(n) if n >= 1 => n,
            _ => {
                warnings.push(crate::i18n::msg(
                    "hooks.timeout_invalid",
                    &[&DEFAULT_TIMEOUT_SECS.to_string()],
                ));
                DEFAULT_TIMEOUT_SECS
            }
        },
    };

    let init = match get("init") {
        serde_yaml::Value::Null => Vec::new(),
        v => parse_command_list(v, "hooks.yaml init", &mut warnings),
    };

    let mut on_event: BTreeMap<String, Vec<Argv>> = BTreeMap::new();
    match get("on-event") {
        serde_yaml::Value::Null => {}
        v => match v.as_mapping() {
            Some(m) => {
                for (k, val) in m {
                    let key = k.as_str().unwrap_or_default().to_string();
                    if !audit::is_valid_event_type(&key) {
                        warnings.push(crate::i18n::msg("hooks.unknown_event_type", &[&key]));
                        continue;
                    }
                    let where_ = format!("hooks.yaml on-event.{key}");
                    on_event.insert(key, parse_command_list(val, &where_, &mut warnings));
                }
            }
            None => warnings.push(crate::i18n::t("hooks.on_event_not_mapping")),
        },
    }

    emit_warnings_once(&warnings);
    Some(HooksConfig { timeout_secs, init, on_event, warnings })
}

// ---------- 执行 ----------

/// 顺序执行一个 hook：写 stdin、转发 stdout→engine stderr、同步等待、超时杀掉。
/// 退出码与输出一律不读不判。
fn exec_hook(argv: &[String], payload: &str, timeout: Duration) {
    let desc = argv.join(" ");
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            warn(&crate::i18n::msg("hooks.spawn_fail", &[&desc, &e.to_string()]));
            return;
        }
    };
    // stdout 转发到 engine stderr：piped-不读会因管道缓冲写满而与 hook 互死，必须排空
    let reader = child.stdout.take().map(|mut out| {
        std::thread::spawn(move || {
            let mut err = std::io::stderr();
            let _ = std::io::copy(&mut out, &mut err);
            let _ = err.flush();
        })
    });
    if let Some(mut stdin) = child.stdin.take() {
        // hook 提前退出导致写 stdin 失败（BrokenPipe）属 hook 自身行为，忽略
        let _ = stdin.write_all(payload.as_bytes());
    }
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    while let Ok(None) = child.try_wait() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    if let Some(handle) = reader {
        let _ = handle.join();
    }
    if timed_out {
        warn(&crate::i18n::msg(
            "hooks.timeout_killed",
            &[&timeout.as_secs().to_string(), &desc],
        ));
    }
}

fn exec_list(list: &[Argv], payload: &str, timeout_secs: u64) {
    let timeout = Duration::from_secs(timeout_secs);
    for argv in list {
        exec_hook(argv, payload, timeout);
    }
}

/// init 点位：start 建会话目录之前调用；无论 hook 结果如何都返回。
pub fn run_init(payload: &Value) {
    let Some(cfg) = load() else { return };
    if cfg.init.is_empty() {
        return;
    }
    let text = serde_json::to_string_pretty(payload).unwrap_or_default();
    exec_list(&cfg.init, &text, cfg.timeout_secs);
}

/// on-event 点位：审计分片落盘后调用；按分片 event_type 查表，命中则顺序执行。
/// stdin = `{"session_dir": ..., "shard": {…分片内容…}}`（与落盘分片 JSON 内容一致）。
pub fn notify(shard: &Value, session_dir: &str) {
    let Some(cfg) = load() else { return };
    let event = shard
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let Some(list) = cfg.on_event.get(event) else { return };
    if list.is_empty() {
        return;
    }
    let text = serde_json::to_string_pretty(&json!({
        "session_dir": session_dir,
        "shard": shard,
    }))
    .unwrap_or_default();
    exec_list(list, &text, cfg.timeout_secs);
}
