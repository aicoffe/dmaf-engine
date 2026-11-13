//! 工作流定义加载与校验测试（对应 test_workflow_validation.py）。

mod common;

use common::TmpGuard;
use dmaf_engine::registry::{load_registry, validate_refs, HarnessLayout};
use dmaf_engine::workflow::{check_templates, load_workflow, parse_workflow, resolve_vars, WorkflowError};
use serde_json::json;

fn base_stages() -> serde_json::Value {
    json!([
        {
            "id": "a", "collaboration": "single",
            "agents": [{
                "agent": "lesson-researcher", "skills": ["lesson-research"],
                "inputs": [], "outputs": ["out/{knowledge_point}-a.md"],
            }],
            "retry": {"max_attempts": 1, "on_exhausted": "abort_workflow"},
            "approval": {"required": false},
        },
        {
            "id": "b", "collaboration": "single",
            "agents": [{
                "agent": "lesson-outliner", "skills": ["lesson-outline"],
                "inputs": ["out/{knowledge_point}-a.md"],
                "outputs": ["out/{knowledge_point}-b.md"],
            }],
            "approval": {"required": false},
        },
    ])
}

/// 写 yaml 并加载；失败时 panic（预期失败的场景用 try_ 变体）。
fn write_wf_ok(tmp: &std::path::Path, data: &serde_json::Value) -> dmaf_engine::workflow::Workflow {
    let path = tmp.join("wf.yaml");
    std::fs::write(&path, serde_yaml_to_string(data)).unwrap();
    load_workflow(&path).unwrap()
}

fn write_wf_try(tmp: &std::path::Path, data: &serde_json::Value) -> Result<dmaf_engine::workflow::Workflow, WorkflowError> {
    let path = tmp.join("wf.yaml");
    std::fs::write(&path, serde_yaml_to_string(data)).unwrap();
    load_workflow(&path)
}

fn serde_yaml_to_string(v: &serde_json::Value) -> String {
    serde_yaml::to_string(v).unwrap()
}

fn wf_doc(vars: &[&str], stages: serde_json::Value) -> serde_json::Value {
    // vars mapping 形态：调用点给名字列表，全部按必填（null）声明
    let vars: serde_json::Map<String, serde_json::Value> = vars
        .iter()
        .map(|v| (v.to_string(), serde_json::Value::Null))
        .collect();
    json!({"workflow": {
        "id": "wf-x",
        "vars": vars,
        "stages": stages,
    }})
}

#[test]
fn valid_wf_lesson_loads() {
    let wf = load_workflow(&common::workflow_file()).unwrap();
    assert_eq!(wf.id, "wf-lesson");
    // 声明序列即执行序
    assert_eq!(
        wf.stage_ids(),
        vec!["intent", "research", "outline", "pptx", "quiz-and-video", "submit"]
    );
    let parallel = wf.stage("quiz-and-video").unwrap();
    assert_eq!(parallel.collaboration, "parallel");
    assert_eq!(parallel.agents.len(), 2);
    assert!(wf.stage("outline").unwrap().approval.required);
    assert_eq!(wf.stage("outline").unwrap().approval.gate, "outline-review");
}

#[test]
fn registry_refs_ok() {
    let driver = common::Driver::with_tag("refs-ok");
    let wf = load_workflow(&common::workflow_file()).unwrap();
    assert!(validate_refs(&wf, &driver.registry_obj()).is_empty());
}

#[test]
fn missing_agent_detected() {
    let driver = common::Driver::with_tag("ghost-agent");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[0]["agents"][0]["agent"] = json!("lesson-ghost");
    let wf = write_wf_ok(&driver.tmp, &wf_doc(&["knowledge_point"], stages));
    let problems = validate_refs(&wf, &driver.registry_obj());
    assert!(problems.iter().any(|p| p.contains("lesson-ghost")));
}

#[test]
fn primary_mode_agent_rejected() {
    // 用 driver 合成注册表（dmaf-conductor 为 primary）：不依赖真实 harness 与残留孤儿资产
    let driver = common::Driver::with_tag("primary-mode");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[0]["agents"][0]["agent"] = json!("dmaf-conductor");
    let wf = write_wf_ok(&driver.tmp, &wf_doc(&["knowledge_point"], stages));
    let problems = validate_refs(&wf, &driver.registry_obj());
    assert!(problems.iter().any(|p| p.contains("subagent")));
}

#[test]
fn missing_skill_detected() {
    let driver = common::Driver::with_tag("ghost-skill");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[0]["agents"][0]["skills"] = json!(["lesson-ghost"]);
    let wf = write_wf_ok(&driver.tmp, &wf_doc(&["knowledge_point"], stages));
    let problems = validate_refs(&wf, &driver.registry_obj());
    assert!(problems.iter().any(|p| p.contains("lesson-ghost")));
}

#[test]
fn depends_on_field_rejected_with_migration_hint() {
    // depends_on 已移除：残留声明报迁移错（顺序写 scope 序列，数据依赖由硬输入表达）
    let driver = common::Driver::with_tag("dep-residue");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[1]["depends_on"] = json!(["a"]);
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], stages)).unwrap_err();
    assert!(
        err.problems
            .iter()
            .any(|p| p.contains("depends_on 字段（已移除）") && p.contains("include_stages")),
        "{:?}",
        err.problems
    );
}

#[test]
fn dead_wiring_soft_input_detected() {
    // 软输入的成员生产者排在消费者之后：线性执行序下本会话永远缺席 → 编译错
    let driver = common::Driver::with_tag("dead-wiring");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    // 反转序列：b（软输入 a 的产物）在前，a（生产者）在后
    stages[1]["agents"][0]["inputs"] = json!([{"artifact": "out/{knowledge_point}-a.md", "required": false}]);
    let swapped = json!([stages[1].clone(), stages[0].clone()]);
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], swapped)).unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("死接线")), "{:?}", err.problems);
}

#[test]
fn producer_after_consumer_detected() {
    // 硬输入的生产者排在消费者之后 → 编译错（顺序即执行序）
    let driver = common::Driver::with_tag("late-producer");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    let swapped = json!([stages[1].clone(), stages[0].clone()]);
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], swapped)).unwrap_err();
    assert!(
        err.problems
            .iter()
            .any(|p| p.contains("排在 stage 'b' 之后") && p.contains("顺序即执行序")),
        "{:?}",
        err.problems
    );
}

#[test]
fn unknown_template_var_detected() {
    let driver = common::Driver::with_tag("bogus-var");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    // task_description 不参与生产者匹配：纯测变量闭合（outputs 改名会先触发生产者缺失）
    stages[0]["agents"][0]["task_description"] = json!("任务 {bogus}");
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], stages)).unwrap_err();
    assert!(
        err.problems
            .iter()
            .any(|p| p.contains("bogus") && p.contains("未知变量")),
        "{:?}",
        err.problems
    );
}

#[test]
fn single_stage_requires_exactly_one_agent() {
    let driver = common::Driver::with_tag("single-1");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    let extra = stages[0]["agents"][0].clone();
    let mut extra = extra;
    extra["agent"] = json!("lesson-outliner");
    stages[0]["agents"].as_array_mut().unwrap().push(extra);
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], stages)).unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("single")));
}

#[test]
fn approval_requires_gate() {
    let driver = common::Driver::with_tag("gate");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[1]["approval"] = json!({"required": true});
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], stages)).unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("gate")));
}

#[test]
fn input_without_producer_detected() {
    let driver = common::Driver::with_tag("no-producer");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[1]["agents"][0]["inputs"] = json!(["不存在的输入.md"]);
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], stages)).unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("不存在的输入")));
}

#[test]
fn producer_before_consumer_via_sequence_ok() {
    // 消费者硬输入引用序列中更早的生产者 → 合法（数据依赖由输入表达，顺序即派发序）
    let driver = common::Driver::with_tag("seq-ok");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages.as_array_mut().unwrap().push(json!({
        "id": "c", "collaboration": "single",
        "agents": [{
            "agent": "lesson-pptxer", "skills": ["lesson-pptx"],
            "inputs": ["out/{knowledge_point}-a.md"],
            "outputs": ["{knowledge_point}-c.md"],
        }],
        "approval": {"required": false},
    }));
    let wf = write_wf_ok(&driver.tmp, &wf_doc(&["knowledge_point"], stages));
    assert_eq!(wf.stage_ids(), vec!["a", "b", "c"]);
    assert_eq!(wf.stage("c").unwrap().depends_on, vec!["a"]);
}

#[test]
fn self_produced_input_detected() {
    let driver = common::Driver::with_tag("self-prod");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[0]["agents"][0]["inputs"] = json!(["out/{knowledge_point}-a.md"]);
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], stages)).unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("自己生产")));
}

#[test]
fn duplicate_stage_ids() {
    let driver = common::Driver::with_tag("dup");
    let _g = TmpGuard(driver.tmp.clone());
    let mut stages = base_stages();
    stages[1]["id"] = json!("a");
    let err = write_wf_try(&driver.tmp, &wf_doc(&["knowledge_point"], stages)).unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("重复")));
}

#[test]
fn resolve_vars_semantics() {
    let driver = common::Driver::with_tag("vars");
    let _g = TmpGuard(driver.tmp.clone());
    let doc = json!({"workflow": {
        "id": "wf-x",
        // mapping 形态：空值 = 必填，字面值 = 内联默认
        "vars": {"knowledge_point": null, "constraints": ""},
        "constants": {"product_line": "课件"},
        "stages": base_stages(),
    }});
    let wf = write_wf_ok(&driver.tmp, &doc);
    // 必填缺失 → 错
    assert!(resolve_vars(&wf, &Default::default(), "20260101-000000", "").is_err());
    // 未知变量 → 错
    let mut bogus = std::collections::BTreeMap::new();
    bogus.insert("knowledge_point".to_string(), "x".to_string());
    bogus.insert("bogus".to_string(), "1".to_string());
    let err = resolve_vars(&wf, &bogus, "20260101-000000", "").unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("未知变量")), "{err:?}");
    // 常量拒收：--var 不可覆盖
    let mut override_c = std::collections::BTreeMap::new();
    override_c.insert("knowledge_point".to_string(), "x".to_string());
    override_c.insert("product_line".to_string(), "另一个".to_string());
    let err = resolve_vars(&wf, &override_c, "20260101-000000", "").unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("常量")), "{err:?}");
    // 正常路径：常量注入、保留变量注入；内联默认可被提供值覆盖
    let mut provided = std::collections::BTreeMap::new();
    provided.insert("knowledge_point".to_string(), "量子计算".to_string());
    provided.insert(
        "constraints".to_string(),
        "面向有Python基础的学生".to_string(),
    );
    let vars_ = resolve_vars(&wf, &provided, "20260101-000000", "做一个递归课件").unwrap();
    assert_eq!(vars_["product_line"], "课件");
    assert_eq!(vars_["constraints"], "面向有Python基础的学生");
    assert_eq!(vars_["SESSION_ID"], "20260101-000000");
    assert_eq!(vars_["TASK"], "做一个递归课件");
    // 省略可选变量 → 取内联默认（空串）
    let mut provided2 = std::collections::BTreeMap::new();
    provided2.insert("knowledge_point".to_string(), "量子计算".to_string());
    let vars_2 = resolve_vars(&wf, &provided2, "20260101-000000", "做一个递归课件").unwrap();
    assert_eq!(vars_2["constraints"], "");
}

#[test]
fn var_decl_syntax_and_name_exclusivity() {
    let driver = common::Driver::with_tag("var-decl");
    let _g = TmpGuard(driver.tmp.clone());
    let stages = base_stages();

    // 列表形式已移除
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "vars": ["topic"], "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("列表形式已移除")), "{err:?}");

    // defaults 已移除（迁移提示）
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "vars": {"topic": null}, "defaults": {"topic": "t"}, "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("defaults 已移除")), "{err:?}");

    // vars 值必须是空或字面标量
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "vars": {"bad": ["a"]}, "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("字面标量")), "{err:?}");

    // constants 值必须是字面标量
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "constants": {"bad": {"k": 1}}, "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("字面标量")), "{err:?}");

    // 名称互斥：vars ∩ constants
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "vars": {"dup": null}, "constants": {"dup": "v"}, "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("同时声明于 vars 与 constants")), "{err:?}");

    // derived_vars 已移除（迁移提示）
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "derived_vars": {"tag": "t-{SESSION_ID}"}, "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("derived_vars 已移除")), "{err:?}");

    // description_from_stage 已移除（迁移提示）
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "description_from_stage": "research", "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("description_from_stage 已移除")), "{err:?}");

    // 保留名不可声明
    let err = write_wf_try(
        &driver.tmp,
        &json!({"workflow": {"id": "wf-x", "vars": {"SESSION_ID": null}, "stages": stages.clone()}}),
    )
    .unwrap_err();
    assert!(err.problems.iter().any(|p| p.contains("保留变量")), "{err:?}");
}

#[test]
fn parse_workflow_requires_top_level() {
    let empty = serde_json::Value::Null;
    assert!(parse_workflow(
        &serde_yaml_from_json(&empty),
        &common::repo_root().join("none.yaml")
    )
    .is_err());
}

fn serde_yaml_from_json(v: &serde_json::Value) -> serde_yaml::Value {
    // 通过序列化字符串桥接两种 Value 类型
    serde_yaml::from_str(&serde_json::to_string(v).unwrap()).unwrap()
}

#[test]
fn task_var_injection_and_omission() {
    let driver = common::Driver::with_tag("task-var");
    let _g = TmpGuard(driver.tmp.clone());
    // workflow 模板引用 TASK（stage task_description），vars 只声明 topic
    let mut stages = base_stages();
    stages[0]["agents"][0]["task_description"] = json!("处理需求：{TASK}");
    let doc = wf_doc(&["knowledge_point", "topic"], stages);
    let path = driver.tmp.join("wf-task.yaml");
    std::fs::write(&path, serde_yaml_to_string(&doc)).unwrap();

    // 未提供 --task：check_templates 报"模板变量不闭合"（TASK 用了就必须给）
    let wf = load_workflow(&path).unwrap();
    let mut provided = std::collections::BTreeMap::new();
    provided.insert("knowledge_point".to_string(), "递归".to_string());
    provided.insert("topic".to_string(), "递归".to_string());
    let vars_ = resolve_vars(&wf, &provided, "20260101-000000", "").unwrap();
    let problems = check_templates(&wf, &vars_);
    assert!(problems.iter().any(|p| p.contains("TASK") && p.contains("闭合") || p.contains("TASK")), "{problems:?}");

    // 提供 --task：TASK 注入且值原样承载
    let vars_2 = resolve_vars(&wf, &provided, "20260101-000000", "给我做递归课件").unwrap();
    assert_eq!(vars_2["TASK"], "给我做递归课件");
    assert!(check_templates(&wf, &vars_2).is_empty());
}
