//! 人面文案目录：按源文件/文案域分组（key 约定 `<域>.<语义>`），组由对应迁移批次维护；
//! zh 必须与原文案逐字节一致（默认输出不变的硬约束），en 按术语表
//! docs-dev/i18n-glossary.md 翻译。占位符统一 `{}` 顺序。

mod cli;
mod engine;
mod help;
mod market;
mod misc;

/// 全部目录组的聚合查找表（线性扫描；文案量级 ~300，无需索引）。
pub static ALL: &[&[(&str, &str, &str)]] = &[&cli::MSGS, &market::MSGS, &engine::MSGS, &misc::MSGS, &help::MSGS];
