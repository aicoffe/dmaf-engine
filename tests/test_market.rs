//! market 对接验收（内部指南 §10 联调验收清单的离线版）：
//! auth login（L1/L2/P7）、publish 门禁与上传（P1/P2/P4/P8/P9）、add 条目 URL（A1/A3/A5/A6）。
//! marketd 用本进程内的假 TCP 服务替代（同端口真实 HTTP 语义）。

mod common;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use common::{lock_cwd, unique_tmp, CwdGuard};
use serde_json::{json, Value};

const VALID_TOKEN: &str = "dmafmt_AbC123dEfG456hIjK789lMnO012pQrS345tUvW678x9"; // 50 字符
const REVOKED_TOKEN: &str = "dmafmt_ZzZ123dEfG456hIjK789lMnO012pQrS345tUvW678x9";
const STAGE_MD: &str = "---\nid: p1\nphase: demo\ncollaboration: single\n\nagents:\n  - agent: pkg-agent\n    skills: []\n    inputs: []\n    outputs: [\"out/p1.md\"]\n    task_description: \"包任务\"\nretry:\n  max_attempts: 1\n  on_exhausted: abort_workflow\napproval:\n  required: false\nexecution: always\n---\n\n# p1\n";
const SCOPE_MD: &str = "---\nname: pkg-scope\ndescription: 包 scope\ninclude_stages: [p1]\n---\n\n# pkg-scope\n";
const AGENT_MD: &str = "---\nname: pkg-agent\nmode: subagent\ndescription: 包 agent\n---\nbody\n";
const INTRO_YAML: &str = "what_it_is: 测试包是什么\nwhat_it_does: 测试包有什么用\nhow_to_use: dmaf add 安装即用\n";

// ---------- 假 market（TCP 层 HTTP/1.1） ----------

struct Req {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Req {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn clone_fields(&self) -> Req {
        Req {
            method: self.method.clone(),
            path: self.path.clone(),
            query: self.query.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
}

struct Resp {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn resp_json(status: u16, body: Value) -> Resp {
    Resp { status, headers: vec![], body: serde_json::to_vec(&body).unwrap() }
}

fn resp_err(status: u16, envelope: Value) -> Resp {
    resp_json(status, envelope)
}

struct FakeMarket {
    base: String,
    requests: Arc<Mutex<Vec<Req>>>,
}

fn serve(handler: Arc<dyn Fn(&Req) -> Resp + Send + Sync>) -> FakeMarket {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests: Arc<Mutex<Vec<Req>>> = Arc::new(Mutex::new(Vec::new()));
    let req_clone = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let handler = handler.clone();
            let requests = req_clone.clone();
            std::thread::spawn(move || {
                if let Ok(req) = read_request(&stream) {
                    requests.lock().unwrap().push(req.clone_fields());
                    let resp = handler(&req);
                    write_response(&stream, &resp);
                }
            });
        }
    });
    FakeMarket { base: format!("http://{addr}"), requests }
}

fn read_request(stream: &TcpStream) -> std::io::Result<Req> {
    let mut stream = stream;
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    // 读到头部终止符
    loop {
        let n = stream.read(&mut byte)?;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof"));
        }
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if buf.len() > 64 * 1024 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "header too large"));
        }
    }
    let head = String::from_utf8_lossy(&buf).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split(' ');
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target, String::new()),
    };
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let content_length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; content_length];
    stream.read_exact(&mut body)?;
    Ok(Req { method, path, query, headers, body })
}

fn write_response(mut stream: &TcpStream, resp: &Resp) {
    let reason = match resp.status {
        200 => "OK",
        201 => "Created",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        429 => "Too Many Requests",
        _ => "OK",
    };
    let mut head = format!("HTTP/1.1 {} {}\r\n", resp.status, reason);
    for (k, v) in &resp.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", resp.body.len()));
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&resp.body);
}

fn multipart_part<'a>(body: &'a [u8], name: &str) -> Option<&'a [u8]> {
    // 从 multipart body 提取指定 name 的 part 载荷（固定两 part 的简单解析）
    let text_marker = format!("name=\"{name}\"");
    let pos = find_subslice(body, text_marker.as_bytes())?;
    let after_headers = find_subslice(&body[pos..], b"\r\n\r\n")? + pos + 4;
    let end = find_subslice(&body[after_headers..], b"\r\n--")? + after_headers;
    Some(&body[after_headers..end])
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// ---------- 测试工程构造 ----------

/// 构造可发布的工程（manifest + agents/skills/stages/scopes + 真实编译产物）。
fn make_project(tag: &str, name: &str) -> PathBuf {
    let root = unique_tmp(tag);
    let w = |rel: &str, content: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    };
    w("dmaf/manifest.yaml", format!("name: {name}\nversion: 0.1.0\ndescription: 测试包\nengine: \">=0.5\"\nharnesses: []\n").as_str());
    w("dmaf/agents/pkg-agent.md", AGENT_MD);
    w("dmaf/skills/pkg-skill/SKILL.md", "---\nname: pkg-skill\ndescription: 包技能\n---\nbody\n");
    w("dmaf/workflows/stages/demo/p1.md", STAGE_MD);
    w("dmaf/workflows/scopes/pkg-scope.md", SCOPE_MD);
    compile_project(&root);
    root
}

fn compile_project(root: &Path) {
    let compiled = dmaf_engine::compile::compile(
        &root.join("dmaf/workflows/stages"),
        &root.join("dmaf/workflows/scopes"),
        None,
    )
    .unwrap();
    let out = root.join("dmaf/compiled");
    std::fs::create_dir_all(&out).unwrap();
    for (file, value) in [
        (dmaf_engine::compile::graph_file_name(), &compiled.stage_graph),
        (dmaf_engine::compile::grid_file_name(), &compiled.scope_grid),
    ] {
        std::fs::write(out.join(file), dmaf_engine::compile::serialize_artifact(value)).unwrap();
    }
}

/// dmaf_engine::run 收纯参数（argv[0] 由其内部补占位名）。
fn run(argv: &[&str]) -> (i32, Value, String) {
    // 锁定 zh 缺省：宿主系统语言不参与测试（显式设置使 run 的探测预设跳过）
    dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
    let full: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    let out = dmaf_engine::run(&full);
    let stdout_json = if out.stdout.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&out.stdout).unwrap_or(Value::Null)
    };
    (out.code, stdout_json, out.stderr)
}

/// 把凭据写进测试文件（DMAF_CREDENTIALS_FILE 指向）。
fn write_credentials(dir: &Path, base: &str, token: &str) {
    let path = dir.join("credentials.toml");
    std::fs::write(
        &path,
        format!("[credentials.\"{base}\"]\ntoken = \"{token}\"\nuserid = \"alice\"\n"),
    )
    .unwrap();
}

/// 凭据文件环境 + 可发布工程。
fn login_and_project(tag: &str) -> (PathBuf, PathBuf) {
    let tmp = unique_tmp(tag);
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));
    let project = make_project(tag, "pkg-demo");
    (tmp, project)
}

// ---------- auth login（L1 / L2 / P7） ----------

#[test]
fn auth_login_success_saves_credentials() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-auth-ok");
    let cred = tmp.join("credentials.toml");
    std::env::set_var("DMAF_CREDENTIALS_FILE", &cred);

    let market = serve(Arc::new(|req: &Req| {
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/api/me");
        assert_eq!(req.header("authorization").unwrap(), format!("Bearer {VALID_TOKEN}"));
        resp_json(200, json!({"user": {"userid": "alice", "display_name": "Alice", "provider": "github"}}))
    }));
    let (code, out, _) = run(&["auth", "login", "--token", VALID_TOKEN, "--market", &market.base]);
    assert_eq!(code, 0, "stderr 见测试输出");
    assert_eq!(out["userid"], "alice");
    assert_eq!(out["provider"], "github");
    let saved = std::fs::read_to_string(&cred).unwrap();
    assert!(saved.contains(&format!("token = \"{VALID_TOKEN}\"")));
    assert!(saved.contains(&market.base));
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn auth_login_bad_format_never_sends_request() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-auth-fmt");
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));

    let market = serve(Arc::new(|_: &Req| panic!("格式预检失败不应发请求")));
    let (code, _, stderr) = run(&["auth", "login", "--token", "short", "--market", &market.base]);
    assert_eq!(code, 2);
    assert!(stderr.contains("格式不符"), "{stderr}");
    assert!(market.requests.lock().unwrap().is_empty());
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn auth_login_revoked_token_not_saved() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-auth-revoked");
    let cred = tmp.join("credentials.toml");
    std::env::set_var("DMAF_CREDENTIALS_FILE", &cred);

    let market = serve(Arc::new(|req: &Req| {
        assert!(req.header("authorization").unwrap().ends_with("678x9"));
        resp_err(401, json!({
            "code": "unauthorized",
            "message": "认证失败",
            "trace_id": "t-rev",
            "details": [{"field": "authorization", "issue": "token_revoked"}],
        }))
    }));
    let (code, _, stderr) = run(&["auth", "login", "--token", REVOKED_TOKEN, "--market", &market.base]);
    assert_eq!(code, 3);
    assert!(stderr.contains("吊销"), "{stderr}");
    assert!(!cred.exists(), "401 不得保存凭据");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

// ---------- publish（P1 / P2 / P4 / P8 / P9） ----------

#[test]
fn publish_dry_run_gates_and_digest_no_request() {
    let _env = lock_cwd();
    let (tmp, project) = login_and_project("mkt-pub-dry");
    write_credentials(&tmp, "http://127.0.0.1:1", VALID_TOKEN);
    let _guard = CwdGuard::enter(&project);
    let intro = tmp.join("intro.yaml");
    std::fs::write(&intro, INTRO_YAML).unwrap();

    // 127.0.0.1:1 不会连上：dry-run 不应发出任何请求（失败即门禁前有问题）
    let (code, out, stderr) = run(&[
        "publish", "./dmaf", "--intro", intro.to_str().unwrap(), "--market", "http://127.0.0.1:1",
        "--dry-run",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(out["dry_run"], true);
    assert_eq!(out["project"], "pkg-demo");
    assert_eq!(out["version"], "0.1.0");
    let digest = out["package"]["digest"].as_str().unwrap();
    assert!(digest.starts_with("sha256:") && digest.len() == 71);
    assert_eq!(out["meta"]["intro"]["what_it_is"], "测试包是什么");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn publish_compile_gate_failure_sends_no_request() {
    let _env = lock_cwd();
    let (tmp, project) = login_and_project("mkt-pub-compile");
    write_credentials(&tmp, "http://127.0.0.1:1", VALID_TOKEN);
    // 破坏 stage 定义 → 编译门禁失败
    std::fs::write(project.join("dmaf/workflows/stages/demo/p1.md"), "坏的 frontmatter\n").unwrap();
    let _guard = CwdGuard::enter(&project);
    let intro = tmp.join("intro.yaml");
    std::fs::write(&intro, INTRO_YAML).unwrap();

    let (code, _, stderr) = run(&[
        "publish", "./dmaf", "--intro", intro.to_str().unwrap(), "--market", "http://127.0.0.1:1",
    ]);
    assert_eq!(code, 3, "{stderr}");
    assert!(stderr.contains("门禁一（编译）"), "{stderr}");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn publish_intro_missing_field_rejected_locally() {
    let _env = lock_cwd();
    let (tmp, project) = login_and_project("mkt-pub-intro");
    write_credentials(&tmp, "http://127.0.0.1:1", VALID_TOKEN);
    let _guard = CwdGuard::enter(&project);
    let intro = tmp.join("intro.yaml");
    std::fs::write(&intro, "what_it_is: 只有一项\n").unwrap();

    let (code, _, stderr) = run(&[
        "publish", "./dmaf", "--intro", intro.to_str().unwrap(), "--market", "http://127.0.0.1:1",
    ]);
    assert_eq!(code, 3);
    assert!(stderr.contains("what_it_does"), "{stderr}");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn publish_upload_success_and_idempotent_replay() {
    let _env = lock_cwd();
    let (tmp, project) = login_and_project("mkt-pub-upload");
    let uploads: Arc<Mutex<Vec<Req>>> = Arc::new(Mutex::new(Vec::new()));
    let u2 = uploads.clone();
    let market = serve(Arc::new(move |req: &Req| {
        if req.path == "/api/publications" && req.method == "POST" {
            assert!(req.header("authorization").unwrap().starts_with("Bearer dmafmt_"));
            let mut guard = u2.lock().unwrap();
            guard.push(req.clone_fields());
            let n = guard.len();
            let mut r = resp_json(201, json!({
                "namespace": {"userid": "alice", "project": "pkg-demo"},
                "version": "0.1.0",
                "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "entry_url": "http://market.test/alice/pkg-demo",
                "lifecycle_status": "listed",
                "published_at": "2026-11-08T00:00:00Z",
            }));
            if n >= 2 {
                r.headers.push(("X-Idempotent-Replay".into(), "true".into()));
            }
            r
        } else {
            resp_json(200, json!({"status": "ok"}))
        }
    }));
    write_credentials(&tmp, &market.base, VALID_TOKEN);
    let _guard = CwdGuard::enter(&project);
    let intro = tmp.join("intro.yaml");
    std::fs::write(&intro, INTRO_YAML).unwrap();

    // 第一次：首次受理（P1/P8）
    let (code, out, stderr) = run(&[
        "publish", "./dmaf", "--intro", intro.to_str().unwrap(), "--market", &market.base,
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(out["idempotent_replay"], false);
    assert_eq!(out["namespace"]["userid"], "alice");
    assert_eq!(out["entry_url"], "http://market.test/alice/pkg-demo");

    // 第二次：同版本重发 → 幂等重放（P4）
    let (code2, out2, _) = run(&[
        "publish", "./dmaf", "--intro", intro.to_str().unwrap(), "--market", &market.base,
    ]);
    assert_eq!(code2, 0);
    assert_eq!(out2["idempotent_replay"], true);

    // 校验 multipart：meta 与 package 两个 part（P9）
    let uploads = uploads.lock().unwrap();
    assert_eq!(uploads.len(), 2);
    let first = &uploads[0];
    let meta_part = multipart_part(&first.body, "meta").unwrap();
    let meta: Value = serde_json::from_slice(meta_part).unwrap();
    assert_eq!(meta["project"], "pkg-demo");
    assert_eq!(meta["version"], "0.1.0");
    assert_eq!(meta["compatibility"]["dmaf_engine"], env!("CARGO_PKG_VERSION"));
    let pkg_part = multipart_part(&first.body, "package").unwrap();
    // 归档成员：manifest 必含；compiled/packages.yaml 不入包
    let members = archive_members(pkg_part);
    assert!(members.contains(&"dmaf/manifest.yaml".to_string()), "成员: {members:?}");
    assert!(members.iter().any(|m| m.contains("pkg-skill")));
    assert!(!members.iter().any(|m| m.contains("compiled")), "成员: {members:?}");
    assert!(!members.iter().any(|m| m.contains("packages.yaml")), "成员: {members:?}");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

fn archive_members(gz: &[u8]) -> Vec<String> {
    let decoder = flate2::read::GzDecoder::new(gz);
    let mut ar = tar::Archive::new(decoder);
    let mut out = Vec::new();
    for e in ar.entries().unwrap() {
        let e = e.unwrap();
        out.push(e.path().unwrap().to_string_lossy().into_owned());
    }
    out.sort();
    out
}

#[test]
fn publish_excludes_other_packages_files_e9_and_p9() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-pub-e9");
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));
    // 发布者工程装过其他包（构造第二个包并 add 进来）
    let other = make_project("mkt-pub-e9-other", "other-pkg");
    let project = make_project("mkt-pub-e9-main", "pkg-demo");
    let opts = dmaf_engine::package::MergeOptions {
        check_only: false,
        chain_install: false,
        source_label: other.to_string_lossy().into_owned(),
        tag: None,
        digest: None,
        command: "add",
        force: false,
    };
    dmaf_engine::package::add_package(&project, other.to_str().unwrap(), None, &opts).unwrap();
    assert!(project.join("dmaf/packages.yaml").is_file());

    let uploads: Arc<Mutex<Vec<Req>>> = Arc::new(Mutex::new(Vec::new()));
    let u2 = uploads.clone();
    let market = serve(Arc::new(move |req: &Req| {
        if req.path == "/api/publications" {
            u2.lock().unwrap().push(req.clone_fields());
            resp_json(201, json!({
                "namespace": {"userid": "alice", "project": "pkg-demo"},
                "version": "0.1.0",
                "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "entry_url": "http://market.test/alice/pkg-demo",
                "lifecycle_status": "listed",
                "published_at": "2026-11-08T00:00:00Z",
            }))
        } else {
            resp_json(200, json!({"status": "ok"}))
        }
    }));
    write_credentials(&tmp, &market.base, VALID_TOKEN);
    let _guard = CwdGuard::enter(&project);
    let intro = tmp.join("intro.yaml");
    std::fs::write(&intro, INTRO_YAML).unwrap();

    let (code, out, stderr) = run(&[
        "publish", "./dmaf", "--intro", intro.to_str().unwrap(), "--market", &market.base,
    ]);
    assert_eq!(code, 0, "{stderr}");
    // E-9：他包文件被排除并列出
    let excluded: Vec<String> = out["excluded_other_packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(excluded.contains(&"dmaf/agents/pkg-agent.md".to_string()), "excluded: {excluded:?}");
    let guard = uploads.lock().unwrap();
    let pkg_part = multipart_part(&guard[0].body, "package").unwrap();
    let members = archive_members(pkg_part);
    assert!(!members.contains(&"dmaf/agents/pkg-agent.md".to_string()), "成员: {members:?}");
    assert!(!members.contains(&"dmaf/packages.yaml".to_string()), "P9：账本不随包分发，成员: {members:?}");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

// ---------- add 条目 URL（A1 / A3 / A5 / A6） ----------

/// 构造一个待安装的发布归档（走 publish 同款打包：package_files + manifest 补回）。
fn build_release_archive(tag: &str) -> (Vec<u8>, String) {
    let root = make_project(tag, "pkg-demo");
    let mut files = dmaf_engine::package::package_files(&root).unwrap();
    files.push((
        "dmaf/manifest.yaml".into(),
        root.join("dmaf/manifest.yaml"),
        "x".into(),
    ));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let bytes = dmaf_engine::market::pack_archive(&files).unwrap();
    let digest = format!("sha256:{}", dmaf_engine::audit::sha256_hex(&bytes));
    (bytes, digest)
}

#[test]
fn add_entry_installs_with_digest_and_ledger() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-add-ok");
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));
    let (archive, digest) = build_release_archive("mkt-add-ok-arc");
    let target = unique_tmp("mkt-add-ok-target");

    let archive2 = archive.clone();
    let digest2 = digest.clone();
    let market = serve(Arc::new(move |req: &Req| {
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/api/entries/alice/pkg-demo/packages/latest");
        assert_eq!(req.query, "source=add");
        assert!(req.header("authorization").is_none(), "取源匿名，不带凭据");
        Resp {
            status: 200,
            headers: vec![
                ("X-Content-Digest".into(), digest2.clone()),
                ("X-Entry-Version".into(), "1.0.0".into()),
            ],
            body: archive2.clone(),
        }
    }));
    let _guard = CwdGuard::enter(&target);
    let (code, out, stderr) = run(&["add", &format!("{}/alice/pkg-demo", market.base)]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(out["market"]["version"], "1.0.0");
    assert_eq!(out["market"]["digest"], digest);
    assert_eq!(out["market"]["entry"], format!("{}/alice/pkg-demo", market.base));
    assert!(!out["created"].as_array().unwrap().is_empty());
    // 账本：tag = 实际版本，digest = 托管摘要（A3 / §7.5）
    let ledger_text = std::fs::read_to_string(target.join("dmaf/packages.yaml")).unwrap();
    let ledger: Value = serde_yaml::from_str(&ledger_text).unwrap();
    let entry = &ledger["packages"][0];
    assert_eq!(entry["name"], "pkg-demo");
    assert_eq!(entry["tag"], "1.0.0");
    assert_eq!(entry["digest"], digest);
    assert_eq!(entry["source"], format!("{}/alice/pkg-demo", market.base));
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn add_entry_digest_mismatch_writes_nothing() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-add-bad");
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));
    let (archive, _) = build_release_archive("mkt-add-bad-arc");
    let target = unique_tmp("mkt-add-bad-target");

    let archive2 = archive.clone();
    let market = serve(Arc::new(move |_: &Req| {
        Resp {
            status: 200,
            headers: vec![
                ("X-Content-Digest".into(), "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into()),
                ("X-Entry-Version".into(), "1.0.0".into()),
            ],
            body: archive2.clone(),
        }
    }));
    let _guard = CwdGuard::enter(&target);
    let (code, _, stderr) = run(&["add", &format!("{}/alice/pkg-demo", market.base)]);
    assert_eq!(code, 5, "{stderr}");
    assert!(stderr.contains("摘要核对失败") && stderr.contains("fail closed"), "{stderr}");
    assert!(!target.join("dmaf").exists(), "目标工程零写入");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn add_entry_not_found_maps_to_schema() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-add-404");
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));
    let target = unique_tmp("mkt-add-404-target");

    // 404 entry_not_found（A5：原因可理解、零写入）
    let market = serve(Arc::new(|_: &Req| {
        resp_err(404, json!({"code": "entry_not_found", "message": "不存在", "trace_id": "t-404"}))
    }));
    let _guard = CwdGuard::enter(&target);
    let (code, _, stderr) = run(&["add", &format!("{}/alice/no-such", market.base)]);
    assert_eq!(code, 3, "{stderr}");
    assert!(stderr.contains("条目不存在"), "{stderr}");
    assert!(!target.join("dmaf").exists(), "目标工程零写入");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn add_entry_tag_pins_version() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-add-tag");
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));
    let (archive, digest) = build_release_archive("mkt-add-tag-arc");
    let target = unique_tmp("mkt-add-tag-target");

    let archive2 = archive.clone();
    let digest2 = digest.clone();
    let market = serve(Arc::new(move |req: &Req| {
        assert_eq!(req.path, "/api/entries/alice/pkg-demo/packages/0.9.0", "A6：钉选版本路径");
        assert_eq!(req.query, "source=add");
        Resp {
            status: 200,
            headers: vec![
                ("X-Content-Digest".into(), digest2.clone()),
                ("X-Entry-Version".into(), "0.9.0".into()),
            ],
            body: archive2.clone(),
        }
    }));
    let _guard = CwdGuard::enter(&target);
    let (code, out, stderr) = run(&[
        "add", &format!("{}/alice/pkg-demo", market.base), "--tag", "0.9.0",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(out["market"]["version"], "0.9.0");
    let ledger_text = std::fs::read_to_string(target.join("dmaf/packages.yaml")).unwrap();
    let ledger: Value = serde_yaml::from_str(&ledger_text).unwrap();
    assert_eq!(ledger["packages"][0]["tag"], "0.9.0");
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}

#[test]
fn add_positional_url_and_from_are_mutually_exclusive() {
    let _env = lock_cwd();
    let tmp = unique_tmp("mkt-add-clash");
    std::env::set_var("DMAF_CREDENTIALS_FILE", tmp.join("credentials.toml"));
    let (code, _, _) = run(&["add", "https://m.test/a/b", "--from", "/tmp/x"]);
    assert_eq!(code, 2);
    std::env::remove_var("DMAF_CREDENTIALS_FILE");
}
