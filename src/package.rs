//! 工作流包的合入与交付：`dmaf add`（使用者，工作流项目 → 当前工程）与
//! `dmaf push`（开发者，当前工作流项目 → 目标工程）共用的合并核心。
//!
//! 语义要点：
//! - 包文件集 = 源 `dmaf/` 下全部文件，**排除工程自属槽位**（`dmaf/manifest.yaml`、
//!   `dmaf/README.md`、`dmaf/docs/流程定义指南.md`、引擎下发的定义技能
//!   `dmaf/skills/create-workflow/`）与**编译产物**（`dmaf/compiled/`，
//!   产物是工程级单份，合入后统一重编译）；
//! - 目标无 `dmaf/` 时先补齐最小骨架（标准目录 + .gitignore 追加条目），再按**账本哈希门**合并：
//!   目标缺失 → created；内容相同 → unchanged；包属且账本哈希一致（目标侧未改动）→ updated
//!   （升级 = add/push 重跑）；其余覆盖情形 → conflicted（整体不落盘，退出码 4）；
//! - 账本 `dmaf/packages.yaml`（引擎写入与维护，请勿手改）记录每个包的包属文件哈希；
//!   账本独立于 manifest.yaml，避免引擎重写用户手写的清单（serde_yaml 不保注释）；
//! - 合入后若目标存在 stage 库则重编译产物（同时是 stage/scope id、agent 命名冲突的检测点）；
//! - 串联目标工程的 harness 自动探测与部署（`--no-install` 可关）；`--force` 同样作用于
//!   串联部署（install 账本哈希门强制覆盖 harness 内用户改动，覆盖清单见报告 overwritten）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};
use serde_yaml::Value as Yaml;

/// 骨架补齐创建的标准目录（相对工程根）。
const SKELETON_DIRS: &[&str] = &[
    "dmaf/agents",
    "dmaf/skills",
    "dmaf/compiled",
    "dmaf/workflows/stages",
    "dmaf/workflows/scopes",
];

/// 工程自属槽位：包不携带、合入不触碰（宿主工程的身份、文档与账本由宿主自持）。
/// packages.yaml 是安装方本地的引擎账本（记录 source 为本地路径），
/// 不排除会随包分发：装过包的发布者把账本带进包 → 安装方已有账本即判冲突整体拒绝，
/// 且账本里的 source 是发布者本地绝对路径（指南 §2 既有缺陷，push/publish 一并修复）。
const PROJECT_SLOT_FILES: &[&str] = &[
    "dmaf/manifest.yaml",
    "dmaf/README.md",
    "dmaf/docs/流程定义指南.md",
    "dmaf/packages.yaml",
];

/// 目录形态的引擎槽位：dmaf new 随骨架下发的定义技能（src/scaffold.rs 内嵌写入 dmaf/skills/ 直属目录）。
/// 包永不携带——通用技能名（无包名前缀）随包分发会在 market 多包安装下跨包碰撞，
/// 故按目录前缀整体排除，各项目的技能副本由引擎骨架自持。
const PROJECT_SLOT_DIRS: &[&str] = &["dmaf/skills/create-workflow"];

const LEDGER_HEADER_KEY: &str = "package.ledger_header";

#[derive(Debug, Clone)]
pub enum PackageError {
    /// 用法/源不合法（退出码 2）
    Usage(String),
    /// 校验失败（退出码 3；market 来源的 404/信封错误等按指南映射到此）
    Schema { message: String, problems: Vec<String> },
    /// 合并冲突（退出码 4；目标侧已改动且不可按账本更新）
    Conflict(Vec<String>),
    /// 合入后编译失败（退出码 3；stage/scope id、agent 命名冲突在此暴露）
    Compile(Vec<String>),
    /// 串联 harness 部署失败（退出码 5；合并本身已完成）
    Install(String),
    /// IO（退出码 5）
    Io(String),
}

#[derive(Debug, Clone)]
pub struct MergeOptions {
    /// dry-run：只报告将发生的变更，不写任何文件
    pub check_only: bool,
    /// 合入后串联 harness 自动探测与部署（--no-install 关闭）
    pub chain_install: bool,
    /// 账本 source 字段记录值（add = --from 原值或 market 条目 URL 规范形；push = 源项目绝对路径）
    pub source_label: String,
    /// git 源 tag / market 条目实际版本（记入账本）
    pub tag: Option<String>,
    /// market 托管内容摘要 sha256:<hex>（X-Content-Digest，market 来源记入账本，指南 §7.5）
    pub digest: Option<String>,
    /// 报告与账本中的命令名（"add" | "push"）
    pub command: &'static str,
    /// 目标侧已改动的包属文件强制覆盖（--force）：覆盖后按新哈希登记账本，用户改动不保留（overwritten 清单报告）
    pub force: bool,
}

/// 单个包的账本条目。
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub source: String,
    pub tag: Option<String>,
    /// market 托管内容摘要（sha256:<hex>；仅 market 来源有，向后兼容省略）
    pub digest: Option<String>,
    pub installed_at: String,
    /// 包属文件（工程根相对 posix 路径）→ 合入时的内容哈希
    pub files: BTreeMap<String, String>,
}

type Ledger = BTreeMap<String, LedgerEntry>;

// ---------- 入口 ----------

/// `dmaf add`：把 `from`（本地路径或 git URL）指向的工作流包合入 `target_root`。
pub fn add_package(
    target_root: &Path,
    from: &str,
    tag: Option<&str>,
    opts: &MergeOptions,
) -> Result<Value, PackageError> {
    let temp = if from.contains("://") {
        Some(clone_to_temp(from, tag)?)
    } else {
        None
    };
    let source_root = temp.as_deref().unwrap_or(Path::new(from));
    let result = merge_package(source_root, target_root, opts);
    if let Some(t) = &temp {
        let _ = std::fs::remove_dir_all(t);
    }
    result
}

/// `dmaf push`：把当前工作流项目交付到 `target_root`。push 专属验证门：
/// 源清单（含非空 name）必须存在；源有 stage 库时产物不得漂移（"已验证好"的机器定义）。
pub fn push_package(
    source_root: &Path,
    target_root: &Path,
    opts: &MergeOptions,
) -> Result<Value, PackageError> {
    if !target_root.is_dir() {
        return Err(PackageError::Usage(crate::i18n::msg(
            "package.target_dir_missing",
            &[&crate::paths::to_posix(target_root)],
        )));
    }
    let manifest_ok = matches!(
        crate::manifest::load(source_root),
        Ok(Some(ref m)) if !m.name.trim().is_empty()
    );
    if !manifest_ok {
        return Err(PackageError::Usage(
            crate::i18n::t("package.push_manifest_required"),
        ));
    }
    let stages = source_root.join("dmaf/workflows/stages");
    let scopes = source_root.join("dmaf/workflows/scopes");
    if stages.is_dir() && scopes.is_dir() {
        let compiled = crate::compile::compile(&stages, &scopes, None).map_err(
            |e: crate::compile::CompileError| PackageError::Compile(e.problems),
        )?;
        let artifacts = source_root.join("dmaf/compiled");
        for (file, value) in [
            (
                crate::compile::graph_file_name(),
                &compiled.stage_graph,
            ),
            (crate::compile::grid_file_name(), &compiled.scope_grid),
        ] {
            let text = crate::compile::serialize_artifact(value);
            let existing = std::fs::read_to_string(artifacts.join(file)).unwrap_or_default();
            if existing != text {
                return Err(PackageError::Compile(vec![crate::i18n::msg(
                    "package.source_artifact_drift",
                    &[file],
                )]));
            }
        }
    }
    merge_package(source_root, target_root, opts)
}

// ---------- 合并核心 ----------

pub fn merge_package(
    source_root: &Path,
    target_root: &Path,
    opts: &MergeOptions,
) -> Result<Value, PackageError> {
    // 1. 源校验：dmaf/ 布局 + 包身份清单
    let mut source_problems: Vec<String> = Vec::new();
    if !source_root.join("dmaf").is_dir() {
        source_problems.push(crate::i18n::msg(
            "package.missing_dmaf_layer",
            &[&crate::paths::to_posix(source_root)],
        ));
    }
    let manifest = match crate::manifest::load(source_root) {
        Ok(Some(m)) if !m.name.trim().is_empty() => Some(m),
        Ok(Some(_)) => {
            source_problems.push(crate::i18n::t("package.manifest_name_empty"));
            None
        }
        Ok(None) => {
            source_problems
                .push(crate::i18n::t("package.manifest_missing"));
            None
        }
        Err(e) => {
            source_problems.push(e);
            None
        }
    };
    if !source_problems.is_empty() {
        return Err(PackageError::Usage(crate::i18n::msg(
            "package.source_invalid",
            &[&source_problems.join("；")],
        )));
    }
    let manifest = manifest.expect("上方已确保 Some");

    // 同目录守卫（add 源=目标、push 自交付都是误用）
    let src_canon =
        std::fs::canonicalize(source_root).map_err(|e| PackageError::Io(e.to_string()))?;
    let tgt_canon = std::fs::canonicalize(target_root)
        .map_err(|e| PackageError::Io(crate::i18n::msg("package.target_unavailable", &[&e.to_string()])))?;
    if src_canon == tgt_canon {
        return Err(PackageError::Usage(crate::i18n::t("package.same_dir")));
    }

    // 2. 账本读取（缺失 = 空账本）
    let ledger_path = target_root.join("dmaf").join("packages.yaml");
    let mut ledger = read_ledger(&ledger_path)?;
    let prior = ledger.get(&manifest.name).cloned();

    // 3. 包文件集 + 逐文件决策（账本哈希门）
    let files = package_files(source_root)?;
    let mut created: Vec<String> = Vec::new();
    let mut unchanged: Vec<String> = Vec::new();
    let mut updated: Vec<String> = Vec::new();
    let mut conflicted: Vec<String> = Vec::new();
    let mut overwritten: Vec<String> = Vec::new();
    for (rel, _, src_hash) in &files {
        let target = target_root.join(rel);
        if !target.exists() {
            created.push(rel.clone());
            continue;
        }
        let tgt_hash = file_hash(&target);
        if tgt_hash == *src_hash {
            unchanged.push(rel.clone());
        } else if prior
            .as_ref()
            .and_then(|p| p.files.get(rel))
            .map(|stored| stored == &tgt_hash)
            .unwrap_or(false)
        {
            // 包属且目标侧未改动 → 允许更新（升级 = add/push 重跑）
            updated.push(rel.clone());
        } else if opts.force {
            // 目标侧已改动 + --force → 覆盖（用户改动不保留，账本按新哈希重记）
            overwritten.push(rel.clone());
        } else {
            conflicted.push(rel.clone());
        }
    }
    // 源里已不再随包分发的旧包属文件：信息性报告，不删除（v1 无 prune 语义）
    let no_longer_shipped: Vec<String> = prior
        .as_ref()
        .map(|p| {
            p.files
                .keys()
                .filter(|k| !files.iter().any(|(rel, _, _)| rel == *k))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    if !conflicted.is_empty() {
        return Err(PackageError::Conflict(conflicted));
    }

    let bootstrap = !target_root.join("dmaf").is_dir();

    if opts.check_only {
        return Ok(json!({
            "ok": true,
            "command": opts.command,
            "check_only": true,
            "package": package_json(&manifest, opts),
            "target": crate::paths::to_posix(target_root),
            "skeleton": {"bootstrap": bootstrap,
                         "dirs": if bootstrap { json!(SKELETON_DIRS) } else { json!([])}},
            "created": created,
            "unchanged": unchanged,
            "updated": updated,
            "overwritten": overwritten,
            "force": opts.force,
            "no_longer_shipped": no_longer_shipped,
            "ledger": {"path": "dmaf/packages.yaml", "written": false},
            "compiled": {"skipped": crate::i18n::t("package.dryrun_compile_skip")},
            "install": {"skipped": if opts.force {
                crate::i18n::t("package.dryrun_install_skip_force")
            } else {
                crate::i18n::t("package.dryrun_install_skip")
            }},
        }));
    }

    // 4. 骨架补齐（.gitignore 追加 .dmaf/ 等条目；业务产物目录 dmaf-output/ 是工作成果，引擎不写入忽略清单、不触碰用户既有条目）
    let mut gitignore_appended = Vec::new();
    if bootstrap {
        for d in SKELETON_DIRS {
            std::fs::create_dir_all(target_root.join(d))
                .map_err(|e| PackageError::Io(crate::i18n::msg("package.mkdir_skeleton_fail", &[d, &e.to_string()])))?;
        }
        gitignore_appended = crate::scaffold::append_gitignore_entries(target_root)
            .map_err(PackageError::Io)?;
    }

    // 5. 写入 created + updated + overwritten（非 force 的冲突已在上方整体拒绝，此处目标侧必然允许覆盖）
    for (rel, src_path, _) in files
        .iter()
        .filter(|(rel, _, _)| {
            created.contains(rel) || updated.contains(rel) || overwritten.contains(rel)
        })
    {
        let target = target_root.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PackageError::Io(e.to_string()))?;
        }
        std::fs::copy(src_path, &target).map_err(|e| PackageError::Io(e.to_string()))?;
    }

    // 6. 账本写入（全量重写；同名单包条目替换，其他包条目保留）
    let mut files_map = BTreeMap::new();
    for (rel, _, hash) in &files {
        files_map.insert(rel.clone(), hash.clone());
    }
    ledger.insert(
        manifest.name.clone(),
        LedgerEntry {
            source: opts.source_label.clone(),
            tag: opts.tag.clone(),
            digest: opts.digest.clone(),
            installed_at: crate::paths::format_ts(&crate::paths::now_cst()),
            files: files_map,
        },
    );
    write_ledger(&ledger_path, &ledger)?;

    // 7. 重编译（产物是工程级单份；同时检测 stage/scope id、agent 命名冲突）
    let stages_dir = target_root.join("dmaf/workflows/stages");
    let scopes_dir = target_root.join("dmaf/workflows/scopes");
    let compiled = if stages_dir.is_dir() && scopes_dir.is_dir() {
        let compiled = crate::compile::compile(&stages_dir, &scopes_dir, None).map_err(
            |e: crate::compile::CompileError| PackageError::Compile(e.problems),
        )?;
        let out_dir = target_root.join("dmaf/compiled");
        std::fs::create_dir_all(&out_dir).map_err(|e| PackageError::Io(e.to_string()))?;
        for (file, value) in [
            (
                crate::compile::graph_file_name(),
                &compiled.stage_graph,
            ),
            (crate::compile::grid_file_name(), &compiled.scope_grid),
        ] {
            let path = out_dir.join(file);
            crate::paths::atomic_write_text(&path, &crate::compile::serialize_artifact(value))
                .map_err(|e| PackageError::Io(e.to_string()))?;
        }
        json!({
            "stages": compiled.stage_graph["stages"].as_array().map(|a| a.len()).unwrap_or(0),
            "scopes": compiled.scope_grid["scopes"].as_object().map(|m| m.len()).unwrap_or(0),
        })
    } else {
        json!({"skipped": crate::i18n::t("package.compile_skip_no_stages")})
    };

    // 8. 串联 harness 部署
    let install = chain_install(target_root, opts)?;

    Ok(json!({
        "ok": true,
        "command": opts.command,
        "check_only": false,
        "package": package_json(&manifest, opts),
        "target": crate::paths::to_posix(target_root),
        "skeleton": {"bootstrap": bootstrap, "gitignore_appended": gitignore_appended},
        "created": created,
        "unchanged": unchanged,
        "updated": updated,
        "overwritten": overwritten,
        "force": opts.force,
        "no_longer_shipped": no_longer_shipped,
        "ledger": {"path": "dmaf/packages.yaml", "written": true},
        "compiled": compiled,
        "install": install,
    }))
}

// ---------- 串联部署 / 克隆 / 文件集 ----------

fn chain_install(target_root: &Path, opts: &MergeOptions) -> Result<Value, PackageError> {
    if opts.check_only {
        return Ok(json!({"skipped": crate::i18n::t("package.dryrun_chain_skip")}));
    }
    if !opts.chain_install {
        return Ok(json!({"skipped": "--no-install"}));
    }
    let found = crate::registry::detect_harnesses(target_root);
    match found.as_slice() {
        [] => Ok(json!({"skipped": crate::i18n::t("package.no_harness_found")})),
        [only] => {
            let rh = crate::registry::harness_root(only).expect("探测到的名称必为已知 harness");
            let mut report =
                crate::install::install(&target_root.join(&rh.dir), rh.layout, &target_root.join("dmaf"), false, false, rh.skills_nested, opts.force)
                    .map_err(install_err(only))?;
            if let Some(name) = rh.name {
                let entry = crate::install::install_entry_config(name, target_root, false, opts.force)
                    .map_err(|e| {
                        PackageError::Install(crate::i18n::msg(
                            "package.entry_config_fail",
                            &[&format!("{e:?}")],
                        ))
                    })?;
                report["entry_config"] = entry;
            }
            report["harness_name"] = json!(rh.name);
            Ok(report)
        }
        many => Ok(json!({"skipped": crate::i18n::msg(
            "package.multi_harness",
            &[&many.join("、")]
        )})),
    }
}

fn install_err(harness: &str) -> impl Fn(crate::install::InstallError) -> PackageError + '_ {
    move |e| match e {
        crate::install::InstallError::Invalid(problems) => PackageError::Install(crate::i18n::msg(
            "package.install_chain_fail",
            &[harness, &problems.join("；")],
        )),
        crate::install::InstallError::Io(msg) => PackageError::Install(crate::i18n::msg(
            "package.install_chain_fail",
            &[harness, &msg],
        )),
    }
}

fn package_json(
    manifest: &crate::manifest::Manifest,
    opts: &MergeOptions,
) -> Value {
    json!({
        "name": manifest.name,
        "version": manifest.version,
        "description": manifest.description,
        "source": opts.source_label,
        "tag": opts.tag,
    })
}

static CLONE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// git 浅克隆到一次性临时目录（--tag → --branch）。调用方负责清理返回的目录。
fn clone_to_temp(url: &str, tag: Option<&str>) -> Result<PathBuf, PackageError> {
    let dir = std::env::temp_dir().join(format!(
        "dmaf-clone-{}-{}",
        std::process::id(),
        CLONE_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cmd = std::process::Command::new("git");
    cmd.args(["clone", "--depth", "1"]);
    if let Some(t) = tag {
        cmd.args(["--branch", t]);
    }
    cmd.arg(url).arg(&dir);
    let out = cmd.output().map_err(|e| {
        PackageError::Io(crate::i18n::msg("package.git_clone_exec_fail", &[&e.to_string()]))
    })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(PackageError::Io(crate::i18n::msg(
            "package.git_clone_fail",
            &[url, &stderr.trim()],
        )));
    }
    Ok(dir)
}

/// 包文件集：(工程根相对 posix 路径, 源绝对路径, 内容 sha256)，按路径排序。
/// 排除工程自属槽位与编译产物。publish（E-9 他包文件排除）与打包复用。
pub fn package_files(source_root: &Path) -> Result<Vec<(String, PathBuf, String)>, PackageError> {
    let dmaf = source_root.join("dmaf");
    let mut out: Vec<(String, PathBuf, String)> = Vec::new();
    for f in crate::install::walk_files(&dmaf) {
        let rel_platform = f
            .strip_prefix(source_root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let rel = crate::paths::to_posix(Path::new(&rel_platform));
        let in_slot_dir = PROJECT_SLOT_DIRS.iter().any(|d| rel.starts_with(&format!("{d}/")));
        if PROJECT_SLOT_FILES.contains(&rel.as_str()) || in_slot_dir || rel.starts_with("dmaf/compiled/") {
            continue;
        }
        let hash = file_hash(&f);
        out.push((rel, f, hash));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn file_hash(path: &Path) -> String {
    let data = std::fs::read(path).unwrap_or_default();
    crate::audit::sha256_hex(&data)
}

// ---------- 账本（dmaf/packages.yaml） ----------

/// 账本读取（publish 读取发布者本地账本做 E-9 他包文件排除，也复用此处）。
pub fn read_ledger(path: &Path) -> Result<Ledger, PackageError> {
    if !path.is_file() {
        return Ok(Ledger::new());
    }
    let text = std::fs::read_to_string(path).map_err(|e| PackageError::Io(e.to_string()))?;
    let value: Yaml = serde_yaml::from_str(&text).map_err(|e| {
        PackageError::Io(crate::i18n::msg(
            "package.ledger_parse_fail",
            &[&crate::paths::to_posix(path), &e.to_string()],
        ))
    })?;
    let mut ledger = Ledger::new();
    let Some(packages) = value.get("packages").and_then(|p| p.as_sequence()) else {
        return Ok(ledger);
    };
    for p in packages {
        let ystr = |v: &Yaml| crate::workflow::ystr(v);
        let Some(name) = p.get("name").map(|v| ystr(v)).filter(|n| !n.is_empty()) else {
            continue;
        };
        let mut files = BTreeMap::new();
        if let Some(list) = p.get("files").and_then(|f| f.as_sequence()) {
            for f in list {
                if let (Some(path), Some(hash)) = (
                    f.get("path").map(|v| ystr(v)),
                    f.get("hash").map(|v| ystr(v)),
                ) {
                    files.insert(path, hash);
                }
            }
        }
        ledger.insert(
            name,
            LedgerEntry {
                source: p.get("source").map(|v| ystr(v)).unwrap_or_default(),
                tag: p.get("tag").map(|v| ystr(v)).filter(|t| !t.is_empty()),
                digest: p.get("digest").map(|v| ystr(v)).filter(|t| !t.is_empty()),
                installed_at: p.get("installed_at").map(|v| ystr(v)).unwrap_or_default(),
                files,
            },
        );
    }
    Ok(ledger)
}

fn write_ledger(path: &Path, ledger: &Ledger) -> Result<(), PackageError> {
    let mut packages: Vec<Yaml> = Vec::new();
    for (name, entry) in ledger {
        let mut m = serde_yaml::Mapping::new();
        m.insert(Yaml::String("name".into()), Yaml::String(name.clone()));
        m.insert(
            Yaml::String("source".into()),
            Yaml::String(entry.source.clone()),
        );
        if let Some(tag) = &entry.tag {
            m.insert(Yaml::String("tag".into()), Yaml::String(tag.clone()));
        }
        if let Some(digest) = &entry.digest {
            m.insert(Yaml::String("digest".into()), Yaml::String(digest.clone()));
        }
        m.insert(
            Yaml::String("installed_at".into()),
            Yaml::String(entry.installed_at.clone()),
        );
        let files: Vec<Yaml> = entry
            .files
            .iter()
            .map(|(path, hash)| {
                let mut f = serde_yaml::Mapping::new();
                f.insert(Yaml::String("path".into()), Yaml::String(path.clone()));
                f.insert(Yaml::String("hash".into()), Yaml::String(hash.clone()));
                Yaml::Mapping(f)
            })
            .collect();
        m.insert(Yaml::String("files".into()), Yaml::Sequence(files));
        packages.push(Yaml::Mapping(m));
    }
    let mut root = serde_yaml::Mapping::new();
    root.insert(Yaml::String("packages".into()), Yaml::Sequence(packages));
    let body = serde_yaml::to_string(&Yaml::Mapping(root))
        .map_err(|e| PackageError::Io(crate::i18n::msg("package.ledger_serialize_fail", &[&e.to_string()])))?;
    std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))
        .map_err(|e| PackageError::Io(e.to_string()))?;
    std::fs::write(path, format!("{}{body}", crate::i18n::t(LEDGER_HEADER_KEY)))
        .map_err(|e| PackageError::Io(e.to_string()))?;
    Ok(())
}
