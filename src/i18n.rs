//! i18n：人面文案的双语目录与 locale 解析（方案 docs-dev/国际化实施方案.md §7）。
//!
//! 设计约束：
//! - 手写静态三元组目录 `(key, zh, en)`，编译期同构、零新依赖；
//! - locale 是进程事实（CLI 单命令），预设解析顺序：`DMAF_LOCALE` 显式 >
//!   系统语言探测（zh* → zh，其余语言 → en，探测失败兜底 en）；
//!   start 按 `--locale` > 预设覆盖并烘焙进会话状态；会话命令在
//!   `state::load` 后以会话 locale 覆盖（next/report/... 读会话不读 env）；
//!   旧会话无 locale 字段回退 zh；不支持运行中途热切换（改 locale = 新会话）；
//! - 显式 [`set`] 过的 locale 不被预设覆盖（测试驱动锁定 zh / 同进程会话回退
//!   依赖此语义，探测只作用于真实进程的全新状态）；
//! - 目录文案占位符用 `{}` 顺序占位，由 [`msg`] 按序替换（与 format! 解耦）。

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Locale {
    Zh,
    En,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Locale::Zh => "zh",
            Locale::En => "en",
        }
    }

    /// 未知值回退 zh（旧会话无字段 / 任意串）。
    pub fn from_str(s: &str) -> Locale {
        match s {
            "en" => Locale::En,
            _ => Locale::Zh,
        }
    }
}

/// bit0-1 = locale（0 = Zh，1 = En）；bit2 = 显式设置标记（[set] 置位，
/// 预设见到即跳过——探测只允许作用于未被显式锁定的进程状态）。
static CURRENT: AtomicU8 = AtomicU8::new(0);

const EXPLICIT: u8 = 0b100;

pub fn set(l: Locale) {
    CURRENT.store(l as u8 | EXPLICIT, Ordering::SeqCst);
}

pub fn current() -> Locale {
    match CURRENT.load(Ordering::SeqCst) & 0b11 {
        1 => Locale::En,
        _ => Locale::Zh,
    }
}

/// run() 入口的 locale 预设：`DMAF_LOCALE` 显式（按 [`Locale::from_str`]，
/// 未知值 zh）> 系统语言探测 > en 兜底。进程内已被显式 [`set`] 过则跳过。
pub fn preset() {
    if CURRENT.load(Ordering::SeqCst) & EXPLICIT != 0 {
        return;
    }
    let l = match std::env::var("DMAF_LOCALE") {
        Ok(v) => Locale::from_str(&v),
        Err(_) => detect_system_locale().unwrap_or(Locale::En),
    };
    CURRENT.store(l as u8, Ordering::SeqCst);
}

/// 系统语言探测：zh 系语言 → [`Locale::Zh`]，其余一切语言 → [`Locale::En`]
/// （国际开源缺省），探测不到 → None（调用方兜底）。
pub fn detect_system_locale() -> Option<Locale> {
    let subtag = system_locale_subtag()?;
    Some(match subtag.to_ascii_lowercase().starts_with("zh") {
        true => Locale::Zh,
        false => Locale::En,
    })
}

/// 平台语言子标签（`en_US.UTF-8` / `zh-CN` → `"en"` / `"zh"`）；探测不到 → None。
#[cfg(unix)]
fn system_locale_subtag() -> Option<String> {
    // POSIX 惯例优先级：LC_ALL > LC_MESSAGES > LANG
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if let Some(t) = parse_posix_locale(&v) {
                return Some(t);
            }
        }
    }
    None
}

/// Windows 用用户默认 locale 名（BCP-47，如 `zh-CN`），kernel32 FFI 直链，零新依赖。
#[cfg(windows)]
fn system_locale_subtag() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(lpLocaleName: *mut u16, cchLocaleName: i32) -> i32;
    }
    let mut buf = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 1 {
        return None; // 0 = 失败；1 = 空名
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or((n as usize) - 1);
    let name = String::from_utf16_lossy(&buf[..len]);
    name.split('-').next().filter(|s| !s.is_empty()).map(str::to_string)
}

/// POSIX locale 值解析：`"en_US.UTF-8"` → `Some("en")`；`"C"` / `"POSIX"` / 空 → None。
fn parse_posix_locale(s: &str) -> Option<String> {
    let lang = s.split(['_', '.', '@']).next().unwrap_or("");
    match lang.to_ascii_lowercase().as_str() {
        "" | "c" | "posix" => None,
        t => Some(t.to_string()),
    }
}

/// 查当前 locale 文案；未知 key 回落原样 key（目录缺项在测试中被 key 对齐断言拦截）。
pub fn t(key: &str) -> String {
    match lookup(key) {
        Some((zh, en)) => match current() {
            Locale::En => en,
            Locale::Zh => zh,
        }
        .to_string(),
        None => key.to_string(),
    }
}

/// [`t`] 的静态引用版：目录值本就是 `&'static str`，供 clap derive 属性等
/// 需要 `&'static str`（`Str: From<&str>` 而非 `From<String>`）的调用点使用。
/// 目录缺项无法回落非 `'static` 的入参 key，返回哨兵串（缺项由测试与基准比对拦截）。
pub fn t_static(key: &str) -> &'static str {
    match lookup(key) {
        Some((zh, en)) => match current() {
            Locale::En => en,
            Locale::Zh => zh,
        },
        None => "MISSING-I18N-KEY",
    }
}

/// 带占位替换：目录文案用 `{}` 顺序占位，按 args 顺序逐个替换一次。
pub fn msg(key: &str, args: &[&str]) -> String {
    let mut out = t(key).to_string();
    for a in args {
        out = out.replacen("{}", a, 1);
    }
    out
}

/// 目录全量条目（key 唯一性与 zh/en 非空由 tests/test_i18n.rs 强制）。
pub fn all_entries() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    catalogs::ALL.iter().flat_map(|g| g.iter().copied())
}

fn lookup(key: &str) -> Option<(&'static str, &'static str)> {
    for group in catalogs::ALL {
        for (k, zh, en) in *group {
            if *k == key {
                return Some((*zh, *en));
            }
        }
    }
    None
}

mod catalogs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_locale_parsing() {
        assert_eq!(parse_posix_locale("en_US.UTF-8").as_deref(), Some("en"));
        assert_eq!(parse_posix_locale("zh_CN").as_deref(), Some("zh"));
        assert_eq!(parse_posix_locale("de_DE@euro").as_deref(), Some("de"));
        assert_eq!(parse_posix_locale("C"), None);
        assert_eq!(parse_posix_locale("POSIX"), None);
        assert_eq!(parse_posix_locale(""), None);
    }

    #[test]
    fn detect_maps_zh_to_zh_and_everything_else_to_en() {
        assert_eq!(detect_by("zh_CN.UTF-8"), Some(Locale::Zh));
        assert_eq!(detect_by("zh"), Some(Locale::Zh));
        assert_eq!(detect_by("en_US.UTF-8"), Some(Locale::En));
        assert_eq!(detect_by("de_DE.UTF-8"), Some(Locale::En));
        assert_eq!(detect_by("ja_JP"), Some(Locale::En));
        // "C" 系 locale 不是语言：解析层即 None，由 preset 兜底 en
        assert_eq!(detect_by("C.UTF-8"), None);
        assert_eq!(detect_by(""), None);
    }

    /// 经 posix 解析路径驱动 detect（unix/win 共用同一映射逻辑）。
    fn detect_by(posix_value: &str) -> Option<Locale> {
        let subtag = parse_posix_locale(posix_value)?;
        Some(match subtag.to_ascii_lowercase().starts_with("zh") {
            true => Locale::Zh,
            false => Locale::En,
        })
    }

    /// 显式 set 过的 locale 不被 preset 覆盖（无论 env 如何；探测不参与）。
    #[test]
    fn preset_respects_explicit_set() {
        set(Locale::Zh);
        preset();
        assert_eq!(current(), Locale::Zh);
        set(Locale::En);
        preset();
        assert_eq!(current(), Locale::En);
        set(Locale::Zh);
    }
}
