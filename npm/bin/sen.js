#!/usr/bin/env node
// SenAgent CLI 启动器：按当前平台定位子包中的二进制并转发执行。
// 子包通过 optionalDependencies 安装，npm 只会拉取匹配 os/cpu 的那一个。
'use strict';

const { spawn } = require('node:child_process');
const path = require('node:path');

const SCOPE = '@senknight';
const PACKAGES = {
  'linux-x64': `${SCOPE}/sen-linux-x64-gnu`,
  'darwin-arm64': `${SCOPE}/sen-darwin-arm64`,
  'win32-x64': `${SCOPE}/sen-win32-x64-msvc`,
};

const key = `${process.platform}-${process.arch}`;
const pkg = PACKAGES[key];
if (!pkg) {
  console.error(
    `sen: 暂不支持的平台 ${key}（当前提供 linux-x64 / darwin-arm64 / win32-x64）；` +
      '请从 https://github.com/SenKnight/SenAgent/releases 手动下载对应二进制',
  );
  process.exit(1);
}

let binary;
try {
  const pkgRoot = path.dirname(require.resolve(`${pkg}/package.json`));
  binary = path.join(pkgRoot, 'bin', process.platform === 'win32' ? 'sen.exe' : 'sen');
} catch {
  console.error(
    `sen: 未找到平台包 ${pkg}（可能因 npm 可选依赖被跳过）；` +
      `请重试 npm i -g ${SCOPE}/sen，或从 https://github.com/SenKnight/SenAgent/releases 手动下载`,
  );
  process.exit(1);
}

const child = spawn(binary, process.argv.slice(2), { stdio: 'inherit' });
child.on('error', (err) => {
  console.error(`sen: 启动失败: ${err.message}`);
  process.exit(1);
});
child.on('exit', (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 1);
});
