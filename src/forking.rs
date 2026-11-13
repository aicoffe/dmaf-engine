//! fork：从既有会话的审计 checkpoint 分叉出新会话。
//!
//! 语义：
//! - 源会话只读、不可变；
//! - 新会话复制审计分片前缀（seq 1..N，字节原样，哈希链完整延续），
//!   并追加一条 `workflow.forked` 收尾分片：记录来源（from_session/from_seq），
//!   并把回放得到的 session_dir 校正为新会话（源分片里的 started 载荷指向旧会话）；
//! - 已完成 action 的产出物从源会话桶复制到新会话桶：`SESSION_ID` 由 router::effective_vars
//!   活绑定为当前会话 ID，分叉会话的产物路径随 session_dir 切到新桶，两条分支互不覆盖；
//!   不含 `{SESSION_ID}` 的路径两侧同址（会话无关产物，分支共享），只做存在性确认；
//!   未完成阶段在新会话重新生成；
//! - 分叉点状态不允许存在在途（dispatched）action——checkpoint 必须落在动作全部
//!   终结的边界上。

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};

use crate::audit::{verify_chain, AuditTrail};
use crate::paths;
use crate::registry::Registry;
use crate::router::{effective_vars, session_id_of};
use crate::state::{self, StateError};
use crate::workflow::{artifact_index, resolve_agent_paths, Workflow};

pub struct ForkConflict(pub String);

#[allow(clippy::too_many_arguments)]
pub fn fork_session(
    workflow: &Workflow,
    _source_session_dir: &Path,
    source_audit_dir: &Path,
    seq: u64,
    output_root: &Path,
    now: DateTime<FixedOffset>,
    _registry: Option<&Registry>,
    scope: &str,
) -> Result<Value, ForkConflict> {
    let audit_src = AuditTrail::new(source_audit_dir)
        .map_err(|e| ForkConflict(crate::i18n::msg("forking.read_audit_fail", &[&e.to_string()])))?;
    let shards = audit_src.load_all();
    let prefix: Vec<Value> = shards
        .iter()
        .filter(|s| s["seq"].as_u64().map(|q| q <= seq).unwrap_or(false))
        .cloned()
        .collect();
    if prefix.is_empty() || prefix.last().unwrap()["seq"].as_u64() != Some(seq) {
        let max_seq = shards.last().map(|s| s["seq"].as_u64().unwrap_or(0)).unwrap_or(0);
        return Err(ForkConflict(crate::i18n::msg(
            "forking.shard_not_found",
            &[&seq.to_string(), &max_seq.to_string()],
        )));
    }
    let chain_problems = verify_chain(&prefix);
    if !chain_problems.is_empty() {
        return Err(ForkConflict(crate::i18n::msg(
            "forking.chain_incomplete",
            &[&seq.to_string(), &format!("{chain_problems:?}")],
        )));
    }

    let replayed = state::replay(&prefix).map_err(|e: StateError| {
        ForkConflict(crate::i18n::msg("forking.replay_fail", &[&e.to_string()]))
    })?;
    let in_flight: Vec<(String, String)> = replayed
        .stages
        .iter()
        .flat_map(|st| {
            st.actions
                .iter()
                .filter(|a| a.status == "dispatched")
                .map(move |a| (st.id.clone(), a.action_id.clone()))
        })
        .collect();
    if !in_flight.is_empty() {
        return Err(ForkConflict(crate::i18n::msg(
            "forking.in_flight_actions",
            &[&format!("{in_flight:?}")],
        )));
    }

    // 新会话目录
    let new_dir =
        paths::session_dir_for(output_root, &paths::new_session_id(Some(&now)));
    std::fs::create_dir_all(new_dir.join("audit")).map_err(|e| {
        ForkConflict(crate::i18n::msg("forking.mkdir_session_fail", &[&e.to_string()]))
    })?;

    // 复制审计分片前缀（字节原样）
    for s in &prefix {
        let wf = s["workflow_id"].as_str().unwrap_or_default();
        let q = s["seq"].as_u64().unwrap_or(0);
        let name = format!("shard-{wf}-{q:06}.json");
        std::fs::copy(source_audit_dir.join(&name), new_dir.join("audit").join(&name)).map_err(
            |e| {
                ForkConflict(crate::i18n::msg("forking.copy_shard_fail", &[&e.to_string()]))
            },
        )?;
    }

    // workflow.forked 收尾分片
    let audit_new = AuditTrail::new(&new_dir.join("audit")).map_err(|e| {
        ForkConflict(crate::i18n::msg("forking.mkdir_audit_fail", &[&e.to_string()]))
    })?;
    let hash_at_checkpoint = state::canonical_hash(&replayed);
    let forked_shard = audit_new
        .append(
            &workflow.id,
            "workflow.forked",
            None,
            None,
            json!({
                "from_session": replayed.session_dir,
                "from_seq": seq,
                "to_session_dir": paths::to_posix(&new_dir),
            }),
            &hash_at_checkpoint,
            &hash_at_checkpoint,
            &paths::format_ts(&now),
        )
        .map_err(|e| ForkConflict(e))?;
    crate::hooks::notify(&forked_shard, &paths::to_posix(&new_dir));

    // 状态文件 = 新会话全部分片的回放结果（与 verify-audit 同一代码路径）
    let mut final_state = state::replay(&audit_new.load_all()).map_err(|e: StateError| {
        ForkConflict(crate::i18n::msg("forking.replay_fail", &[&e.to_string()]))
    })?;
    // 回放不含 scope（非路由字段）；源会话为 scope 模式时延续到新会话
    if !scope.is_empty() {
        final_state.scope = scope.to_string();
        // fork 继承源会话 locale（load_state 已把进程 i18n 设为源会话 locale）
        final_state.locale = crate::i18n::current().as_str().to_string();
    }
    // 产物快照：SESSION_ID 活绑定——源会话按源 ID 渲染、新会话按新 ID 渲染，
    // 分叉点前已完成 action 的产物从源桶复制到新桶，两条分支从此各写各桶。
    // 不含 {SESSION_ID} 的路径两侧同址（会话无关产物，分支天然共享），只做存在性确认。
    let src_vars: BTreeMap<String, String> = effective_vars(&replayed);
    let dst_vars: BTreeMap<String, String> = effective_vars(&final_state);
    let src_index = artifact_index(workflow, &src_vars);
    let dst_index = artifact_index(workflow, &dst_vars);
    let mut copied: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for st in &replayed.stages {
        let wf_stage = workflow.stage(&st.id).expect("状态中的 stage 必在定义内");
        for a in &st.actions {
            if a.status != "success" {
                continue;
            }
            let r = wf_stage
                .agents
                .iter()
                .find(|r| r.agent == a.agent)
                .expect("action 的 agent 必在定义内");
            let src_paths =
                resolve_agent_paths(workflow, wf_stage, r, &src_vars, &src_index).outputs;
            let dst_paths =
                resolve_agent_paths(workflow, wf_stage, r, &dst_vars, &dst_index).outputs;
            for (src, dst) in src_paths.iter().zip(dst_paths.iter()) {
                if seen.contains(dst) {
                    continue;
                }
                seen.push(dst.clone());
                if !Path::new(src.as_str()).is_file() {
                    missing.push(dst.clone());
                    continue;
                }
                if src != dst {
                    if let Some(parent) = Path::new(dst.as_str()).parent() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            ForkConflict(crate::i18n::msg(
                                "forking.mkdir_artifact_fail",
                                &[&paths::to_posix(parent), &e.to_string()],
                            ))
                        })?;
                    }
                    std::fs::copy(src, dst).map_err(|e| {
                        ForkConflict(crate::i18n::msg(
                            "forking.copy_artifact_fail",
                            &[src, dst, &e.to_string()],
                        ))
                    })?;
                }
                copied.push(dst.clone());
            }
        }
    }

    let state_path = new_dir.join("dmaf-state.md");
    state::save(&mut final_state, &state_path, &paths::format_ts(&now)).map_err(|e| {
        ForkConflict(crate::i18n::msg("forking.write_state_fail", &[&e.to_string()]))
    })?;

    Ok(json!({
        "ok": true,
        "source_session": replayed.session_dir,
        "from_seq": seq,
        "session_dir": paths::to_posix(&new_dir),
        "session_id": session_id_of(&final_state),
        "state_path": paths::to_posix(&state_path),
        "status": final_state.status,
        "shards": prefix.len() + 1,
        "copied_artifacts": copied,
        "missing_artifacts": missing,
    }))
}
