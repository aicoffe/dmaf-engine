//! 资产安装：标准目录（agents/、skills/）→ 指定 harness 目录。
//! skills 支持一层分类子目录（skills/<分类>/<技能>/SKILL.md）；部署形态取决于 harness 能力：
//! opencode/cursor 支持嵌套发现 → 保留分类路径，其余 harness 按技能名拍平。
//!
//! - 源侧校验：agent frontmatter 合法、skill SKILL.md 的 name 与目录一致；失败即拒装；
//! - 同步语义：**账本哈希门**——harness 目录根的 `dmaf-installed.yaml` 记录引擎写入每个目标时的指纹
//!   （agent/入口配置逐文件、skill 整目录）：目标缺失 → installed；与源一致 → unchanged（纳入账本）；
//!   目标指纹 == 账本记录（用户未改动）→ updated（升级自动传播）；其余（用户改过、或无账本记录的既有文件）
//!   → skipped，`--force` 时覆盖为 overwritten；默认不删除孤儿，--prune 显式清理；
//! - 两种布局规范：flat-md（opencode 系，agent 原样复制）与 toml-agents（codex/dsh，
//!   agent 转换为 TOML）；skills 一律整目录复制。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use serde_yaml::Value as Yaml;

use crate::registry::HarnessLayout;

#[derive(Debug, Clone)]
pub enum InstallError {
    Invalid(Vec<String>),
    Io(String),
}

/// 单个资产写入目标的方式：原样复制（文件/目录）或转换文本（toml-agents 布局的 agent）。
#[derive(Debug, Clone)]
enum AssetPayload {
    File(PathBuf),
    Dir(PathBuf),
    Text(String),
}

struct AgentAsset {
    target_rel: String, // agents/<stem>.md 或 agents/<stem>.toml
    payload: AssetPayload,
}

struct SkillAsset {
    name: String,
    target_rel: String, // skills/<name>
    payload: AssetPayload,
}

fn fingerprint_of(path: &Path) -> String {
    // 单文件：内容哈希；目录：相对路径+内容哈希的稳定组合
    if path.is_file() {
        let data = std::fs::read(path).unwrap_or_default();
        return crate::audit::sha256_hex(&data);
    }
    let mut combined = String::new();
    let mut files: Vec<PathBuf> = walk_files(path);
    files.sort();
    for f in files {
        let rel = f.strip_prefix(path).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let data = std::fs::read(&f).unwrap_or_default();
        combined.push_str(&rel);
        combined.push('\n');
        combined.push_str(&crate::audit::sha256_hex(&data));
        combined.push('\n');
    }
    crate::audit::sha256_hex(combined.as_bytes())
}

pub(crate) fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_files(&p));
        } else {
            out.push(p);
        }
    }
    out
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), InstallError> {
    std::fs::create_dir_all(dst).map_err(|e| InstallError::Io(e.to_string()))?;
    for f in walk_files(src) {
        let rel = f.strip_prefix(src).map(|p| p.to_path_buf()).unwrap_or_else(|_| f.clone());
        let target = dst.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| InstallError::Io(e.to_string()))?;
        }
        std::fs::copy(&f, &target).map_err(|e| InstallError::Io(e.to_string()))?;
    }
    Ok(())
}

// ---------- 安装账本与哈希门 ----------

/// 安装账本文件名（位于 harness 目录根）：rel → 引擎写入该目标时的指纹。引擎写入与维护，勿手改。
pub const INSTALL_LEDGER_FILE: &str = "dmaf-installed.yaml";

const INSTALL_LEDGER_HEADER_KEY: &str = "install.ledger_header";

fn load_install_ledger(harness: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(harness.join(INSTALL_LEDGER_FILE)) else {
        return BTreeMap::new();
    };
    let Ok(Yaml::Mapping(m)) = serde_yaml::from_str::<Yaml>(&text) else {
        return BTreeMap::new();
    };
    m.iter()
        .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
        .collect()
}

fn save_install_ledger(harness: &Path, ledger: &BTreeMap<String, String>) -> Result<(), InstallError> {
    let mut text = crate::i18n::t(INSTALL_LEDGER_HEADER_KEY);
    for (rel, fp) in ledger {
        text.push_str(&format!("\"{rel}\": \"{fp}\"\n"));
    }
    std::fs::create_dir_all(harness).map_err(|e| InstallError::Io(e.to_string()))?;
    std::fs::write(harness.join(INSTALL_LEDGER_FILE), text).map_err(|e| InstallError::Io(e.to_string()))
}

fn payload_fingerprint(payload: &AssetPayload) -> String {
    match payload {
        AssetPayload::File(src) | AssetPayload::Dir(src) => fingerprint_of(src),
        AssetPayload::Text(t) => crate::audit::sha256_hex(t.as_bytes()),
    }
}

/// 哈希门逐目标裁决。
#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    Install,
    Unchanged,
    Update,
    Overwrite,
    Skip,
}

fn gate(target: &Path, rel: &str, src_fp: &str, ledger: &BTreeMap<String, String>, force: bool) -> Verdict {
    if !target.exists() {
        return Verdict::Install;
    }
    let tgt_fp = fingerprint_of(target);
    if tgt_fp == src_fp {
        return Verdict::Unchanged;
    }
    if ledger.get(rel).map(|recorded| recorded == &tgt_fp).unwrap_or(false) {
        return Verdict::Update;
    }
    if force {
        Verdict::Overwrite
    } else {
        Verdict::Skip
    }
}

/// 写入目标：目录资产整目录替换（先清旧目录，避免旧版残留文件）。
fn write_payload(target: &Path, payload: &AssetPayload) -> Result<(), InstallError> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| InstallError::Io(e.to_string()))?;
    }
    match payload {
        AssetPayload::Dir(src) => {
            if target.exists() {
                std::fs::remove_dir_all(target).map_err(|e| InstallError::Io(e.to_string()))?;
            }
            copy_dir_recursive(src, target)
        }
        AssetPayload::File(src) => std::fs::copy(src, target)
            .map(|_| ())
            .map_err(|e| InstallError::Io(e.to_string())),
        AssetPayload::Text(t) => std::fs::write(target, t).map_err(|e| InstallError::Io(e.to_string())),
    }
}

#[derive(Default)]
struct SyncReport {
    installed: Vec<String>,
    unchanged: Vec<String>,
    updated: Vec<String>,
    skipped: Vec<String>,
    overwritten: Vec<String>,
}

/// 按账本哈希门同步单个资产；`check_only` 只裁决不写（账本亦不动）。
fn sync_asset(
    harness: &Path,
    rel: &str,
    payload: &AssetPayload,
    ledger: &mut BTreeMap<String, String>,
    force: bool,
    check_only: bool,
    rep: &mut SyncReport,
) -> Result<(), InstallError> {
    let target = harness.join(rel);
    let src_fp = payload_fingerprint(payload);
    let verdict = gate(&target, rel, &src_fp, ledger, force);
    match verdict {
        Verdict::Unchanged => {
            rep.unchanged.push(rel.to_string());
            ledger.insert(rel.to_string(), src_fp);
        }
        Verdict::Skip => rep.skipped.push(rel.to_string()),
        Verdict::Install | Verdict::Update | Verdict::Overwrite => {
            if !check_only {
                write_payload(&target, payload)?;
            }
            ledger.insert(rel.to_string(), src_fp);
            let bucket = match verdict {
                Verdict::Install => &mut rep.installed,
                Verdict::Update => &mut rep.updated,
                _ => &mut rep.overwritten,
            };
            bucket.push(rel.to_string());
        }
    }
    Ok(())
}

/// 校验并收集源资产。返回 (agents, skills, problems)。toml-agents 布局在收集期即完成转换校验。
/// `skills_nested`：harness 支持子目录分类时，分类下技能的 target_rel 保留分类路径，否则拍平。
fn collect_source(
    source_root: &Path,
    layout: HarnessLayout,
    skills_nested: bool,
) -> (Vec<AgentAsset>, Vec<SkillAsset>, Vec<String>) {
    let mut problems = Vec::new();
    let mut agents = Vec::new();
    let mut skills = Vec::new();

    // agents/*.md
    let agents_dir = source_root.join("agents");
    if !agents_dir.is_dir() {
        problems.push(crate::i18n::msg("install.std_dir_missing", &[&agents_dir.display().to_string()]));
    } else {
        let mut names: BTreeMap<String, String> = BTreeMap::new(); // name -> file
        let Ok(rd) = std::fs::read_dir(&agents_dir) else {
            return (agents, skills, problems);
        };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
            .collect();
        files.sort();
        for f in files {
            let file_name = f.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let text = std::fs::read_to_string(&f).unwrap_or_default();
            let Some(fm) = crate::registry::parse_frontmatter(&text) else {
                problems.push(crate::i18n::msg("install.agent_missing_frontmatter", &[&file_name]));
                continue;
            };
            let g = |k: &str| fm.get(&Yaml::String(k.into()));
            let name = g("name").map(crate::workflow::ystr).unwrap_or_default();
            if name.is_empty() {
                problems.push(crate::i18n::msg("install.agent_missing_name", &[&file_name]));
                continue;
            }
            if let Some(prev) = names.insert(name.clone(), file_name.clone()) {
                problems.push(crate::i18n::msg("install.agent_name_dup", &[&name, &prev, &file_name]));
            }
            let mode = g("mode").map(crate::workflow::ystr).unwrap_or_default();
            if !matches!(mode.as_str(), "primary" | "subagent" | "all") {
                problems.push(crate::i18n::msg("install.agent_bad_mode", &[&file_name, &mode]));
            }
            let description = g("description").map(crate::workflow::ystr).unwrap_or_default();
            if description.trim().is_empty() {
                problems.push(crate::i18n::msg("install.agent_missing_description", &[&file_name]));
            }
            let (target_rel, payload) = match layout {
                HarnessLayout::FlatMd => {
                    (format!("agents/{file_name}"), AssetPayload::File(f.clone()))
                }
                HarnessLayout::TomlAgents => {
                    let stem = file_name.strip_suffix(".md").unwrap_or(&file_name).to_string();
                    match render_agent_toml(&name, &mode, &description, body_after_frontmatter(&text)) {
                        Ok(rendered) => (format!("agents/{stem}.toml"), AssetPayload::Text(rendered)),
                        Err(e) => {
                            problems.push(crate::i18n::msg("install.agent_toml_fail", &[&file_name, &e]));
                            continue;
                        }
                    }
                }
            };
            agents.push(AgentAsset {
                target_rel,
                payload,
            });
        }
    }

    // skills/<name>/SKILL.md 或 skills/<分类>/<name>/SKILL.md（最多一层分类，安装按 <name> 拍平）
    let skills_dir = source_root.join("skills");
    if !skills_dir.is_dir() {
        problems.push(crate::i18n::msg("install.std_dir_missing", &[&skills_dir.display().to_string()]));
    } else {
        let Ok(rd) = std::fs::read_dir(&skills_dir) else {
            return (agents, skills, problems);
        };
        let mut top: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_dir()
                    && p.file_name()
                        .map(|n| !n.to_string_lossy().starts_with('.'))
                        .unwrap_or(false)
            })
            .collect();
        top.sort();
        // (技能名, 目录, 显示路径, 分类)：分类子目录里的技能名字取叶子目录
        let mut candidates: Vec<(String, PathBuf, String, Option<String>)> = Vec::new();
        for d in top {
            let top_name = d.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if d.join("SKILL.md").is_file() {
                candidates.push((top_name.clone(), d.clone(), format!("skills/{top_name}"), None));
                continue;
            }
            let Ok(sub_rd) = std::fs::read_dir(&d) else {
                problems.push(crate::i18n::msg("install.skill_unreadable", &[&top_name]));
                continue;
            };
            let mut subs: Vec<PathBuf> = sub_rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .map(|n| !n.to_string_lossy().starts_with('.'))
                            .unwrap_or(false)
                })
                .collect();
            subs.sort();
            if subs.is_empty() {
                problems.push(crate::i18n::msg("install.skill_no_entry", &[&top_name]));
                continue;
            }
            for s in subs {
                let leaf = s.file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
                if !s.join("SKILL.md").is_file() {
                    problems.push(crate::i18n::msg("install.skill_nested_md_missing", &[&top_name, &leaf]));
                    continue;
                }
                candidates.push((leaf.clone(), s.clone(), format!("skills/{top_name}/{leaf}"), Some(top_name.clone())));
            }
        }
        candidates.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
        let mut seen: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        for (name, d, display, category) in candidates {
            if let Some(first) = seen.get(&name) {
                problems.push(crate::i18n::msg("install.skill_name_dup", &[&name, &first, &display]));
                continue;
            }
            seen.insert(name.clone(), display.clone());
            let skill_md = d.join("SKILL.md");
            let text = std::fs::read_to_string(&skill_md).unwrap_or_default();
            let fm_name = crate::registry::parse_frontmatter(&text)
                .and_then(|fm| fm.get(&Yaml::String("name".into())).map(|v| crate::workflow::ystr(v)))
                .unwrap_or_default();
            if fm_name != name {
                problems.push(crate::i18n::msg("install.skill_name_mismatch", &[&display, &fm_name, &name]));
                continue;
            }
            // nested harness 保留分类子目录；顶层技能（dmaf/rules 等）与拍平 harness 均落 skills/<名>
            let target_rel = match (&category, skills_nested) {
                (Some(cat), true) => format!("skills/{cat}/{name}"),
                _ => format!("skills/{name}"),
            };
            skills.push(SkillAsset {
                name: name.clone(),
                target_rel,
                payload: AssetPayload::Dir(d),
            });
        }
    }

    (agents, skills, problems)
}

/// 执行安装。`harness` 为目标 harness 根目录，`layout` 决定 agent 资产形态（原样复制或转 TOML），
/// `source_root` 为标准目录所在根（通常是项目根）。
/// 账本哈希门：目标未改动的资产随源升级自动更新；用户改过的跳过，`force` 时覆盖（见模块说明）。
/// `skills_nested`：harness 支持 skills 子目录分类时，分类下技能保留分类路径部署，否则拍平。
pub fn install(
    harness: &Path,
    layout: HarnessLayout,
    source_root: &Path,
    prune: bool,
    check_only: bool,
    skills_nested: bool,
    force: bool,
) -> Result<Value, InstallError> {
    let (agents, skills, problems) = collect_source(source_root, layout, skills_nested);
    if !problems.is_empty() {
        return Err(InstallError::Invalid(problems));
    }

    let mut ledger = load_install_ledger(harness);
    let mut rep = SyncReport::default();
    let mut pruned = Vec::new();

    for a in &agents {
        sync_asset(harness, &a.target_rel, &a.payload, &mut ledger, force, check_only, &mut rep)?;
    }
    for s in &skills {
        sync_asset(harness, &s.target_rel, &s.payload, &mut ledger, force, check_only, &mut rep)?;
    }

    // 孤儿清理（仅 --prune 且非 check-only）；agents 按布局扩展名清理，用户自建的其他文件不动
    if prune && !check_only {
        let harness_agents = harness.join("agents");
        if harness_agents.is_dir() {
            let source_names: Vec<String> = agents
                .iter()
                .map(|a| {
                    a.target_rel
                        .strip_prefix("agents/")
                        .unwrap_or(&a.target_rel)
                        .to_string()
                })
                .collect();
            let mut entries: Vec<PathBuf> = std::fs::read_dir(&harness_agents)
                .map_err(|e| InstallError::Io(e.to_string()))?
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .map(|x| x == layout.agents_ext())
                        .unwrap_or(false)
                })
                .collect();
            entries.sort();
            for p in entries {
                let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                if !source_names.contains(&name) {
                    std::fs::remove_file(&p).map_err(|e| InstallError::Io(e.to_string()))?;
                    pruned.push(format!("agents/{name}"));
                }
            }
        }
        let harness_skills = harness.join("skills");
        if harness_skills.is_dir() {
            let source_names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
            // 已部署技能目录：flat 一层即技能；nested 顶层技能（含 SKILL.md）+ 分类下二层。
            // 分类目录本身不是资产，孤儿清理后若被清空则一并移除
            let mut skill_dirs: Vec<PathBuf> = Vec::new();
            let mut category_dirs: Vec<PathBuf> = Vec::new();
            let mut top: Vec<PathBuf> = std::fs::read_dir(&harness_skills)
                .map_err(|e| InstallError::Io(e.to_string()))?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            top.sort();
            for d in top {
                if d.join("SKILL.md").is_file() {
                    skill_dirs.push(d);
                } else if skills_nested {
                    category_dirs.push(d.clone());
                    let mut subs: Vec<PathBuf> = std::fs::read_dir(&d)
                        .map_err(|e| InstallError::Io(e.to_string()))?
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.is_dir())
                        .collect();
                    subs.sort();
                    skill_dirs.extend(subs);
                } else {
                    skill_dirs.push(d);
                }
            }
            for p in &skill_dirs {
                let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                if !source_names.contains(&name) {
                    std::fs::remove_dir_all(p).map_err(|e| InstallError::Io(e.to_string()))?;
                    let rel = p
                        .strip_prefix(harness)
                        .map(|x| crate::paths::to_posix(x))
                        .unwrap_or_else(|_| format!("skills/{name}"));
                    pruned.push(rel);
                }
            }
            for c in &category_dirs {
                let empty = std::fs::read_dir(c)
                    .map(|mut r| r.next().is_none())
                    .unwrap_or(false);
                if empty {
                    std::fs::remove_dir(c).map_err(|e| InstallError::Io(e.to_string()))?;
                }
            }
        }
        for rel in &pruned {
            ledger.remove(rel);
        }
    }

    if !check_only {
        save_install_ledger(harness, &ledger)?;
    }

    Ok(json!({
        "ok": true,
        "harness": crate::paths::to_posix(harness),
        "layout": match layout {
            HarnessLayout::FlatMd => "flat-md",
            HarnessLayout::TomlAgents => "toml-agents",
        },
        "check_only": check_only,
        "prune": prune,
        "force": force,
        "ledger": crate::paths::to_posix(&harness.join(INSTALL_LEDGER_FILE)),
        "installed": rep.installed,
        "unchanged": rep.unchanged,
        "updated": rep.updated,
        "skipped": rep.skipped,
        "overwritten": rep.overwritten,
        "pruned": pruned,
    }))
}

// ---------- 入口配置落位 ----------

/// 入口配置落位：把 `dmaf/harness/<name>/` 下的文件复制到 **harness 目录内**（如
/// `.opencode/opencode.json`，不再落项目根）。逐文件走同一账本哈希门（未改动随源更新、
/// 改过跳过、`force` 覆盖）。`check_only` 只报告不写入。
/// 仅裸名 harness 生效（路径透传的调用方不必调用）。
pub fn install_entry_config(
    harness_name: &str,
    project_root: &Path,
    check_only: bool,
    force: bool,
) -> Result<Value, InstallError> {
    let source = project_root.join("dmaf").join("harness").join(harness_name);
    if !source.is_dir() {
        return Ok(json!({
            "harness": harness_name,
            "source": crate::paths::to_posix(&source),
            "skipped": crate::i18n::t("install.entry_config_no_dir"),
        }));
    }
    let rh = crate::registry::harness_root(harness_name)
        .map_err(InstallError::Io)?;
    let target_base = project_root.join(rh.dir);
    let mut files = walk_files(&source);
    files.sort();
    let mut ledger = load_install_ledger(&target_base);
    let mut rep = SyncReport::default();
    for f in &files {
        let rel = f
            .strip_prefix(&source)
            .map(|p| crate::paths::to_posix(p))
            .unwrap_or_default();
        sync_asset(&target_base, &rel, &AssetPayload::File(f.clone()), &mut ledger, force, check_only, &mut rep)?;
    }
    if !check_only {
        save_install_ledger(&target_base, &ledger)?;
    }
    Ok(json!({
        "harness": harness_name,
        "source": crate::paths::to_posix(&source),
        "target": crate::paths::to_posix(&target_base),
        "check_only": check_only,
        "force": force,
        "created": rep.installed,
        "unchanged": rep.unchanged,
        "updated": rep.updated,
        "skipped": rep.skipped,
        "overwritten": rep.overwritten,
    }))
}

// ---------- TOML 转换（toml-agents 布局） ----------

/// md agent → codex/dsh 风格 TOML。字节确定：字段顺序固定、正文用 literal 多行字符串零转义。
///
/// `mode` 以首行注释保留（对 harness 解析器透明，dmaf 注册表读回时用于派发模式校验）；
/// 正文含 `'''`（literal 定界符）时不可无损承载，返回 Err 由调用方拒装。
pub fn render_agent_toml(
    name: &str,
    mode: &str,
    description: &str,
    body: &str,
) -> Result<String, String> {
    if body.contains("'''") {
        return Err(crate::i18n::t("install.toml_body_delimiter"));
    }
    let name = toml_basic_string(name)?;
    let description = toml_basic_string(description)?;
    let mut body = body.to_string();
    if !body.ends_with('\n') {
        body.push('\n');
    }
    Ok(format!(
        "# dmaf-mode: {mode}\nname = {name}\ndescription = {description}\n\ndeveloper_instructions = '''\n{body}'''\n"
    ))
}

/// TOML basic string：最小转义（\\、\"、\n、\r、\t）；其余控制字符拒绝。
fn toml_basic_string(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                return Err(crate::i18n::msg("install.toml_control_char", &[&format!("{:04X}", c as u32)]))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(out)
}

/// 取 md 正文：frontmatter 闭合 `---` 行之后的内容（保留原始换行）。
fn body_after_frontmatter(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("---") else {
        return text;
    };
    let Some(idx) = rest.find("---") else {
        return text;
    };
    let after = &rest[idx + 3..];
    after
        .strip_prefix("\r\n")
        .or_else(|| after.strip_prefix('\n'))
        .unwrap_or(after)
}
