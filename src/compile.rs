//! 工作流编译：stage/scope 定义（作者层）→ 字节确定的 JSON 产物（产物层）。
//!
//! 全部结构校验发生在编译期；任何 errors 非空则不写产物。
//! stage 级结构/语义校验复用 `workflow::parse_workflow`（协作/枚举约束、模板变量、
//! completion 占位符等与内联工作流同一套规则）；本模块额外负责：
//! - 文件形态解析（md frontmatter、id=stem、phase=目录名）；
//! - scope 侧成员序列（include_stages，顺序即执行序）：序列校验（硬输入生产者
//!   必须是成员且在前、软输入死接线拦截）与派生数据依赖边（D5）；
//! - 产物生成与字节确定性。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use serde_yaml::Value as Yaml;

use crate::registry::{load_registry, validate_refs, ResolvedHarness};
use crate::workflow::{parse_workflow, validate_and_derive_sequence, Workflow, WorkflowError};

pub const GRAPH_VERSION: u64 = 3;
/// v3：stage 条目不再携带作者 depends_on 与 display_order（顺序语义移交 scope 序列）。
pub const GRID_VERSION: u64 = 5;
/// v5：include_stages 从规范化集合变为**有序成员表**（顺序即执行序），新增 `depends`
/// 键 = 编译期按各 scope 序列推导的数据依赖边（stage → 硬输入的、排在消费者之前的
/// 成员生产者，见 `workflow::validate_and_derive_sequence`）。
/// v2：vars 改 mapping 形态（名 → 内联默认/null=必填）、新增 constants、移除 defaults。
/// v3：移除 derived_vars（scope-grid 不含该键）。
/// v4：移除 description_from_stage（死字段：消费者早于 6213742 删除，仅余声明管线）。

#[derive(Debug, Clone)]
pub struct CompileError {
    pub problems: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledOutput {
    pub stage_graph: Value,
    pub scope_grid: Value,
    /// 由全部 stage 组装并通过 parse_workflow 校验的工作流（供 check/黄金对照复用）
    pub workflow: Workflow,
}

fn compile_err(problems: Vec<String>) -> CompileError {
    CompileError { problems }
}

// ---------- 文件解析 ----------

fn read_md_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    if !dir.is_dir() {
        return Err(crate::i18n::msg("compile.dir_not_found", &[&dir.display().to_string()]));
    }
    let mut files: Vec<std::path::PathBuf> = walk_md(dir);
    files.sort();
    Ok(files)
}

fn walk_md(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_md(&p));
        } else if p.extension().map(|x| x == "md").unwrap_or(false) {
            out.push(p);
        }
    }
    out
}

fn frontmatter_of(path: &Path, problems: &mut Vec<String>) -> Option<serde_yaml::Mapping> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            problems.push(crate::i18n::msg(
                "compile.read_failed",
                &[&path.display().to_string(), &e.to_string()],
            ));
            return None;
        }
    };
    match crate::registry::parse_frontmatter(&text) {
        Some(m) => Some(m),
        None => {
            problems.push(crate::i18n::msg(
                "compile.frontmatter_missing",
                &[&path.display().to_string()],
            ));
            None
        }
    }
}

/// inputs 条目归一化：字符串 → {artifact, required: true}；对象校验恰为 artifact+required。
fn normalize_inputs(raw: Option<&Yaml>, where_: &str, problems: &mut Vec<String>) -> Vec<Value> {
    let Some(v) = raw else {
        return Vec::new();
    };
    let Some(seq) = v.as_sequence() else {
        problems.push(crate::i18n::msg("compile.inputs_must_be_list", &[where_]));
        return Vec::new();
    };
    seq.iter()
        .map(|item| match item {
            Yaml::String(s) => json!({"artifact": s, "required": true}),
            Yaml::Mapping(m) => {
                let keys: Vec<String> = m
                    .iter()
                    .map(|(k, _)| crate::workflow::ystr(k))
                    .collect();
                let artifact = m
                    .get(&Yaml::String("artifact".into()))
                    .map(crate::workflow::ystr)
                    .unwrap_or_default();
                let required = m
                    .get(&Yaml::String("required".into()))
                    .and_then(|r| r.as_bool());
                if artifact.is_empty() || keys.len() != 2 || required.is_none() {
                    problems.push(crate::i18n::msg(
                        "compile.inputs_entry_form",
                        &[where_],
                    ));
                    json!({"artifact": artifact, "required": required.unwrap_or(true)})
                } else {
                    json!({"artifact": artifact, "required": required.unwrap()})
                }
            }
            other => {
                problems.push(crate::i18n::msg(
                    "compile.inputs_entry_type",
                    &[where_, &crate::workflow::ystr(other)],
                ));
                json!({"artifact": "", "required": true})
            }
        })
        .collect()
}

/// 解析单个 stage 文件 → (id, phase, 工作流文档用 stage mapping, 产物用 normalized stage json)
#[allow(clippy::type_complexity)]
fn parse_stage_file(
    path: &std::path::Path,
    phase: &str,
    problems: &mut Vec<String>,
) -> Option<(String, Yaml, Value)> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let fm = frontmatter_of(path, problems)?;
    let g = |k: &str| fm.get(&Yaml::String(k.into()));

    let id = g("id").map(crate::workflow::ystr).unwrap_or_default();
    if id.is_empty() {
        problems.push(crate::i18n::msg("compile.id_missing", &[&path.display().to_string()]));
        return None;
    }
    if id != stem {
        problems.push(crate::i18n::msg(
            "compile.id_stem_mismatch",
            &[&id, &stem, &path.display().to_string()],
        ));
    }
    let declared_phase = g("phase").map(crate::workflow::ystr).unwrap_or_default();
    if declared_phase != phase {
        problems.push(crate::i18n::msg(
            "compile.phase_mismatch",
            &[&id, &declared_phase, &phase],
        ));
    }
    if fm.contains_key(&Yaml::String("scopes".into())) {
        problems.push(crate::i18n::msg("compile.scopes_field_forbidden", &[&id]));
    }
    if fm.contains_key(&Yaml::String("depends_on".into())) {
        problems.push(crate::i18n::msg("compile.depends_on_removed", &[&id]));
    }
    let execution = g("execution").map(crate::workflow::ystr).unwrap_or_default();
    if execution.is_empty() {
        problems.push(crate::i18n::msg("compile.execution_missing", &[&id]));
    } else if execution != "always" && execution != "conditional" {
        problems.push(crate::i18n::msg(
            "compile.execution_invalid",
            &[&id, &execution],
        ));
    }
    if let Some(os) = g("on_skip").map(crate::workflow::ystr).filter(|v| !v.is_empty()) {
        if os != "skip" && os != "run" {
            problems.push(crate::i18n::msg("compile.on_skip_invalid", &[&id, &os]));
        }
    }

    // 工作流文档用 stage mapping：喂给 parse_workflow 做结构校验（不含顺序语义）
    let mut doc_stage = serde_yaml::Mapping::new();
    for key in ["id", "collaboration", "agents", "retry", "approval"] {
        if let Some(v) = g(key) {
            doc_stage.insert(Yaml::String(key.into()), v.clone());
        }
    }
    // agents 原样透传：parse_agent_ref 已支持 inputs 的字符串/映射两种形态
    if let Some(agents) = g("agents") {
        doc_stage.insert(Yaml::String("agents".into()), agents.clone());
    }

    // 产物用 normalized stage json
    let agents_json = g("agents")
        .and_then(|a| a.as_sequence())
        .map(|seq| {
            seq.iter()
                .map(|a| {
                    let m = a.as_mapping();
                    let g2 = |k: &str| m.and_then(|mm| mm.get(&Yaml::String(k.into())));
                    json!({
                        "agent": g2("agent").map(crate::workflow::ystr).unwrap_or_default(),
                        "skills": g2("skills").map(yaml_seq_strings).unwrap_or_default(),
                        "dir": g2("dir").map(crate::workflow::ystr).unwrap_or_default(),
                        "inputs": normalize_inputs(g2("inputs"), &format!("stage '{id}'"), problems),
                        "outputs": g2("outputs").map(yaml_seq_strings).unwrap_or_default(),
                        "task_description": g2("task_description").map(crate::workflow::ystr).unwrap_or_default(),
                        "vars": g2("vars").map(yaml_seq_strings).unwrap_or_default(),
                        "watches": g2("watches").map(yaml_seq_strings).unwrap_or_default(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let on_skip_v = g("on_skip").map(crate::workflow::ystr).unwrap_or_default();
    let on_skip_v = if on_skip_v.is_empty() { "skip".to_string() } else { on_skip_v };
    let stage_json = json!({
        "id": id,
        "phase": phase,
        "collaboration": g("collaboration").map(crate::workflow::ystr).unwrap_or_default(),
        "agents": agents_json,
        "retry": {
            "max_attempts": g("retry").and_then(|r| r.as_mapping()).and_then(|r| r.get(&Yaml::String("max_attempts".into()))).and_then(|v| v.as_u64()).unwrap_or(1),
            "on_exhausted": g("retry").and_then(|r| r.as_mapping()).and_then(|r| r.get(&Yaml::String("on_exhausted".into()))).map(crate::workflow::ystr).unwrap_or_else(|| "abort_workflow".into()),
        },
        "approval": {
            "required": g("approval").and_then(|r| r.as_mapping()).and_then(|r| r.get(&Yaml::String("required".into()))).and_then(|v| v.as_bool()).unwrap_or(false),
            "gate": g("approval").and_then(|r| r.as_mapping()).and_then(|r| r.get(&Yaml::String("gate".into()))).map(crate::workflow::ystr).unwrap_or_default(),
            "reviewer": g("approval").and_then(|r| r.as_mapping()).and_then(|r| r.get(&Yaml::String("reviewer".into()))).filter(|v| !v.is_null()).map(crate::workflow::ystr),
        },
        "execution": execution,
        "on_skip": on_skip_v,
    });
    Some((id, Yaml::Mapping(doc_stage), stage_json))
}

fn yaml_seq_strings(v: &Yaml) -> Vec<String> {
    v.as_sequence()
        .map(|s| s.iter().map(crate::workflow::ystr).collect())
        .unwrap_or_default()
}

// ---------- scope 解析 ----------

fn parse_scope_file(
    path: &std::path::Path,
    stage_ids: &std::collections::BTreeSet<String>,
    problems: &mut Vec<String>,
) -> Option<(String, Value)> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let fm = frontmatter_of(path, problems)?;
    let g = |k: &str| fm.get(&Yaml::String(k.into()));

    let name = g("name").map(crate::workflow::ystr).unwrap_or_default();
    if name != stem {
        problems.push(crate::i18n::msg(
            "compile.name_stem_mismatch",
            &[&name, &stem, &path.display().to_string()],
        ));
    }
    if name.is_empty() {
        return None;
    }
    let include = g("include_stages")
        .map(yaml_seq_strings)
        .ok_or_else(|| {
            problems.push(crate::i18n::msg("compile.include_stages_missing", &[&name]));
        })
        .unwrap_or_default();
    let mut seen = std::collections::BTreeSet::new();
    for s in &include {
        if !seen.insert(s.clone()) {
            problems.push(crate::i18n::msg(
                "compile.include_stages_duplicate",
                &[&name, s],
            ));
        }
        if !stage_ids.contains(s) {
            problems.push(crate::i18n::msg(
                "compile.include_stages_unknown_stage",
                &[&name, s],
            ));
        }
    }
    // vars/constants 声明解析 + 名称互斥校验（defaults/derived_vars 已移除，误用由 parse_var_decls 报迁移提示）
    let scope_prefix = format!("scope '{name}'");
    let (vars, constants) = crate::workflow::parse_var_decls(&fm, &scope_prefix, problems);
    crate::workflow::validate_var_names(&scope_prefix, &vars, &constants, problems);
    let completion_template = g("completion_template")
        .filter(|v| !v.is_null())
        .map(crate::workflow::ystr);
    if let Some(tpl) = &completion_template {
        let declared: std::collections::BTreeSet<String> = vars
            .keys()
            .chain(constants.keys())
            .cloned()
            .collect();
        let ids: std::collections::BTreeSet<String> = include.iter().cloned().collect();
        problems.extend(crate::workflow::validate_completion_placeholders(
            tpl,
            &declared,
            &ids,
            &format!("scope '{name}'"),
        ));
    }
    let grid_entry = json!({
        "description": g("description").map(crate::workflow::ystr).unwrap_or_default(),
        "vars": Value::Object(
            vars.iter()
                .map(|(k, d)| (k.clone(), d.clone().map(Value::String).unwrap_or(Value::Null)))
                .collect::<serde_json::Map<String, Value>>()
        ),
        "constants": Value::Object(
            constants
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect::<serde_json::Map<String, Value>>()
        ),
        "completion_template": completion_template
            .map(Value::String)
            .unwrap_or(Value::Null),
        "stages": include,
    });
    Some((name, grid_entry))
}

// ---------- 编译主流程 ----------

pub fn compile(
    stages_dir: &Path,
    scopes_dir: &Path,
    harness: Option<&ResolvedHarness>,
) -> Result<CompiledOutput, CompileError> {
    let mut problems: Vec<String> = Vec::new();

    // 1. 读 stage 库：phase 目录排序 × 文件排序（确定性读序）
    let mut stage_files: Vec<(String, std::path::PathBuf)> = Vec::new();
    let mut categories: Vec<std::path::PathBuf> = std::fs::read_dir(stages_dir)
        .map_err(|e| compile_err(vec![crate::i18n::msg("compile.stage_library_read_failed", &[&e.to_string()])]))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    categories.sort();
    for cat in &categories {
        let phase = cat.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        for f in read_md_files(cat).map_err(|s| compile_err(vec![s]))? {
            stage_files.push((phase.clone(), f));
        }
    }

    let mut doc_stages: Vec<Yaml> = Vec::new();
    let mut stages_json: Vec<Value> = Vec::new();
    for (phase, path) in &stage_files {
        if let Some((_, doc_stage, stage_json)) = parse_stage_file(path, phase, &mut problems) {
            doc_stages.push(doc_stage);
            stages_json.push(stage_json);
        }
    }

    // 2. 读 scope 文件
    let scope_files = read_md_files(scopes_dir).map_err(|s| compile_err(vec![s]))?;
    let mut scopes: Vec<(String, Value)> = Vec::new();
    let stage_ids: std::collections::BTreeSet<String> =
        stages_json.iter().map(|s| s["id"].as_str().unwrap().to_string()).collect();
    for path in &scope_files {
        if let Some((name, entry)) = parse_scope_file(path, &stage_ids, &mut problems) {
            scopes.push((name, entry));
        }
    }

    if !problems.is_empty() {
        return Err(compile_err(problems));
    }

    // 3. 组装工作流文档，复用 parse_workflow 做全部 stage 级校验
    let doc = Yaml::Mapping({
        let mut root = serde_yaml::Mapping::new();
        let mut wf = serde_yaml::Mapping::new();
        wf.insert(Yaml::String("id".into()), Yaml::String("compiled".into()));
        wf.insert(Yaml::String("stages".into()), Yaml::Sequence(doc_stages));
        root.insert(Yaml::String("workflow".into()), Yaml::Mapping(wf));
        root
    });
    let workflow = parse_workflow(&doc, stages_dir).map_err(|e: WorkflowError| compile_err(e.problems))?;

    // 规则 10：conditional stage 若是某硬输入的唯一/全部生产者 → 缺席时下游必缺料。
    // 提示改为 required: false 软输入或将生产者改为 always。
    {
        let mut producers_by_tpl: BTreeMap<String, Vec<(String, &str)>> = BTreeMap::new();
        for s in &stages_json {
            let sid = s["id"].as_str().unwrap_or_default().to_string();
            let exec = s["execution"].as_str().unwrap_or("always");
            for a in s["agents"].as_array().into_iter().flatten() {
                for o in a["outputs"].as_array().into_iter().flatten() {
                    if let Some(tpl) = o.as_str() {
                        producers_by_tpl
                            .entry(tpl.to_string())
                            .or_default()
                            .push((sid.clone(), exec));
                    }
                }
            }
        }
        // 按 (stage, 硬输入) 检查
        for s in &stages_json {
            let sid = s["id"].as_str().unwrap_or_default().to_string();
            for a in s["agents"].as_array().into_iter().flatten() {
                for i in a["inputs"].as_array().into_iter().flatten() {
                    if !i["required"].as_bool().unwrap_or(true) {
                        continue;
                    }
                    let Some(tpl) = i["artifact"].as_str() else { continue };
                    if let Some(prods) = producers_by_tpl.get(tpl) {
                        if !prods.is_empty()
                            && prods.iter().all(|(_, exec)| *exec == "conditional")
                        {
                            let plist: Vec<String> = prods
                                .iter()
                                .map(|(pid, _)| format!("'{pid}'"))
                                .collect();
                            problems.push(crate::i18n::msg(
                                "compile.hard_input_conditional_producers",
                                &[&sid, &tpl, &plist.join(", ")],
                            ));
                        }
                    }
                }
            }
        }
    }
    if !problems.is_empty() {
        return Err(compile_err(problems));
    }

    // scope 序列校验 + 派生数据依赖边（D5/D3）：include_stages 顺序即执行序——
    // 硬输入的生产者必须是成员且排在消费者之前；软输入/watches 死接线（成员生产者
    // 无一在前）编译错拦截；派生边烘进 scope-grid 的 depends 键（runtime 组装用）。
    for (name, entry) in scopes.iter_mut() {
        let order: Vec<String> = entry["stages"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        match validate_and_derive_sequence(&workflow.stages, &order, &format!("scope '{name}'")) {
            Ok(edges) => {
                entry["depends"] = json!(edges);
            }
            Err(ps) => problems.extend(ps),
        }
    }
    // 模板变量闭合：保留名 ∪ 全 scope 声明并集（stage 库先于认领合并；认领级闭合由 start 兜底）。
    // 放在成员闭包校验之后，使词表错误与结构错误在同一次编译中一并报出。
    let mut declared: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (_, entry) in &scopes {
        if let Some(m) = entry["vars"].as_object() {
            declared.extend(m.keys().cloned());
        }
        if let Some(m) = entry["constants"].as_object() {
            declared.extend(m.keys().cloned());
        }
    }
    problems.extend(crate::workflow::check_template_vars(&workflow, &declared));
    if !problems.is_empty() {
        return Err(compile_err(problems));
    }

    // 4. 成员网格回填：stage.scopes 由 scope include_stages 转置
    // （include_stages 保持作者书写顺序——顺序即执行序，编译器不再规范化）
    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new(); // stage -> scopes
    let mut grid: serde_json::Map<String, Value> = serde_json::Map::new();
    for (name, mut entry) in scopes {
        let stages = entry["stages"].take();
        let stages: Vec<String> = serde_json::from_value(stages).unwrap_or_default();
        for s in &stages {
            members.entry(s.clone()).or_default().push(name.clone());
        }
        entry["stages"] = json!(stages);
        grid.insert(name.clone(), entry);
    }
    // 孤儿 stage advisory
    for s in &stages_json {
        let id = s["id"].as_str().unwrap();
        if !members.contains_key(id) {
            eprintln!("{}", crate::i18n::msg("compile.orphan_stage_advisory", &[id]));
        }
    }
    for s in stages_json.iter_mut() {
        let id = s["id"].as_str().unwrap().to_string();
        let member_scopes = members.remove(&id).unwrap_or_default();
        s["scopes"] = json!(member_scopes);
    }

    let output = CompiledOutput {
        stage_graph: json!({"version": GRAPH_VERSION, "stages": stages_json}),
        scope_grid: json!({"version": GRID_VERSION, "scopes": grid}),
        workflow,
    };

    // 5. 可选 harness 绑定校验
    if let Some(h) = harness {
        let registry = load_registry(&h.dir, h.layout);
        let ref_problems = validate_refs(&output.workflow, &registry);
        if !ref_problems.is_empty() {
            return Err(compile_err(ref_problems));
        }
    }
    Ok(output)
}

pub fn graph_file_name() -> &'static str {
    "stage-graph.json"
}

pub fn grid_file_name() -> &'static str {
    "scope-grid.json"
}

/// 从编译产物构建 Workflow 的 scope 视图（scope 模式运行）。
///
/// 产物在编译期已通过全部结构校验，这里只做忠实反推：
/// 成员按 scope-grid 的 `stages` 有序表排列（**顺序即执行序**，compute_next 依序迭代），
/// 数据依赖边取自编译期推导烘入的 `depends` 键，流程级元数据取自 scope 条目，
/// workflow.id = scope 名。
pub fn workflow_from_artifacts(
    graph: &Value,
    grid: &Value,
    scope: &str,
) -> Result<Workflow, WorkflowError> {
    let Some(entry) = grid
        .get("scopes")
        .and_then(|s| s.get(scope))
        .cloned()
    else {
        return Err(WorkflowError {
            problems: vec![crate::i18n::msg("compile.scope_not_in_artifacts", &[scope])],
        });
    };
    let members: Vec<String> = entry["stages"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let depends: BTreeMap<String, Vec<String>> = entry["depends"]
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let mut problems: Vec<String> = Vec::new();
    let graph_by_id: BTreeMap<String, &Value> = graph["stages"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|s| (s["id"].as_str().unwrap_or_default().to_string(), s))
                .collect()
        })
        .unwrap_or_default();
    let mut stages: Vec<crate::workflow::Stage> = Vec::new();
    for id in &members {
        let Some(s) = graph_by_id.get(id) else {
            problems.push(crate::i18n::msg(
                "compile.member_missing_in_graph",
                &[scope, id],
            ));
            continue;
        };
        if let Some(mut st) = stage_from_graph(s, &mut problems) {
            st.depends_on = depends.get(id).cloned().unwrap_or_default();
            stages.push(st);
        }
    }
    if stages.len() != members.len() {
        problems.push(crate::i18n::msg(
            "compile.members_missing_count",
            &[scope, &(members.len() - stages.len()).to_string()],
        ));
    }
    if !problems.is_empty() {
        return Err(WorkflowError { problems });
    }

    let mut vars = BTreeMap::new();
    if let Some(m) = entry["vars"].as_object() {
        for (k, v) in m {
            // null = 必填（无内联默认值）
            vars.insert(k.clone(), v.as_str().map(String::from));
        }
    }
    let mut constants = BTreeMap::new();
    if let Some(m) = entry["constants"].as_object() {
        for (k, v) in m {
            constants.insert(k.clone(), v.as_str().unwrap_or_default().to_string());
        }
    }
    let completion_template = entry["completion_template"].as_str().map(String::from);

    Ok(Workflow {
        id: scope.to_string(),
        name: scope.to_string(),
        path: PathBuf::new(),
        vars,
        constants,
        completion_template,
        stages,
    })
}

fn stage_from_graph(s: &Value, problems: &mut Vec<String>) -> Option<crate::workflow::Stage> {
    use serde_json::Value as V;
    let id = s["id"].as_str().unwrap_or_default().to_string();
    let gstr = |v: &V, k: &str| v[k].as_str().unwrap_or_default().to_string();
    let agents: Vec<crate::workflow::AgentRef> = s["agents"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|a| crate::workflow::AgentRef {
                    agent: gstr(a, "agent"),
                    skills: a["skills"]
                        .as_array()
                        .map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    inputs: a["inputs"]
                        .as_array()
                        .map(|x| {
                            x.iter()
                                .map(|i| crate::workflow::InputRef {
                                    artifact: i["artifact"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string(),
                                    required: i["required"].as_bool().unwrap_or(true),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    watches: a["watches"]
                        .as_array()
                        .map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    outputs: a["outputs"]
                        .as_array()
                        .map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    task_description: gstr(a, "task_description"),
                    vars: a["vars"]
                        .as_array()
                        .map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    if agents.is_empty() {
        problems.push(crate::i18n::msg("compile.stage_missing_agents", &[&id]));
        return None;
    }
    Some(crate::workflow::Stage {
        on_skip: s["on_skip"].as_str().unwrap_or("skip").to_string(),
        collaboration: gstr(s, "collaboration"),
        // 派生数据依赖边由 workflow_from_artifacts 从 scope-grid 的 depends 键回填
        depends_on: Vec::new(),
        agents,
        retry: crate::workflow::RetryPolicy {
            max_attempts: s["retry"]["max_attempts"].as_u64().unwrap_or(1),
            on_exhausted: s["retry"]["on_exhausted"]
                .as_str()
                .unwrap_or("abort_workflow")
                .to_string(),
        },
        approval: crate::workflow::ApprovalPolicy {
            required: s["approval"]["required"].as_bool().unwrap_or(false),
            gate: s["approval"]["gate"].as_str().unwrap_or_default().to_string(),
            reviewer: s["approval"]["reviewer"].as_str().map(String::from),
        },
        id,
    })
}

/// 字节确定序列化（键排序、2 空格缩进、尾换行）。
pub fn serialize_artifact(v: &Value) -> String {
    let mut text = serde_json::to_string_pretty(v).expect("json 序列化失败");
    text.push('\n');
    text
}
