//! 测试公共设施：唯一临时目录、注册表 fixture、进程内 CLI 驱动。
//!
//! 与 Python 版 conftest 的差异：不用 chdir（Rust 测试并行运行），
//! 全部以绝对路径传参，语义等价。

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use serde_json::{json, Value};

use dmaf_engine::registry::{load_registry, HarnessLayout, Registry};
use dmaf_engine::state::{self, WorkflowState};
use dmaf_engine::workflow::{load_workflow, Workflow};

pub fn svec(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// 进程级 chdir 串行锁：`std::env::set_current_dir` 是进程全局状态，
/// 所有需要切换 CWD 的测试（doctor/install 按 CWD 读 dmaf/）共用此锁，跨测试文件互斥。
static CWD_LOCK: Mutex<()> = Mutex::new(());

pub fn lock_cwd() -> MutexGuard<'static, ()> {
    CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct CwdGuard(PathBuf);

impl CwdGuard {
    pub fn enter(dir: &Path) -> Self {
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        CwdGuard(orig)
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

static DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn unique_tmp(tag: &str) -> PathBuf {
    let n = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!(
        "dmaf-test-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 测试结束时清理临时目录。
pub struct TmpGuard(pub PathBuf);
impl Drop for TmpGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 引擎测试夹具工作流（与 dmaf/ 样例解耦：样例更换不影响引擎行为测试）
pub fn workflow_file() -> PathBuf {
    repo_root().join("tests").join("fixtures").join("wf-lesson.yaml")
}

pub fn stages_dir() -> PathBuf {
    repo_root().join("dmaf").join("workflows").join("stages")
}

pub fn scopes_dir() -> PathBuf {
    repo_root().join("dmaf").join("workflows").join("scopes")
}

/// (name, mode, description)
pub const AGENTS: &[(&str, &str, &str)] = &[
    ("dmaf-conductor", "primary", "dmaf 工作流编排Agent"),
    ("lesson-intent", "subagent", "意图判断与参数提取Agent"),
    ("lesson-researcher", "subagent", "研究分析Agent"),
    ("lesson-outliner", "subagent", "PPT大纲Agent"),
    ("lesson-pptxer", "subagent", "PPT制作Agent"),
    ("lesson-quizzer", "subagent", "题库生成Agent"),
    ("lesson-videoer", "subagent", "视频转换Agent"),
    ("lesson-submiter", "subagent", "课程提交Agent"),
];

/// (name, description)
pub const SKILLS: &[(&str, &str)] = &[
    ("lesson-intent", "课件意图判断与参数提取技能"),
    ("lesson-research", "知识点研究分析技能"),
    ("lesson-outline", "PPT大纲设计技能"),
    ("lesson-pptx", "PPT制作技能"),
    ("lesson-quiz", "题库生成技能"),
    ("lesson-video", "PPT转视频技能"),
    ("lesson-submit", "课程提交技能"),
];

pub struct Driver {
    pub tmp: PathBuf,
    pub registry: PathBuf,
    pub workflow: PathBuf,
    pub output_root: PathBuf,
}

impl Driver {
    pub fn new() -> Self {
        Driver::with_tag("d")
    }

    pub fn with_tag(tag: &str) -> Self {
        let tmp = unique_tmp(tag);
        let registry = tmp.join("opencode");
        std::fs::create_dir_all(registry.join("agents")).unwrap();
        std::fs::create_dir_all(registry.join("skills")).unwrap();
        for (name, mode, desc) in AGENTS {
            std::fs::write(
                registry.join("agents").join(format!("{name}.md")),
                format!("---\nname: {name}\nmode: {mode}\ndescription: {desc}\n---\nbody\n"),
            )
            .unwrap();
        }
        for (name, desc) in SKILLS {
            let dir = registry.join("skills").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: {desc}\n---\nbody\n"),
            )
            .unwrap();
        }
        let output_root = tmp.join("output");
        Driver {
            tmp,
            registry,
            workflow: workflow_file(),
            output_root,
        }
    }

    pub fn run(&self, argv: &[String]) -> (i32, String, String) {
        // 锁定 zh 缺省：宿主系统语言不参与测试（显式设置使 run 的探测预设跳过），断言语言确定性
        dmaf_engine::i18n::set(dmaf_engine::i18n::Locale::Zh);
        let out = dmaf_engine::run(argv);
        (out.code, out.stdout, out.stderr)
    }

    pub fn start(&self, knowledge_point: &str, constraints: Option<&str>) -> Value {
        // task 驱动：知识点与约束拼入 TASK 原文（提取归流程内 intent stage）
        let task = match constraints {
            Some(c) => format!("讲解「{knowledge_point}」，额外约束：{c}"),
            None => format!("讲解「{knowledge_point}」"),
        };
        let argv = svec(&[
            "start",
            "--workflow",
            self.workflow.to_str().unwrap(),
            "--task",
            &task,
            "--output-root",
            self.output_root.to_str().unwrap(),
        ]);
        let (rc, out, err) = self.run(&argv);
        assert_eq!(rc, 0, "start 失败: {err}");
        serde_json::from_str(&out).unwrap()
    }

    pub fn start_default(&self) -> Value {
        self.start("大语言模型", None)
    }

    pub fn next(&self, session: &str) -> Value {
        let (rc, out, err) = self.run(&svec(&[
            "next",
            "--session",
            session,
            "--workflow",
            self.workflow.to_str().unwrap(),
        ]));
        assert_eq!(rc, 0, "next 失败: {err}");
        serde_json::from_str(&out).unwrap()
    }

    pub fn report_payload(&self, session: &str, payload: &Value) -> (i32, Value, String) {
        let payload_file = self.tmp.join("_report_payload.json");
        std::fs::write(&payload_file, serde_json::to_string(payload).unwrap()).unwrap();
        let (rc, out, err) = self.run(&svec(&[
            "report",
            "--session",
            session,
            "--payload",
            payload_file.to_str().unwrap(),
            "--workflow",
            self.workflow.to_str().unwrap(),
        ]));
        let _ = std::fs::remove_file(&payload_file);
        let body_source = if rc == 0 { &out } else { &err };
        let body = serde_json::from_str(body_source)
            .unwrap_or_else(|_| panic!("report 输出不是 JSON: {body_source}"));
        (rc, body, out)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn report_action(
        &self,
        session: &str,
        instruction: &Value,
        index: usize,
        status: &str,
        summary: &str,
        create_outputs: bool,
        error: Option<&str>,
    ) -> (i32, Value) {
        let action = &instruction["payload"]["actions"][index];
        if create_outputs && status == "success" {
            for out in action["outputs"].as_array().unwrap() {
                let p = PathBuf::from(out.as_str().unwrap());
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(&p, "mock").unwrap();
            }
        }
        let payload = json!({
            "action_id": action["action_id"],
            "status": status,
            "summary": summary,
            "error_message": error,
            "agent_outputs": action["outputs"],
        });
        let (rc, body, _) = self.report_payload(session, &payload);
        (rc, body)
    }

    pub fn report_success(&self, session: &str, instruction: &Value) -> (i32, Value) {
        self.report_action(session, instruction, 0, "success", "ok", true, None)
    }

    pub fn report_approval(&self, session: &str, gate: &str, decision: &str, note: &str) -> (i32, Value) {
        let (rc, body, _) = self.report_payload(
            session,
            &json!({"kind": "approval", "gate": gate, "decision": decision, "note": note}),
        );
        (rc, body)
    }

    pub fn verify_audit(&self, session: &str) -> Value {
        let (rc, out, err) = self.run(&svec(&["verify-audit", "--session", session]));
        assert_eq!(rc, 0, "verify-audit 失败: {err}{out}");
        serde_json::from_str(&out).unwrap()
    }

    pub fn state(&self, session: &str) -> WorkflowState {
        state::load(&PathBuf::from(session).join("dmaf-state.md")).unwrap()
    }

    pub fn wf(&self) -> Workflow {
        load_workflow(&self.workflow).unwrap()
    }

    pub fn registry_obj(&self) -> Registry {
        load_registry(&self.registry, HarnessLayout::FlatMd)
    }
}

impl Default for Driver {
    fn default() -> Self {
        Self::new()
    }
}

/// 推进到 pptx 完成后的 checkpoint，返回该时刻的审计 seq。
pub fn drive_to_after_pptx(driver: &Driver, session: &str) -> u64 {
    // intent：意图判断与提取
    let instruction = driver.next(session);
    let (rc, _) = driver.report_success(session, &instruction);
    assert_eq!(rc, 0);
    let instruction = driver.next(session);
    let (rc, _) = driver.report_success(session, &instruction);
    assert_eq!(rc, 0);
    let instruction = driver.next(session);
    let (rc, body) = driver.report_success(session, &instruction);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "awaiting-approval");
    let instruction = driver.next(session);
    assert_eq!(instruction["type"], "await_approval");
    let (rc, _) = driver.report_approval(session, "outline-review", "granted", "");
    assert_eq!(rc, 0);
    let instruction = driver.next(session);
    assert_eq!(instruction["stage"], "pptx");
    let (rc, body) = driver.report_success(session, &instruction);
    assert_eq!(rc, 0);
    assert_eq!(body["stage_state"], "completed");
    driver.state(session).seq
}
