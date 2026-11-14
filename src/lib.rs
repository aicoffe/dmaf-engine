//! D-MAF Engine —— 确定性 multi-agent 路由权威（Rust 实现）。
//!
//! Engine 是唯一路由权威：相同的状态文件 + 工作流定义，永远产生相同的下一跳指令。
//! LLM（Orchestrator）只在 Engine 指定的单个动作内发挥判断力。

pub mod audit;
pub mod cli;
pub mod compile;
pub mod forking;
pub mod hooks;
pub mod i18n;
pub mod install;
pub mod manifest;
pub mod market;
pub mod package;
pub mod paths;
pub mod registry;
pub mod reports;
pub mod router;
pub mod scaffold;
pub mod schema_check;
pub mod state;
pub mod workflow;

use serde_json::Value;

pub use cli::run;

/// 一次命令执行的完整输出（stdout/stderr/退出码）。
#[derive(Debug, Clone)]
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// 命令失败类型 → 退出码：2 用法 / 3 schema / 4 状态冲突 / 5 IO。
#[derive(Debug)]
pub enum Fail {
    Usage(String),
    Schema { message: String, problems: Vec<String> },
    Conflict(String),
    Io(String),
}

impl Fail {
    pub fn to_outcome(&self) -> Outcome {
        match self {
            Fail::Usage(msg) => cli::fail_outcome(2, msg, &[]),
            Fail::Schema { message, problems } => {
                cli::fail_outcome(3, message, problems)
            }
            Fail::Conflict(msg) => cli::fail_outcome(4, msg, &[]),
            Fail::Io(msg) => cli::fail_outcome(5, msg, &[]),
        }
    }
}

impl From<std::io::Error> for Fail {
    fn from(e: std::io::Error) -> Self {
        Fail::Io(e.to_string())
    }
}

pub type CmdResult = Result<Outcome, Fail>;

/// 紧凑 JSON 文本（供结果 JSON 组装）。
pub fn json_text(v: &Value) -> String {
    serde_json::to_string(v).expect("json 序列化失败")
}

/// Python 风格列表 repr：['a', 'b']（错误消息保持与原实现一致）。
pub fn py_list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("'{s}'")).collect();
    format!("[{}]", quoted.join(", "))
}
