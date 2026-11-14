//! 路径与命名原语：会话目录、文件名清洗、原子写入。
//!
//! 所有路径在状态文件与指令 JSON 中一律使用 posix 风格相对/绝对路径
//! （`dmaf-output/<session>/...`），仅在与文件系统接触时转为 Path。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use serde_json::Value;

/// Asia/Shanghai 无夏令时，固定 +08:00 偏移即可。
pub fn cst() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).expect("固定时区偏移")
}

pub fn now_cst() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&cst())
}

pub fn format_ts(dt: &DateTime<FixedOffset>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, false)
}

/// Path → posix 风格字符串（Windows 反斜杠转正斜杠）。
pub fn to_posix(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn is_illegal_char(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        || (c as u32) < 0x20
}

pub fn sanitize_filename(name: &str) -> String {
    sanitize_filename_len(name, 60)
}

pub fn sanitize_filename_len(name: &str, max_len: usize) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if is_illegal_char(c) { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_string();
    let cleaned = if cleaned.chars().count() > max_len {
        let cut: String = cleaned.chars().take(max_len).collect();
        cut.trim_end_matches(['.', ' ']).to_string()
    } else {
        cleaned
    };
    if cleaned.is_empty() {
        "_".to_string()
    } else {
        cleaned
    }
}

pub fn new_session_id(now: Option<&DateTime<FixedOffset>>) -> String {
    let n = now.copied().unwrap_or_else(now_cst);
    n.format("%Y%m%d-%H%M%S").to_string()
}

/// 返回一个不存在的会话目录；冲突时追加 -NN 后缀。
pub fn session_dir_for(output_root: &Path, session_id: &str) -> PathBuf {
    let mut candidate = output_root.join(session_id);
    let mut n = 1;
    while candidate.exists() {
        candidate = output_root.join(format!("{session_id}-{n:02}"));
        n += 1;
    }
    candidate
}

/// 构造会话内的 posix 风格相对路径：`<session_dir>/<parts...>`，过滤空段与 "."。
pub fn to_rel(session_dir: &str, parts: &[&str]) -> String {
    let mut all: Vec<&str> = vec![session_dir];
    all.extend_from_slice(parts);
    all.iter()
        .filter(|p| !p.is_empty() && **p != ".")
        .map(|s| *s)
        .collect::<Vec<_>>()
        .join("/")
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn atomic_write(path: &Path, write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let fname = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let tmp = parent.join(format!("{fname}.tmp-{}-{n}", process::id()));
    let result = (|| -> io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        write(&mut f)?;
        f.sync_all().ok();
        // std::fs::rename 在 Windows 上使用 MOVEFILE_REPLACE_EXISTING，可覆盖已存在目标
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn atomic_write_json(path: &Path, data: &Value) -> io::Result<()> {
    let text = serde_json::to_string_pretty(data).expect("json 序列化失败");
    atomic_write(path, |f| {
        use std::io::Write;
        f.write_all(text.as_bytes())
    })
}

pub fn atomic_write_text(path: &Path, text: &str) -> io::Result<()> {
    atomic_write(path, |f| {
        use std::io::Write;
        f.write_all(text.as_bytes())
    })
}
