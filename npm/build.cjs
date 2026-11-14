// npm 平台包装配脚本：从 dist 归档产物装配 @aicoffe/dmaf-engine-* 平台包并统一改写版本号。
// 用法：node npm/build.cjs <版本(如 0.7.0)> [dist产物目录(默认 target/distrib)]
// 设计：docs-dev/引擎安装与分发实施方案.md §6.5（D6 拍板 esbuild 多包模式；linux 侧直接复用 musl 静态二进制）。
// 产物命名冻结为 dist 0.34 实际形态（§6.1）：dmaf-engine-<target>.tar.xz / .zip（无版本段）；归档经系统 tar 解包（bsdtar/GNU tar 均可，Windows 10+ 自带）。
"use strict";
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const ROOT = path.join(__dirname, "dmaf-engine");
const OUT = path.join(__dirname, "pkg");

// dist target triple → npm 平台包
const TARGETS = {
  "aarch64-apple-darwin": { slug: "darwin-arm64", exe: "dmaf", os: "darwin", cpu: "arm64" },
  "x86_64-apple-darwin": { slug: "darwin-x64", exe: "dmaf", os: "darwin", cpu: "x64" },
  "x86_64-unknown-linux-musl": { slug: "linux-x64", exe: "dmaf", os: "linux", cpu: "x64" },
  "aarch64-unknown-linux-musl": { slug: "linux-arm64", exe: "dmaf", os: "linux", cpu: "arm64" },
  "x86_64-pc-windows-msvc": { slug: "win32-x64", exe: "dmaf.exe", os: "win32", cpu: "x64", zip: true },
};

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+/.test(version)) {
  console.error("usage: node npm/build.cjs <version> [distrib-dir]");
  process.exit(2);
}
const distrib = path.resolve(process.argv[3] || "target/distrib");

const mainPkg = JSON.parse(fs.readFileSync(path.join(ROOT, "package.json"), "utf8"));
const repo = mainPkg.repository;
const platNames = [];

for (const [target, meta] of Object.entries(TARGETS)) {
  const archive = path.join(distrib, `dmaf-engine-${target}${meta.zip ? ".zip" : ".tar.xz"}`);
  if (!fs.existsSync(archive)) {
    console.error(`build.cjs: archive not found: ${archive} (run the release build first, or pass the distrib dir)`);
    process.exit(1);
  }
  const name = `@aicoffe/dmaf-engine-${meta.slug}`;
  const dir = path.join(OUT, meta.slug);
  const extractDir = path.join(dir, ".extract");
  fs.mkdirSync(extractDir, { recursive: true });
  // GNU tar reads "F:/..." as a remote host (colon); bsdtar (Windows/macOS) doesn't — forward slashes
  // everywhere and --force-local for GNU keep the invocation portable across shells on Windows.
  // GNU tar cannot read .zip at all — bsdtar tar -xf handles zips, otherwise unzip (Linux/macOS)
  // or PowerShell Expand-Archive (Windows)
  const tarPath = (p) => p.replace(/\\/g, "/");
  const tarVersion = spawnSync("tar", ["--version"], { encoding: "utf8" }).stdout || "";
  const gnuTar = /GNU tar/.test(tarVersion);
  let r = { status: 1 };
  if (!(meta.zip && gnuTar)) {
    const tarArgs = ["-xf", tarPath(archive), "-C", tarPath(extractDir)];
    if (gnuTar) tarArgs.unshift("--force-local");
    r = spawnSync("tar", tarArgs, { stdio: "inherit" });
  }
  if (r.status !== 0 && meta.zip) {
    r = process.platform === "win32"
      ? spawnSync("powershell", ["-NoProfile", "-Command", `Expand-Archive -LiteralPath '${archive}' -DestinationPath '${tarPath(extractDir)}' -Force`], { stdio: "inherit" })
      : spawnSync("unzip", ["-o", tarPath(archive), "-d", tarPath(extractDir)], { stdio: "inherit" });
  }
  if (r.status !== 0) {
    console.error(`build.cjs: tar -xf failed for ${archive} (xz-capable tar required; Windows 10+ built-in tar works)`);
    process.exit(1);
  }
  let binSrc = path.join(extractDir, meta.exe);
  if (!fs.existsSync(binSrc)) {
    // dist 0.34 nests tar.xz contents one level deep in <archive-basename>/ (zips stay flat)
    const nested = fs
      .readdirSync(extractDir, { withFileTypes: true })
      .filter((e) => e.isDirectory())
      .map((e) => path.join(extractDir, e.name, meta.exe))
      .find((p) => fs.existsSync(p));
    if (!nested) {
      console.error(`build.cjs: archive does not contain ./${meta.exe}: ${archive}`);
      process.exit(1);
    }
    binSrc = nested;
  }
  fs.mkdirSync(path.join(dir, "bin"), { recursive: true });
  fs.copyFileSync(binSrc, path.join(dir, "bin", meta.exe));
  fs.chmodSync(path.join(dir, "bin", meta.exe), 0o755);
  fs.rmSync(extractDir, { recursive: true, force: true });

  const manifest = {
    name,
    version,
    description: `dmaf engine binary for ${meta.slug}`,
    license: "MIT OR Apache-2.0",
    repository: repo,
    os: [meta.os],
    cpu: [meta.cpu],
    files: ["bin/"],
  };
  fs.writeFileSync(path.join(dir, "package.json"), JSON.stringify(manifest, null, 2) + "\n");
  platNames.push(name);
}

// 主包版本与 optionalDependencies 统一改写（严格同版纪律）
mainPkg.version = version;
for (const name of platNames) mainPkg.optionalDependencies[name] = version;
fs.writeFileSync(path.join(ROOT, "package.json"), JSON.stringify(mainPkg, null, 2) + "\n");

console.log(`assembled ${platNames.length} platform packages into npm/pkg/ (version ${version})`);
console.log("publish order: platform packages first, then npm/dmaf-engine (see npm/README.md)");
