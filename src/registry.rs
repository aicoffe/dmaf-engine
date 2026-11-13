//! Agent/Skill 注册表：直接读取 harness 目录的 frontmatter，单一事实源。
//!
//! 不另建注册表文件，避免两处真相漂移；validate 时核对工作流引用的存在性与授权。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_yaml::Value as Yaml;

use crate::workflow::Workflow;

#[derive(Debug, Default, Clone)]
pub struct RegistryEntry {
    pub meta: serde_yaml::Mapping,
    pub path: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct Registry {
    /// name -> {meta, path}
    pub agents: BTreeMap<String, RegistryEntry>,
    /// name -> {meta, path}
    pub skills: BTreeMap<String, RegistryEntry>,
}

pub(crate) fn parse_frontmatter(text: &str) -> Option<serde_yaml::Mapping> {
    let rest = text.strip_prefix("---")?;
    let idx = rest.find("---")?;
    let front = &rest[..idx];
    let data: Yaml = serde_yaml::from_str(front).ok()?;
    data.as_mapping().cloned()
}

fn load_frontmatter_dir(directory: &Path) -> BTreeMap<String, RegistryEntry> {
    let mut result = BTreeMap::new();
    if !directory.is_dir() {
        return result;
    }
    let mut paths: Vec<PathBuf> = walk_md(directory);
    paths.sort();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let meta = parse_frontmatter(&text).unwrap_or_default();
        let name = meta
            .get(&Yaml::String("name".into()))
            .map(|v| match v {
                Yaml::String(s) => s.clone(),
                other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
            })
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
        result.insert(name, RegistryEntry { meta, path });
    }
    result
}

fn walk_md(dir: &Path) -> Vec<PathBuf> {
    walk_ext(dir, "md")
}

fn walk_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_ext(&p, ext));
        } else if p.extension().map(|x| x == ext).unwrap_or(false) {
            out.push(p);
        }
    }
    out
}/// harness 资产布局：flat-md = md 原样复制（opencode 系）；toml-agents = agent 转 TOML（codex/dsh）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessLayout {
    FlatMd,
    TomlAgents,
}

impl HarnessLayout {
    /// 该布局下 harness agents 目录的资产扩展名。
    pub fn agents_ext(self) -> &'static str {
        match self {
            HarnessLayout::FlatMd => "md",
            HarnessLayout::TomlAgents => "toml",
        }
    }
}

/// `--harness` 解析结果：规范名（入口配置等按名索引的场景用）+ 目标目录（相对 CWD，即项目根）+ 资产布局。
#[derive(Debug, Clone)]
pub struct ResolvedHarness {
    /// 规范名（opencode/claude/zcode/cursor/gemini/codex/dsh）；路径透传时为 None
    pub name: Option<&'static str>,
    pub dir: PathBuf,
    pub layout: HarnessLayout,
    /// harness 能否递归发现 skills/<分类>/<技能>/SKILL.md：true → install 保留分类子目录部署，
    /// false → 按技能名拍平。能力矩阵：opencode、cursor = true；其余 = false。
    pub skills_nested: bool,
}

/// 已知 harness 的 (目录名, 规范名)。harness_root 的正向映射与 detect_harnesses 的
/// 逆向探测共用这张表；目录名必须是规范目录（`.claude-code` 之类别名目录不存在）。
const KNOWN_HARNESSES: &[(&str, &str)] = &[
    (".opencode", "opencode"),
    (".claude", "claude"),
    (".zcode", "zcode"),
    (".cursor", "cursor"),
    (".gemini", "gemini"),
    (".codex", "codex"),
    (".dsh", "dsh"),
];

/// harness 名称 → 项目级目录 + 布局（相对 CWD，即项目根）。install/check/doctor/compile 共用。
///
/// 已知名称按表映射；形如路径的值（含分隔符或以 `.` 开头，如 `.opencode`）按 flat-md 原样透传；
/// 其余裸名报错，防止拼写笔误静默新建陌生目录。
pub fn harness_root(value: &str) -> Result<ResolvedHarness, String> {
    let (name, dir, layout, skills_nested) = match value {
        "opencode" => ("opencode", ".opencode", HarnessLayout::FlatMd, true),
        "claude" | "claude-code" => ("claude", ".claude", HarnessLayout::FlatMd, false),
        "zcode" => ("zcode", ".zcode", HarnessLayout::FlatMd, false),
        "cursor" => ("cursor", ".cursor", HarnessLayout::FlatMd, true),
        "gemini" | "gemini-cli" => ("gemini", ".gemini", HarnessLayout::FlatMd, false),
        "codex" => ("codex", ".codex", HarnessLayout::TomlAgents, false),
        "dsh" => ("dsh", ".dsh", HarnessLayout::TomlAgents, false),
        other => {
            if other.contains('/') || other.contains('\\') || other.starts_with('.') {
                ("", other, HarnessLayout::FlatMd, false)
            } else {
                return Err(crate::i18n::msg("registry.unknown_harness", &[other]));
            }
        }
    };
    let name = if name.is_empty() { None } else { Some(name) };
    Ok(ResolvedHarness { name, dir: PathBuf::from(dir), layout, skills_nested })
}

/// 自动探测项目根下已安装的 harness 目录，返回规范名（字典序）。
/// install 缺省 `--harness` 时使用：唯一 → 直装；多个 → 列出让用户显式指定。
pub fn detect_harnesses(project_root: &Path) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = KNOWN_HARNESSES
        .iter()
        .filter(|(dir, _)| project_root.join(dir).is_dir())
        .map(|(_, name)| *name)
        .collect();
    found.sort_unstable();
    found
}

pub fn load_registry(root: &Path, layout: HarnessLayout) -> Registry {
    Registry {
        agents: match layout {
            HarnessLayout::FlatMd => load_frontmatter_dir(&root.join("agents")),
            HarnessLayout::TomlAgents => load_toml_agent_dir(&root.join("agents")),
        },
        skills: load_frontmatter_dir(&root.join("skills")),
    }
}

/// toml-agents 布局的 agent 注册表：解析 `*.toml` 的 name/description；
/// mode 从 `# dmaf-mode:` 注释恢复，缺失（非 dmaf 安装的手写 agent）按 subagent 放行。
fn load_toml_agent_dir(directory: &Path) -> BTreeMap<String, RegistryEntry> {
    let mut result = BTreeMap::new();
    if !directory.is_dir() {
        return result;
    }
    let mut paths: Vec<PathBuf> = walk_ext(directory, "toml");
    paths.sort();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = text.parse::<toml::Value>() else {
            continue;
        };
        let s = |k: &str| value.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let Some(name) = s("name").filter(|n| !n.is_empty()) else {
            continue;
        };
        let mut meta = serde_yaml::Mapping::new();
        meta.insert(Yaml::String("name".into()), Yaml::String(name.clone()));
        if let Some(d) = s("description") {
            meta.insert(Yaml::String("description".into()), Yaml::String(d));
        }
        let mode = comment_mode(&text).unwrap_or_else(|| "subagent".into());
        meta.insert(Yaml::String("mode".into()), Yaml::String(mode));
        result.insert(name, RegistryEntry { meta, path });
    }
    result
}

/// 首个 `# dmaf-mode: <mode>` 注释行的值。
fn comment_mode(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.trim().strip_prefix("# dmaf-mode:"))
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string)
}

/// 校验工作流引用的 agent/skill 是否存在；agent 必须是 subagent 模式。
pub fn validate_refs(workflow: &Workflow, registry: &Registry) -> Vec<String> {
    let mut problems = Vec::new();
    for stage in &workflow.stages {
        // M4a：审批评审者也是被派发对象，同样须存在且可派发
        if let Some(reviewer) = &stage.approval.reviewer {
            match registry.agents.get(reviewer) {
                None => problems.push(crate::i18n::msg(
                    "registry.reviewer_missing",
                    &[&stage.id, reviewer],
                )),
                Some(entry) => {
                    let mode = entry
                        .meta
                        .get(&Yaml::String("mode".into()))
                        .map(crate::workflow::ystr)
                        .unwrap_or_default();
                    if mode != "subagent" && mode != "all" {
                        problems.push(crate::i18n::msg(
                            "registry.reviewer_bad_mode",
                            &[reviewer, &mode],
                        ));
                    }
                }
            }
        }
        for r in &stage.agents {
            let Some(entry) = registry.agents.get(&r.agent) else {
                problems.push(crate::i18n::msg(
                    "registry.agent_missing",
                    &[&stage.id, &r.agent],
                ));
                continue;
            };
            let mode = entry
                .meta
                .get(&Yaml::String("mode".into()))
                .map(crate::workflow::ystr)
                .unwrap_or_default();
            if mode != "subagent" && mode != "all" {
                problems.push(crate::i18n::msg("registry.agent_bad_mode", &[&r.agent, &mode]));
            }
            for skill in &r.skills {
                if !registry.skills.contains_key(skill) {
                    problems.push(crate::i18n::msg(
                        "registry.skill_missing",
                        &[&r.agent, skill],
                    ));
                }
            }
        }
    }
    problems
}

pub fn skill_briefs(registry: &Registry, names: &[String]) -> Vec<serde_json::Value> {
    names
        .iter()
        .map(|name| {
            let description = registry
                .skills
                .get(name)
                .map(|e| {
                    e.meta
                        .get(&Yaml::String("description".into()))
                        .map(crate::workflow::ystr)
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                })
                .unwrap_or_default();
            serde_json::json!({"name": name, "description": description})
        })
        .collect()
}
