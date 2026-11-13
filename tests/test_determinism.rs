//! 确定性测试：相同状态输入，永远产生相同的下一跳指令（逐字节）。
//! （对应 test_router_determinism.py；随机源为本地 xorshift，与 Python random 不同但语义等价）

mod common;

use dmaf_engine::router;
use serde_json::json;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x1234_5678_9ABC_DEF1) | 1)
    }

    /// [0, 1)
    fn next_f64(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (r >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn determinism_check(driver: &common::Driver, session: &str) {
    let workflow = driver.wf();
    let state = driver.state(session);
    let (i1, e1) = router::compute_next(&workflow, &state).unwrap();
    let (i2, e2) = router::compute_next(&workflow, &state).unwrap();
    assert_eq!(dmaf_engine::json_text(&i1), dmaf_engine::json_text(&i2));
    assert_eq!(e1, e2);
}

#[test]
fn resume_instruction_identical() {
    let driver = common::Driver::with_tag("resume");
    let session = driver.start_default()["session_dir"]
        .as_str()
        .unwrap()
        .to_string();
    let instruction = driver.next(&session);
    assert_eq!(instruction["type"], "dispatch");
    let workflow = driver.wf();
    let state = driver.state(&session);
    let (replayed, effects) = router::compute_next(&workflow, &state).unwrap();
    assert!(effects.is_empty()); // resume 无副作用
    // dispatch 的审计写入推进了 seq，instruction_id 随之演进；其余字段必须逐字节一致
    let mut a = replayed.clone();
    let mut b = instruction.clone();
    if let Some(o) = a.as_object_mut() {
        o.remove("instruction_id");
    }
    if let Some(o) = b.as_object_mut() {
        o.remove("instruction_id");
    }
    assert_eq!(dmaf_engine::json_text(&a), dmaf_engine::json_text(&b));
}

#[test]
fn instruction_id_derived_from_seq() {
    let driver = common::Driver::with_tag("iid");
    let session = driver.start_default()["session_dir"]
        .as_str()
        .unwrap()
        .to_string();
    let seq_before = driver.state(&session).seq;
    let instruction = driver.next(&session);
    assert_eq!(
        instruction["instruction_id"].as_str().unwrap(),
        format!("ins-wf-lesson-{seq_before:06}")
    );
}

fn run_random_trace(driver: &common::Driver, session: &str, rng: &mut Rng) -> String {
    for _ in 0..60 {
        determinism_check(driver, session);
        let instruction = driver.next(session);
        let kind = instruction["type"].as_str().unwrap().to_string();
        if kind == "complete" || kind == "fail" {
            determinism_check(driver, session);
            return kind;
        }
        if kind == "await_approval" {
            let decision = if rng.next_f64() < 0.7 { "granted" } else { "rejected" };
            let (rc, body) = driver.report_approval(
                session,
                instruction["payload"]["gate"].as_str().unwrap(),
                decision,
                "评审意见",
            );
            assert_eq!(rc, 0, "{body}");
            continue;
        }
        assert!(kind == "dispatch" || kind == "retry", "kind={kind}");
        let stage = instruction["stage"].as_str().unwrap().to_string();
        let n_actions = instruction["payload"]["actions"].as_array().unwrap().len();
        for idx in 0..n_actions {
            let status = if stage == "research" {
                // 留出触发 abort_workflow 的路径
                if rng.next_f64() < 0.85 { "success" } else { "failed" }
            } else if stage == "outline" || stage == "pptx" || stage == "submit" {
                "success"
            } else if rng.next_f64() < 0.6 {
                "success"
            } else {
                "failed"
            };
            let (rc, body) = driver.report_action(
                session,
                &instruction,
                idx,
                status,
                "随机摘要",
                status == "success",
                None,
            );
            assert_eq!(rc, 0, "{body}");
            determinism_check(driver, session);
        }
    }
    panic!("60 步未到达终态");
}

#[test]
fn determinism_across_random_traces() {
    // 30 条随机轨迹（含失败/重试/审批拒绝），每步验证同状态同指令
    let driver = common::Driver::with_tag("random");
    let mut outcomes = std::collections::BTreeSet::new();
    for seed in 0..30u64 {
        let mut rng = Rng::new(seed);
        let session = driver
            .start(&format!("知识点{seed}"), None)["session_dir"]
            .as_str()
            .unwrap()
            .to_string();
        let outcome = run_random_trace(&driver, &session, &mut rng);
        outcomes.insert(outcome);
        assert!(driver.verify_audit(&session)["replay_matches"].as_bool().unwrap());
    }
    // 覆盖了成功与失败两种终局
    assert!(outcomes.contains("complete") && outcomes.contains("fail"));
    let _ = json!(null); // 保持 json! 导入
}
