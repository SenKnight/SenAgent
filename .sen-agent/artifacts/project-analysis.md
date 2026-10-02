# SenAgent 项目分析报告

> 分析时间：基于当前工作树（HEAD = `cc6805e`，`main` 领先 origin 1 个提交，另有大量未提交改动）

## 1. 一句话概览

**SenAgent 是一个"三端同核"的跨平台个人 AI Agent**：CLI / Web / 桌面端共用同一套 Rust 内核（`sen-core`）与同一套 React 前端，通过单二进制、零运行时依赖的方式分发。

## 2. 技术栈

| 层 | 技术 |
|---|---|
| 内核 / 服务 / CLI | Rust 2021、tokio（async）、axum 0.8（含 WS）、rusqlite（SQLite）、reqwest（rustls）、clap 4、rustyline、rust-embed、schemars |
| 前端 | React 19 + TypeScript 5.7 + Vite 6 + Tailwind 4 + Zustand 5 + react-markdown |
| 桌面端 | Tauri 2（独立 workspace，进程内嵌入 axum） |
| 分发 | cargo install / npm（`@senknight/sen` + 平台子包）/ GitHub Release（3 平台矩阵） |

## 3. 架构与模块划分

```
CLI(clap) ──进程内直连──┐
Web 浏览器 ──HTTP/WS──→ sen-server(axum) ──┐
桌面端(Tauri) ──进程内──┘                  ▼
                            sen-core（Agent Loop / Provider / Tools / Skills / Memory）
                                        │
                              SQLite · LLM Providers · MCP
```

| Crate | 行数 | 职责 |
|---|---|---|
| `sen-core` | ~4.1k | Agent 主循环（流式 + 多轮工具调用 + 上下文截断）、Provider 双协议（chat/responses）、工具注册表 + 8 个内置工具、技能系统、AGENTS.md 指导提示词、SQLite 持久化 |
| `sen-server` | ~0.9k | axum：WebSocket 聊天 + REST 会话/配置/文件/工作目录 + 静态资源托管（磁盘或内嵌） |
| `sen-cli` | ~0.7k | `sen` 二进制，clap 命令（chat/run/sessions/config/skill/web/serve） |
| `sen-desktop` | — | Tauri 2 壳（exclude 出主 workspace，避免 WebKitGTK 依赖） |
| `frontend` | ~2.4k | React 单页应用，Web 与桌面端共用 |

**设计亮点**
- **单一内核、多壳复用**：`sen-server` 是 lib，被 CLI 与桌面端进程内启动，不存在 sidecar 进程。
- **渐进式技能披露**：启动只注入技能索引，Agent 按需 `load_skill`，避免上下文占用；目录变化实时生效。
- **配置热更新**：Web/桌面端图形化编辑 providers 后，下一轮对话立即生效（每轮取最新 Agent 快照）。
- **前端双形态**：`sen web` 编译期内嵌（单文件即用），`sen serve` 托管磁盘目录（便于热更新调试）。
- **版本一致性**：`scripts/sync-version.mjs` + CI 校验 tag 与源码版本，避免包版本漂移。

## 4. 当前工作树：正在开发的功能（未提交）

未提交改动约 **+1015 / −566 行**，集中在一条清晰的功能线上（`git diff --stat`）：

### 4.1 计划模式（Plan Mode）
- `TurnMode::{Normal, Plan}`；计划模式只暴露只读工具白名单（`read_file/glob/grep/fetch/load_skill`），系统提示词追加"只做只读分析、输出结构化计划"指令。
- 越权调用在 Agent 与工具层**双重拦截**。
- 计划持久化到新增 `plans` 表（`draft`/`executed`），删除会话时级联清理；回合结束发 `AgentEvent::Plan`。
- WS 新增 `execute_plan` 消息：把已确认计划作为新输入交给常规模式执行，并置状态 `executed`。
- 测试：`crates/sen-core/src/memory/store.rs::plan_crud_and_cascade`、`agent::tests::plan_mode_filters_tools_and_persists_plan`、`crates/sen-server/tests/plans.rs`。

### 4.2 项目工作目录 + 产出物面板
- `Config.workspace` 持久化项目目录；CLI 启动时优先使用（无效则回落进程目录）。
- 新增 REST：`GET /api/fs/dirs`（浏览目录选择器）、`PUT /api/workspace`（切换 cwd 并热替换 Agent）、`GET /api/files/tree`、`GET /api/files/content`。
- 路径安全：`resolve_within()` 做 canonicalize + `starts_with` 前缀校验，防目录穿越；二进制文件与越界路径返回 400。
- 前端新增 `RightPanel`（文件树 / 修改记录 / 计划三标签）、`DirectoryPicker`、`SettingsModal`（替换旧 `SettingsPanel`）、明暗主题。
- 测试：`crates/sen-server/tests/files.rs`（树 / 预览 / 穿越 / 二进制防护）。

### 4.3 文档
- README 重排"开发"章节（4 种运行方式）、架构图对齐、新增 `AGENTS.md` 产出物约定片段（`prompts.rs` 引导 Agent 写 `.sen-agent/artifacts/`）。

## 5. 质量评估（已实测）

| 检查项 | 结果 |
|---|---|
| `cargo check --workspace --all-targets` | ✅ 通过（无 warning） |
| `cargo test --workspace` | ✅ 35 个 core 单测 + 3 个 server 集成测试全通过 |
| `cargo clippy --workspace --all-targets` | ✅ 零告警 |
| `npx tsc --noEmit`（前端） | ✅ 通过 |

测试覆盖有明显提升：内核的 SSE 分块解析、工具截断/预算、Provider 双协议归一化、计划模式白名单、计划 CRUD 级联都有针对性用例。

## 6. 发现的问题与建议

### 6.1 文档与实现不一致（低风险，建议修）
- README"内置工具"章节写 **`web_fetch`**，但 `crates/sen-core/src/tools/builtin/web.rs:29` 实际工具名为 **`fetch`**（`PLAN_MODE_TOOLS` 用的也是 `fetch`，代码内部自洽）。README 需更正。
- README 尚未记录本轮新增的**计划模式、`/api/files/*`、`/api/fs/dirs`、`/api/workspace`、`/api/sessions/{id}/plans`、右侧产出物面板**等能力，WebSocket 协议章节也只列了 `chat`/`ping`，缺 `execute_plan`。功能已实现但文档滞后。

### 6.2 安全边界（中风险，视部署方式而定）
- `/api/fs/dirs` 可浏览**文件系统任意目录**，`/api/workspace` 可把 cwd 切到任意目录，且**均无鉴权**。默认绑定 `127.0.0.1` 时风险可控；但 `sen web --host 0.0.0.0` / `sen serve` 暴露到局域网时，等于开放主机目录浏览与工作目录切换。建议：非回环绑定时加 token 校验，或对 `fs_dirs` 限定在主目录/上次目录子树内。
- `resolve_within` 对**符号链接**的处理依赖 canonicalize 后的前缀判断（已覆盖穿越），但 `file_tree` 列目录时未过滤软链指向外部的项，属可接受的信息泄露面，建议一并评估。

### 6.3 工程细节（低风险）
- `crates/sen-server/tests/{files,plans}.rs` 使用 `std::env::set_var` 设置进程级 `SEN_AGENT_HOME`/`SEN_AGENTS_DIR`。当前因每个集成测试是独立二进制进程而安全，但**同文件内并行测试会互相干扰**，建议在测试文件加注释约束或在 CI 用 `--test-threads=1` 兜底。
- `/api/sessions/{id}/plans` 未校验 session 是否存在（不存在时返回空数组而非 404），语义上可接受，但与其他接口的 404 行为不一致。
- 未提交改动约 1000 行且跨越 core/server/cli/frontend 四层，建议尽快拆分提交（feat(plan) / feat(workspace) / refactor(settings-ui) / docs），降低 review 与回滚成本。

## 7. 结论

项目**架构清晰、分层合理、工程质量高**（编译/clippy/测试/类型检查四项全绿），核心抽象（Agent Loop、Provider 适配、工具注册表、技能系统）职责边界干净，三端复用设计成熟。

当前正处于一条**"计划模式 + 项目工作目录 + 产出物面板"**的功能迭代中，实现完整且带测试，主要待办是：**补齐 README 文档、评估 `0.0.0.0` 暴露时的接口鉴权、并把这批改动提交入库**。
