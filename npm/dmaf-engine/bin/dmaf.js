#!/usr/bin/env node
"use strict";
// @aicoffe/dmaf-engine bin shim: locate the platform package's binary and exec it.
// Design: docs-dev/引擎安装与分发实施方案.md §6.5 (D6 拍板 esbuild 多包模式——二进制随平台包预置分发，不做安装期下载)。
const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const PLATFORMS = {
  "win32-x64": "@aicoffe/dmaf-engine-win32-x64",
  "darwin-arm64": "@aicoffe/dmaf-engine-darwin-arm64",
  "darwin-x64": "@aicoffe/dmaf-engine-darwin-x64",
  "linux-x64": "@aicoffe/dmaf-engine-linux-x64",
  "linux-arm64": "@aicoffe/dmaf-engine-linux-arm64",
};

const key = `${process.platform}-${process.arch}`;
const pkg = PLATFORMS[key];
if (!pkg) {
  console.error(`dmaf: unsupported platform: ${key}`);
  process.exit(1);
}

let binPath;
try {
  const pkgRoot = path.dirname(require.resolve(`${pkg}/package.json`));
  binPath = path.join(pkgRoot, "bin", process.platform === "win32" ? "dmaf.exe" : "dmaf");
} catch (e) {
  console.error(`dmaf: platform package ${pkg} is not installed (optionalDependencies skipped for this platform?)`);
  process.exit(1);
}
if (!fs.existsSync(binPath)) {
  console.error(`dmaf: binary not found in ${pkg}: ${binPath}`);
  process.exit(1);
}

const r = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });
if (r.error) {
  console.error(`dmaf: failed to run ${binPath}: ${r.error.message}`);
  process.exit(1);
}
process.exit(r.status === null ? 1 : r.status);
