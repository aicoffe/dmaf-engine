//! 项目脚手架：`dmaf new` 创建 dmaf 标准目录布局（骨架，不含工作流内容）。
//!
//! 同时覆盖两种场景（永不覆盖已存在文件）：
//! - greenfield：`dmaf new --dir my-project` 创建新项目骨架，随后自行编写流程或 `dmaf add` 安装工作流包；
//! - brownfield：`dmaf new --dir .` 在已有仓库补齐 dmaf 相关目录。
//!
//! 唯一模板为 `minimal`；可运行的样例流程走包分发（`dmaf add --from <工作流项目|git URL>`，
//! 目标无 dmaf/ 时 add 自行补齐骨架），引擎二进制不携带示例工作流内容。
//! 随二进制下发的还有两份内嵌资产：流程定义指南（docs/，写入 dmaf/docs/）与
//! 定义技能（tools/skills/，写入 dmaf/skills/——都是引擎槽位，见 src/package.rs）。
//! `dmaf init` 是另一个语义（工作流启动前的一次性初始化），**尚未实现**，此处不涉及。
//! 模板内嵌二进制（include_dir），与引擎一起分发。
//! 清单模板的 engine 范围占位符 `@ENGINE_RANGE@` 在落盘时按当前二进制渲染（见 engine_range）：
//! 模板静态内嵌而引擎版本随发版演进，渲染放在写入侧才不会腐化。

use std::path::Path;

use include_dir::{include_dir, Dir};
use serde_json::json;

use crate::paths;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// 定义技能（工作流定义的 AI 辅助创建）：唯一事实源在引擎仓库 tools/skills/，编译期内嵌，
/// dmaf new 时写入目标 dmaf/skills/ 下——经 `dmaf install` 部署到 harness 即可触发。
/// 技能目录是引擎槽位（包分发排除，避免 market 多包安装下通用名跨包碰撞）。
static AUTHORING_SKILLS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/tools/skills");

/// 面向使用者的流程定义指南：中文真源在引擎仓库 docs/zh/（英文缺省版 docs/flow-definition-guide.md，
/// M3 起双语内嵌按 locale 选写），编译期内嵌中文版，
/// dmaf new 时写入用户项目 dmaf/docs/ 下（模板不携带副本，机制上无漂移）。
const GUIDE_REL: &str = "dmaf/docs/流程定义指南.md";
const GUIDE: &str = include_str!("../docs/zh/流程定义指南.md");
/// 英文版指南（用户发布面 docs/flow-definition-guide.md，中文为编辑真源）：
/// dmaf new 按 locale 二选一写入用户项目（M3 双语嵌入，见国际化实施方案 §7.3）。
const GUIDE_EN_REL: &str = "dmaf/docs/flow-definition-guide.md";
const GUIDE_EN: &str = include_str!("../docs/flow-definition-guide.md");

pub fn available_templates() -> Vec<String> {
    let mut names: Vec<String> = TEMPLATES
        .dirs()
        .map(|d| {
            d.path()
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty())
        .collect();
    names.sort();
    names
}

fn template_dir(template: &str) -> Option<&'static Dir<'static>> {
    TEMPLATES.get_dir(template)
}

/// 模板内全部文件（递归）：(去掉模板名前缀的相对路径, 内容)。
fn scaffold_files(template: &str) -> Vec<(String, &'static [u8])> {
    let Some(dir) = template_dir(template) else {
        return Vec::new();
    };
    let prefix = format!("{template}/");
    let mut out: Vec<(String, &'static [u8])> = Vec::new();
    collect_files(dir, &prefix, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect_files(dir: &Dir<'static>, prefix: &str, out: &mut Vec<(String, &'static [u8])>) {
    for f in dir.files() {
        let rel = f.path().to_string_lossy().into_owned();
        if let Some(rel) = rel.strip_prefix(prefix) {
            out.push((rel.to_string(), f.contents()));
        }
    }
    for sub in dir.dirs() {
        collect_files(sub, prefix, out);
    }
}

/// 内嵌目录的全部文件（递归）：(目标相对路径, 内容)。`target_prefix` 直接拼在文件相对路径前。
fn embedded_files(
    embedded: &'static Dir<'static>,
    target_prefix: &str,
) -> Vec<(String, &'static [u8])> {
    let mut out: Vec<(String, &'static [u8])> = Vec::new();
    collect_files(embedded, "", &mut out);
    let mut prefixed: Vec<(String, &'static [u8])> = out
        .into_iter()
        .map(|(rel, contents)| (format!("{target_prefix}{rel}"), contents))
        .collect();
    prefixed.sort_by(|a, b| a.0.cmp(&b.0));
    prefixed
}

// .dmaf/ 引擎运行独占区整体忽略；.opencode/ 为 harness 派生物。
// dmaf/ 定义层（含 compiled/ 产物）全部入库，不在此列；业务产物目录（样例用 dmaf-output/）是工作成果，
// 引擎永不写入忽略清单，也不触碰用户 .gitignore 中的任何既有条目。
const GITIGNORE_ENTRIES: &[&str] = &[".dmaf/", ".opencode/"];

#[derive(Debug, Clone)]
pub enum ScaffoldError {
    Usage(String),
    Io(String),
}

/// `.gitignore` 追加语义：存在则只追加缺失条目（带分组注释头），不存在则整份创建；
/// 用户既有条目一律不动。返回本次追加的条目。`dmaf new` 与包合入的骨架补齐（src/package.rs）共用。
pub(crate) fn append_gitignore_entries(dir: &Path) -> Result<Vec<String>, String> {
    let gitignore = dir.join(".gitignore");
    let mut appended = Vec::new();
    if gitignore.exists() {
        let existing = std::fs::read_to_string(&gitignore).unwrap_or_default();
        let have: std::collections::BTreeSet<String> =
            existing.lines().map(|l| l.trim().to_string()).collect();
        let missing: Vec<&str> = GITIGNORE_ENTRIES
            .iter()
            .copied()
            .filter(|e| !have.contains(*e))
            .collect();
        if !missing.is_empty() {
            let mut text = existing.clone();
            if !text.ends_with('\n') && !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&crate::i18n::t("scaffold.gitignore_group_header"));
            text.push('\n');
            for e in &missing {
                text.push_str(e);
                text.push('\n');
                appended.push(e.to_string());
            }
            std::fs::write(&gitignore, text).map_err(|e| e.to_string())?;
        }
    } else {
        let text = format!("{}\n", GITIGNORE_ENTRIES.join("\n"));
        std::fs::write(&gitignore, text).map_err(|e| e.to_string())?;
        appended.extend(GITIGNORE_ENTRIES.iter().map(|s| s.to_string()));
    }
    Ok(appended)
}

fn io_err(e: std::io::Error) -> ScaffoldError {
    ScaffoldError::Io(e.to_string())
}

fn ensure_template(template: &str) -> Result<(), ScaffoldError> {
    if template_dir(template).is_some() {
        Ok(())
    } else {
        Err(ScaffoldError::Usage(crate::i18n::msg(
            "scaffold.unknown_template",
            &[template, &format!("{:?}", available_templates())],
        )))
    }
}

/// 清单 engine 兼容带：`>=MAJOR.MINOR,<MAJOR.(MINOR+1)`，与当前二进制同段。
/// 模板静态内嵌、引擎版本随发版演进，占位符在 dmaf new 落盘时按本二进制渲染。
fn engine_range() -> String {
    let major = env!("CARGO_PKG_VERSION_MAJOR");
    let minor: u64 = env!("CARGO_PKG_VERSION_MINOR")
        .parse()
        .expect("CARGO_PKG_VERSION_MINOR 必为数字");
    format!(">={major}.{minor},<{major}.{}", minor + 1)
}

/// 创建 dmaf 标准目录布局（greenfield 建新项目 / brownfield 补齐已有仓库）。
///
/// 合并语义：目录已存在时只创建缺失文件，**绝不覆盖**已存在文件
/// （`force` 才覆盖模板来源的占位文件）；`.gitignore` 只追加缺失条目。
pub fn new_project(
    template: &str,
    dir: &Path,
    force: bool,
) -> Result<serde_json::Value, ScaffoldError> {
    ensure_template(template)?;
    let mut created = Vec::new();
    let mut unchanged = Vec::new();
    let mut overwritten = Vec::new();

    let mut files: Vec<(String, &'static [u8])> = scaffold_files(template)
        .into_iter()
        .filter(|(rel, _)| rel != ".gitignore")
        .collect(); // .gitignore 单独走追加逻辑
    // 流程定义指南：固定附加文件（无论模板），按 locale 选语言写入（中文为编辑真源；
    // dmaf new 的语言来自进程 locale：DMAF_LOCALE 预设，未设时系统语言探测（zh 系 zh、其余 en））
    let (guide_rel, guide) = match crate::i18n::current() {
        crate::i18n::Locale::En => (GUIDE_EN_REL, GUIDE_EN),
        crate::i18n::Locale::Zh => (GUIDE_REL, GUIDE),
    };
    files.push((guide_rel.to_string(), guide.as_bytes()));
    // 定义技能：固定附加资产（无论模板），写入 dmaf/skills/ 直属目录（引擎槽位，见 src/package.rs）
    files.extend(embedded_files(
        &AUTHORING_SKILLS,
        "dmaf/skills/",
    ));

    let range = engine_range();
    for (rel, contents) in &files {
        let (rel, contents) = (rel.as_str(), *contents);
        // 清单模板的 engine 范围按当前二进制渲染（机制上无漂移，同流程定义指南内嵌）
        let manifest_rendered;
        let contents: &[u8] = if rel == "dmaf/manifest.yaml" {
            manifest_rendered = String::from_utf8_lossy(contents)
                .replace("@ENGINE_RANGE@", &range)
                .into_bytes();
            &manifest_rendered
        } else {
            contents
        };
        let target = dir.join(&rel);
        if target.exists() {
            if !force {
                unchanged.push(rel);
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io_err)?;
            }
            std::fs::write(&target, contents).map_err(io_err)?;
            overwritten.push(rel);
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io_err)?;
            }
            std::fs::write(&target, contents).map_err(io_err)?;
            created.push(rel);
        }
    }

    // .gitignore：存在则只追加缺失条目；不存在则整份创建
    let appended = append_gitignore_entries(dir).map_err(ScaffoldError::Io)?;

    created.sort();
    unchanged.sort();
    overwritten.sort();
    Ok(json!({
        "ok": true,
        "command": "new",
        "template": template,
        "dir": paths::to_posix(dir),
        "created": created,
        "unchanged": unchanged,
        "overwritten": overwritten,
        "gitignore_appended": appended,
    }))
}
