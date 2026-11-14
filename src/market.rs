//! market 对接：共享底座 + `dmaf auth` / `dmaf publish` / `dmaf add <条目URL>`。
//!
//! 语义唯一依据为内部 market 指南与 openapi 契约（契约 v0；存 docs-dev/，不入库）。
//! 四条不可破的不变量：
//! 1. 归档解包 = 一个可被 `dmaf add --from <目录>` 直接合入的工程根（安装侧零新合并逻辑）；
//! 2. 打包必须确定性（market 以服务端字节 SHA-256 判幂等与冲突，打包不确定会把
//!    「提交中断后重跑」从幂等 201 变成 409）；
//! 3. 安装侧 fail closed：摘要不符、解包违规一律中止，目标工程零写入；
//! 4. 命令输出是拒绝原因的唯一通道（market 不持久化任何拒绝记录）。
//!
//! 待决策点全部按指南建议落地：E-2 `--bump` 一键升版 / E-3 doctor 仅 error 级拒绝 /
//! E-4 `--intro` 文件缺失交互补问 / E-5 用户级配置目录按 base 分键 /
//! E-6 复用 `--tag` 钉版本 / E-7 engine 不兼容告警后继续 / E-8 ureq（rustls）/
//! E-9 发布者账本内他包文件默认排除并在输出列出。

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{paths, Fail};

// ---------- 常量（market 契约默认值，见指南 §4.4/§6.3/§6.4） ----------

/// 内置默认 market 实例（官方地址，真机联调 2026-11-10 验证可用）；
/// 实际可用地址经 `--market` / `DMAF_MARKET_URL` / 用户配置覆盖。
pub const DEFAULT_MARKET_URL: &str = "https://dmaf.aicoffe.com";

/// market 包字节上限（MARKET_PACKAGE_MAX_BYTES 默认 20 MiB，超限 400 payload_too_large）。
const PACKAGE_MAX_BYTES: u64 = 20 * 1024 * 1024;
/// 归档解包总量上限（market 扫描规则，安装侧同规则纵深防御）。
const UNPACKED_MAX_BYTES: u64 = 200 * 1024 * 1024;
/// 归档成员数上限。
const MAX_MEMBERS: usize = 10_000;
/// multipart meta part 上限。
const META_MAX_BYTES: usize = 1024 * 1024;
/// compatibility 字段上限。
const COMPAT_MAX_BYTES: usize = 16 * 1024;
/// 取源下载读取上限：20 MiB 加余量。
const DOWNLOAD_CAP: u64 = 24 * 1024 * 1024;
/// 可重试错误的重试总次数（同一份字节，指数退避 1s/2s）。
const RETRY_ATTEMPTS: u32 = 3;

const PAT_PREFIX: &str = "dmafmt_";
const PAT_LEN: usize = 50;

// ---------- market 地址（指南 §4.1） ----------

/// 地址解析顺序：`--market` > 环境变量 `DMAF_MARKET_URL` > 用户配置 `[default] market` > 内置默认实例。
pub fn resolve_base(explicit: Option<&str>) -> Result<String, Fail> {
    let from_config = credentials_path()
        .ok()
        .and_then(|p| config_default_market(&p));
    let raw = explicit
        .map(str::to_string)
        .or_else(|| std::env::var("DMAF_MARKET_URL").ok())
        .or(from_config)
        .unwrap_or_else(|| DEFAULT_MARKET_URL.to_string());
    let base = raw.trim().trim_end_matches('/').to_string();
    validate_base(&base)
        .map_err(|e| Fail::Usage(crate::i18n::msg("market.invalid_base", &[&base, &e])))?;
    Ok(base)
}

/// 只接受 https://；http:// 仅放行 localhost / 127.0.0.1 / [::1]（与 market 对 MARKET_BASE_URL 的规则对称）。
fn validate_base(base: &str) -> Result<(), String> {
    if base.contains('?') || base.contains('#') {
        return Err(crate::i18n::t("market.base_no_query_fragment"));
    }
    let Some((scheme, rest)) = base.split_once("://") else {
        return Err(crate::i18n::t("market.base_missing_scheme"));
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err(crate::i18n::t("market.host_invalid"));
    }
    match scheme {
        "https" => Ok(()),
        "http" => {
            let host = authority.rsplit_once(':').map(|(h, _)| h).unwrap_or(authority);
            let host = host.trim_start_matches('[').trim_end_matches(']');
            if matches!(host, "localhost" | "127.0.0.1" | "::1") {
                Ok(())
            } else {
                Err(crate::i18n::t("market.http_local_only"))
            }
        }
        _ => Err(crate::i18n::t("market.scheme_https_only")),
    }
}

/// 路径段 percent-encoding（非 unreserved 字节 → %XX）。
fn url_encode_segment(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// ---------- 凭据存储（指南 §4.3） ----------

#[derive(Debug, Clone)]
pub struct Credentials {
    pub token: String,
    pub userid: String,
}

/// 凭据文件路径：用户级配置目录（Windows %APPDATA%\dmaf\credentials.toml，
/// Unix $XDG_CONFIG_HOME|$HOME/.config /dmaf/credentials.toml）。
/// 测试与 CI 可用 DMAF_CREDENTIALS_FILE 显式指定；不放工程目录、不放 .dmaf/。
fn credentials_path() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("DMAF_CREDENTIALS_FILE") {
        return Ok(PathBuf::from(p));
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return Ok(PathBuf::from(appdata).join("dmaf").join("credentials.toml"));
        }
        return Err(crate::i18n::msg("market.user_config_dir_not_found", &["APPDATA"]));
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("XDG_CONFIG_HOME")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| crate::i18n::msg("market.user_config_dir_not_found", &["HOME"]))?;
        return Ok(PathBuf::from(home)
            .join(".config")
            .join("dmaf")
            .join("credentials.toml"));
    }
}

fn read_cred_table(path: &Path) -> Result<toml::Table, Fail> {
    if !path.is_file() {
        return Ok(toml::Table::new());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| Fail::Io(crate::i18n::msg("market.credentials_read_failed", &[&e.to_string()])))?;
    toml::from_str(&text).map_err(|e| {
        Fail::Io(crate::i18n::msg(
            "market.credentials_not_toml",
            &[&paths::to_posix(path), &e.to_string()],
        ))
    })
}

fn cred_entry(table: &toml::Table, base: &str) -> Option<Credentials> {
    let entry = table.get("credentials")?.get(base)?;
    let token = entry.get("token")?.as_str()?.to_string();
    if token.is_empty() {
        return None;
    }
    Some(Credentials {
        token,
        userid: entry.get("userid").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
    })
}

fn write_cred_table(path: &Path, table: &toml::Table) -> Result<(), Fail> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Fail::Io(crate::i18n::msg("market.credentials_dir_create_failed", &[&e.to_string()])))?;
    }
    let body = toml::to_string(table)
        .map_err(|e| Fail::Io(crate::i18n::msg("market.credentials_serialize_failed", &[&e.to_string()])))?;
    std::fs::write(path, format!("{}{body}", crate::i18n::t("market.credentials_file_header")))
        .map_err(|e| Fail::Io(crate::i18n::msg("market.credentials_write_failed", &[&e.to_string()])))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// 保存凭据（auth login 成功后调用）。
fn save_credentials(path: &Path, base: &str, token: &str, userid: &str) -> Result<(), Fail> {
    let mut table = read_cred_table(path)?;
    let entry = table
        .entry("credentials")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if !entry.is_table() {
        return Err(Fail::Io(crate::i18n::t("market.credentials_section_not_table")));
    }
    if let toml::Value::Table(t) = entry {
        let mut slot = toml::Table::new();
        slot.insert("token".into(), toml::Value::String(token.to_string()));
        slot.insert("userid".into(), toml::Value::String(userid.to_string()));
        slot.insert(
            "saved_at".into(),
            toml::Value::String(paths::format_ts(&paths::now_cst())),
        );
        t.insert(base.to_string(), toml::Value::Table(slot));
    }
    write_cred_table(path, &table)
}

/// 删除本地凭据（auth logout）。返回是否确有删除。
fn remove_credentials(path: &Path, base: &str) -> Result<bool, Fail> {
    let mut table = read_cred_table(path)?;
    let removed = table
        .get_mut("credentials")
        .and_then(|c| c.as_table_mut())
        .map(|t| t.remove(base).is_some())
        .unwrap_or(false);
    if removed {
        write_cred_table(path, &table)?;
    }
    Ok(removed)
}

/// 本次请求用的 token：环境变量 DMAF_MARKET_TOKEN 覆盖（CI 场景不落盘），否则读本地凭据。
fn token_for_request(path: &Path, base: &str) -> Result<Option<String>, Fail> {
    if let Ok(t) = std::env::var("DMAF_MARKET_TOKEN") {
        if !t.trim().is_empty() {
            return Ok(Some(t.trim().to_string()));
        }
    }
    Ok(cred_entry(&read_cred_table(path)?, base).map(|c| c.token))
}

/// 用户配置的默认 market 地址（[default] market = "…"）。
fn config_default_market(path: &Path) -> Option<String> {
    let table = read_cred_table(path).ok()?;
    table
        .get("default")
        .and_then(|d| d.get("market"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// 任何输出和日志只显示前 12 字符（与 market token_prefix 同口径）。
fn mask_token(token: &str) -> String {
    let shown: String = token.chars().take(12).collect();
    if token.chars().count() > 12 {
        format!("{shown}…")
    } else {
        shown
    }
}

/// 本地格式预检：^dmafmt_[A-Za-z0-9_-]{43}$（共 50 字符）。不符直接拒绝，不发请求。
fn valid_token_format(token: &str) -> bool {
    token.len() == PAT_LEN
        && token.starts_with(PAT_PREFIX)
        && token[PAT_PREFIX.len()..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

// ---------- HTTP 客户端与错误信封（指南 §4.2 / §4.4） ----------

static HTTP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 每次命令调用生成一个 X-Request-ID（market 以它为 trace_id 入口）。
fn new_request_id() -> String {
    let seed = format!(
        "{}|{}|{}",
        std::process::id(),
        HTTP_COUNTER.fetch_add(1, Ordering::SeqCst),
        paths::now_cst().timestamp_micros(),
    );
    let hex = crate::audit::sha256_hex(seed.as_bytes());
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

struct Http {
    agent: ureq::Agent,
    request_id: String,
}

impl Http {
    fn new() -> Http {
        Http {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .timeout(Duration::from_secs(600))
                .user_agent(concat!("dmaf/", env!("CARGO_PKG_VERSION")))
                .build(),
            request_id: new_request_id(),
        }
    }

    /// 发出请求；非 2xx 也作为 Ok(响应) 返回（由调用方读状态与信封）。
    /// 匿名端点传 bearer = None（取源不携带凭据）。
    fn request(
        &self,
        method: &str,
        url: &str,
        bearer: Option<&str>,
        body: Option<(&str, &[u8])>,
    ) -> Result<ureq::Response, HttpFail> {
        let mut req = match method {
            "GET" => self.agent.get(url),
            "POST" => self.agent.post(url),
            _ => self.agent.request(method, url),
        };
        req = req.set("X-Request-ID", &self.request_id);
        if let Some(b) = bearer {
            req = req.set("Authorization", &format!("Bearer {b}"));
        }
        let res = match body {
            Some((content_type, bytes)) => {
                req.set("Content-Type", content_type).send_bytes(bytes)
            }
            None => req.call(),
        };
        match res {
            Ok(r) => Ok(r),
            // 非 2xx：ureq 把响应体放进 Error::Status，取出来当普通响应处理
            Err(ureq::Error::Status(_code, r)) => Ok(r),
            Err(ureq::Error::Transport(t)) => {
                Err(HttpFail::Transport(crate::i18n::msg("market.network_error", &[&t.to_string()])))
            }
        }
    }
}

fn header_of(resp: &ureq::Response, name: &str) -> Option<String> {
    resp.header(name).map(str::to_string)
}

/// market 错误信封（契约 Error：code/message/details/trace_id）。
#[derive(Debug, Clone)]
struct ErrorEnvelope {
    code: String,
    message: String,
    details: Vec<(String, String)>,
    trace_id: String,
}

fn parse_envelope(body: &str) -> Option<ErrorEnvelope> {
    let v: Value = serde_json::from_str(body).ok()?;
    let code = v.get("code")?.as_str()?.to_string();
    let message = v
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_string();
    let details = v
        .get("details")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    Some((
                        d.get("field")?.as_str()?.to_string(),
                        d.get("issue")?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let trace_id = v
        .get("trace_id")
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_string();
    Some(ErrorEnvelope { code, message, details, trace_id })
}

enum HttpFail {
    /// HTTP 非 2xx
    Status { status: u16, body: String, retry_after: Option<String> },
    /// 网络层失败（可重试）
    Transport(String),
    /// 立即失败的本地判定（如摘要失配——fail closed，不重试）
    Fatal(String),
}

impl HttpFail {
    fn status(status: u16, resp: ureq::Response) -> HttpFail {
        let retry_after = header_of(&resp, "Retry-After");
        let body = resp.into_string().unwrap_or_default();
        HttpFail::Status { status, body, retry_after }
    }
}

#[derive(Debug)]
enum Verdict {
    Retry(String),
    Fatal(Fail),
}

/// 错误映射（指南 §4.4 建议映射；退出码沿用引擎 2/3/4/5 约定）。
fn classify(fail: HttpFail, ctx: &str) -> Verdict {
    match fail {
        HttpFail::Transport(t) => Verdict::Retry(t),
        HttpFail::Fatal(m) => Verdict::Fatal(Fail::Io(m)),
        HttpFail::Status { status, body, retry_after } => {
            let env = parse_envelope(&body);
            let code = env.as_ref().map(|e| e.code.as_str());
            let summary = |env: Option<&ErrorEnvelope>, status: u16| match env {
                Some(e) => {
                    let trace = if e.trace_id.is_empty() {
                        String::new()
                    } else {
                        crate::i18n::msg("market.trace_id_suffix", &[&e.trace_id])
                    };
                    crate::i18n::msg(
                        "market.envelope_summary",
                        &[ctx, &status.to_string(), &e.code, &e.message, &trace],
                    )
                }
                None => crate::i18n::msg(
                    "market.not_error_envelope",
                    &[ctx, &status.to_string()],
                ),
            };
            // 唯一带语义的 5xx：托管内容完整性异常——禁止重试
            if code == Some("content_integrity_violation") {
                let s = summary(env.as_ref(), status);
                return Verdict::Fatal(Fail::Io(crate::i18n::msg(
                    "market.content_integrity_violation",
                    &[&s],
                )));
            }
            match status {
                400 | 401 | 403 | 404 => {
                    let mut problems: Vec<String> = env
                        .as_ref()
                        .map(|e| {
                            e.details
                                .iter()
                                .map(|(f, i)| format!("{f}: {i}"))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let mut message = summary(env.as_ref(), status);
                    match (status, code) {
                        (401, Some("unauthorized")) => {
                            let issue = env
                                .as_ref()
                                .and_then(|e| e.details.first().map(|(_, i)| i.clone()))
                                .unwrap_or_default();
                            message = match issue.as_str() {
                                "token_revoked" => crate::i18n::msg("market.token_revoked", &[ctx]),
                                _ => crate::i18n::msg("market.token_invalid", &[ctx]),
                            };
                            problems.clear();
                        }
                        (403, Some("namespace_not_owned")) => {
                            message = crate::i18n::msg("market.namespace_not_owned", &[ctx]);
                        }
                        (403, Some("forbidden")) => {
                            message = crate::i18n::msg("market.forbidden_credential_class", &[ctx]);
                        }
                        (404, Some("entry_not_found")) => {
                            message = crate::i18n::msg("market.entry_not_found", &[ctx]);
                        }
                        (404, Some("version_not_found")) => {
                            message = crate::i18n::msg("market.version_not_found", &[ctx]);
                        }
                        _ => {}
                    }
                    Verdict::Fatal(Fail::Schema { message, problems })
                }
                409 => {
                    let s = summary(env.as_ref(), status);
                    Verdict::Fatal(Fail::Conflict(crate::i18n::msg("market.version_conflict", &[&s])))
                }
                429 => {
                    let s = summary(env.as_ref(), status);
                    let wait = match retry_after {
                        Some(ra) => crate::i18n::msg("market.retry_after_hint", &[&ra]),
                        None => String::new(),
                    };
                    Verdict::Fatal(Fail::Io(crate::i18n::msg("market.rate_limited", &[&s, &wait])))
                }
                s if s >= 500 => {
                    let fallback = crate::i18n::t("market.no_error_code");
                    Verdict::Retry(crate::i18n::msg(
                        "market.server_error",
                        &[&s.to_string(), code.unwrap_or(&fallback)],
                    ))
                }
                s => Verdict::Fatal(Fail::Schema {
                    message: summary(env.as_ref(), s),
                    problems: vec![],
                }),
            }
        }
    }
}

/// 有界重试驱动：可重试错误用同一份请求重发（总尝试 RETRY_ATTEMPTS 次，退避 1s/2s）。
fn run_with_retry<T>(
    mut op: impl FnMut() -> Result<T, HttpFail>,
    ctx: &str,
) -> Result<T, Fail> {
    let mut attempt = 0u32;
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(fail) => match classify(fail, ctx) {
                Verdict::Fatal(f) => return Err(f),
                Verdict::Retry(reason) => {
                    attempt += 1;
                    if attempt >= RETRY_ATTEMPTS {
                        return Err(Fail::Io(crate::i18n::msg(
                            "market.retry_exhausted",
                            &[ctx, &(RETRY_ATTEMPTS - 1).to_string(), &reason],
                        )));
                    }
                    std::thread::sleep(Duration::from_secs(1 << (attempt - 1)));
                }
            },
        }
    }
}

// ---------- dmaf auth（指南 §5） ----------

fn whoami(base: &str, token: Option<&str>) -> Result<Option<(String, String, String)>, Fail> {
    let http = Http::new();
    let url = format!("{base}/api/me");
    let (status, body) = run_with_retry(
        || {
            let resp = http
                .request("GET", &url, token, None)
                .map_err(|e| e)?;
            let status = resp.status();
            if status != 200 {
                return Err(HttpFail::status(status, resp));
            }
            let text = resp.into_string().unwrap_or_default();
            let body = serde_json::from_str(&text).unwrap_or(Value::Null);
            Ok((status, body))
        },
        "GET /api/me",
    )?;
    let user = body.get("user");
    if status != 200 {
        // run_with_retry 的 Fatal 分支已把非 2xx 转成 Err，这里只是兜底
        return Err(Fail::Io(crate::i18n::msg("market.me_unexpected_status", &[&status.to_string()])));
    }
    match user {
        Some(u) if u.is_object() => {
            let userid = u.get("userid").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let provider = u.get("provider").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let display_name = u.get("display_name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if userid.is_empty() {
                return Err(Fail::Schema {
                    message: crate::i18n::t("market.me_missing_userid"),
                    problems: vec![],
                });
            }
            Ok(Some((userid, display_name, provider)))
        }
        _ => Ok(None),
    }
}

/// `dmaf auth login --token <PAT>`：格式预检 → GET /api/me 校验 → 保存本地凭据。
/// 不调用任何 /api/auth/* 端点（CLI 不走 OIDC，也不走 refresh 通道）。
pub fn auth_login(token: &str, market: Option<&str>) -> Result<Value, Fail> {
    let token = token.trim();
    if !valid_token_format(token) {
        return Err(Fail::Usage(crate::i18n::msg(
            "market.token_format_mismatch",
            &[PAT_PREFIX, &PAT_LEN.to_string(), &token.chars().count().to_string()],
        )));
    }
    let base = resolve_base(market)?;
    let Some((userid, display_name, provider)) = whoami(&base, Some(token))? else {
        return Err(Fail::Schema {
            message: crate::i18n::t("market.me_anonymous_with_credentials"),
            problems: vec![],
        });
    };
    let cred_path = credentials_path().map_err(Fail::Io)?;
    save_credentials(&cred_path, &base, token, &userid)?;
    Ok(json!({
        "ok": true,
        "command": "auth login",
        "market": base,
        "userid": userid,
        "display_name": display_name,
        "provider": provider,
        "credentials": paths::to_posix(&cred_path),
        "token": mask_token(token),
    }))
}

/// `dmaf auth logout`：删除本地凭据（服务端吊销走 Web tokens 页）。
pub fn auth_logout(market: Option<&str>) -> Result<Value, Fail> {
    let base = resolve_base(market)?;
    let cred_path = credentials_path().map_err(Fail::Io)?;
    let removed = remove_credentials(&cred_path, &base)?;
    Ok(json!({
        "ok": true,
        "command": "auth logout",
        "market": base,
        "removed": removed,
        "hint": crate::i18n::t("market.auth_logout_hint"),
    }))
}

/// `dmaf auth status`：调 /api/me 报告当前登录身份。
pub fn auth_status(market: Option<&str>) -> Result<Value, Fail> {
    let base = resolve_base(market)?;
    let cred_path = credentials_path().map_err(Fail::Io)?;
    let Some(token) = token_for_request(&cred_path, &base)? else {
        return Ok(json!({
            "ok": true,
            "command": "auth status",
            "market": base,
            "logged_in": false,
            "hint": crate::i18n::t("market.status_no_credentials"),
        }));
    };
    match whoami(&base, Some(&token)) {
        Ok(Some((userid, display_name, provider))) => Ok(json!({
            "ok": true,
            "command": "auth status",
            "market": base,
            "logged_in": true,
            "userid": userid,
            "display_name": display_name,
            "provider": provider,
            "token": mask_token(&token),
        })),
        Ok(None) => Ok(json!({
            "ok": true,
            "command": "auth status",
            "market": base,
            "logged_in": false,
            "hint": crate::i18n::t("market.status_anonymous"),
        })),
        Err(Fail::Schema { message, .. }) => Ok(json!({
            "ok": true,
            "command": "auth status",
            "market": base,
            "logged_in": false,
            "reason": message,
            "hint": crate::i18n::t("market.status_credentials_invalid"),
        })),
        Err(e) => Err(e),
    }
}

// ---------- dmaf publish（指南 §6） ----------

pub struct PublishArgs<'a> {
    pub dmaf_dir: &'a str,
    pub project: Option<&'a str>,
    pub version: Option<&'a str>,
    pub bump: bool,
    pub intro_file: Option<&'a str>,
    pub github_url: Option<&'a str>,
    pub market: Option<&'a str>,
    pub dry_run: bool,
}

/// 解析 publish 的工程根：位置参数指向 dmaf/ 定义层目录（默认 ./dmaf），
/// 工程根 = 其父目录；传工程根（其下有 dmaf/）也可。
fn resolve_project_root(dmaf_dir: &str) -> Result<(PathBuf, PathBuf), Fail> {
    let given = PathBuf::from(dmaf_dir);
    if given.file_name().and_then(|n| n.to_str()) == Some("dmaf") && given.is_dir() {
        let root = given
            .parent()
            .map(|p| {
                if p.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    p.to_path_buf()
                }
            })
            .unwrap_or_else(|| PathBuf::from("."));
        return Ok((root, given));
    }
    let inner = given.join("dmaf");
    if inner.is_dir() {
        return Ok((given, inner));
    }
    Err(Fail::Usage(crate::i18n::msg(
        "market.dmaf_dir_not_found",
        &[&paths::to_posix(&given)],
    )))
}

fn project_regex_ok(s: &str) -> bool {
    // ^[a-z0-9][a-z0-9-]{0,98}[a-z0-9]$
    let b = s.as_bytes();
    if b.len() < 2 || b.len() > 100 {
        return false;
    }
    let head = b[0].is_ascii_lowercase() || b[0].is_ascii_digit();
    let tail = b[b.len() - 1].is_ascii_lowercase() || b[b.len() - 1].is_ascii_digit();
    let body = b
        .iter()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-');
    head && tail && body
}

fn version_regex_ok(s: &str) -> bool {
    // ^[0-9A-Za-z][0-9A-Za-z._+-]{0,63}$
    let b = s.as_bytes();
    if b.is_empty() || b.len() > 64 || !b[0].is_ascii_alphanumeric() {
        return false;
    }
    b[1..]
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'+' | b'-'))
}

/// E-2 一键升版：数字尾段（形如 1.2.3 / 2026.11）递增最后一段；其余无法机械升版。
fn bump_version(v: &str) -> Option<String> {
    let mut segments: Vec<String> = v.split('.').map(String::from).collect();
    let last = segments.last_mut()?;
    let n = last.parse::<u64>().ok()?;
    *last = (n + 1).to_string();
    let joined = segments.join(".");
    // 复核产物合法
    if version_regex_ok(&joined) {
        Some(joined)
    } else {
        None
    }
}

/// 把升版结果回写 dmaf/manifest.yaml（逐行替换顶层 version: 行，保留其余内容与注释）。
fn rewrite_manifest_version(root: &Path, from: &str, to: &str) -> Result<(), Fail> {
    let path = root.join("dmaf").join("manifest.yaml");
    let text = std::fs::read_to_string(&path).map_err(|e| {
        Fail::Io(crate::i18n::msg("market.read_failed", &[&paths::to_posix(&path), &e.to_string()]))
    })?;
    let mut out: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in text.lines() {
        if !replaced && line.starts_with("version:") && line.contains(from) {
            out.push(format!("version: {to}"));
            replaced = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !replaced {
        return Err(Fail::Io(crate::i18n::msg(
            "market.manifest_version_line_missing",
            &[&paths::to_posix(&path), from],
        )));
    }
    out.push(String::new());
    crate::paths::atomic_write_text(&path, &out.join("\n"))
        .map_err(|e| Fail::Io(crate::i18n::msg("market.manifest_rewrite_failed", &[&e.to_string()])))?;
    Ok(())
}

/// E-4 介绍三要素：--intro 文件（YAML：what_it_is/what_it_does/how_to_use），
/// 缺失时交互逐项补问（提示走 stderr，stdout 保持纯 JSON）。
fn load_intro(file: Option<&str>) -> Result<[String; 3], Fail> {
    const KEYS: [&str; 3] = ["what_it_is", "what_it_does", "how_to_use"];
    if let Some(f) = file {
        let text = std::fs::read_to_string(f)
            .map_err(|e| Fail::Io(crate::i18n::msg("market.intro_read_failed", &[f, &e.to_string()])))?;
        let value: serde_yaml::Value = serde_yaml::from_str(&text)
            .map_err(|e| Fail::Schema { message: crate::i18n::msg("market.intro_not_yaml", &[f]), problems: vec![e.to_string()] })?;
        let map = value.as_mapping().ok_or_else(|| Fail::Schema {
            message: crate::i18n::msg("market.intro_top_not_mapping", &[f]),
            problems: vec![],
        })?;
        let mut out = [String::new(), String::new(), String::new()];
        let mut missing: Vec<&str> = vec![];
        for (i, key) in KEYS.iter().enumerate() {
            match map.get(&serde_yaml::Value::String((*key).into())) {
                Some(v) => out[i] = crate::workflow::ystr(v),
                None => missing.push(key),
            }
        }
        if !missing.is_empty() {
            let sep = crate::i18n::t("market.list_separator");
            return Err(Fail::Schema {
                message: crate::i18n::msg("market.intro_missing_fields", &[&missing.join(sep.as_str())]),
                problems: vec![],
            });
        }
        return Ok(out);
    }
    // 交互补问
    let prompts = [
        crate::i18n::t("market.intro_prompt_what_it_is"),
        crate::i18n::t("market.intro_prompt_what_it_does"),
        crate::i18n::t("market.intro_prompt_how_to_use"),
    ];
    let mut out = [String::new(), String::new(), String::new()];
    for (i, p) in prompts.iter().enumerate() {
        eprintln!("{}", crate::i18n::msg("market.intro_prompt_line", &[p]));
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| Fail::Io(crate::i18n::msg("market.intro_read_input_failed", &[&e.to_string()])))?;
        let line = line.trim();
        if line.is_empty() {
            return Err(Fail::Schema {
                message: crate::i18n::t("market.intro_missing_all"),
                problems: vec![],
            });
        }
        out[i] = line.to_string();
    }
    Ok(out)
}

fn validate_intro(intro: &[String; 3]) -> Result<(), Fail> {
    const KEYS: [&str; 3] = ["what_it_is", "what_it_does", "how_to_use"];
    const LIMITS: [usize; 3] = [2000, 2000, 4000];
    let mut problems: Vec<String> = vec![];
    for (i, text) in intro.iter().enumerate() {
        let text = text.trim();
        let key = KEYS[i];
        if text.is_empty() {
            problems.push(crate::i18n::msg("market.intro_field_missing", &[key, &LIMITS[i].to_string()]));
        } else if text.chars().count() > LIMITS[i] {
            problems.push(crate::i18n::msg("market.intro_field_too_long", &[key, &LIMITS[i].to_string()]));
        }
        if text.contains('<') {
            problems.push(crate::i18n::msg("market.intro_html_forbidden", &[key]));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(Fail::Schema { message: crate::i18n::t("market.intro_precheck_failed"), problems })
    }
}

/// 组装 multipart meta part 的 JSON（契约 PublishMeta，additionalProperties: false）。
fn build_meta(
    project: &str,
    version: &str,
    intro: &[String; 3],
    github_url: Option<&str>,
    manifest: &crate::manifest::Manifest,
) -> Result<String, Fail> {
    let mut meta = serde_json::Map::new();
    meta.insert("project".into(), json!(project));
    meta.insert("version".into(), json!(version));
    meta.insert(
        "intro".into(),
        json!({
            "what_it_is": intro[0].trim(),
            "what_it_does": intro[1].trim(),
            "how_to_use": intro[2].trim(),
        }),
    );
    if let Some(g) = github_url {
        meta.insert("github_url".into(), json!(g));
    }
    let mut compat = serde_json::Map::new();
    compat.insert("dmaf_engine".into(), json!(env!("CARGO_PKG_VERSION")));
    if !manifest.engine.trim().is_empty() {
        compat.insert("engine".into(), json!(manifest.engine.trim()));
    }
    if !manifest.harnesses.is_empty() {
        compat.insert("harnesses".into(), json!(manifest.harnesses));
    }
    let compat_len = serde_json::to_string(&Value::Object(compat.clone()))
        .expect("compat serialize failed")
        .len();
    if compat_len > COMPAT_MAX_BYTES {
        return Err(Fail::Schema {
            message: crate::i18n::msg("market.compat_too_large", &[&compat_len.to_string()]),
            problems: vec![],
        });
    }
    meta.insert("compatibility".into(), Value::Object(compat));
    let text = serde_json::to_string(&Value::Object(meta)).expect("meta serialize failed");
    if text.len() > META_MAX_BYTES {
        return Err(Fail::Schema {
            message: crate::i18n::msg("market.meta_too_large", &[&text.len().to_string()]),
            problems: vec![],
        });
    }
    Ok(text)
}

/// 门禁一（编译）：stage/scope 库存在时编译零错误才通过；纯资产包跳过。
fn compile_gate(project_root: &Path) -> Result<Value, Fail> {
    let stages = project_root.join("dmaf/workflows/stages");
    let scopes = project_root.join("dmaf/workflows/scopes");
    if !stages.is_dir() || !scopes.is_dir() {
        return Ok(json!({"skipped": crate::i18n::t("market.compile_gate_skipped")}));
    }
    crate::compile::compile(&stages, &scopes, None).map_err(|e: crate::compile::CompileError| {
        Fail::Schema {
            message: crate::i18n::t("market.compile_gate_failed"),
            problems: e.problems,
        }
    })?;
    Ok(json!({"ok": true}))
}

/// 门禁二（doctor 核心检查，E-3 仅 error 级拒绝）：
/// layout / artifacts_drift / manifest 失败 = error（拒绝）；
/// dmaf_hygiene 失败 = warning（不随包分发的运行区问题，打印后放行）。
fn doctor_gate(project_root: &Path) -> Result<(Value, Vec<String>), Fail> {
    let checks = crate::cli::doctor_core_checks(project_root, "dmaf/compiled");
    let mut errors: Vec<String> = vec![];
    let mut warnings: Vec<String> = vec![];
    for c in &checks {
        if c.ok {
            continue;
        }
        match c.name {
            "layout" => {
                let missing: Vec<String> = c.detail["missing"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let sep = crate::i18n::t("market.list_separator");
                errors.push(crate::i18n::msg(
                    "market.gate_layout_incomplete",
                    &[&missing.join(sep.as_str())],
                ));
            }
            "artifacts_drift" => {
                let default = crate::i18n::t("market.artifacts_drift_default_reason");
                let reason = c.detail["reason"].as_str().unwrap_or(&default);
                errors.push(crate::i18n::msg("market.gate_artifacts_drift", &[reason]));
            }
            "manifest" => {
                let default = crate::i18n::t("market.manifest_incompatible_default");
                let hint = c.detail["hint"]
                    .as_str()
                    .or_else(|| c.detail["error"].as_str())
                    .unwrap_or(&default);
                errors.push(crate::i18n::msg("market.gate_manifest_compat", &[hint]));
            }
            "dmaf_hygiene" => warnings.push(crate::i18n::t("market.gate_dmaf_hygiene")),
            _ => {}
        }
    }
    if !errors.is_empty() {
        return Err(Fail::Schema {
            message: crate::i18n::t("market.doctor_gate_failed"),
            problems: errors,
        });
    }
    Ok((json!({"ok": true, "errors": 0, "warnings": warnings.len()}), warnings))
}

/// E-9：按发布者账本排除属于其他已安装包的文件（否则同一文件被两个包的账本同时认领，
/// 且等于转发他人的包），并在输出中列出。
fn exclude_other_packages(
    project_root: &Path,
    mut files: Vec<(String, PathBuf, String)>,
    own_name: &str,
) -> Result<(Vec<(String, PathBuf, String)>, Vec<String>), Fail> {
    let ledger_path = project_root.join("dmaf").join("packages.yaml");
    let ledger = crate::package::read_ledger(&ledger_path).map_err(fail_from_package)?;
    let mut excluded: Vec<String> = vec![];
    files.retain(|(rel, _, _)| {
        let claimed = ledger
            .iter()
            .any(|(name, entry)| name != own_name && entry.files.contains_key(rel));
        if claimed {
            excluded.push(rel.clone());
        }
        !claimed
    });
    Ok((files, excluded))
}

/// multipart/form-data 手拼（固定两个 part：meta JSON + package 字节流）。
fn multipart_body(meta_json: &str, package: &[u8], boundary: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"meta\"\r\nContent-Type: application/json\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(meta_json.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"package\"; filename=\"package.tar.gz\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(package);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

/// `dmaf publish` 流水线（指南 §6.2）：每一步失败即终止，前四步不发出任何请求。
pub fn publish(args: PublishArgs) -> Result<Value, Fail> {
    // 0. 工程根与 market 地址
    let (project_root, _) = resolve_project_root(args.dmaf_dir)?;
    let base = resolve_base(args.market)?;

    // 1. 登录态
    let cred_path = credentials_path().map_err(Fail::Io)?;
    let Some(token) = token_for_request(&cred_path, &base)? else {
        return Err(Fail::Usage(crate::i18n::msg("market.not_logged_in", &[&base])));
    };

    // 2. 门禁一：编译
    let compile_report = compile_gate(&project_root)?;

    // 3. 门禁二：doctor（error 拒绝 / warning 放行）
    let (doctor_report, warnings) = doctor_gate(&project_root)?;

    // 4. 本地预检：manifest、版本、project、介绍、包完整性与大小
    let manifest = match crate::manifest::load(&project_root) {
        Ok(Some(m)) if !m.name.trim().is_empty() => m,
        Ok(Some(_)) => {
            return Err(Fail::Schema {
                message: crate::i18n::t("market.manifest_missing_name"),
                problems: vec![],
            })
        }
        Ok(None) => {
            return Err(Fail::Schema {
                message: crate::i18n::t("market.manifest_missing"),
                problems: vec![],
            })
        }
        Err(e) => {
            return Err(Fail::Schema { message: crate::i18n::t("market.manifest_invalid"), problems: vec![e] })
        }
    };

    let current_version = manifest.version.trim().to_string();
    let version = if let Some(v) = args.version {
        v.trim().to_string()
    } else if args.bump {
        let bumped = bump_version(&current_version).ok_or_else(|| Fail::Schema {
            message: crate::i18n::msg("market.bump_failed", &[&current_version]),
            problems: vec![],
        })?;
        rewrite_manifest_version(&project_root, &current_version, &bumped)?;
        bumped
    } else {
        current_version.clone()
    };
    if version.is_empty() {
        return Err(Fail::Schema {
            message: crate::i18n::t("market.version_missing"),
            problems: vec![],
        });
    }
    if version == "latest" {
        return Err(Fail::Schema {
            message: crate::i18n::t("market.version_latest_forbidden"),
            problems: vec![],
        });
    }
    if !version_regex_ok(&version) {
        return Err(Fail::Schema {
            message: crate::i18n::msg("market.version_invalid", &[&version]),
            problems: vec![],
        });
    }

    let project = args
        .project
        .map(str::trim)
        .map(str::to_string)
        .unwrap_or_else(|| manifest.name.trim().to_string());
    if !project_regex_ok(&project) {
        return Err(Fail::Schema {
            message: crate::i18n::msg("market.project_invalid", &[&project]),
            problems: vec![],
        });
    }

    let intro = load_intro(args.intro_file)?;
    validate_intro(&intro)?;
    if let Some(g) = args.github_url {
        let ok = g.starts_with("https://github.com/")
            && g[19..].split('/').filter(|s| !s.is_empty()).count() == 2
            && !g[19..].contains('?');
        if !ok {
            return Err(Fail::Schema {
                message: crate::i18n::msg("market.github_url_invalid", &[&g]),
                problems: vec![],
            });
        }
    }

    // 包文件集 + E-9 排除 + 上限（归档安全规则本地同检，保证合法包永不触发服务端拒绝）
    let files = crate::package::package_files(&project_root).map_err(fail_from_package)?;
    let (mut files, excluded_other) =
        exclude_other_packages(&project_root, files, manifest.name.trim())?;
    // 归档必含 dmaf/manifest.yaml（merge_package 源校验需要 name；合入时它属工程自属槽位，
    // 不会覆盖安装方的 manifest——指南 §6.3。package_files 按槽位规则排除了它，这里补回）
    let manifest_path = project_root.join("dmaf").join("manifest.yaml");
    let manifest_hash = crate::audit::sha256_hex(
        &std::fs::read(&manifest_path)
            .map_err(|e| Fail::Io(crate::i18n::msg("market.read_manifest_failed", &[&e.to_string()])))?,
    );
    files.push(("dmaf/manifest.yaml".into(), manifest_path, manifest_hash));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    if files.is_empty() {
        return Err(Fail::Schema {
            message: crate::i18n::t("market.package_empty"),
            problems: vec![],
        });
    }
    if files.len() > MAX_MEMBERS {
        return Err(Fail::Schema {
            message: crate::i18n::msg(
                "market.too_many_members",
                &[&files.len().to_string(), &MAX_MEMBERS.to_string()],
            ),
            problems: vec![],
        });
    }
    let mut total_bytes: u64 = 0;
    for (_, path, _) in &files {
        let meta = std::fs::metadata(path).map_err(|e| {
            Fail::Io(crate::i18n::msg("market.metadata_failed", &[&format!("{path:?}"), &e.to_string()]))
        })?;
        total_bytes += meta.len();
    }
    if total_bytes > UNPACKED_MAX_BYTES {
        return Err(Fail::Schema {
            message: crate::i18n::msg("market.total_size_exceeds", &[&total_bytes.to_string()]),
            problems: vec![],
        });
    }

    // 5. 确定性打包（这一份字节供重试复用）
    let archive = pack_archive(&files).map_err(Fail::Io)?;
    if archive.len() as u64 > PACKAGE_MAX_BYTES {
        return Err(Fail::Schema {
            message: crate::i18n::msg(
                "market.archive_too_large",
                &[&archive.len().to_string(), &PACKAGE_MAX_BYTES.to_string()],
            ),
            problems: vec![],
        });
    }
    let digest = format!("sha256:{}", crate::audit::sha256_hex(&archive));
    let meta_json = build_meta(&project, &version, &intro, args.github_url, &manifest)?;

    let package_summary = json!({
        "files": files.len(),
        "total_bytes": total_bytes,
        "archive_bytes": archive.len(),
        "digest": digest,
    });

    if args.dry_run {
        return Ok(json!({
            "ok": true,
            "command": "publish",
            "dry_run": true,
            "market": base,
            "project_root": paths::to_posix(&project_root),
            "project": project,
            "version": version,
            "meta": serde_json::from_str::<Value>(&meta_json).unwrap_or(Value::Null),
            "package": package_summary,
            "gates": {"compile": compile_report, "doctor": doctor_report},
            "excluded_other_packages": excluded_other,
            "warnings": warnings,
            "hint": crate::i18n::t("market.dry_run_hint"),
        }));
    }

    // 6-7. 上传（可重试错误只重发第 5 步的那份字节）
    let http = Http::new();
    let boundary = format!("dmaf-{}", &http.request_id.replace('-', "")[..24]);
    let body = multipart_body(&meta_json, &archive, &boundary);
    let url = format!("{base}/api/publications");
    let request_id = http.request_id.clone();
    let outcome = run_with_retry(
        || {
            let resp = http
                .request(
                    "POST",
                    &url,
                    Some(&token),
                    Some((
                        &format!("multipart/form-data; boundary={boundary}"),
                        &body,
                    )),
                )
                .map_err(|e| e)?;
            let status = resp.status();
            if status != 201 {
                return Err(HttpFail::status(status, resp));
            }
            let idempotent = header_of(&resp, "X-Idempotent-Replay").as_deref() == Some("true");
            let text = resp.into_string().unwrap_or_default();
            let body: Value = serde_json::from_str(&text).map_err(|e| {
                HttpFail::Fatal(crate::i18n::msg("market.publish_201_not_json", &[&e.to_string()]))
            })?;
            Ok((body, idempotent))
        },
        "POST /api/publications",
    )?;
    let (result, idempotent_replay) = outcome;
    let get_str = |k: &str| result.get(k).and_then(|v| v.as_str()).map(String::from);
    let namespace = result.get("namespace").cloned().unwrap_or(Value::Null);
    for key in ["namespace", "version", "digest", "entry_url"] {
        if result.get(key).map(|v| v.is_null()).unwrap_or(true) {
            return Err(Fail::Schema {
                message: crate::i18n::msg("market.publish_201_missing_field", &[key]),
                problems: vec![format!("request_id: {request_id}")],
            });
        }
    }

    // 8. 输出受理结果（引擎 stdout JSON 约定）
    Ok(json!({
        "ok": true,
        "command": "publish",
        "market": base,
        "namespace": namespace,
        "version": get_str("version"),
        "digest": get_str("digest"),
        "entry_url": get_str("entry_url"),
        "lifecycle_status": get_str("lifecycle_status").unwrap_or_else(|| "listed".into()),
        "idempotent_replay": idempotent_replay,
        "message": if idempotent_replay { crate::i18n::t("market.publish_idempotent_replay") } else { crate::i18n::t("market.publish_accepted") },
        "package": package_summary,
        "gates": {"compile": compile_report, "doctor": doctor_report},
        "excluded_other_packages": excluded_other,
        "warnings": warnings,
    }))
}

// ---------- 确定性打包与安全解包（指南 §6.3 / §7.4） ----------

/// 确定性 tar.gz：成员按路径排序（package_files 已排序）、mtime=0、uid/gid=0、
/// uname/gname 空、mode 规范化 0644、gzip 头 mtime=0 不写文件名、OS 字节固定 255、
/// 压缩级别固定 6；文件字节原样打包。同一工程连续两次打包 SHA-256 必须相同。
pub fn pack_archive(files: &[(String, PathBuf, String)]) -> Result<Vec<u8>, String> {
    let gz = flate2::GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::new(), flate2::Compression::new(6));
    let mut builder = tar::Builder::new(gz);
    for (rel, path, _) in files {
        let meta = std::fs::metadata(path)
            .map_err(|e| crate::i18n::msg("market.metadata_failed", &[&format!("{path:?}"), &e.to_string()]))?;
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_path(rel)
            .map_err(|e| crate::i18n::msg("market.pack_member_path_invalid", &[rel, &e.to_string()]))?;
        header.set_size(meta.len());
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        // new_gnu() 的 uname/gname 字段为零字节（空），天然确定
        header.set_cksum();
        let f = std::fs::File::open(path)
            .map_err(|e| crate::i18n::msg("market.open_failed", &[&format!("{path:?}"), &e.to_string()]))?;
        builder
            .append_data(&mut header, rel, f)
            .map_err(|e| crate::i18n::msg("market.pack_member_failed", &[rel, &e.to_string()]))?;
    }
    let gz = builder
        .into_inner()
        .map_err(|e| crate::i18n::msg("market.tar_finish_failed", &[&e.to_string()]))?;
    gz.finish().map_err(|e| crate::i18n::msg("market.gzip_finish_failed", &[&e.to_string()]))
}

/// 安全解包（market 扫描规则同款，纵深防御，不因服务端已校验而省略）：
/// 拒绝绝对路径、`..`、反斜杠路径、符号链接/硬链接/设备文件/FIFO 等非普通成员；
/// 解压总量 ≤ 200 MiB、成员数 ≤ 10000；所有成员必须位于 dmaf/ 之下。
/// 任一违规即中止（调用方负责清理临时目录，目标工程零写入）。
pub fn safe_unpack(archive: &[u8], dest: &Path) -> Result<(), String> {
    let gz = flate2::read::GzDecoder::new(archive);
    let mut ar = tar::Archive::new(gz);
    let mut total: u64 = 0;
    let mut count: usize = 0;
    let entries = ar
        .entries()
        .map_err(|e| crate::i18n::msg("market.archive_read_failed", &[&e.to_string()]))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| crate::i18n::msg("market.entry_read_failed", &[&e.to_string()]))?;
        let header = entry.header();
        let etype = header.entry_type();
        if !matches!(etype, tar::EntryType::Regular | tar::EntryType::Directory) {
            return Err(crate::i18n::msg("market.non_regular_member", &[&format!("{etype:?}")]));
        }
        let rel = header
            .path()
            .map_err(|e| crate::i18n::msg("market.entry_path_failed", &[&e.to_string()]))?
            .to_string_lossy()
            .into_owned();
        let rel = rel.trim_start_matches("./").to_string();
        if rel.is_empty() {
            return Err(crate::i18n::t("market.empty_member_name"));
        }
        if rel.starts_with('/') || rel.contains('\\') || rel.contains('\0') {
            return Err(crate::i18n::msg("market.member_path_invalid", &[&rel]));
        }
        if rel.split('/').any(|seg| seg == "..") {
            return Err(crate::i18n::msg("market.member_traversal", &[&rel]));
        }
        if rel != "dmaf" && !rel.starts_with("dmaf/") {
            return Err(crate::i18n::msg("market.member_outside_dmaf", &[&rel]));
        }
        count += 1;
        if count > MAX_MEMBERS {
            return Err(crate::i18n::msg("market.member_count_exceeds", &[&MAX_MEMBERS.to_string()]));
        }
        let size = header
            .entry_size()
            .map_err(|e| crate::i18n::msg("market.entry_size_failed", &[&e.to_string()]))?;
        total += size;
        if total > UNPACKED_MAX_BYTES {
            return Err(crate::i18n::msg("market.unpacked_exceeds", &[&total.to_string()]));
        }
        let out = dest.join(&rel);
        if etype == tar::EntryType::Directory {
            std::fs::create_dir_all(&out)
                .map_err(|e| crate::i18n::msg("market.mkdir_member_failed", &[&rel, &e.to_string()]))?;
        } else {
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .map_err(|e| crate::i18n::msg("market.member_data_read_failed", &[&rel, &e.to_string()]))?;
            if data.len() as u64 != size {
                return Err(crate::i18n::msg(
                    "market.member_size_mismatch",
                    &[&rel, &size.to_string(), &data.len().to_string()],
                ));
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| crate::i18n::msg("market.mkdir_failed", &[&e.to_string()]))?;
            }
            std::fs::write(&out, &data)
                .map_err(|e| crate::i18n::msg("market.member_write_failed", &[&rel, &e.to_string()]))?;
        }
    }
    if count == 0 {
        return Err(crate::i18n::t("market.archive_empty"));
    }
    Ok(())
}

// ---------- dmaf add <条目URL>（指南 §7） ----------

/// 条目 URL 三元组：{base}/{userid}/{project}（ADR-0007）。
#[derive(Debug, Clone)]
pub struct EntryRef {
    pub base: String,
    pub userid: String,
    pub project: String,
}

/// URL 解析（指南 §7.2）：去尾 `/`；带 query/fragment 直接拒绝；
/// 路径末两段 = userid、project，其前全部 = market base（base 允许带路径前缀）。
pub fn parse_entry_url(input: &str) -> Result<EntryRef, String> {
    let s = input.trim();
    if s.contains('?') || s.contains('#') {
        return Err(crate::i18n::t("market.entry_url_no_query"));
    }
    let (scheme, rest) = s
        .split_once("://")
        .ok_or_else(|| crate::i18n::t("market.entry_url_no_scheme"))?;
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, p),
        None => (rest, ""),
    };
    if authority.is_empty() || authority.contains('@') {
        return Err(crate::i18n::t("market.host_invalid"));
    }
    let segments: Vec<&str> = path.split('/').filter(|seg| !seg.is_empty()).collect();
    if segments.len() < 2 {
        return Err(crate::i18n::msg("market.entry_url_too_few_segments", &[path]));
    }
    let project = segments[segments.len() - 1];
    let userid = segments[segments.len() - 2];
    if userid.is_empty() || userid.len() > 64 {
        return Err(crate::i18n::msg("market.userid_invalid", &[userid]));
    }
    if !project_regex_ok(project) {
        return Err(crate::i18n::msg("market.project_regex_invalid", &[project]));
    }
    let prefix = segments[..segments.len() - 2].join("/");
    let base = if prefix.is_empty() {
        format!("{scheme}://{authority}")
    } else {
        format!("{scheme}://{authority}/{prefix}")
    };
    validate_base(&base)?;
    Ok(EntryRef { base, userid: userid.to_string(), project: project.to_string() })
}

/// 条目 URL 规范形（不含版本）：账本 source 字段记录值。
fn normalized_entry(entry: &EntryRef) -> String {
    format!("{}/{}/{}", entry.base, entry.userid, entry.project)
}

struct Download {
    /// 归档临时文件（调用方读完负责清理）
    file: PathBuf,
    /// sha256:<hex>（本地计算 = X-Content-Digest）
    digest: String,
    /// X-Entry-Version（latest 已由服务端展开）
    entry_version: String,
}

static DOWNLOAD_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 匿名取源：流式落临时文件并计算 SHA-256，读取上限 20 MiB 加余量。
fn download_package(http: &Http, url: &str) -> Result<Download, Fail> {
    let ctx = crate::i18n::t("market.ctx_get_source");
    run_with_retry(
        || {
            let resp = http.request("GET", url, None, None).map_err(|e| e)?;
            let status = resp.status();
            if status != 200 {
                return Err(HttpFail::status(status, resp));
            }
            let digest = header_of(&resp, "X-Content-Digest").ok_or_else(|| {
                HttpFail::Fatal(crate::i18n::t("market.missing_digest_header"))
            })?;
            let entry_version = header_of(&resp, "X-Entry-Version").ok_or_else(|| {
                HttpFail::Fatal(crate::i18n::t("market.missing_entry_version_header"))
            })?;
            if !(digest.starts_with("sha256:") && digest[7..].len() == 64) {
                return Err(HttpFail::Fatal(crate::i18n::msg(
                    "market.digest_format_invalid",
                    &[&digest],
                )));
            }
            if let Some(cl) = header_of(&resp, "Content-Length").and_then(|v| v.parse::<u64>().ok()) {
                if cl > DOWNLOAD_CAP {
                    return Err(HttpFail::Fatal(crate::i18n::msg(
                        "market.download_over_cap",
                        &[&cl.to_string(), &DOWNLOAD_CAP.to_string()],
                    )));
                }
            }
            let file = std::env::temp_dir().join(format!(
                "dmaf-mkt-{}-{}",
                std::process::id(),
                DOWNLOAD_COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
            let mut out = std::fs::File::create(&file)
                .map_err(|e| HttpFail::Fatal(crate::i18n::msg("market.create_tmp_failed", &[&e.to_string()])))?;
            let mut hasher = Sha256::new();
            let mut reader = resp.into_reader();
            let mut buf = [0u8; 16 * 1024];
            let mut total: u64 = 0;
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|e| HttpFail::Fatal(crate::i18n::msg("market.download_read_failed", &[&e.to_string()])))?;
                if n == 0 {
                    break;
                }
                total += n as u64;
                if total > DOWNLOAD_CAP {
                    let _ = std::fs::remove_file(&file);
                    return Err(HttpFail::Fatal(crate::i18n::msg(
                        "market.download_exceeded_cap",
                        &[&DOWNLOAD_CAP.to_string()],
                    )));
                }
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])
                    .map_err(|e| HttpFail::Fatal(crate::i18n::msg("market.tmp_write_failed", &[&e.to_string()])))?;
            }
            out.flush()
                .map_err(|e| HttpFail::Fatal(crate::i18n::msg("market.tmp_flush_failed", &[&e.to_string()])))?;
            let local = format!("sha256:{}", hex_of(&hasher));
            if local != digest {
                let _ = std::fs::remove_file(&file);
                return Err(HttpFail::Fatal(crate::i18n::msg(
                    "market.digest_mismatch",
                    &[&local, &digest],
                )));
            }
            Ok(Download { file, digest: local, entry_version })
        },
        &ctx,
    )
}

fn hex_of(h: &Sha256) -> String {
    let digest = h.clone().finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// `dmaf add <条目URL>`（指南 §7.3）：全部在临时目录完成，
/// 交给 merge_package 之前的任何失败都不会写目标工程。
/// 版本钉选 = tag 参数（E-6 复用 --tag），缺省 latest（以 X-Entry-Version 为实际版本）。
pub fn add_entry(
    target_root: &Path,
    entry_url: &str,
    tag: Option<&str>,
    opts: &crate::package::MergeOptions,
) -> Result<Value, crate::package::PackageError> {
    let entry = parse_entry_url(entry_url).map_err(crate::package::PackageError::Usage)?;
    let source_label = normalized_entry(&entry);
    let version_seg = tag.unwrap_or("latest");
    let url = format!(
        "{}/api/entries/{}/{}/packages/{}?source=add",
        entry.base,
        url_encode_segment(&entry.userid),
        url_encode_segment(&entry.project),
        url_encode_segment(version_seg),
    );
    let http = Http::new();
    let dl = download_package(&http, &url).map_err(fail_from_market)?;
    let archive = std::fs::read(&dl.file)
        .map_err(|e| crate::package::PackageError::Io(crate::i18n::msg("market.read_download_tmp_failed", &[&e.to_string()])));
    let _ = std::fs::remove_file(&dl.file);
    let archive = archive?;

    let unpack_dir = std::env::temp_dir().join(format!(
        "dmaf-mkt-unpack-{}-{}",
        std::process::id(),
        DOWNLOAD_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&unpack_dir);
    let result = unpack_and_merge(&archive, &unpack_dir, target_root, &source_label, Some(&dl), opts);
    let _ = std::fs::remove_dir_all(&unpack_dir);
    let mut report = result?;
    if let Some(obj) = report.as_object_mut() {
        obj.insert(
            "market".into(),
            json!({
                "entry": source_label,
                "requested": version_seg,
                "version": dl.entry_version,
                "digest": dl.digest,
            }),
        );
    }
    Ok(report)
}

/// `dmaf add --from <本地 .tar.gz>`（指南 §8 可选增强）：离线下载的归档仍走
/// 同一套安全解包与合入，不出现手工拷贝的安装通道。
pub fn add_archive(
    target_root: &Path,
    archive_path: &Path,
    opts: &crate::package::MergeOptions,
) -> Result<Value, crate::package::PackageError> {
    let meta = std::fs::metadata(archive_path).map_err(|e| {
        crate::package::PackageError::Usage(crate::i18n::msg(
            "market.archive_unreadable",
            &[&paths::to_posix(archive_path), &e.to_string()],
        ))
    })?;
    if meta.len() > DOWNLOAD_CAP {
        return Err(crate::package::PackageError::Usage(crate::i18n::msg(
            "market.archive_over_cap",
            &[&meta.len().to_string()],
        )));
    }
    let archive = std::fs::read(archive_path).map_err(|e| {
        crate::package::PackageError::Io(crate::i18n::msg(
            "market.archive_read_file_failed",
            &[&paths::to_posix(archive_path), &e.to_string()],
        ))
    })?;
    let unpack_dir = std::env::temp_dir().join(format!(
        "dmaf-mkt-unpack-{}-{}",
        std::process::id(),
        DOWNLOAD_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&unpack_dir);
    let result = unpack_and_merge(&archive, &unpack_dir, target_root, &paths::to_posix(archive_path), None, opts);
    let _ = std::fs::remove_dir_all(&unpack_dir);
    result
}

/// 安全解包 → manifest 预检 → engine 兼容性（E-7 告警后继续）→ 既有 merge_package。
fn unpack_and_merge(
    archive: &[u8],
    unpack_dir: &Path,
    target_root: &Path,
    source_label: &str,
    dl: Option<&Download>,
    opts: &crate::package::MergeOptions,
) -> Result<Value, crate::package::PackageError> {
    use crate::package::PackageError;
    safe_unpack(archive, unpack_dir).map_err(|m| {
        PackageError::Usage(crate::i18n::msg("market.unpack_rejected", &[&m]))
    })?;
    let manifest = match crate::manifest::load(unpack_dir) {
        Ok(Some(m)) if !m.name.trim().is_empty() => m,
        Ok(Some(_)) => {
            return Err(PackageError::Usage(crate::i18n::t(
                "market.archive_manifest_no_name",
            )))
        }
        Ok(None) => {
            return Err(PackageError::Usage(crate::i18n::t(
                "market.archive_manifest_missing",
            )))
        }
        Err(e) => return Err(PackageError::Usage(crate::i18n::msg("market.archive_manifest_invalid", &[&e.to_string()]))),
    };
    // E-7：engine 范围不兼容 → 告警后继续（与现行 add 行为一致）
    let compatibility = if manifest.engine.trim().is_empty() {
        None
    } else {
        match crate::manifest::engine_matches(manifest.engine.trim(), env!("CARGO_PKG_VERSION")) {
            Ok(true) => None,
            Ok(false) => Some(crate::i18n::msg(
                "market.engine_incompatible",
                &[manifest.engine.trim(), env!("CARGO_PKG_VERSION")],
            )),
            Err(e) => Some(crate::i18n::msg("market.engine_range_unparseable", &[&e.to_string()])),
        }
    };

    let mut opts = opts.clone();
    opts.source_label = source_label.to_string();
    if let Some(dl) = dl {
        // 账本记录实际版本（不记 latest 字面量）与托管摘要（指南 §7.5）
        opts.tag = Some(dl.entry_version.clone());
        opts.digest = Some(dl.digest.clone());
    }
    let mut report = crate::package::merge_package(unpack_dir, target_root, &opts)?;
    if let (Some(warning), Some(obj)) = (compatibility, report.as_object_mut()) {
        obj.insert("compatibility_warning".into(), json!(warning));
    }
    Ok(report)
}

fn fail_from_package(e: crate::package::PackageError) -> Fail {
    use crate::package::PackageError;
    match e {
        PackageError::Usage(m) => Fail::Usage(m),
        PackageError::Schema { message, problems } => Fail::Schema { message, problems },
        PackageError::Conflict(list) => Fail::Conflict(crate::py_list(&list)),
        PackageError::Compile(problems) => Fail::Schema {
            message: crate::i18n::t("market.merge_compile_failed"),
            problems,
        },
        PackageError::Install(m) | PackageError::Io(m) => Fail::Io(m),
    }
}

fn fail_from_market(e: Fail) -> crate::package::PackageError {
    match e {
        Fail::Usage(m) => crate::package::PackageError::Usage(m),
        Fail::Schema { message, problems } => crate::package::PackageError::Schema { message, problems },
        Fail::Conflict(m) => crate::package::PackageError::Usage(m),
        Fail::Io(m) => crate::package::PackageError::Io(m),
    }
}

// ---------- 测试 ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_format_precheck() {
        assert!(valid_token_format(&format!("{PAT_PREFIX}{}{}", "A1".repeat(21), "A")));
        assert!(!valid_token_format("short"));
        assert!(!valid_token_format(&format!("{PAT_PREFIX}{}", "x".repeat(42))));
        assert!(!valid_token_format(&format!("{PAT_PREFIX}{}", "x".repeat(44))));
        assert!(!valid_token_format(&format!("nope_{}", "x".repeat(43))));
        assert!(!valid_token_format(&format!("{PAT_PREFIX}{}!", "x".repeat(42))));
    }

    #[test]
    fn mask_keeps_12_chars() {
        let t = format!("{PAT_PREFIX}abcdefghijklm");
        assert_eq!(mask_token(&t), "dmafmt_abcde…");
    }

    #[test]
    fn base_validation() {
        // 内置默认实例必须是合法 base（防常量笔误）
        assert!(validate_base(DEFAULT_MARKET_URL).is_ok());
        assert!(validate_base("https://market.example.dev").is_ok());
        assert!(validate_base("https://market.example.dev/prefix").is_ok());
        assert!(validate_base("http://localhost:8080").is_ok());
        assert!(validate_base("http://127.0.0.1:3000").is_ok());
        assert!(validate_base("http://[::1]:3000").is_ok());
        assert!(validate_base("http://example.com").is_err());
        assert!(validate_base("ftp://x").is_err());
        assert!(validate_base("https://host?a=1").is_err());
    }

    #[test]
    fn entry_url_parse() {
        let e = parse_entry_url("https://market.example.dev/alice/demo").unwrap();
        assert_eq!(e.base, "https://market.example.dev");
        assert_eq!(e.userid, "alice");
        assert_eq!(e.project, "demo");
        // 尾斜杠与路径前缀 base
        let e = parse_entry_url("https://m.example.dev/prefix/bob/my-pkg/").unwrap();
        assert_eq!(e.base, "https://m.example.dev/prefix");
        assert_eq!(e.userid, "bob");
        assert_eq!(e.project, "my-pkg");
        // 拒绝 query / fragment
        assert!(parse_entry_url("https://m/x/y?download=1").is_err());
        assert!(parse_entry_url("https://m/x/y#top").is_err());
        // project 形 / 段数不足
        assert!(parse_entry_url("https://m/x/Bad_Project").is_err());
        assert!(parse_entry_url("https://m/only-one-segment").is_err());
        // http 非本机
        assert!(parse_entry_url("http://m.dev/x/y").is_err());
        let e = parse_entry_url("http://127.0.0.1:9000/alice/demo").unwrap();
        assert_eq!(e.base, "http://127.0.0.1:9000");
    }

    #[test]
    fn version_regex() {
        assert!(version_regex_ok("0.1.0"));
        assert!(version_regex_ok("20261108"));
        assert!(version_regex_ok("v2+build.1"));
        // 形态合法；字面量禁令是语义规则，由 publish 单独拦
        assert!(version_regex_ok("latest"));
        assert!(!version_regex_ok(".1.0"));
        assert!(!version_regex_ok(""));
    }

    #[test]
    fn bump_increments_last_numeric_segment() {
        assert_eq!(bump_version("0.1.0").unwrap(), "0.1.1");
        assert_eq!(bump_version("2026.11").unwrap(), "2026.12");
        assert_eq!(bump_version("7").unwrap(), "8");
        assert!(bump_version("v1").is_none());
        assert!(bump_version("").is_none());
    }

    #[test]
    fn project_regex() {
        assert!(project_regex_ok("demo"));
        assert!(project_regex_ok("my-pkg-2"));
        assert!(!project_regex_ok("-lead"));
        assert!(!project_regex_ok("trail-"));
        assert!(!project_regex_ok("Upper"));
        assert!(!project_regex_ok("a"));
    }

    #[test]
    fn pack_is_deterministic_and_unpack_roundtrips() {
        let root = std::env::temp_dir().join(format!("dmaf-mkt-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("dmaf/skills/x")).unwrap();
        std::fs::write(root.join("dmaf/manifest.yaml"), "name: t\nversion: 0.1.0\n").unwrap();
        std::fs::write(root.join("dmaf/skills/x/SKILL.md"), "content body\n").unwrap();
        // publish 真实形态：package_files（已排除槽位）+ manifest 补回（指南 §6.3 必含）
        let mut files = crate::package::package_files(&root).unwrap();
        files.push((
            "dmaf/manifest.yaml".into(),
            root.join("dmaf/manifest.yaml"),
            crate::audit::sha256_hex(b"name: t\nversion: 0.1.0\n"),
        ));
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let a1 = pack_archive(&files).unwrap();
        let a2 = pack_archive(&files).unwrap();
        assert_eq!(crate::audit::sha256_hex(&a1), crate::audit::sha256_hex(&a2));
        assert_eq!(a1, a2);
        // 反向：解包回工程根
        let dest = root.join("unpacked");
        std::fs::create_dir_all(&dest).unwrap();
        safe_unpack(&a1, &dest).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("dmaf/manifest.yaml")).unwrap(),
            "name: t\nversion: 0.1.0\n"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("dmaf/skills/x/SKILL.md")).unwrap(),
            "content body\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 手工构造单成员 ustar 字节（tar::Builder 拒绝写 .. / dmaf 外成员，恶意归档须 raw 构造）。
    fn raw_tar_entry(name: &str, data: &[u8], typeflag: u8) -> Vec<u8> {
        let mut hdr = [0u8; 512];
        let nb = name.as_bytes();
        hdr[..nb.len()].copy_from_slice(nb);
        hdr[100..108].copy_from_slice(b"0000644\0");
        hdr[108..116].copy_from_slice(b"0000000\0");
        hdr[116..124].copy_from_slice(b"0000000\0");
        hdr[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
        hdr[136..148].copy_from_slice(b"00000000000\0");
        // chksum 占位（空格）
        hdr[148..156].copy_from_slice(b"        ");
        hdr[156] = typeflag;
        hdr[257..263].copy_from_slice(b"ustar\0");
        hdr[263..265].copy_from_slice(b"00");
        let sum: u32 = hdr.iter().map(|b| *b as u32).sum();
        hdr[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
        let mut out = hdr.to_vec();
        out.extend_from_slice(data);
        let pad = (512 - data.len() % 512) % 512;
        out.extend(std::iter::repeat(0u8).take(pad));
        // 归档结束块
        out.extend_from_slice(&[0u8; 1024]);
        // safe_unpack 入口是 gzip 流
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
        gz.write_all(&out).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn safe_unpack_rejects_traversal_and_foreign_roots() {
        let dest = std::env::temp_dir().join(format!("dmaf-mkt-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::create_dir_all(&dest).unwrap();
        // 成员不在 dmaf/ 之下
        let err = safe_unpack(&raw_tar_entry("outside/x.txt", b"1", b'0'), &dest).unwrap_err();
        assert!(err.contains("dmaf/"), "actual error: {err}");
        // .. 穿越
        let err = safe_unpack(&raw_tar_entry("../escape.txt", b"2", b'0'), &dest).unwrap_err();
        assert!(err.contains(".."), "actual error: {err}");
        // 符号链接成员（typeflag '2'）
        let err = safe_unpack(&raw_tar_entry("dmaf/link", b"", b'2'), &dest).unwrap_err();
        assert!(err.contains("Symlink"), "actual error: {err}");
        assert!(!dest.join("dmaf").exists() || std::fs::read_dir(dest.join("dmaf")).unwrap().count() == 0);
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn envelope_mapping_exit_classes() {
        // 409 → Conflict（退出码 4）
        let v = classify(
            HttpFail::Status {
                status: 409,
                body: r#"{"code":"version_conflict","message":"content differs","trace_id":"t1"}"#.into(),
                retry_after: None,
            },
            "POST",
        );
        assert!(matches!(v, Verdict::Fatal(Fail::Conflict(_))));
        // 401 → Schema（退出码 3），issue=token_revoked 给吊销口径
        let v = classify(
            HttpFail::Status {
                status: 401,
                body: r#"{"code":"unauthorized","message":"auth failed","details":[{"field":"authorization","issue":"token_revoked"}],"trace_id":"t2"}"#.into(),
                retry_after: None,
            },
            "GET /api/me",
        );
        match v {
            Verdict::Fatal(Fail::Schema { message, .. }) => assert!(message.contains("Web tokens")),
            other => panic!("expected Schema, actual {other:?}"),
        }
        // 429 → Io（退出码 5）带 Retry-After
        let v = classify(
            HttpFail::Status {
                status: 429,
                body: r#"{"code":"rate_limit_exceeded","message":"too fast","trace_id":"t3"}"#.into(),
                retry_after: Some("17".into()),
            },
            "POST",
        );
        match v {
            Verdict::Fatal(Fail::Io(m)) => assert!(m.contains("17")),
            other => panic!("expected Io, actual {other:?}"),
        }
        // content_integrity_violation（500）→ 禁止重试的 Io
        let v = classify(
            HttpFail::Status {
                status: 500,
                body: r#"{"code":"content_integrity_violation","message":"m","trace_id":"t4"}"#.into(),
                retry_after: None,
            },
            "GET",
        );
        assert!(matches!(v, Verdict::Fatal(Fail::Io(_))));
        // 503 blob_unavailable → 可重试
        let v = classify(
            HttpFail::Status {
                status: 503,
                body: r#"{"code":"blob_unavailable","message":"m","trace_id":"t5"}"#.into(),
                retry_after: None,
            },
            "GET",
        );
        assert!(matches!(v, Verdict::Retry(_)));
    }
}
