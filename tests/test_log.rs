//! log 子命令：审计时间线渲染与过滤。（对应 test_log.py）

mod common;

use std::path::PathBuf;

use common::Driver;
use serde_json::json;

fn shard_files(session: &str) -> Vec<PathBuf> {
    let audit_dir = PathBuf::from(session).join("audit");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&audit_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|f| f.to_string_lossy().starts_with("shard-"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn log_renders_timeline() {
    let driver = Driver::with_tag("log-basic");
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    common::drive_to_after_pptx(&driver, &session);
    let (rc, out, _) = driver.run(&common::svec(&["log", "--session", &session]));
    assert_eq!(rc, 0);
    assert!(out.contains("workflow.started"));
    assert!(out.contains("讲解「大语言模型」"));
    assert!(out.contains("派发") && out.contains("lesson-researcher"));
    assert!(out.contains("审批通过") && out.contains("outline-review"));
    assert!(out.contains("成功"));
    // 时间线按 seq 升序
    let seqs: Vec<u64> = out
        .lines()
        .filter(|l| l.starts_with('#'))
        .map(|l| l[1..7].parse().unwrap())
        .collect();
    let mut sorted = seqs.clone();
    sorted.sort();
    assert_eq!(seqs, sorted);
    assert!(seqs.len() >= 8);
}

#[test]
fn log_type_filter() {
    let driver = Driver::with_tag("log-type");
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    common::drive_to_after_pptx(&driver, &session);
    let (rc, out, _) = driver.run(&common::svec(&[
        "log", "--session", &session, "--type", "action.reported",
    ]));
    assert_eq!(rc, 0);
    assert!(out.contains("action.reported"));
    assert!(!out.contains("workflow.started") && !out.contains("action.dispatched"));
    let count = if out.starts_with('#') { 1 } else { 0 } + out.matches("\n#").count();
    assert_eq!(count, 4); // intent/research/outline/pptx 四次回报

    let (rc, _, _) = driver.run(&common::svec(&[
        "log", "--session", &session, "--type", "bogus.event",
    ]));
    assert_eq!(rc, 2);
}

#[test]
fn log_stage_filter() {
    let driver = Driver::with_tag("log-stage");
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    common::drive_to_after_pptx(&driver, &session);
    let (rc, out, _) = driver.run(&common::svec(&[
        "log", "--session", &session, "--stage", "research",
    ]));
    assert_eq!(rc, 0);
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with('#')).collect();
    assert_eq!(lines.len(), 2); // dispatched + reported
    assert!(!out.contains("outline") && !out.contains("pptx"));
}

#[test]
fn log_limit() {
    let driver = Driver::with_tag("log-limit");
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    common::drive_to_after_pptx(&driver, &session);
    let (rc, out, _) = driver.run(&common::svec(&[
        "log", "--session", &session, "--limit", "2",
    ]));
    assert_eq!(rc, 0);
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with('#')).collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines.last().unwrap().split_whitespace().nth(2), Some("action.reported"));
}

#[test]
fn log_warns_on_broken_chain() {
    let driver = Driver::with_tag("log-warn");
    let session = driver.start_default()["session_dir"].as_str().unwrap().to_string();
    driver.next(&session);
    let files = shard_files(&session);
    let mut tampered: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&files[0]).unwrap()).unwrap();
    tampered["payload"]["vars"] = json!({});
    std::fs::write(&files[0], serde_json::to_string(&tampered).unwrap()).unwrap();

    let (rc, out, _) = driver.run(&common::svec(&["log", "--session", &session]));
    assert_eq!(rc, 0); // 日志仍可读
    assert!(out.contains("审计链校验发现"));
}
