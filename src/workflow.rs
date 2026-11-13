//! workflow.yaml 加载、结构校验、工件模板解析。
//!
//! 工件（artifact）在定义中只写名称模板（如 "{knowledge_point}-研究报告.md"），
//! Engine 在 start/next 时基于会话变量解析为完整相对路径。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_yaml::Value as Yaml;

use crate::paths;

pub const COLLABORATIONS: &[&str] = &["single", "sequential", "parallel"];
pub const ON_EXHAUSTED: &[&str] = &["abort_workflow", "continue"];

/// 引擎保留变量：机制内注入的名字，任何 workflow 无需声明即可在模板中使用。
/// SESSION_ID 是会话 ID（时间戳字符串，非路径）；TASK 是需求原文（start --task 提供，
/// 工作流以此承接自然语言输入，参数提取归流程内首个 LLM stage，仅在提供时注入）。
/// 业务变量由 workflow/scope 的 vars/constants 声明（见 check_template_vars）。
pub const RESERVED_VARS: &[&str] = &["SESSION_ID", "TASK"];

#[derive(Debug, Clone)]
pub struct WorkflowError {
    pub problems: Vec<String>,
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.problems.join("; "))
    }
}

/// 输入工件引用：required=false 为软输入（缺失容忍，不参与序列数据依赖校验）。
#[derive(Debug, Clone, PartialEq)]
pub struct InputRef {
    pub artifact: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct AgentRef {
    pub agent: String,
    pub skills: Vec<String>,
    /// 硬依赖输入（生产者必须是执行序列中的成员且排在消费者之前）
    pub inputs: Vec<InputRef>,
    /// 软依赖（阅读兴趣）：参与路径解析、不参与拓扑，缺失容忍；成员生产者排在其后即死接线（编译错）
    pub watches: Vec<String>,
    pub outputs: Vec<String>,
    pub task_description: String,
    pub vars: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u64,
    pub on_exhausted: String,
}

#[derive(Debug, Clone)]
pub struct ApprovalPolicy {
    pub required: bool,
    pub gate: String,
    /// M4a：一等评审者 agent（须可派发）。None = 人批（现状）。
    /// 决策仍以 approval report 落审计（granted/rejected 分片记录 reviewer）。
    pub reviewer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Stage {
    pub id: String,
    /// 硬输入生产者被跳过时的行为：skip（默认，本 stage 传递跳过）| run（照常派发）
    pub on_skip: String,
    pub collaboration: String,
    /// 派生数据依赖边：硬输入的、排在消费者之前的成员生产者（由
    /// `validate_and_derive_sequence` 推导，作者不声明——顺序唯一事实源是 scope 的
    /// include_stages 序列；解析层遇作者声明即报迁移错）
    pub depends_on: Vec<String>,
    pub agents: Vec<AgentRef>,
    pub retry: RetryPolicy,
    pub approval: ApprovalPolicy,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    /// 业务变量声明：名 → 内联默认值（None = 必填，start 必须提供）。
    pub vars: BTreeMap<String, Option<String>>,
    /// 常量：字面值，模板可引用，恒取定义值（--var 拒收）。
    pub constants: BTreeMap<String, String>,
    pub completion_template: Option<String>,
    pub stages: Vec<Stage>,
}

impl Workflow {
    pub fn stage(&self, stage_id: &str) -> Option<&Stage> {
        self.stages.iter().find(|s| s.id == stage_id)
    }

    pub fn stage_ids(&self) -> Vec<String> {
        self.stages.iter().map(|s| s.id.clone()).collect()
    }

    /// 全工作流引用的 agent 名（去重排序），供 check 输出。
    pub fn agents_referenced(&self) -> Vec<String> {
        let names: BTreeSet<String> = self
            .stages
            .iter()
            .flat_map(|s| s.agents.iter().map(|a| a.agent.clone()))
            .collect();
        let sorted: Vec<String> = names.into_iter().collect();
        sorted
    }

    /// 全工作流引用的 skill 名（去重排序），供 check 输出。
    pub fn skills_referenced(&self) -> Vec<String> {
        let names: BTreeSet<String> = self
            .stages
            .iter()
            .flat_map(|s| s.agents.iter().flat_map(|a| a.skills.iter().cloned()))
            .collect();
        let sorted: Vec<String> = names.into_iter().collect();
        sorted
    }
}

// ---------- YAML 访问助手 ----------

pub fn ystr(v: &Yaml) -> String {
    match v {
        Yaml::Null => String::new(),
        Yaml::String(s) => s.clone(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Number(n) => n.to_string(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn ymap<'a>(v: &'a Yaml) -> Option<&'a serde_yaml::Mapping> {
    v.as_mapping()
}

fn yget<'a>(m: &'a serde_yaml::Mapping, key: &str) -> Option<&'a Yaml> {
    m.get(&Yaml::String(key.into()))
}

fn py_list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("'{s}'")).collect();
    format!("[{}]", quoted.join(", "))
}

// ---------- 模板解析 ----------

/// 提取模板中的 `{var}` 变量名（\w 语义，Unicode 字母数字下划线）。
pub fn template_vars(tpl: &str) -> Vec<String> {
    let chars: Vec<char> = tpl.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            if j > i + 1 && j < chars.len() && chars[j] == '}' {
                out.push(chars[i + 1..j].iter().collect());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[derive(Debug, Clone)]
pub enum TplError {
    MissingVar(String),
    BadFormat(String),
}

/// Python str.format(**vars) 的常用子集：`{name}` 替换、`{{`/`}}` 转义；
/// 缺失变量报 MissingVar（对应 KeyError），其余为 BadFormat。
pub fn format_template(tpl: &str, vars: &BTreeMap<String, String>) -> Result<String, TplError> {
    let chars: Vec<char> = tpl.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if chars.get(i + 1) == Some(&'{') {
                out.push('{');
                i += 2;
                continue;
            }
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }
            if j >= chars.len() {
                return Err(TplError::BadFormat(
                    "Single '{' encountered in format string".into(),
                ));
            }
            let key: String = chars[i + 1..j].iter().collect();
            match vars.get(&key) {
                Some(v) => out.push_str(v),
                None => return Err(TplError::MissingVar(key)),
            }
            i = j + 1;
        } else if c == '}' {
            if chars.get(i + 1) == Some(&'}') {
                out.push('}');
                i += 2;
                continue;
            }
            return Err(TplError::BadFormat(
                "Single '}' encountered in format string".into(),
            ));
        } else {
            out.push(c);
            i += 1;
        }
    }
    Ok(out)
}

pub fn format_or_panic(tpl: &str, vars: &BTreeMap<String, String>) -> String {
    format_template(tpl, vars).expect("模板变量应已在 start 时校验闭合")
}

// ---------- completion_template（专用语法，独立于工件模板） ----------

/// completion_template 占位符词法：`{vars.<name>}` / `{outputs.<stage_id>}`，`{{`/`}}` 转义。
/// key 取至 `}` 全串匹配（stage_id 可含连字符，不受 \w 限制）；
/// 其余形式（无前缀、空 key、未闭合、孤立 `}`）一律非法。
pub fn completion_placeholders(tpl: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for token in tokenize_completion(tpl)? {
        if let CompletionToken::Placeholder { kind, key } = token {
            out.push((kind, key));
        }
    }
    Ok(out)
}

/// 渲染 completion_template。lookup(kind, key) 返回 None 即缺键（编译期校验保证不发生）。
pub fn render_completion(
    tpl: &str,
    lookup: &dyn Fn(&str, &str) -> Option<String>,
) -> Result<String, String> {
    let mut out = String::new();
    for token in tokenize_completion(tpl)? {
        match token {
            CompletionToken::Text(s) => out.push_str(&s),
            CompletionToken::Placeholder { kind, key } => {
                let Some(v) = lookup(&kind, &key) else {
                    return Err(crate::i18n::msg(
                        "workflow.completion_placeholder_unresolved",
                        &[&kind, &key],
                    ));
                };
                out.push_str(&v);
            }
        }
    }
    Ok(out)
}

enum CompletionToken {
    Text(String),
    Placeholder { kind: String, key: String },
}

/// completion_template 占位符校验：vars 须在声明集内，outputs 须为已知 stage。
/// 供内联 parse_workflow 与编译期 parse_scope_file 共用；`what` 用于错误定位（如 "workflow" / "scope 'lesson'"）。
pub fn validate_completion_placeholders(
    tpl: &str,
    declared_vars: &BTreeSet<String>,
    stage_ids: &BTreeSet<String>,
    what: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    match completion_placeholders(tpl) {
        Err(e) => problems.push(format!("{what}: {e}")),
        Ok(placeholders) => {
            let mut seen = BTreeSet::new();
            for (kind, key) in placeholders {
                if !seen.insert(format!("{kind}.{key}")) {
                    continue;
                }
                let ok = match kind.as_str() {
                    "vars" => declared_vars.contains(&key),
                    "outputs" => stage_ids.contains(&key),
                    _ => false,
                };
                if !ok {
                    if kind == "vars" {
                        problems.push(crate::i18n::msg(
                            "workflow.completion_undeclared_var",
                            &[what, &kind, &key, &key],
                        ));
                    } else {
                        problems.push(crate::i18n::msg(
                            "workflow.completion_undeclared_output",
                            &[what, &kind, &key, &key],
                        ));
                    }
                }
            }
        }
    }
    problems
}

fn tokenize_completion(tpl: &str) -> Result<Vec<CompletionToken>, String> {
    let chars: Vec<char> = tpl.chars().collect();
    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if chars.get(i + 1) == Some(&'{') {
                text.push('{');
                i += 2;
                continue;
            }
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }
            if j >= chars.len() {
                return Err(crate::i18n::t("workflow.completion_unclosed_brace"));
            }
            let inner: String = chars[i + 1..j].iter().collect();
            let (kind, key) = inner
                .split_once('.')
                .ok_or_else(|| crate::i18n::msg("workflow.completion_missing_prefix", &[&inner]))?;
            if kind != "vars" && kind != "outputs" {
                return Err(crate::i18n::msg("workflow.completion_bad_prefix", &[&inner]));
            }
            if key.is_empty() {
                return Err(crate::i18n::msg("workflow.completion_empty_key", &[&inner]));
            }
            if !text.is_empty() {
                tokens.push(CompletionToken::Text(std::mem::take(&mut text)));
            }
            tokens.push(CompletionToken::Placeholder {
                kind: kind.to_string(),
                key: key.to_string(),
            });
            i = j + 1;
        } else if c == '}' {
            if chars.get(i + 1) == Some(&'}') {
                text.push('}');
                i += 2;
                continue;
            }
            return Err(crate::i18n::t("workflow.completion_lone_brace"));
        } else {
            text.push(c);
            i += 1;
        }
    }
    if !text.is_empty() {
        tokens.push(CompletionToken::Text(text));
    }
    Ok(tokens)
}

// ---------- 定义解析 ----------

fn str_list(value: Option<&Yaml>, what: &str, problems: &mut Vec<String>) -> Vec<String> {
    let Some(v) = value else {
        return Vec::new();
    };
    if v.is_null() {
        return Vec::new();
    }
    match v.as_sequence() {
        Some(seq) if seq.iter().all(|i| i.is_string()) => {
            seq.iter().map(|i| i.as_str().unwrap().to_string()).collect()
        }
        _ => {
            problems.push(crate::i18n::msg("workflow.must_be_string_list", &[what]));
            Vec::new()
        }
    }
}

// ---------- 变量声明解析（vars/constants） ----------

/// 解析 vars 声明（mapping 形态）：名 → 内联默认值，空值 = 必填。列表形式已移除。
fn parse_vars_decl(
    raw: &serde_yaml::Mapping,
    prefix: &str,
    problems: &mut Vec<String>,
) -> BTreeMap<String, Option<String>> {
    let mut vars = BTreeMap::new();
    match yget(raw, "vars") {
        None | Some(Yaml::Null) => {}
        Some(Yaml::Mapping(m)) => {
            for (k, v) in m {
                let name = ystr(k);
                match v {
                    Yaml::Null => {
                        vars.insert(name, None);
                    }
                    Yaml::Bool(_) | Yaml::Number(_) | Yaml::String(_) => {
                        vars.insert(name, Some(ystr(v)));
                    }
                    _ => problems.push(crate::i18n::msg(
                        "workflow.vars_value_form",
                        &[prefix, &name],
                    )),
                }
            }
        }
        Some(_) => problems.push(crate::i18n::msg(
            "workflow.vars_must_be_mapping",
            &[prefix],
        )),
    }
    vars
}

/// 解析 constants 声明（mapping 形态）：名 → 字面值。模板可引用，恒取定义值，--var 拒收。
fn parse_constants_decl(
    raw: &serde_yaml::Mapping,
    prefix: &str,
    problems: &mut Vec<String>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    match yget(raw, "constants") {
        None | Some(Yaml::Null) => {}
        Some(Yaml::Mapping(m)) => {
            for (k, v) in m {
                let name = ystr(k);
                match v {
                    Yaml::Bool(_) | Yaml::Number(_) | Yaml::String(_) => {
                        out.insert(name, ystr(v));
                    }
                    _ => problems.push(crate::i18n::msg(
                        "workflow.constants_value_form",
                        &[prefix, &name],
                    )),
                }
            }
        }
        Some(_) => problems.push(crate::i18n::msg(
            "workflow.constants_must_be_mapping",
            &[prefix],
        )),
    }
    out
}

/// 变量声明统一入口：vars + constants；已移除字段（defaults/derived_vars/description_from_stage）误用给出迁移提示。
pub fn parse_var_decls(
    raw: &serde_yaml::Mapping,
    prefix: &str,
    problems: &mut Vec<String>,
) -> (
    BTreeMap<String, Option<String>>,
    BTreeMap<String, String>,
) {
    if yget(raw, "defaults").is_some() {
        problems.push(crate::i18n::msg("workflow.defaults_removed", &[prefix]));
    }
    if yget(raw, "derived_vars").is_some() {
        problems.push(crate::i18n::msg("workflow.derived_vars_removed", &[prefix]));
    }
    if yget(raw, "description_from_stage").is_some() {
        problems.push(crate::i18n::msg(
            "workflow.description_from_stage_removed",
            &[prefix],
        ));
    }
    (
        parse_vars_decl(raw, prefix, problems),
        parse_constants_decl(raw, prefix, problems),
    )
}

/// 变量名互斥校验：vars/constants 不重叠，且不得占用保留名。
pub fn validate_var_names(
    prefix: &str,
    vars: &BTreeMap<String, Option<String>>,
    constants: &BTreeMap<String, String>,
    problems: &mut Vec<String>,
) {
    for name in vars.keys().chain(constants.keys()) {
        if RESERVED_VARS.contains(&name.as_str()) {
            problems.push(crate::i18n::msg(
                "workflow.reserved_var_name",
                &[prefix, name],
            ));
        }
    }
    for name in vars.keys() {
        if constants.contains_key(name) {
            problems.push(crate::i18n::msg("workflow.var_name_conflict", &[prefix, name]));
        }
    }
}

fn parse_agent_ref(raw: &serde_yaml::Mapping, stage_id: &str, problems: &mut Vec<String>) -> Option<AgentRef> {
    let agent_v = yget(raw, "agent").filter(|v| !v.is_null());
    let agent = agent_v.map(ystr).unwrap_or_default();
    if agent.is_empty() {
        problems.push(crate::i18n::msg("workflow.agent_field_missing", &[stage_id]));
        return None;
    }
    let ref_prefix = format!("{stage_id}/{agent}");
    let skills = str_list(yget(raw, "skills"), &format!("{ref_prefix}.skills"), problems);
    if yget(raw, "dir").is_some() {
        problems.push(crate::i18n::msg("workflow.dir_removed", &[&ref_prefix]));
    }
    let mut inputs: Vec<InputRef> = Vec::new();
    match yget(raw, "inputs") {
        None | Some(Yaml::Null) => {}
        Some(v) => match v.as_sequence() {
            Some(seq) => {
                for item in seq {
                    match item {
                        Yaml::String(s) => inputs.push(InputRef {
                            artifact: s.clone(),
                            required: true,
                        }),
                        Yaml::Mapping(m) => {
                            let artifact = yget(m, "artifact").map(ystr).unwrap_or_default();
                            let required = yget(m, "required")
                                .and_then(|r| r.as_bool())
                                .unwrap_or(true);
                            if artifact.is_empty() {
                                problems.push(crate::i18n::msg(
                                    "workflow.inputs_entry_missing_artifact",
                                    &[&ref_prefix],
                                ));
                                continue;
                            }
                            inputs.push(InputRef { artifact, required });
                        }
                        other => problems.push(crate::i18n::msg(
                            "workflow.inputs_entry_form",
                            &[&ref_prefix, &ystr(other)],
                        )),
                    }
                }
            }
            None => problems.push(crate::i18n::msg(
                "workflow.inputs_must_be_list",
                &[&ref_prefix],
            )),
        },
    }
    let watches = str_list(yget(raw, "watches"), &format!("{ref_prefix}.watches"), problems);
    let outputs = str_list(yget(raw, "outputs"), &format!("{ref_prefix}.outputs"), problems);
    let task_description = yget(raw, "task_description").map(ystr).unwrap_or_default();
    let vars = str_list(yget(raw, "vars"), &format!("{ref_prefix}.vars"), problems);

    let r = AgentRef {
        agent,
        skills,
        inputs,
        watches,
        outputs,
        task_description,
        vars,
    };
    Some(r)
}

fn parse_stage(raw: &serde_yaml::Mapping, problems: &mut Vec<String>) -> Option<Stage> {
    let stage_id = yget(raw, "id").map(ystr).unwrap_or_default();
    if stage_id.is_empty() {
        problems.push(crate::i18n::t("workflow.stage_id_missing"));
        return None;
    }
    let where_ = format!("stage '{stage_id}'");

    let mut collab = yget(raw, "collaboration").map(ystr).unwrap_or_default();
    if !COLLABORATIONS.contains(&collab.as_str()) {
        problems.push(crate::i18n::msg(
            "workflow.collab_invalid",
            &[&where_, &py_list(&["parallel", "sequential", "single"].iter().map(|s| s.to_string()).collect::<Vec<_>>())],
        ));
        collab = "single".to_string();
    }

    let agents_raw = yget(raw, "agents");
    let agents_list = agents_raw.and_then(|v| v.as_sequence());
    let agents_list = match agents_list {
        Some(l) if !l.is_empty() => l,
        _ => {
            problems.push(crate::i18n::msg("workflow.agents_empty", &[&where_]));
            &Vec::new()
        }
    };
    let agents: Vec<AgentRef> = agents_list
        .iter()
        .filter_map(|r| {
            r.as_mapping()
                .and_then(|m| parse_agent_ref(m, &stage_id, problems))
        })
        .collect();
    if collab == "single" && agents.len() != 1 {
        problems.push(crate::i18n::msg("workflow.single_agent_count", &[&where_]));
    }
    if (collab == "sequential" || collab == "parallel") && agents.len() < 2 {
        problems.push(crate::i18n::msg("workflow.multi_agent_count", &[&where_, &collab]));
    }

    let retry_raw = yget(raw, "retry").and_then(|v| v.as_mapping());
    let retry = RetryPolicy {
        max_attempts: retry_raw
            .and_then(|m| yget(m, "max_attempts"))
            .map(|v| match v.as_u64() {
                Some(n) => n,
                None => ystr(v).parse::<u64>().unwrap_or(1),
            })
            .unwrap_or(1),
        on_exhausted: retry_raw
            .and_then(|m| yget(m, "on_exhausted"))
            .map(ystr)
            .unwrap_or_else(|| "abort_workflow".to_string()),
    };
    if retry.max_attempts < 1 {
        problems.push(crate::i18n::msg("workflow.max_attempts_invalid", &[&where_]));
    }
    if !ON_EXHAUSTED.contains(&retry.on_exhausted.as_str()) {
        problems.push(crate::i18n::msg(
            "workflow.on_exhausted_invalid",
            &[&where_, &py_list(&["abort_workflow", "continue"].iter().map(|s| s.to_string()).collect::<Vec<_>>())],
        ));
    }

    let approval_raw = yget(raw, "approval").and_then(|v| v.as_mapping());
    let approval = ApprovalPolicy {
        required: approval_raw
            .and_then(|m| yget(m, "required"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        gate: approval_raw
            .and_then(|m| yget(m, "gate"))
            .map(ystr)
            .unwrap_or_default(),
        reviewer: approval_raw
            .and_then(|m| yget(m, "reviewer"))
            .filter(|v| !v.is_null())
            .map(ystr),
    };
    if approval.required && approval.gate.is_empty() {
        problems.push(crate::i18n::msg("workflow.gate_required", &[&where_]));
    }
    if approval.reviewer.is_some() && !approval.required {
        problems.push(crate::i18n::msg(
            "workflow.reviewer_requires_required",
            &[&where_],
        ));
    }

    let on_skip = yget(raw, "on_skip").map(ystr).unwrap_or_default();
    if !on_skip.is_empty() && on_skip != "skip" && on_skip != "run" {
        problems.push(crate::i18n::msg(
            "workflow.on_skip_invalid",
            &[&where_, &on_skip],
        ));
    }
    if yget(raw, "depends_on").is_some() {
        problems.push(crate::i18n::msg("workflow.depends_on_removed", &[&where_]));
    }
    Some(Stage {
        id: stage_id,
        on_skip: if on_skip.is_empty() { "skip".to_string() } else { on_skip },
        collaboration: collab,
        depends_on: Vec::new(),
        agents,
        retry,
        approval,
    })
}

pub fn parse_workflow(data: &Yaml, path: &Path) -> Result<Workflow, WorkflowError> {
    let mut problems: Vec<String> = Vec::new();
    let wf_raw = ymap(data)
        .and_then(|m| yget(m, "workflow"))
        .and_then(ymap)
        .ok_or_else(|| WorkflowError {
            problems: vec![crate::i18n::t("workflow.top_level_workflow_missing")],
        })?;

    let wf_id = yget(wf_raw, "id").map(ystr).unwrap_or_default();
    if wf_id.is_empty() {
        problems.push(crate::i18n::t("workflow.id_missing"));
    }
    let wf_id = if wf_id.is_empty() { "wf-unknown".to_string() } else { wf_id };

    let stages_list = yget(wf_raw, "stages").and_then(|v| v.as_sequence());
    let stages_list = match stages_list {
        Some(l) if !l.is_empty() => l,
        _ => {
            problems.push(crate::i18n::t("workflow.stages_empty"));
            &Vec::new()
        }
    };
    let stages: Vec<Stage> = stages_list
        .iter()
        .filter_map(|r| r.as_mapping().and_then(|m| parse_stage(m, &mut problems)))
        .collect();

    // id 唯一（执行顺序 = 声明序列，无依赖图故无环检测）
    let ids: Vec<String> = stages.iter().map(|s| s.id.clone()).collect();
    let known: BTreeSet<String> = ids.iter().cloned().collect();
    let mut dupes: BTreeSet<String> = BTreeSet::new();
    for id in &ids {
        if ids.iter().filter(|i| *i == id).count() > 1 {
            dupes.insert(id.clone());
        }
    }
    if !dupes.is_empty() {
        problems.push(crate::i18n::msg(
            "workflow.stage_id_duplicated",
            &[&py_list(&dupes.into_iter().collect::<Vec<_>>())],
        ));
    }

    let completion_template = yget(wf_raw, "completion_template")
        .filter(|v| !v.is_null())
        .map(ystr);
    let (vars, constants) = parse_var_decls(wf_raw, "workflow", &mut problems);

    let wf = Workflow {
        id: wf_id.clone(),
        name: yget(wf_raw, "name").map(ystr).unwrap_or(wf_id),
        path: path.to_path_buf(),
        vars,
        constants,
        completion_template,
        stages,
    };
    validate_var_names("workflow", &wf.vars, &wf.constants, &mut problems);

    if let Some(tpl) = &wf.completion_template {
        let declared: BTreeSet<String> = wf
            .vars
            .keys()
            .chain(wf.constants.keys())
            .cloned()
            .collect();
        problems.extend(validate_completion_placeholders(tpl, &declared, &known, "workflow"));
    }


    // 序列校验（数据依赖：生产者必须是成员且排在消费者之前；软输入死接线拦截）
    // 与派生数据依赖边由 `validate_and_derive_sequence` 完成——内联在 load_workflow 调用，
    // scope 模式在编译期按各 scope 的 include_stages 序列调用。

    if !problems.is_empty() {
        return Err(WorkflowError { problems });
    }
    Ok(wf)
}

pub fn load_workflow(path: &Path) -> Result<Workflow, WorkflowError> {
    if !path.is_file() {
        return Err(WorkflowError {
            problems: vec![crate::i18n::msg(
                "workflow.definition_not_found",
                &[&paths::to_posix(path)],
            )],
        });
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| WorkflowError { problems: vec![crate::i18n::msg("workflow.yaml_parse_failed", &[&e.to_string()])] })?;
    let data: Yaml = serde_yaml::from_str(&text)
        .map_err(|e| WorkflowError { problems: vec![crate::i18n::msg("workflow.yaml_parse_failed", &[&e.to_string()])] })?;
    let mut wf = parse_workflow(&data, path)?;
    // 内联工作流：声明序列即执行序——序列校验 + 派生数据依赖边
    let order = wf.stage_ids();
    let edges = validate_and_derive_sequence(&wf.stages, &order, "workflow")
        .map_err(|problems| WorkflowError { problems })?;
    apply_depends_edges(&mut wf.stages, &edges);
    let problems = check_template_vars(&wf, &BTreeSet::new());
    if !problems.is_empty() {
        return Err(WorkflowError { problems });
    }
    Ok(wf)
}

/// 模板变量闭合校验：变量 ∈ RESERVED_VARS ∪ workflow 自身声明集 ∪ extra。
/// scope 模式编译期传全量 scope 声明并集（stage 库先于认领合并，并集是编译期最强可行近似，
/// 认领级精确闭合由 start 期 resolve_vars/check_templates 兜底）；内联路径 extra 传空集。
pub fn check_template_vars(wf: &Workflow, extra: &BTreeSet<String>) -> Vec<String> {
    let mut declared: BTreeSet<String> = RESERVED_VARS.iter().map(|s| s.to_string()).collect();
    declared.extend(wf.vars.keys().cloned());
    declared.extend(wf.constants.keys().cloned());
    declared.extend(extra.iter().cloned());
    let mut problems = Vec::new();
    for s in &wf.stages {
        for a in &s.agents {
            let ref_prefix = format!("stage '{}'/{}", s.id, a.agent);
            let tpls = a
                .inputs
                .iter()
                .map(|i| &i.artifact)
                .chain(a.outputs.iter())
                .chain(a.watches.iter())
                .chain(std::iter::once(&a.task_description));
            for tpl in tpls {
                for var in template_vars(tpl) {
                    if !declared.contains(&var) {
                        problems.push(crate::i18n::msg(
                            "workflow.unknown_template_var",
                            &[&ref_prefix, tpl, &var],
                        ));
                    }
                }
            }
            for v in &a.vars {
                if !declared.contains(v) {
                    problems.push(crate::i18n::msg("workflow.unknown_action_var", &[v]));
                }
            }
        }
    }
    problems
}

/// 序列校验 + 数据依赖边推导（scope 顺序化模型：顺序即执行序）。
///
/// `library` 为 stage 全库（不限成员），`order` 为执行序列（成员 id 按序）。
/// 对序列中每个成员的每个 agent 校验：
/// - **硬输入**：必须有 outputs 模板精确匹配的生产者；唯一生产者是自己 = 自产自销；
///   生产者必须是成员且至少一个排在消费者之前（否则运行期文件永不就绪）；
/// - **软输入/watches 死接线**：存在成员生产者但无一排在消费者之前——线性串行下本会话
///   必然永远缺席，区别于两类容忍缺席（生产者 skipped、生产者非成员=按需裁剪），编译错拦截；
/// - 非成员生产者不参检（轻流程裁剪设计链的合法形态）。
///
/// 派生：stage 的数据依赖边 = 其硬输入的、排在消费者之前的成员生产者（直接生产者；
/// 多级就绪由 router 的 deps_terminal 逐级传递）。作者声明 depends_on 已移除
/// （解析层报迁移错），本函数返回值是 `depends_on` 字段的唯一来源。
pub fn validate_and_derive_sequence(
    library: &[Stage],
    order: &[String],
    label: &str,
) -> Result<BTreeMap<String, Vec<String>>, Vec<String>> {
    let mut problems: Vec<String> = Vec::new();
    let members: BTreeSet<String> = order.iter().cloned().collect();
    let pos: BTreeMap<String, usize> =
        order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
    let by_id: BTreeMap<String, &Stage> =
        library.iter().map(|s| (s.id.clone(), s)).collect();
    let mut producers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for s in library {
        for a in &s.agents {
            for tpl in &a.outputs {
                producers.entry(tpl.clone()).or_default().insert(s.id.clone());
            }
        }
    }
    let member_producer_list = |tpl: &str| -> Vec<String> {
        producers
            .get(tpl)
            .map(|ps| {
                ps.iter()
                    .filter(|p| members.contains(*p))
                    .map(|p| (*p).clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    let all_producer_list = |tpl: &str| -> Vec<String> {
        producers
            .get(tpl)
            .map(|ps| ps.iter().map(|p| format!("'{p}'")).collect())
            .unwrap_or_default()
    };

    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for sid in order {
        let Some(stage) = by_id.get(sid) else {
            problems.push(crate::i18n::msg("workflow.sequence_unknown_stage", &[label, sid]));
            continue;
        };
        let self_pos = pos[sid];
        for a in &stage.agents {
            let ref_prefix = format!("stage '{sid}'/{}", a.agent);
            for inp in &a.inputs {
                let tpl = inp.artifact.as_str();
                let Some(prod_stages) = producers.get(tpl) else {
                    if inp.required {
                        problems.push(crate::i18n::msg(
                            "workflow.hard_input_no_producer",
                            &[label, &ref_prefix, tpl],
                        ));
                    }
                    continue;
                };
                if !inp.required {
                    // 死接线：软输入的其他成员生产者无一排在消费者之前。
                    // 自身 stage 产出的软引用是 stage 内 sequential 交接（合法），豁免。
                    let member_prods: Vec<String> = member_producer_list(tpl)
                        .into_iter()
                        .filter(|p| *p != *sid)
                        .collect();
                    let has_earlier = member_prods.iter().any(|p| pos[p] < self_pos);
                    if !member_prods.is_empty() && !has_earlier {
                        let plist = member_prods
                            .iter()
                            .map(|p| format!("'{p}'"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        problems.push(crate::i18n::msg(
                            "workflow.soft_input_dead_wiring",
                            &[label, &ref_prefix, tpl, &plist],
                        ));
                    }
                    continue;
                }
                if prod_stages.len() == 1 && prod_stages.contains(sid) {
                    problems.push(crate::i18n::msg(
                        "workflow.input_self_produced",
                        &[label, &ref_prefix, tpl],
                    ));
                    continue;
                }
                let member_prods = member_producer_list(tpl);
                if member_prods.is_empty() {
                    let plist = all_producer_list(tpl).join(", ");
                    problems.push(crate::i18n::msg(
                        "workflow.hard_input_producer_not_member",
                        &[label, &ref_prefix, tpl, &plist],
                    ));
                    continue;
                }
                let earlier: Vec<String> = member_prods
                    .iter()
                    .filter(|p| pos[*p] < self_pos)
                    .cloned()
                    .collect();
                if earlier.is_empty() {
                    let plist = member_prods
                        .iter()
                        .map(|p| format!("'{p}'"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    problems.push(crate::i18n::msg(
                        "workflow.hard_input_producer_later",
                        &[label, &ref_prefix, tpl, &plist, sid],
                    ));
                    continue;
                }
                edges.entry(sid.clone()).or_default().extend(earlier);
            }
            // 死接线检查（watches 与软输入同规则，豁免自身 stage 的 stage 内交接；只检不产边）
            for tpl in &a.watches {
                let member_prods: Vec<String> = member_producer_list(tpl)
                    .into_iter()
                    .filter(|p| *p != *sid)
                    .collect();
                let has_earlier = member_prods.iter().any(|p| pos[p] < self_pos);
                if !member_prods.is_empty() && !has_earlier {
                    let plist = member_prods
                        .iter()
                        .map(|p| format!("'{p}'"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    problems.push(crate::i18n::msg(
                        "workflow.watches_dead_wiring",
                        &[label, &ref_prefix, tpl, &plist],
                    ));
                }
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(edges
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect::<Vec<_>>()))
        .collect())
}

/// 把派生数据依赖边写回 stages（按 stage id 对号入座）。
pub fn apply_depends_edges(stages: &mut [Stage], edges: &BTreeMap<String, Vec<String>>) {
    for s in stages.iter_mut() {
        if let Some(e) = edges.get(&s.id) {
            s.depends_on = e.clone();
        }
    }
}

/// 用具体变量试格式化所有模板，返回问题清单。
pub fn check_templates(workflow: &Workflow, vars: &BTreeMap<String, String>) -> Vec<String> {
    let mut problems = Vec::new();
    for s in &workflow.stages {
        for a in &s.agents {
            let templates: Vec<&String> = a
                .inputs
                .iter()
                .map(|i| &i.artifact)
                .chain(a.outputs.iter())
                .chain(a.watches.iter())
                .chain(std::iter::once(&a.task_description))
                .collect();
            for tpl in templates {
                match format_template(tpl, vars) {
                    Ok(_) => {}
                    Err(TplError::MissingVar(k)) => {
                        problems.push(crate::i18n::msg(
                            "workflow.template_missing_var",
                            &[tpl, &k],
                        ));
                    }
                    Err(TplError::BadFormat(e)) => {
                        problems.push(crate::i18n::msg(
                            "workflow.template_bad_format",
                            &[tpl, &e],
                        ));
                    }
                }
            }
        }
    }
    problems
}

pub fn resolve_vars(
    workflow: &Workflow,
    provided: &BTreeMap<String, String>,
    session_id: &str,
    task: &str,
) -> Result<BTreeMap<String, String>, WorkflowError> {
    let missing: Vec<String> = workflow
        .vars
        .iter()
        .filter(|(k, default)| default.is_none() && !provided.contains_key(*k))
        .map(|(k, _)| k.clone())
        .collect();
    if !missing.is_empty() {
        return Err(WorkflowError {
            problems: vec![crate::i18n::msg(
                "workflow.missing_required_vars",
                &[&py_list(&missing)],
            )],
        });
    }
    let overridden: Vec<String> = provided
        .keys()
        .filter(|k| workflow.constants.contains_key(*k))
        .cloned()
        .collect();
    if !overridden.is_empty() {
        return Err(WorkflowError {
            problems: vec![crate::i18n::msg(
                "workflow.constants_not_overridable",
                &[&py_list(&overridden)],
            )],
        });
    }
    let unknown: Vec<String> = provided
        .keys()
        .filter(|k| !workflow.vars.contains_key(*k))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(WorkflowError {
            problems: vec![crate::i18n::msg("workflow.unknown_vars", &[&py_list(&unknown)])],
        });
    }
    // 合成顺序：常量 → vars 内联默认 → --var 提供值 → 保留变量
    let mut vars_ = workflow.constants.clone();
    for (k, default) in &workflow.vars {
        if let Some(d) = default {
            vars_.insert(k.clone(), d.clone());
        }
    }
    vars_.extend(provided.clone());
    vars_.insert("SESSION_ID".to_string(), session_id.to_string());
    if !task.is_empty() {
        vars_.insert("TASK".to_string(), task.to_string());
    }
    Ok(vars_)
}

/// 渲染路径模板：变量替换后，仅对最后一段（文件名）做非法字符清洗；
/// 目录段原样保留（可含 / 分隔符），支持 dmaf-output/{SESSION_ID}/reports/x.md 式完整路径。
pub fn render_path(tpl: &str, vars: &BTreeMap<String, String>) -> String {
    let rendered = format_or_panic(tpl, vars);
    match rendered.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/{}", paths::sanitize_filename(file)),
        None => paths::sanitize_filename(&rendered),
    }
}

/// 全工作流的工件索引：渲染后的完整路径 → 自身（生产者-消费者按路径精确匹配）。
pub fn artifact_index(
    workflow: &Workflow,
    vars: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut index = BTreeMap::new();
    for s in &workflow.stages {
        for a in &s.agents {
            for tpl in &a.outputs {
                let path = render_path(tpl, vars);
                index.insert(path.clone(), path);
            }
        }
    }
    index
}

/// 解析某个 agent 的输入/输出为渲染后的完整路径。路径即工件标识：
/// inputs 渲染结果与某 outputs 渲染结果相同即为生产者-消费者；无生产者 = 外部产物，路径原样使用。
#[allow(clippy::too_many_arguments)]
/// 单个输入的解析结果：定义 + 项目根相对路径。
#[derive(Debug, Clone)]
pub struct ResolvedInput {
    pub artifact: String,
    pub required: bool,
    pub path: String,
}

pub struct ResolvedAgentPaths {
    pub inputs: Vec<ResolvedInput>,
    pub watches: Vec<String>,
    pub outputs: Vec<String>,
}

pub fn resolve_agent_paths(
    _workflow: &Workflow,
    _stage: &Stage,
    r: &AgentRef,
    vars: &BTreeMap<String, String>,
    artifacts: &BTreeMap<String, String>,
) -> ResolvedAgentPaths {
    let outputs: Vec<String> = r.outputs.iter().map(|tpl| render_path(tpl, vars)).collect();
    let mut inputs = Vec::new();
    for inp in &r.inputs {
        let key = format_or_panic(&inp.artifact, vars);
        let path = artifacts.get(&key).cloned().unwrap_or_else(|| render_path(&inp.artifact, vars));
        inputs.push(ResolvedInput {
            artifact: key,
            required: inp.required,
            path,
        });
    }
    let watches: Vec<String> = r.watches.iter().map(|tpl| render_path(tpl, vars)).collect();
    ResolvedAgentPaths { inputs, watches, outputs }
}
