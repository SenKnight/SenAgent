#!/usr/bin/env node
// 版本号单一来源：写入（发版）或校验（CI）。
//
// 用法:
//   node scripts/sync-version.mjs 0.1.5      # 写入所有版本位置（可带 v 前缀）
//   node scripts/sync-version.mjs            # 版本号取自 GITHUB_REF_NAME / git describe
//   node scripts/sync-version.mjs --check    # 仅校验，不一致时退出码 1（CI 使用）
//
// 同步位置:
//   Cargo.toml（workspace，sen-core/sen-server/sen-cli 继承）
//   crates/sen-desktop/Cargo.toml、crates/sen-desktop/tauri.conf.json（决定安装包命名）
//   frontend/package.json
//   npm/package.json（主包 + optionalDependencies 平台子包）+ npm/<platform>/package.json
//
// 发版流程: node scripts/sync-version.mjs <版本> → commit → git tag v<版本> → push
// CI 只校验 tag 与源码版本一致，不再改写源码，保证 tag 内容与版本号一一对应。

import { execSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SCOPE = '@senknight';
const CHECK = process.argv.includes('--check');
const args = process.argv.slice(2).filter((a) => !a.startsWith('--'));

function resolveVersion() {
  if (args[0]) return args[0];
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
  console.error('用法: node scripts/sync-version.mjs <版本> [--check]  例如: node scripts/sync-version.mjs 0.1.5');
  process.exit(1);
}

// 每项 = 文件 + 三段捕获组（前缀 / 版本 / 后缀）。
// write 模式把中间组替换为 version；check 模式比对中间组并报告不一致。
const targets = [
  { file: 'Cargo.toml', re: /^(version = ")([^"]+)(")/m },
  { file: 'crates/sen-desktop/Cargo.toml', re: /^(version = ")([^"]+)(")/m },
  { file: 'crates/sen-desktop/tauri.conf.json', re: /("version":\s*")([^"]+)(")/ },
  { file: 'frontend/package.json', re: /("version":\s*")([^"]+)(")/ },
  { file: 'npm/package.json', re: /("version":\s*")([^"]+)(")/ },
  // 主包 optionalDependencies 的平台子包版本：与主包不一致时 npm 会静默跳过可选依赖。
  // 内部包名必须用非捕获组，否则捕获组编号错位会把包名当版本写进值里。
  { file: 'npm/package.json', re: new RegExp(`("${SCOPE}/(?:sen-[^"]+)":\\s*")([^"]+)(")`, 'g') },
];

// npm 平台子包（目录动态发现）
const npmDir = join(ROOT, 'npm');
if (existsSync(npmDir)) {
  for (const dir of readdirSync(npmDir)) {
    if (existsSync(join(npmDir, dir, 'package.json'))) {
      targets.push({ file: `npm/${dir}/package.json`, re: /("version":\s*")([^"]+)(")/ });
    }
  }
}

const changed = [];
const problems = [];

for (const { file, re } of targets) {
  const path = join(ROOT, file);
  if (!existsSync(path)) continue;
  const src = readFileSync(path, 'utf8');
  const global = new RegExp(re.source, re.flags.includes('g') ? re.flags : `${re.flags}g`);
  const matches = [...src.matchAll(global)];

  // 一处都匹配不到说明正则与文件结构已脱节，必须暴露（否则会静默漏改版本号）
  if (matches.length === 0) {
    problems.push(`${file}: 未匹配到版本字段（${re.source}）`);
    continue;
  }

  if (CHECK) {
    for (const m of matches) {
      if (m[2] !== version) problems.push(`${file}: ${m[1].trim()} 当前 "${m[2]}"（应为 ${version}）`);
    }
  } else {
    const out = src.replace(global, (_m, pre, _v, post) => `${pre}${version}${post}`);
    if (out !== src) {
      writeFileSync(path, out);
      changed.push(file);
    }
  }
}

if (problems.length > 0) {
  console.error(CHECK ? `版本校验失败（源码版本应为 ${version}）：` : `版本同步异常（版本应为 ${version}）：`);
  for (const p of problems) console.error(`  - ${p}`);
  if (CHECK) console.error(`\n发版前先执行: node scripts/sync-version.mjs ${version}`);
  process.exit(1);
}

if (CHECK) {
  console.log(`版本校验通过：所有位置均为 ${version}`);
} else if (changed.length === 0) {
  console.log(`版本已是 ${version}，无改动`);
} else {
  console.log(`已同步版本 ${version}:`);
  for (const f of changed) console.log(`  - ${f}`);
}
