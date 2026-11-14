//! 项目清单 `dmaf/manifest.yaml`：工作流包的身份与兼容性声明。
//!
//! 消费方：`dmaf new`（模板内嵌样例）与 `dmaf doctor`（engine 范围判定，可选文件缺失 = 正常态）；
//! `dependencies` 段为将来的 dmaf add/push 预留。

use std::path::Path;

use serde_yaml::Value as Yaml;

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub engine: String,
    pub harnesses: Vec<String>,
}

/// 读取项目根的 `dmaf/manifest.yaml`。`Ok(None)` = 未配置（正常态）。
pub fn load(project_root: &Path) -> Result<Option<Manifest>, String> {
    let path = project_root.join("dmaf").join("manifest.yaml");
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| crate::i18n::msg("manifest.read_fail", &[&crate::paths::to_posix(&path), &e.to_string()]))?;
    let value: Yaml = serde_yaml::from_str(&text)
        .map_err(|e| crate::i18n::msg("manifest.invalid_yaml", &[&e.to_string()]))?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| crate::i18n::t("manifest.top_not_mapping"))?;
    let s = |k: &str| mapping.get(&Yaml::String(k.into())).map(crate::workflow::ystr);
    Ok(Some(Manifest {
        name: s("name").unwrap_or_default(),
        version: s("version").unwrap_or_default(),
        description: s("description").unwrap_or_default(),
        engine: s("engine").unwrap_or_default(),
        harnesses: mapping
            .get(&Yaml::String("harnesses".into()))
            .and_then(|v| v.as_sequence())
            .map(|seq| seq.iter().map(crate::workflow::ystr).collect())
            .unwrap_or_default(),
    }))
}

/// 极简引擎版本范围判定：逗号分隔的比较子（`=`、`>`、`>=`、`<`、`<=`，缺省 `=`）
/// + `MAJOR.MINOR[.PATCH]`（缺省补丁位按 0），全部满足才算兼容。
/// 例：`">=0.3,<0.4"`、`"0.3"`、`"=0.3.0"`。
pub fn engine_matches(req: &str, current: &str) -> Result<bool, String> {
    let cur = parse_version(current)
        .map_err(|e| crate::i18n::msg("manifest.current_version_unparsable", &[current, &e]))?;
    let mut any = false;
    for term in req.split(',') {
        let term = term.trim();
        if term.is_empty() {
            continue;
        }
        any = true;
        let (op, rest) = if let Some(r) = term.strip_prefix(">=") {
            (Op::Ge, r)
        } else if let Some(r) = term.strip_prefix("<=") {
            (Op::Le, r)
        } else if let Some(r) = term.strip_prefix('>') {
            (Op::Gt, r)
        } else if let Some(r) = term.strip_prefix('<') {
            (Op::Lt, r)
        } else if let Some(r) = term.strip_prefix('=') {
            (Op::Eq, r)
        } else {
            (Op::Eq, term)
        };
        let v = parse_version(rest.trim())
            .map_err(|e| crate::i18n::msg("manifest.range_term_unparsable", &[term, &e]))?;
        let ok = match op {
            Op::Eq => cur == v,
            Op::Gt => cur > v,
            Op::Ge => cur >= v,
            Op::Lt => cur < v,
            Op::Le => cur <= v,
        };
        if !ok {
            return Ok(false);
        }
    }
    if !any {
        return Err(crate::i18n::t("manifest.range_empty"));
    }
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u64, u64, u64);

#[derive(Clone, Copy)]
enum Op {
    Eq,
    Gt,
    Ge,
    Lt,
    Le,
}

fn parse_version(s: &str) -> Result<Version, String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|p| p.is_empty()) {
        return Err(crate::i18n::msg("manifest.invalid_version", &[s]));
    }
    let mut nums = [0u64; 3];
    for (i, p) in parts.iter().enumerate() {
        nums[i] = p
            .parse::<u64>()
            .map_err(|_| crate::i18n::msg("manifest.segment_not_int", &[p]))?;
    }
    Ok(Version(nums[0], nums[1], nums[2]))
}
