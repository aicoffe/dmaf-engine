# npm 包装（@aicoffe/dmaf）

引擎二进制的 npm 分发通道。设计见 `docs-dev/引擎安装与分发实施方案.md` §6.5（D6 拍板：**esbuild 多包模式**——二进制随平台包预置分发、安装期零下载；明确不用 dist 自带的 npm 生成器，其形态是安装期运行时下载 `binary-install.js`）。

## 形态

- 主包 `@aicoffe/dmaf`：`bin/dmaf.js` shim 按当前平台解析平台包内二进制并 exec；平台二进制声明在 `optionalDependencies`（npm 按 `os`/`cpu` 字段自动跳过不匹配平台）。
- 平台包 `@aicoffe/dmaf-{win32-x64, darwin-arm64, darwin-x64, linux-x64, linux-arm64}`：各含单文件二进制于 `bin/`；linux 侧直接复用 musl 静态二进制（glibc 系统可用，无需双份）。
- `npm i -g @aicoffe/dmaf` 落 npm 全局 bin（npm 安装时已入 PATH），零 PATH 操作。

## 发布动线（每次发版）

1. 前置（一次性）：创建 npm org `aicoffe`（D8 核查 scope 未被注册）；CI token 或 2FA 就绪。
2. tag 触发 release workflow 产出五平台产物到 `target/distrib/`（GitHub Release 同时托管）。
3. 装配：`node npm/build.cjs <版本>` —— 从 `target/distrib/` 复制二进制、生成 `npm/pkg/<平台>/` 平台包、统一改写主包与依赖版本（**严格同版纪律**）。
4. 发布（顺序：平台包先、主包后）：
   ```bash
   for d in npm/pkg/*/; do (cd "$d" && npm publish --access public); done
   cd npm/dmaf && npm publish --access public
   ```
5. 发布后自检（M1-6 端到端验证）：干净目录 `npm i -g @aicoffe/dmaf` → `dmaf --version` → `dmaf doctor`。
6. 国内可达：npmmirror 对新包自动同步（首次可手动触发 sync）。

## 版本占位

仓库内 `npm/dmaf/package.json` 版本恒为 `0.0.0-dev`，发布前由 `build.mjs` 按实际版本改写——仓库不跟踪发版号，避免与引擎 `Cargo.toml` 版本漂移。

## 自动化边界

发布动作（步骤 4-5）当前为手工执行（需 npm 凭据）；并入 release workflow 自动化属 M2 范围（方案 §7 M2-3 收尾项）。
