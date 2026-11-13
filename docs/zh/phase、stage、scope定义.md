# Phase、Stage、Scope 定义

> 语言：中文 | [English](../phase-stage-scope.md)

**状态：** 与仓库内 `dmaf/workflows/` 定义同步（2026-11-12，engine 0.6.0：12 stage / 2 scope / 8 agent / 29 skill·12 分类）
**参考：** `idea/流程定义/ai-native-sdlc phase、stage、scope定义.md`（AI-native SDLC 综合生命周期）
**产物：** 本文档所列定义经 `dmaf compile` 编译为 `dmaf/compiled/`（字节确定、入库、`--check` 守卫）。
**编写规范：** 全文见 `docs/zh/流程定义指南.md`（引擎仓库单源，随二进制分发）；本文是样例清单速览。

---

## 1. 总览

三个**标准目录**构成作者层：

| 目录 | 内容 | 规范 |
|---|---|---|
| `workflows/stages/<phase>/<slug>.md` | stage 库（frontmatter 声明 + 正文执行说明） | id = 文件名 stem；phase = 目录名；不得携带 scope / depends_on 字段（顺序归 scope，供数归硬输入） |
| `workflows/scopes/<name>.md` | scope：`include_stages` **有序**成员表（顺序即执行序）+ 流程级元数据（vars / constants / completion_template） | name = 文件名 stem；硬输入的生产者必须是成员且排在消费者之前（编译期强制） |
| `agents/`、`skills/` | AI 资产（stage 的 agents/skills 字段引用它们） | `dmaf install --harness <name>` 部署（账本哈希门）；`dmaf check` 校验引用齐备性 |

**Phase 即目录分类**：纯分类标签，不参与路由。执行顺序的唯一事实源 = scope 的 `include_stages` 序列——引擎按序列严格串行推进（一次一个 stage），无 stage 级并行；并行只发生在 stage 内部多 agent（`collaboration: parallel`）。

**数据依赖**：生产者-消费者按 inputs/outputs **路径模板精确匹配**（渲染后相等）。硬输入表达"必须供数"（生产者必须是成员且在前；数据依赖边由编译期推导，无需声明）；软输入（`required: false`）表达"可选参考"——生产者缺席（按需 skipped 或非成员裁剪）即按需未产出，缺失进派发指令 `missing_inputs` 提示。

**审批门语义**：生成 + 评审合并为带 `approval.gate` 的 stage——生成完成后停在 `awaiting-approval`，批准（granted → completed）或拒绝（rejected → 修订轮，派发携带 `revision_feedback`，轮次不限）。

## 2. Stage 总表（12 stage / 4 phase）

产物路径均为 `dmaf-output/{SESSION_ID}/<子目录>/{topic}-<工件>.md` 会话分桶（tech-stack 声明除外）；retry 全部 `1 / abort_workflow`（atomic-execution 的任务级验证内环重试 3 次由 task_description 约定）。"软" = `required: false` 软输入。

### 概念（concept）

| Stage | Agent / Skill | 输入 → 输出 | 审批门 | 执行 |
|---|---|---|---|---|
| intent-capture | product-manager / requirement-analysis | `{TASK}` 需求原文 → `{topic}-intent.md` | **intent-accept** | always |

### 需求（requirements）

| Stage | Agent / Skill | 输入 → 输出 | 审批门 | 执行 |
|---|---|---|---|---|
| spec-generation | product-manager / requirement-analysis | 意图 → `{topic}-spec.md`（v1） | **spec-review** | always |

### 设计（design）

| Stage | Agent / Skill | 输入 → 输出 | 审批门 | 执行 |
|---|---|---|---|---|
| hifi-ue | ux-designer / ue-prototyper-react-mantine | spec → `{topic}-high-fidelity-ue.md`（proto/** 代码化原型交付索引） | **ue-review** | conditional（纯后端/无界面 skipped） |
| architecture-analysis | architect / architecture-analysis | spec + UE（软） → `{topic}-architecture-impact.md` + `dmaf-output/tech-stack.md` | **architecture-review** | always |
| hld-design | architect / hld-design | spec + 影响声明 → `{topic}-hld.md` | **hld-review** | conditional（无需架构设计 skipped） |
| detailed-design | backend-engineer ∥ frontend-engineer / detailed-design-backend + detailed-design-frontend | spec + 影响声明 + hld（软）+ UE（软） → `{topic}-lld-backend.md`、`{topic}-lld-frontend.md` | **lld-review** | always |
| conflict-resolution | tech-lead → product-manager（sequential）/ — | spec + lld×2 + 按需产物（UE/影响声明/hld 软） → `{topic}-conflict-resolution.md`、`{topic}-design.md`、`{topic}-spec-v2.md` | **design-accept** | always |

### 开发（development）

| Stage | Agent / Skill | 输入 → 输出 | 审批门 | 执行 |
|---|---|---|---|---|
| plan-generation | backend-engineer / test-design, git-workflow | spec v1（**硬**）+ spec-v2 / design 索引 / lld×2 / UE（软） → `{topic}-plan.md` | **plan-review** | always |
| task-breakdown | backend-engineer / test-design | plan → `{topic}-tasks.md`（原子任务 + TDD 步骤） | — | always |
| atomic-execution | backend-engineer → frontend-engineer（sequential）/ automated-testing + backend-development、frontend-development | tasks（硬）+ lld（软）+ UE（软） → `{topic}-implementation-backend.md`、`{topic}-implementation-frontend.md` | — | always |
| system-testing | qa-engineer / automated-testing, contract-testing | 实现验证×2（硬）+ spec-v2 / lld×2 / UE（软） → `{topic}-system-test-report.md` | — | always |
| pr-preparation | backend-engineer / git-workflow, code-review, documentation | plan + tasks + 实现验证×2 + 系统测试报告 → `{topic}-pr.md` | **pr-review** | always |

## 3. Scope 清单（2）

| Scope | 成员（顺序即执行序） | 审批门 | 定位 |
|---|---|---|---|
| `sdlc-standard` | intent-capture → spec-generation → hifi-ue → architecture-analysis → hld-design → detailed-design → conflict-resolution → plan-generation → task-breakdown → atomic-execution → system-testing → pr-preparation（12） | 九道：intent-accept / spec-review / ue-review* / architecture-review / hld-review* / lld-review / design-accept / plan-review / pr-review | 标准全流程；按需 stage（*）在纯后端/无界面或无需架构设计时运行期上报 skipped 消解（不设门） |
| `sdlc-bugfix` | intent-capture → spec-generation → plan-generation → task-breakdown → atomic-execution → system-testing → pr-preparation（7） | 四道：intent-accept / spec-review / plan-review / pr-review | 缺陷修复轻路径：设计链五 stage 非成员，plan 以 spec v1 为必然硬准绳、设计产物软引用按需缺席；需要存量影响评估时把 architecture-analysis 插入 spec-generation 与 plan-generation 之间 |

**命名约定**：scope/agent/skill 名以包名前缀发布（`sdlc-` ≙ 包 `dmaf-sdlc`），避免跨包通用名碰撞——`sdlc-standard` / `sdlc-bugfix` 同族命名即此约定。

**序列与数据依赖的关系**（sdlc-bugfix 是机制示范）：裁掉成员必须连带把消费其产物的硬输入改软引用（否则编译错"生产者不在成员序列中"）；软引用非成员生产者的产物 = 按需裁剪合法形态。

## 4. AI 资产清单

**agents（8）**：`dmaf-conductor`（编排者，primary）+ 7 个 SDLC 角色：product-manager、ux-designer、architect、backend-engineer、frontend-engineer、tech-lead、qa-engineer。

**skills（29 / 12 分类）**：`dmaf`（编排技能）、`rules`（规范宪章层）留顶层；按能力域分类——requirements(1)、architecture(3)、backend(3)、frontend(4)、engineering(3)、testing(3)、governance(1)、operations(1)、security(1)、stacks(7，含两套前端栈绑定 + 两套 UE 原型器 + 三套后端栈绑定)。

## 5. 演进记录（相对 `idea/` 参考文档）

参考文档裁剪决策（Operations / Init / 持续 Evals 全部去除；评审 stage 合并为审批门）历史有效。此后主要演进：

| 参考形态 | 现形态 |
|---|---|
| lofi-ue → hifi-ue 两段设计稿 | hifi-ue 单段代码化原型（proto/**，点选评审环） |
| architecture（5 视图/ADR 一次成型） | architecture-analysis（影响判断 + 栈声明）→ hld-design（按需展开） |
| spec-finalization（独立定稿 stage） | 并入 conflict-resolution（tech-lead 裁决修正 → PO 定稿，`design-accept` 门，产出 design 索引 + spec v2） |
| self-check（独立自检 stage） | 并入 atomic-execution 任务级验证内环 |
| —（无） | 新增 system-testing（qa-engineer 只测不修） |
| sdlc-lean（演示裁剪） | 由 sdlc-bugfix 取代（真实场景驱动的轻路径，engine 0.6.0 scope 顺序化解锁） |

语义变更（0.6.0）：`depends_on` 字段移除，scope `include_stages` 顺序即执行序，数据依赖由硬输入推导派生边，软输入死接线编译错拦截——方案存内部档案（docs-dev/，不入库）。

## 6. 编译与校验

```bash
dmaf compile            # workflows/ → dmaf/compiled/（全部结构校验在编译期）
dmaf compile --check    # 漂移守卫：源与入库产物逐字节比对
dmaf check              # 引用齐备性（无输入源时默认标准布局）
dmaf doctor             # 布局/产物漂移/清单兼容/引用齐备/资产漂移
```

编译期校验要点（完整规则与错误对照见 `docs/zh/流程定义指南.md` §8）：硬输入生产者必须是 scope 成员且排在消费者之前；软输入/watches 死接线（成员生产者无一在前）拒绝；conditional 唯一生产者的硬输入必须软引用（规则 10）；模板变量 ∈ scope 声明集 ∪ 引擎保留变量（SESSION_ID / TASK）；id = stem、phase = 目录名、stage 带 scope / depends_on 字段 = 编译错。

## 7. 扩展指南

**新增 stage**：在 `workflows/stages/<phase>/` 新建 `<slug>.md`（模板见 `docs/zh/流程定义指南.md` §2.4）→ 下游 stage 把它的 outputs 加进 inputs（数据依赖由输入表达）→ scope `include_stages` 序列中把它排在消费者之前 → `dmaf compile` 并提交产物 → `dmaf check` 确认 AI 资产齐备。

**新增 scope**：在 `workflows/scopes/` 新建 `<name>.md`（name = stem），按执行序写 `include_stages`，声明 vars/constants 与 completion_template → `dmaf compile`。

**运行入口**：`dmaf start --scope <名> --task "<需求原文>" --var topic=<短名>`（task 驱动；vars 契约按 scope 声明）。
