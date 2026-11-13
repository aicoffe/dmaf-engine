**语言：** [English](README.md) | 简体中文

# D-MAF Engine（确定性Multi-Agent框架）

多智能体工作流交给 LLM 编排，最怕的不是模型不够聪明，而是过程不可靠：下一步做什么全凭临场发挥，跑偏了无从追溯，上下文一断进度就丢。**dmaf 把流程调度权从 LLM 手里收回来，交给确定性引擎**——你用 YAML 声明流程，引擎逐步派发 LLM 执行：**相同状态永远得到相同指令，每一步落审计账本，可回放、可复现、断点续跑**。

> **Engine 是唯一路由权威，Agent 是受控执行单元**——路由、状态、能力授权、协作拓扑全部由纯代码从「产物 + 状态」计算得出，LLM 只在 Engine 指定的单个动作内发挥判断力。

## 为什么做 D-MAF

LLM 的指令遵循能力不是一个稳定常量，而是一个随任务规模显著衰减的变量：AgentIF 基准显示指令遵循准确率随任务长度、复杂度与约束数量增加而显著下降；IFEval 与 Lost in the Middle 进一步表明，模型对上下文中部信息的遵循最弱。把多智能体协作押注在这种能力上，会得到五类系统性不确定性：

| # | 不确定性来源 | 表现 |
|---|--------------|------|
| 1 | 路由不确定 | LLM 基于 prompt 决定「下一步谁来做」 |
| 2 | 状态不确定 | 任务进度由 LLM 基于上下文自行维护，随窗口压缩而漂移 |
| 3 | 通信不确定 | Agent 间协作依赖自然语言消息，语义逐跳放大歧义 |
| 4 | 拓扑不确定 | Agent 之间直接互相路由，协作结构不可预知 |
| 5 | 能力授权不确定 | Agent 依赖 system prompt 全量注入，自行挑选技能 |

五类来源的共同根源是同一个：**把控制流、状态与通信全部交给 LLM 即兴决策，随机性层层放大**。D-MAF 的目标由此确立——让流程可预测、过程可复现、结果可审计、协作可信任。

## 方案：路由与执行分离

D-MAF（Deterministic Multi-Agent Framework）把确定性工作流引擎的思想用于约束 LLM 多智能体：**控制平面 × 执行平面解耦**。

```
控制平面  dmaf（确定性程序，纯代码）
  ├── workflow.yaml 声明式定义 ──compile──▶ 字节确定的 JSON 产物（入库，--check 守漂移）
  ├── next 纯函数：状态 + 产物 ──▶ 唯一下一跳指令（类型化 JSON，schema 校验）
  └── report：校验回报 → 先写审计分片 → 再改状态（追加式 SHA-256 哈希链）
        ▲                              │
        │ report（回报）               │ next（指令）
        │                              ▼
执行平面  harness 运行时（LLM）
  ├── Orchestrator 降级为薄转发循环：next 取指令 → 派发 Agent → report 回报
  └── Agent 在指令约定的单个动作内发挥判断力，全程不触碰控制流
```

Engine 只依赖 workflow 定义与状态文件这两个磁盘输入，`next` 无时钟、无随机、无 IO——**相同状态输入，永远产生相同下一跳指令**。LLM 的随机性被隔离在动作边界内，不再污染全局流程。五个机制逐一消解上表的不确定性：

### 1. 声明式工作流定义 + 编译期门禁 → 消解拓扑不确定

工作流以 YAML/MD 静态声明：stage 列表、依赖 DAG、协作模式（`single` / `sequential` / `parallel`）、重试策略、审批门。Agent 之间**不直接路由**——handoff、消息、评审全部经产物文件由 Engine 注入，生产者-消费者按路径精确匹配。全部结构校验（DAG 无环、依赖闭包、模板变量声明集）在编译期完成；产物字节确定并入库，diff 即成员变更评审，`compile --check` 守护漂移。

### 2. next/report 契约化通信 → 消解通信不确定

Engine 与运行时之间的唯一契约是**类型化 JSON 指令**：`dispatch` / `await_approval` / `retry` / `park` / `complete` / `fail` 六种，通用字段 + `payload` 承载完整执行契约（agents、skills、inputs/outputs、retry、审批要求），全部带 JSON Schema 校验。流程语义不经自然语言传递，消除歧义；`report` 回报时 Engine 校验期望产物存在性，缺失即按失败处理——指令驱动 Orchestrator，不替代其执行。

### 3. 状态外置 → 消解状态不确定

Stage 八种状态（`pending` / `in-progress` / `awaiting-approval` / `revising` / `completed` / `failed` / `cancelled` / `skipped`）仅由 Engine 维护，**状态文件对 LLM 完全不可见**；状态转换必须经 `dmaf report` 提交，禁止 LLM 直接修改。状态文件是长程任务的唯一真相源——上下文窗口可以压缩、丢弃、重启，Engine 都能从磁盘恢复。

### 4. 审计优先，哈希链账本 → 支撑结果可审计

每次状态转移前**先写审计分片、后改状态**，两者永不背离；审计写入失败则抛异常、状态不变。LLM 绝不写审计。十类审计事件（workflow started/failed/cancelled/forked、action dispatched/reported、approval granted/rejected、retry scheduled）与状态转换一一对应，每个分片带四重 SHA-256 哈希链（`prev_shard_hash` / `state_before_hash` / `state_after_hash` / `shard_hash`）——篡改可检测，账本即日志。

### 5. Skill 按需精准注入 → 消解能力授权不确定

不把技能目录全量塞进 system prompt：每条指令只携带本动作需要的技能名单，由 Orchestrator 在派发时逐个加载，Agent 只能看到和调用被授权的能力。运行期命令零 harness 依赖——`start` / `next` / `report` 不读任何 harness 配置，引用完整性由部署期的 `dmaf check` 校验。

**长程任务支持**：状态外置 + Engine 无状态（每次 `next` 重新计算，不依赖对话历史）+ 上下文最小化（Agent 每次只收当前指令与输入产物），使长程任务摆脱 LLM 记忆依赖；`fork` 可从任意审计 checkpoint 非破坏性分叉，`replay` 可从账本重建状态文件。

## 定位与价值

与多智能体编排常见路线对比：

| 对比维度 | harness（opencode 等） | Dify（可视化编排） | LangGraph（图编排框架） | **D-MAF** |
|----------|----------------------|-------------------|------------------------|-----------|
| 编排范式 | prompt 驱动 | 拖拽画布 + 节点表单配置 | 节点 + 边 + State，全部写代码 | **声明式定义 + 确定性 Engine，零编排代码** |
| 路由决定权 | LLM 临场决定，无强制 | 画布连线配置期确定，节点/Agent 内 LLM 自由 | 边函数决定，可代码可 LLM | **Engine 唯一权威，LLM 不可见** |
| 状态归属 | 会话上下文，随窗口压缩漂移 | 平台变量与会话存储 | State + 检查点 | **外置状态文件，LLM 不可见** |
| 通信契约 | 自然语言 prompt | 平台节点配置（DSL/表单） | State 字段共享 | **类型化 JSON 指令（schema 校验）** |
| 技能注入 | 技能描述进上下文，模型自行取用 | 按应用配置工具/插件 | 开发者自行管理 | **运行时按需精准注入（指令带名单）** |
| 审计与重放 | 会话日志，无原生重放 | 平台运行日志，无哈希链 | 检查点可恢复/时间旅行 | **哈希链审计账本 + 原生 replay/fork** |
| 确定性保证 | **无** | 拓扑确定，节点内行为不确定 | 部分，取决于边实现 | **强且可验证（验证命令族）** |

关键区别：**Dify 的复杂度留在画布配置里，LangGraph 的复杂度留在编排代码里，harness 缺失流程确定性**。dmaf 把确定性整体交给 Engine——零编排代码、零画布配置，人只专注两件事：**写清流程定义（stage/scope），建好 agents/skills 等 AI 资产**。对使用者的直接价值：

- **流程可预测**：协作拓扑编译期固定，运行期不会「跑到哪算哪」。
- **过程可复现**：同状态必同指令，`verify-determinism` 用随机轨迹回归实证。
- **结果可审计**：哈希链账本即日志，`verify-audit` 链校验、`replay` 状态重建，全程可回放。
- **长程可恢复**：上下文可随时丢弃重启，会话不丢；`fork` 从任意 checkpoint 分叉。
- **成果可分发**：`publish` / `add` 对接 market 仓库发布与安装。

## 快速上手

安装 `dmaf` 后不需要引擎源码——在自己的项目仓库里按标准布局开展工作流。按角色分两条动线：**使用工作流**（把现成工作流包装进项目，开展业务）与**开发工作流**（定义流程与 AI 资产）。

### 安装引擎

每平台一行命令；同一命令重跑即原地升级：

```bash
# macOS / Linux
curl -fsSL https://dmaf.aicoffe.com/install.sh | sh

# Windows PowerShell
irm https://dmaf.aicoffe.com/install.ps1 | iex

# npm（全平台）
npm i -g @aicoffe/dmaf
```

下载源缺省走官方 market（自动回退 GitHub），可加 `--source github` / `-Source github` 指定。装完用 `dmaf doctor` 自证。源码构建（`cargo install dmaf-engine`）保留为兜底。

### 场景一：使用工作流

以随本仓库开发并已发布到官方 market 的示例工作流 **dmaf-sdlc**（sdlc-standard：AI 原生 SDLC 标准流程）为例——装进你的工程，即可跑完整的需求 → 设计 → 开发 → 交付：

```bash
# 1. 安装工作流
dmaf push <目标工程dir> # 把当前工作流交付到目标工程
dmaf add https://dmaf.aicoffe.com/aicoffe/dmaf-sdlc    # 安装工作流到当前工程（目标工程中执行）
# 支持多种源，请dmaf -h了解

# 2. 部署 AI 资产到 harness 并体检
dmaf install                 # push/add 已自动串联部署；此后资产变更重跑即可（--harness 缺省自动探测，多 harness 共存时指定或 --harness all）
dmaf doctor --harness opencode   # 布局/产物漂移(含版本兼容)/清单 engine 兼容/引用齐备性/资产漂移

# 3. 启动会话：使用/dmaf skill，无需记忆dmaf命令
/dmaf 给现有商城加购物车

# 4. 查看/运维
/dmaf 当前会话跑到哪个阶段了
/dmaf 从design阶段重跑
```

日常自检动线：git pull 后 `dmaf doctor --harness <name>`——`would_change` 重跑 `dmaf install` 即同步；`modified` 为本地修改过的目标，确认覆盖可加 `--force`。

### 场景二：开发工作流

定义流程（stage/scope）与 AI 资产（agents/skills）。定义语法全解见 [docs/zh/流程定义指南.md](docs/zh/流程定义指南.md)，本仓库 `dmaf/` 即一份完整样例：

```bash
# 1. 补齐标准布局（brownfield 绝不覆盖已有文件、幂等；greenfield 用 dmaf new --dir my-project）
dmaf new --dir .

# 2. 编写定义与资产（语法见定义指南；harness 装有随 dmaf new 下发的定义技能时，可直接让 AI 代劳——见 docs/zh/流程定义指南.md §1）
#    dmaf/workflows/stages/<phase>/<slug>.md            # stage：frontmatter 声明结构，正文写执行说明
#    dmaf/workflows/scopes/<name>.md                    # scope：include_stages 认领成员 + vars/constants 声明
#    dmaf/agents/<name>.md  dmaf/skills/<名>/SKILL.md   # agent 与技能资产

# 3. 编译门禁（全部结构校验在此把关；产物字节确定、入库，diff 即评审）
dmaf compile
dmaf compile --check          # 漂移守卫（CI 可用）

# 4. 部署与校验（AI 资产变更后重跑 install；账本哈希门见 §install 语义）
dmaf install --harness opencode
dmaf check --harness opencode --stages dmaf/workflows/stages --scopes dmaf/workflows/scopes

# 5. 内联试跑（原型/试验/极小流程；与标准目录共享解析器）
dmaf validate --workflow <yaml>
dmaf start --workflow <yaml> --task "…"

# 6. 交付与分发
dmaf push <目标工程dir>                                   # 站在工作流项目侧交付到目标工程（add 的镜像，同一合并核心）
dmaf auth login --token <dmafmt_…> [--market <base>]     # market 登录（凭据落用户级配置目录；logout/status 同组）
dmaf publish [<dmaf目录>] [--intro <文件>] [--dry-run] [--bump]   # 发布到 market：编译+doctor 门禁 → 确定性打包 → 上传（更新 = 升版重发）
```

全部命令的正常输出为 JSON（stdout，UTF-8），错误为 JSON（stderr）；退出码 `0` 成功、`2` 用法错误、`3` schema/定义校验失败、`4` 状态冲突、`5` 文件系统错误。

### 工作流变量

工作流定义写在会话开始之前，而需求原文、会话身份、业务参数都是运行期才存在的信息——**变量机制就是二者之间的桥**：定义时在模板里写名字，启动后引擎把值渲染进去。stage 工件路径、`task_description`、委托报文、交付文案共用同一份会话变量集，声明一次、四处生效。在工作流定义里引用变量前，需要先理解名字从哪来：

- **业务变量**（scope `vars`）：跑流程的人才知道的参数，如样例的 `topic`（产物文件名前缀）。空值 = 必填，`dmaf start --var topic=cart` 提供；带字面值 = 内联默认，可不传。
- **常量**（scope `constants`）：定义期就固定的字面值，模板可引用；`--var` 无法覆盖，改常量 = 改定义。
- **引擎保留变量**：`SESSION_ID`（会话 ID，产物路径按会话分桶，fork 后自动切到新桶）与 `TASK`（需求原文，仅由流程首个 stage 的 `task_description` 承接——下游 stage 一律消费上游工件）。

`dmaf start` 时引擎按序合成变量集：常量 → `vars` 内联默认 → `--var 提供值` → 保留变量；模板引用了未声明的名字，编译期即报错，不会带病运行。完整规则见 [docs/zh/流程定义指南.md](docs/zh/流程定义指南.md) §4。

## 语言支持

- **定义层英文源**：随包分发的 agents/skills/工作流定义为英文撰写，服务所有语言的用户。
- **产出语言跟随任务语言**：需求原文用什么语言写，spec/hld 等全部交付物就用什么语言产出，并沿流水线保持一致；中途评审意见跟随你当前消息的语言，不改变工件正文语言。
- **conductor 跟随你的语言**：编排者面向你的全部表述跟随你最近一条消息的语言；引擎内部报文与审计记录恒为英文（协议层语言稳定，呈现层本地化）。
- **文档**：`docs/` 顶层为英文缺省版，中文版在 `docs/zh/`（中文为编辑真源）；CLI 提示消息跟随系统语言（中文系统 zh，其余 en），可用 `DMAF_LOCALE` / `start --locale` 显式锁定；烘焙进会话。

## install 语义

- **账本哈希门**：harness 目录根的 `dmaf-installed.yaml` 记录引擎写入每个目标时的指纹——目标缺失即安装，未改动的随源升级**自动更新**，用户改过的**跳过**并报告（`--force` 显式覆盖）；agent 与入口配置逐文件，skill 按 SKILL.md 所在目录整目录。报告 `installed/unchanged/updated/skipped/overwritten/pruned`。
- **嵌套能力矩阵**：opencode、cursor 支持递归发现，安装保留分类子目录（`skills/<分类>/<技能>/`）；其余 harness（claude/zcode/gemini/codex/dsh）按技能名拍平。
- **入口配置**：`dmaf/harness/<名>/` 下的文件逐文件落位到 harness 目录内（如 `.opencode/opencode.json`），走同一账本哈希门；不落项目根。
- `--prune` 清孤儿；`--check-only` 只读体检；doctor 把漂移分为 `would_change`（重跑 install 即同步）与 `modified`（用户改过，需 `--force`）。

## 项目结构

```
├── src/                       # 引擎核心（cargo 包 dmaf-engine，二进制 dmaf）
│   ├── workflow.rs / router.rs / state.rs / audit.rs / reports.rs
│   ├── registry.rs            #   harness 注册表（名称→目录/布局/skills 嵌套能力位）
│   ├── compile.rs / install.rs#   编译与资产安装（账本哈希门 + 自动探测 + 入口配置）
│   ├── manifest.rs / package.rs #   项目清单（doctor 兼容判定）/ 工作流包合并核心（add/push）
│   ├── market.rs              #   market 对接（auth/publish/add 条目 URL：HTTP 底座、确定性打包、安全解包）
│   └── forking.rs / schema_check.rs / cli.rs / paths.rs / scaffold.rs / hooks.rs
├── dmaf/                      # 定义层（入库）：示例工作流与 AI 资产的唯一真源（可整体替换）
│   ├── workflows/  agents/  skills/   # stage 库 + scope 网格 / agent / 技能（按能力域分类）
│   ├── docs/流程定义指南.md    #   用户指南（单源，dmaf new 内嵌写入用户项目）
│   ├── harness/  manifest.yaml  hooks.yaml / compiled/
├── templates/                 # dmaf new 骨架模板（仅 minimal；include_dir 内嵌二进制）
├── .dmaf/                     # 运行层（gitignore）：引擎独占的会话状态与审计
├── docs/                      # 用户文档（英文缺省：engine-manual / flow-definition-guide / phase-stage-scope + schemas/；中文在 docs/zh/）
├── docs-dev/                  # 内部开发档案（gitignore，不发布）
├── tools/skills/              # 定义技能源（create-workflow：问答式 + 文档式；include_dir 内嵌，dmaf new 下发到目标 dmaf/skills/）
├── tests/                     # cargo test（180 例，全离线）
└── Cargo.toml / Cargo.lock / .cargo/config.toml
```

## 布局分界线

| 位置 | 性质 | 版本控制 |
|------|------|---------|
| `dmaf/` | 定义层：工作流定义、AI 资产、hook 配置、编译产物 | **全部入库**（`compiled/` 受 `--check` 守卫） |
| `.dmaf/` | 运行层：`<时间戳>/` 会话目录（状态、审计），引擎独占，agent 与人都不要读写 | gitignore 整目录 |
| harness 目录（`.opencode` 等） | install 派生物 | 引擎管理，勿手改 |
| 业务产物 | 完整路径写在 stage 的 `inputs`/`outputs`（支持 `{SESSION_ID}` 会话分桶；样例用 `dmaf-output/`，固定前缀避免与用户目录冲突） | 用户项目自行决定（工作成果，引擎不写入 .gitignore） |

工作流变更闭环：改 `dmaf/workflows/` 源 → `dmaf compile` → 提交 `dmaf/compiled/`（diff 即成员变更评审）→ `dmaf compile --check` 守护漂移。

AI 资产变更闭环：改 `dmaf/agents/`、`dmaf/skills/` → `dmaf install --harness <name>` → `dmaf check --harness <name>` 校验引用完整性。日常自检：git pull 后 `dmaf doctor --harness <name>`，按提示修复。

## 文档

| 文档 | 内容 |
|------|------|
| [docs/zh/引擎手册.md](docs/zh/引擎手册.md)（英文：[engine-manual](docs/engine-manual.md)） | 全部命令的参数/退出码/语义细则（引擎侧最全） |
| [docs/zh/流程定义指南.md](docs/zh/流程定义指南.md)（英文：[flow-definition-guide](docs/flow-definition-guide.md)） | stage/scope 编写指南（单源，`dmaf new` 内嵌写入用户项目） |
| [docs/zh/phase、stage、scope定义.md](docs/zh/phase、stage、scope定义.md)（英文：[phase-stage-scope](docs/phase-stage-scope.md)） | 样例 phase/stage/scope 权威清单 |
| [AGENTS.md](AGENTS.md) | 本仓库开发须知（结构、命令、约定与陷阱） |

## 构建与依赖

Rust 工具链（rustup 1.85+）

```bash
cargo build --release   # target/release/dmaf
cargo test
```

## 许可

双许可：[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)，可任选其一使用。
