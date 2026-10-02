# SenAgent

跨平台个人 AI Agent：CLI / Web / 桌面三端同核，Rust 全栈（内核 + 服务 + 壳），单二进制分发、零运行时依赖。

## 架构

```
CLI (clap, Rust)      Web 浏览器            桌面端 (Tauri 2)
    │                     │                     │
    │ 进程内直连            │ HTTP / WebSocket    │ 进程内直连（后端嵌入同进程）
    ▼                     ▼                     ▼
┌──────────────────────────────────────────────────────┐
│               sen-core Rust 内核（tokio）              │
│   Agent Loop │ Provider 层 │ 工具系统 │ 技能 │ 记忆    │
│          sen-server（axum，sen serve）                 │
└──────────────────────────────────────────────────────┘
        │                    │                │
     SQLite             MCP Servers      LLM Providers
  (会话/记忆/配置)       (外部工具)    (wire: chat / responses / Ollama)
```

- **sen-core**：内核库。Agent 主循环（流式 + 多轮工具调用）、Provider 双协议适配、工具系统、技能系统、AGENTS.md 指导提示词、SQLite 会话持久化。
- **sen-server**：axum 服务。WebSocket 聊天端点 + REST 会话 CRUD + 前端静态资源托管（磁盘目录或内嵌资源两种来源）；被 CLI（`sen serve` / `sen web`）与桌面端（进程内 `serve_on`）复用。
- **sen-cli**：`sen` 二进制，clap 命令，进程内直连内核（不绕 HTTP）。
- **sen-desktop**：Tauri 2 壳，启动时进程内起 axum（127.0.0.1 随机端口）并打开窗口指向本地服务 —— 单进程、单二进制、无 sidecar。
- **frontend**：React 19 + TS + Vite + Tailwind 4 + Zustand。Web 与桌面共用同一套前端代码与 HTTP/WS 协议。

## 快速开始

### 1. 构建并安装 CLI

```bash
# 方式一（推荐）：安装到 ~/.cargo/bin（rustup 默认已加入 PATH，全局可用 sen 命令）
cargo install --path crates/sen-cli

# 方式二：仅构建，手动把产物放进 PATH
cargo build --release -p sen-cli
cp target/release/sen ~/.local/bin/     # Windows：将 target\release 目录加入 PATH

# 方式三：npm（需 Node ≥18，自动按平台选择对应二进制）
npm install -g @senknight/sen
```

注意 `cargo build` 只是产出 `target/release/sen`，**不会注册命令**；不安装时需写完整路径调用（如 `./target/release/sen run "你好"`），或临时 `export PATH="$PWD/target/release:$PATH"`。安装后用 `sen --version` 验证；代码更新后重新执行 `cargo install --path crates/sen-cli` 覆盖即可。

### 2. 初始化配置

```bash
sen config init      # 写入 ~/.sen-agent/config.toml 模板
sen config path      # 打印配置文件路径
sen config show      # 查看当前生效配置（含已解析的 provider）
```

编辑 `~/.sen-agent/config.toml`（下方为首次运行自动生成的模板结构）：

```toml
default_provider = "deepseek"
context_window = 128000     # 上下文 token 近似预算（用于历史截断）
max_tool_rounds = 25        # 单轮用户输入的最大工具调用轮数


[[providers]]
name = "qoder"
base_url = "sungrow-of-enterprise.vpc.qoder.com.cn/v1"
api_key = "env:QODER_API_KEY"   # 支持 env:VAR 引用环境变量，也可直接填 "sk-..."
model = "deepseek-flash"
wire_api = "chat"                  # chat（/v1/chat/completions）或 responses（/v1/responses）

[[providers]]
name = "deepseek"
base_url = "https://api.deepseek.com/v1"
api_key = "env:DEEPSEEK_API_KEY"   # 支持 env:VAR 引用环境变量，也可直接填 "sk-..."
model = "deepseek-chat"
wire_api = "chat"                  # chat（/v1/chat/completions）或 responses（/v1/responses）

[[providers]]
name = "openai"
base_url = "https://api.openai.com/v1"
api_key = "env:OPENAI_API_KEY"
model = "gpt-5"
wire_api = "responses"             # OpenAI 原生 Responses API

[[providers]]
name = "ollama"
base_url = "http://localhost:11434/v1"
model = "qwen3:8b"
wire_api = "chat"                  # 本地模型免 API Key
```

已有配置项均可用环境变量覆盖：`SEN_PROVIDER` / `SEN_BASE_URL` / `SEN_API_KEY` / `SEN_MODEL` / `SEN_WIRE_API`。

也可以完全不动文件：启动 Web / 桌面端后打开右上角**设置**面板，图形化编辑 providers（名称 / Base URL / 模型 / wire_api 协议 / API Key）、默认 provider、上下文窗口、工具轮数上限与追加系统提示词，点「保存并生效」立即热更新（下一条消息即使用新配置，无需重启）。已保存的明文密钥不回显，留空保持不变、输入新值则替换（`env:VAR` 引用原样显示）。

### 3. 开始对话

```bash
sen run "读取 README 并总结"     # 单次问答（流式输出 + 工具调用）
sen chat                         # 交互式 REPL（/exit 退出，/new 新会话）
sen sessions                     # 列出历史会话
sen sessions show <id>           # 查看会话消息
sen --provider ollama run "你好" # 临时切换 provider
```

## CLI 命令

| 命令 | 说明 |
|---|---|
| `sen chat` | 交互式 REPL，流式渲染 token / 推理 / 工具调用 |
| `sen run <prompt>` | 单次问答，结束后退出 |
| `sen sessions [list\|show\|rename\|delete]` | 会话管理（支持 id 前缀匹配） |
| `sen config [show\|init\|path]` | 配置查看 / 初始化 / 路径 |
| `sen skill [list\|show\|new\|dir]` | 技能浏览 / 查看 / 创建 / 打开目录 |
| `sen serve [--host --port --static-dir]` | 启动 Web 服务（默认 :8642） |

## 内置工具

Agent 可调用以下内置工具（schemars 自动生成 JSON Schema）：

- `shell`：执行 shell 命令（超时与截断保护）
- `read_file` / `write_file` / `edit_file`：文件读取、写入、精确替换编辑
- `glob` / `grep`：按模式查找文件、正则搜索内容
- `web_fetch`：抓取网页并转为纯文本
- `load_skill` / `create_skill`：按需加载技能、创建新技能

## 技能（Skills）

遵循社区约定，技能放在 `~/.agents/skills/`（可用 `SEN_AGENTS_DIR` 覆盖），**支持任意层级子目录分组**：

```
~/.agents/skills/
├── coding/
│   ├── git-helper/SKILL.md
│   └── code-review/SKILL.md
└── writing/
    └── blog-post/SKILL.md
```

`SKILL.md` 使用 YAML frontmatter 声明元数据：

```markdown
---
name: git-helper
description: 处理 git 操作的最佳实践
---

技能正文（Markdown，包含步骤、示例、注意事项）...
```

**渐进式披露**：启动时仅将技能索引（名称 + 描述）注入系统提示词，Agent 按需通过 `load_skill` 加载全文，避免占用上下文。目录变化实时生效（每次扫描，无需重启）。

创建技能：

```bash
sen skill new coding/git-helper --description "处理 git 操作"   # CLI
sen skill list                                                  # 按分组列出
sen skill show git-helper                                       # 查看全文
```

也可以直接让 Agent 用 `create_skill` 工具创建。

## 指导提示词（AGENTS.md）

- **全局**：`~/.agents/AGENTS.md`
- **项目级**：从当前工作目录向上逐级查找 `AGENTS.md`（外层在前、内层在后叠加）

两处内容都会注入系统提示词，与内置人设、技能索引合并。

## Web 界面与服务

**终端用户：单文件模式（推荐）** —— CLI 二进制已内嵌前端产物，任意目录直接启动：

```bash
sen web                  # 启动 Web UI：http://127.0.0.1:8642
sen web --open           # 同时自动打开浏览器
```

**开发/自定义：目录托管模式** —— 自行构建前端并由 `sen serve` 从磁盘托管：

```bash
cd frontend
npm install
npm run build            # 产物 frontend/dist

cd ..                    # 回到项目根（sen serve 会自动探测 frontend/dist）
sen serve                # 浏览器打开 http://127.0.0.1:8642 即完整界面
```

两种模式都以**同源方式**同时提供页面、API 与 WebSocket（页面与 WS 都走 `:8642`）：

- `sen web`：前端在**编译期嵌入二进制**（`frontend/dist`，约 0.6MB），单文件即用、可离线分发；构建 CLI 时若前端产物缺失，会内嵌占位页并提示（先 `cd frontend && npm run build` 再重装 CLI 即可）
- `sen serve`：托管磁盘上的目录，支持前端热更新调试；在任意目录运行时用 `--static-dir` 指定产物位置：

```bash
sen serve --static-dir /path/to/SenAgent/frontend/dist
```

桌面端无需此步骤：`cargo tauri build` 会把前端产物作为内嵌资源一并打包（见下文）。

开发模式（Vite 热更新 + 代理到后端）：

```bash
sen serve &              # 后端 :8642
cd frontend && npm run dev   # 前端 :5173，/api 自动代理
```

WebSocket 协议（客户端 → 服务端）：

```json
{"type": "chat", "session_id": "…", "content": "…"}
{"type": "ping"}
```

服务端事件（与内核 `AgentEvent` 一致）：`ready` / `token` / `reasoning` / `tool_call` / `tool_result` / `done` / `error` / `pong`。

REST：`GET/POST /api/sessions`、`GET/PATCH/DELETE /api/sessions/{id}`、`GET /api/health`、`GET /api/config`、`GET/PUT /api/settings`（交互式配置读写，保存后热生效）。

## 桌面端（Tauri 2）

桌面端为独立 workspace（避免 WebKitGTK 系统依赖影响日常开发构建）。首次构建需先装 Tauri CLI 与系统依赖：

```bash
# 1. Tauri CLI（一次性安装，提供 cargo tauri 子命令）
cargo install tauri-cli --version "^2" --locked

# 2. Linux 系统依赖（macOS / Windows 无需此步）
sudo apt install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
  build-essential pkg-config curl wget file libxdo-dev libssl-dev

# 3. 构建（自动先构建前端，再编译并打包）
cd crates/sen-desktop
cargo tauri dev          # 开发模式：窗口指向进程内本地服务
cargo tauri build        # 打包 dmg / msi / AppImage+deb
```

打包产物位于 `crates/sen-desktop/target/release/bundle/`（Linux 为 `deb/`、`rpm/` 与 `appimage/`），文件名含 `tauri.conf.json` 中的版本号。

> 注：AppImage 打包需从 GitHub releases 下载打包工具（AppRun / linuxdeploy），国内网络可能超时；遇到时可直接用 deb，或挂代理后执行 `cargo tauri build --bundles appimage` 补打（工具会缓存到 `~/.cache/tauri/`，也可手动将 `AppRun-x86_64` 与 `linuxdeploy-07333c6-x86_64.AppImage` 放入该目录离线打包）。

- 启动时进程内绑定 `127.0.0.1:0`（随机端口）并启动 axum，窗口指向本地服务；
- 前端资源优先使用打包资源（`frontend-dist`），开发时回退 `frontend/dist`。

应用图标由源图生成：

```bash
npx @tauri-apps/cli@^2 icon crates/sen-desktop/icons/source.png --output crates/sen-desktop/icons
```

## 数据目录

| 路径 | 用途 |
|---|---|
| `~/.sen-agent/config.toml` | 配置（`SEN_AGENT_HOME` 可覆盖根目录） |
| `~/.sen-agent/data.db` | SQLite：会话与消息 |
| `~/.sen-agent/history.txt` | REPL 输入历史 |
| `~/.agents/skills/` | 技能目录（与其他 agent 工具共享） |
| `~/.agents/AGENTS.md` | 全局指导提示词 |

## 开发

```bash
cargo build              # 构建 workspace（core / server / cli）
cargo test -p sen-core   # 单元测试
cargo clippy --all-targets
cd frontend && npm run typecheck && npm run build
```

## CI / 发布

`.github/workflows/release.yml`：

- **cli** job：三平台矩阵（Linux / macOS / Windows）跑单元测试 + release 构建（前端一并内嵌），产物命名为 `sen-cli-v<版本>-<平台>.tar.gz` / `.zip`（如 `sen-cli-v0.1.5-linux-x86_64.tar.gz`）；
- **desktop** job：三平台构建 Tauri 安装包（dmg / msi / setup.exe / AppImage / deb / rpm），平铺保留为 artifact；
- **publish** job：推送 `v*` tag 时触发，汇总全部产物发布到 GitHub Releases；并从 CLI 产物中提取二进制，发布 npm 主包与平台子包（`@senknight/sen` + `@senknight/sen-linux-x64-gnu` / `sen-darwin-arm64` / `sen-win32-x64-msvc`）。

发版流程（版本号落在源码，tag 与源码版本必须一致）：

```bash
node scripts/sync-version.mjs 0.1.5     # 写入 Cargo.toml / tauri.conf.json / frontend / npm
cargo check                             # 让 cargo 同步 Cargo.lock 中工作区成员的版本
git commit -am "chore(release): v0.1.5"
git tag v0.1.5 && git push origin main v0.1.5
```

CI 构建时会先用 `node scripts/sync-version.mjs --check` 校验 tag 与源码版本一致，不一致直接失败，因此不会出现「安装包 / CLI / npm 包版本与 tag 不符」的情况（`sen --version`、产物文件名、npm 包版本三者天然一致）。npm 发布需在仓库 **Settings → Secrets → Actions** 配置 `NPM_TOKEN`，未配置时自动跳过。

手动触发（workflow_dispatch）只构建不发布，产物在该次运行的 **Actions → Artifacts** 中下载（需登录）。

## License

MIT
