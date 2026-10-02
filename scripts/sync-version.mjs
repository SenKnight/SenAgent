#!/usr/bin/env node
// 版本号单一来源同步：把 tag / 参数中的版本写入所有需要版本的位置。
//
// 用法:
//   node scripts/sync-version.mjs 0.1.3   # 显式指定（可带 v 前缀）
//   node scripts/sync-version.mjs         # 从 tag 推导：GITHUB_REF_NAME 或 git describe
//
// 同步位置：
//   Cargo.toml（workspace，sen-core/sen-server/sen-cli 继承）
//   crates/sen-desktop/Cargo.toml、crates/sen-desktop/tauri.conf.json（安装包命名）
//   frontend/package.json
//   npm/package.json（主包，含 optionalDependencies 版本）+ npm/<platform>/package.json

import { execSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SCOPE = '@senknight';

function resolveVersion() {
  const arg = process.argv[2];
  if (arg) return arg;
  if (process.env.GITHUB_REF_NAME) return process.env.GITHUB_REF_NAME;
  try {
    return execSync('git describe --tags --exact-match', { cwd: ROOT, stdio: ['ignore', 'pipe', 'ignore'] })
      .toString()
      .trim();
  } catch {
    return '';
  }
}

const version = resolveVersion().replace(/^v/, '');
if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error('用法: node scripts/sync-version.mjs <version>  例如: node scripts/sync-version.mjs 0.1.3');
  process.exit(1);
}

const changed = [];

/** 对文件做正则替换；内容无变化时跳过（幂等）。replacement 可为字符串或函数。 */
function patch(relPath, pattern, replacement) {
  const file = join(ROOT, relPath);
  if (!existsSync(file)) return;
  const src = readFileSync(file, 'utf8');
  const out = src.replace(pattern, replacement);
  if (out !== src) {
    writeFileSync(file, out);
    changed.push(relPath);
  }
}

// Rust workspace（sen-core / sen-server / sen-cli 均 version.workspace = true）
patch('Cargo.toml', /^version = "[^"]+"/m, `version = "${version}"`);

// 桌面端：crate 版本 + Tauri 打包版本（决定 msi/dmg/deb/AppImage 文件名）
patch('crates/sen-desktop/Cargo.toml', /^version = "[^"]+"/m, `version = "${version}"`);
patch('crates/sen-desktop/tauri.conf.json', /"version":\s*"[^"]+"/, `"version": "${version}"`);

// 前端
patch('frontend/package.json', /"version":\s*"[^"]+"/, `"version": "${version}"`);

// npm 主包：自身版本 + optionalDependencies 中的平台子包版本
patch('npm/package.json', /"version":\s*"[^"]+"/, `"version": "${version}"`);
patch(
  'npm/package.json',
  new RegExp(`"${SCOPE}\\\\/(sen-[^"]+)":\\\\s*"[^"]+"`, 'g'),
  (m, name) => `"${SCOPE}/${name}": "${version}"`,
);

// npm 平台子包
const npmDir = join(ROOT, 'npm');
if (existsSync(npmDir)) {
  for (const dir of readdirSync(npmDir)) {
    const pkg = join(npmDir, dir, 'package.json');
    if (existsSync(pkg)) {
      patch(`npm/${dir}/package.json`, /"version":\s*"[^"]+"/, `"version": "${version}"`);
    }
  }
}

if (changed.length === 0) {
  console.log(`版本已是 ${version}，无改动`);
} else {
  console.log(`已同步版本 ${version}:`);
  for (const f of changed) console.log(`  - ${f}`);
}
