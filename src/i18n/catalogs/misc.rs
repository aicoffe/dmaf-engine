//! 其余模块（install/package/manifest/registry/hooks/reports/forking/schema_check/paths/lib/scaffold）
//! 的人面文案目录（迁移批次维护）。

pub static MSGS: &[(&str, &str, &str)] = &[
    // ---------- install.rs ----------
    // ("install.key", "zh 与 0.6.x 原文案逐字节一致（{} 占位）", "en translation"),
    (
        "install.ledger_header",
        "# dmaf 安装账本（引擎写入与维护，请勿手改）：记录每个已装资产写入时的指纹，\n# 重跑 install 时据此判定目标侧是否被改动——未改动自动更新，改过则跳过（--force 覆盖）。\n",
        "# dmaf install ledger (written and maintained by the engine; do not edit by hand): records the fingerprint captured when each installed asset was written.\n# On re-running install, the engine uses it to decide whether a target was modified — unmodified targets update automatically, modified ones are skipped (--force overwrites).\n",
    ),
    (
        "install.std_dir_missing",
        "标准目录不存在: {}",
        "standard directory does not exist: {}",
    ),
    (
        "install.agent_missing_frontmatter",
        "agents/{} 缺少 YAML frontmatter",
        "agents/{} is missing YAML frontmatter",
    ),
    (
        "install.agent_missing_name",
        "agents/{} 缺少 name",
        "agents/{} is missing name",
    ),
    (
        "install.agent_name_dup",
        "agent name '{}' 重复: {} 与 {}",
        "duplicate agent name '{}': {} vs {}",
    ),
    (
        "install.agent_bad_mode",
        "agents/{} mode 必须是 primary|subagent|all，得到 '{}'",
        "agents/{} mode must be primary|subagent|all, got '{}'",
    ),
    (
        "install.agent_missing_description",
        "agents/{} 缺少非空 description",
        "agents/{} is missing a non-empty description",
    ),
    (
        "install.agent_toml_fail",
        "agents/{} 无法转换为 TOML: {}",
        "agents/{} cannot be converted to TOML: {}",
    ),
    (
        "install.skill_unreadable",
        "skills/{} 缺少 SKILL.md 且无法读取子目录",
        "skills/{} is missing SKILL.md and its subdirectories cannot be read",
    ),
    (
        "install.skill_no_entry",
        "skills/{} 既无 SKILL.md 也无子技能",
        "skills/{} has neither SKILL.md nor sub-skills",
    ),
    (
        "install.skill_nested_md_missing",
        "skills/{}/{} 缺少 SKILL.md（分类目录最多一层，技能目录须直接含 SKILL.md）",
        "skills/{}/{} is missing SKILL.md (category directories allow at most one level; a skill directory must contain SKILL.md directly)",
    ),
    (
        "install.skill_name_dup",
        "技能名 '{}' 重复：{} 与 {}（安装目标名须全局唯一）",
        "duplicate skill name '{}': {} vs {} (installed skill names must be globally unique)",
    ),
    (
        "install.skill_name_mismatch",
        "{}/SKILL.md 的 name '{}' 与目录名 '{}' 不一致",
        "{}/SKILL.md name '{}' does not match directory name '{}'",
    ),
    (
        "install.entry_config_no_dir",
        "无 dmaf/harness/<名>/ 入口配置目录",
        "no dmaf/harness/<name>/ entry config directory",
    ),
    (
        "install.toml_body_delimiter",
        "正文包含 ''' 序列，无法用 TOML literal 多行字符串无损承载",
        "body contains a ''' sequence, which cannot be carried losslessly in a TOML literal multi-line string",
    ),
    (
        "install.toml_control_char",
        "含控制字符 U+{}，无法写入 TOML basic string",
        "contains control character U+{}, which cannot be written into a TOML basic string",
    ),
    // ---------- package.rs ----------
    (
        "package.ledger_header",
        "# dmaf 包账本（引擎写入与维护，请勿手改；误删后重跑 dmaf add/push 可重建）。\n# 记录每个已合入工作流包的包属文件哈希，供 add/push 重跑时的更新规则判定：目标未改动 → 可更新。\n",
        "# dmaf package ledger (written and maintained by the engine; do not edit by hand; if deleted by accident, re-run dmaf add/push to rebuild it).\n# Records the package-owned file hashes of every merged workflow package, used by the update rule when add/push re-runs: an unmodified target can be updated.\n",
    ),
    (
        "package.target_dir_missing",
        "目标工程目录不存在: {}",
        "target project directory does not exist: {}",
    ),
    (
        "package.push_manifest_required",
        "push 需要包身份清单 dmaf/manifest.yaml（含非空 name）——dmaf new 模板自带，缺失时请补齐",
        "push requires the package identity manifest dmaf/manifest.yaml (with a non-empty name) — included in the dmaf new template; create it if missing",
    ),
    (
        "package.source_artifact_drift",
        "源侧产物漂移：push 前请先 dmaf compile 并提交产物（dmaf/compiled/{} 与重编译结果不一致）",
        "source-side artifact drift: run dmaf compile and commit the artifacts before push (dmaf/compiled/{} differs from a fresh compile)",
    ),
    (
        "package.missing_dmaf_layer",
        "缺少 dmaf/ 定义层（{}）",
        "missing the dmaf/ definition layer ({})",
    ),
    (
        "package.manifest_name_empty",
        "dmaf/manifest.yaml 缺少非空 name",
        "dmaf/manifest.yaml is missing a non-empty name",
    ),
    (
        "package.manifest_missing",
        "缺少 dmaf/manifest.yaml（dmaf new 模板自带）；包身份需要它",
        "dmaf/manifest.yaml is missing (included in the dmaf new template); package identity requires it",
    ),
    (
        "package.source_invalid",
        "源不是合法工作流项目：{}",
        "source is not a valid workflow project: {}",
    ),
    (
        "package.same_dir",
        "源与目标是同一目录",
        "source and target are the same directory",
    ),
    (
        "package.target_unavailable",
        "目标工程目录不可用: {}",
        "target project directory is unavailable: {}",
    ),
    (
        "package.dryrun_compile_skip",
        "dry-run 不写产物；实际执行时会重编译并校验",
        "dry-run writes no artifacts; a real run recompiles and verifies",
    ),
    (
        "package.dryrun_install_skip_force",
        "dry-run 不部署；实际执行时将自动探测 harness 并安装，--force 下对 harness 内用户改动同样强制覆盖",
        "dry-run does not deploy; a real run auto-detects harnesses and installs; with --force, user modifications inside the harness are force-overwritten as well",
    ),
    (
        "package.dryrun_install_skip",
        "dry-run 不部署；实际执行时将自动探测 harness 并安装",
        "dry-run does not deploy; a real run auto-detects harnesses and installs",
    ),
    (
        "package.mkdir_skeleton_fail",
        "创建骨架目录 {} 失败: {}",
        "failed to create skeleton directory {}: {}",
    ),
    (
        "package.compile_skip_no_stages",
        "无 stage 库（纯资产包）",
        "no stage library (pure asset package)",
    ),
    (
        "package.dryrun_chain_skip",
        "dry-run 不部署",
        "dry-run does not deploy",
    ),
    (
        "package.no_harness_found",
        "未发现已安装的 harness 目录（.opencode / .claude / .zcode / .cursor / .gemini / .codex / .dsh）；安装任一 harness 后运行 dmaf install",
        "no installed harness directory found (.opencode / .claude / .zcode / .cursor / .gemini / .codex / .dsh); install any harness first, then run dmaf install",
    ),
    (
        "package.entry_config_fail",
        "合并已完成，仅入口配置落位失败: {}",
        "merge completed; only entry config placement failed: {}",
    ),
    (
        "package.multi_harness",
        "发现多个 harness 目录（{}）；请运行 dmaf install --harness <名称> 指定，或 --harness all 全部安装",
        "multiple harness directories found ({}); run dmaf install --harness <name> to pick one, or --harness all to install all of them",
    ),
    (
        "package.install_chain_fail",
        "合并已完成，仅 harness 部署失败（{}）: {}",
        "merge completed; only harness deployment failed ({}): {}",
    ),
    (
        "package.git_clone_exec_fail",
        "执行 git clone 失败（git 须在 PATH 中）: {}",
        "failed to execute git clone (git must be on PATH): {}",
    ),
    (
        "package.git_clone_fail",
        "git clone 失败（{}）: {}",
        "git clone failed ({}): {}",
    ),
    (
        "package.ledger_parse_fail",
        "{} 无法解析（该文件由引擎维护，可删除后重跑 add/push 重建）: {}",
        "{} cannot be parsed (this file is maintained by the engine; delete it and re-run add/push to rebuild): {}",
    ),
    (
        "package.ledger_serialize_fail",
        "账本序列化失败: {}",
        "failed to serialize the ledger: {}",
    ),
    // ---------- manifest.rs ----------
    (
        "manifest.read_fail",
        "读取 {} 失败: {}",
        "failed to read {}: {}",
    ),
    (
        "manifest.invalid_yaml",
        "dmaf/manifest.yaml 不是合法 YAML: {}",
        "dmaf/manifest.yaml is not valid YAML: {}",
    ),
    (
        "manifest.top_not_mapping",
        "dmaf/manifest.yaml 顶层必须是映射",
        "dmaf/manifest.yaml top level must be a mapping",
    ),
    (
        "manifest.current_version_unparsable",
        "当前版本无法解析 '{}': {}",
        "cannot parse the current version '{}': {}",
    ),
    (
        "manifest.range_term_unparsable",
        "engine 范围项 '{}' 无法解析: {}",
        "cannot parse engine range term '{}': {}",
    ),
    (
        "manifest.range_empty",
        "engine 范围为空",
        "engine range is empty",
    ),
    (
        "manifest.invalid_version",
        "非法版本号 '{}'（应为 MAJOR.MINOR[.PATCH]）",
        "invalid version '{}' (expected MAJOR.MINOR[.PATCH])",
    ),
    (
        "manifest.segment_not_int",
        "版本段 '{}' 不是非负整数",
        "version segment '{}' is not a non-negative integer",
    ),
    // ---------- registry.rs ----------
    (
        "registry.unknown_harness",
        "未知 harness '{}'；可用名称：opencode | claude | zcode | cursor | gemini | codex | dsh（或直接传入 harness 目录路径）",
        "unknown harness '{}'; available names: opencode | claude | zcode | cursor | gemini | codex | dsh (or pass a harness directory path directly)",
    ),
    (
        "registry.reviewer_missing",
        "stage '{}' 引用的评审者 agent '{}' 不存在于注册表",
        "stage '{}' references reviewer agent '{}' that does not exist in the registry",
    ),
    (
        "registry.reviewer_bad_mode",
        "评审者 agent '{}' mode='{}'，工作流只能派发 subagent",
        "reviewer agent '{}' has mode='{}'; workflows may only dispatch subagents",
    ),
    (
        "registry.agent_missing",
        "stage '{}' 引用的 agent '{}' 不存在于注册表",
        "stage '{}' references agent '{}' that does not exist in the registry",
    ),
    (
        "registry.agent_bad_mode",
        "agent '{}' mode='{}'，工作流只能派发 subagent",
        "agent '{}' has mode='{}'; workflows may only dispatch subagents",
    ),
    (
        "registry.skill_missing",
        "agent '{}' 引用的 skill '{}' 不存在于注册表",
        "agent '{}' references skill '{}' that does not exist in the registry",
    ),
    // ---------- hooks.rs ----------
    (
        "hooks.empty_command",
        "{} 空命令，已忽略",
        "{}: empty command, ignored",
    ),
    (
        "hooks.item_not_string",
        "{} 列表项必须是字符串，已忽略该项",
        "{}: list items must be strings; this item was ignored",
    ),
    (
        "hooks.command_type_invalid",
        "{} 必须是字符串或字符串列表: {}",
        "{} must be a string or a list of strings: {}",
    ),
    (
        "hooks.command_list_required",
        "{} 必须是命令列表",
        "{} must be a list of commands",
    ),
    (
        "hooks.yaml_parse_fail",
        "hooks.yaml 解析失败，本次忽略全部 hook: {}",
        "failed to parse hooks.yaml; all hooks are ignored for this run: {}",
    ),
    (
        "hooks.timeout_invalid",
        "hooks.yaml timeout_secs 必须是正整数，按缺省 {}s 处理",
        "hooks.yaml timeout_secs must be a positive integer; falling back to the default of {}s",
    ),
    (
        "hooks.unknown_event_type",
        "hooks.yaml on-event 含未知事件类型 '{}'（合法键为审计事件词表），已忽略",
        "hooks.yaml on-event contains unknown event type '{}' (valid keys are the audit event vocabulary); ignored",
    ),
    (
        "hooks.on_event_not_mapping",
        "hooks.yaml on-event 必须是 事件类型 → 命令列表 的映射",
        "hooks.yaml on-event must be a mapping of event type → command list",
    ),
    (
        "hooks.spawn_fail",
        "hook 启动失败，已跳过: {}: {}",
        "failed to start hook, skipped: {}: {}",
    ),
    (
        "hooks.timeout_killed",
        "hook 超时被终止（{}s），继续: {}",
        "hook timed out and was killed ({}s), continuing: {}",
    ),
    // ---------- reports.rs ----------
    (
        "reports.payload_schema_fail",
        "payload schema 校验失败: {}",
        "payload schema validation failed: {}",
    ),
    (
        "reports.action_not_found",
        "action 不存在: {}",
        "action does not exist: {}",
    ),
    (
        "reports.action_already_final",
        "action 已终结（{}），不可重复 report: {}",
        "action already finalized ({}); cannot report it again: {}",
    ),
    (
        "reports.status_invalid",
        "status 必须是 {} 之一",
        "status must be one of {}",
    ),
    (
        "reports.skipped_needs_reason",
        "status 为 skipped 时必须提供非空 reason",
        "a non-empty reason is required when status is skipped",
    ),
    (
        "reports.missing_outputs",
        "期望产出物缺失: {}",
        "expected artifacts missing: {}",
    ),
    (
        "reports.retry_exhausted",
        "stage '{}' 重试耗尽",
        "stage '{}' exhausted its retries",
    ),
    (
        "reports.gate_not_awaiting",
        "没有等待审批门 '{}' 的 stage",
        "no stage is waiting at approval gate '{}'",
    ),
    (
        "reports.decision_invalid",
        "decision 必须是 granted 或 rejected",
        "decision must be granted or rejected",
    ),
    (
        "reports.transitive_skip_reason",
        "传递跳过：硬输入的生产者已跳过（on_skip={}）",
        "transitive skip: producers of hard inputs were all skipped (on_skip={})",
    ),
    // ---------- forking.rs ----------
    (
        "forking.read_audit_fail",
        "读源审计目录失败: {}",
        "failed to read the source audit directory: {}",
    ),
    (
        "forking.shard_not_found",
        "源会话不存在 seq={} 的审计分片（现有 1..{}）",
        "the source session has no audit shard at seq={} (existing 1..{})",
    ),
    (
        "forking.chain_incomplete",
        "源审计链在 seq<={} 范围内不完整: {}",
        "the source audit chain is incomplete within seq<={}: {}",
    ),
    (
        "forking.replay_fail",
        "分片回放失败: {}",
        "failed to replay shards: {}",
    ),
    (
        "forking.in_flight_actions",
        "分叉点存在在途 action（须先全部终结）: {}",
        "in-flight actions at the fork point (all must be finalized first): {}",
    ),
    (
        "forking.mkdir_session_fail",
        "建会话目录失败: {}",
        "failed to create the session directory: {}",
    ),
    (
        "forking.copy_shard_fail",
        "复制审计分片失败: {}",
        "failed to copy an audit shard: {}",
    ),
    (
        "forking.mkdir_audit_fail",
        "建审计目录失败: {}",
        "failed to create the audit directory: {}",
    ),
    (
        "forking.mkdir_artifact_fail",
        "建产物目录失败 {}: {}",
        "failed to create the artifact directory {}: {}",
    ),
    (
        "forking.copy_artifact_fail",
        "复制产物失败 {} -> {}: {}",
        "failed to copy artifact {} -> {}: {}",
    ),
    (
        "forking.write_state_fail",
        "写状态文件失败: {}",
        "failed to write the state file: {}",
    ),
    // ---------- scaffold.rs ----------
    (
        "scaffold.gitignore_group_header",
        "# dmaf 引擎运行区与 harness 派生物",
        "# dmaf engine runtime area and harness derivatives",
    ),
    (
        "scaffold.unknown_template",
        "未知模板 '{}'，可用: {}",
        "unknown template '{}', available: {}",
    ),
];
