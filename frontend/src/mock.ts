/** 纯前端演示数据层（GitHub Pages 静态预览用）。
 *
 * 仅当构建期变量 `VITE_MOCK=1` 时启用：本地开发 / CLI 内嵌 / 桌面端构建都不设置该变量，
 * 因此与真实后端交互的行为完全不变。启用后：
 * - `mockRequest` 拦截 REST 调用并返回内置演示数据（会话 / 文件树 / 设置 / 计划）；
 * - `createMockSocket` 提供一个假的 WebSocket，模拟 ready / 流式 token / 工具调用 / done。
 */

import type {
  DirListing,
  DraftSettings,
  EditableSettings,
  FileNode,
  Plan,
  RuntimeInfo,
  Session,
  ServerEvent,
  StoredMessage,
} from "./types";

export const MOCK = import.meta.env.VITE_MOCK === "1";

const DEMO_HOME = "/home/demo";
const DEMO_WORKSPACE = "/home/demo/projects/senagent-demo";
const DEMO_WORKSPACE_2 = "/home/demo/projects/pi-web-demo";
const DEMO_MODEL = "deepseek-chat";

const nowSec = () => Math.floor(Date.now() / 1000);

/* ------------------------------ 内存态 ------------------------------ */

let workspace = DEMO_WORKSPACE;

let sessions: Session[] = [
  {
    id: "demo-pay",
    title: "重构支付模块",
    workspace,
    created_at: nowSec() - 5400,
    updated_at: nowSec() - 3600,
    message_count: 4,
  },
  {
    id: "demo-readme",
    title: "整理 README 文档",
    workspace: DEMO_WORKSPACE_2,
    created_at: nowSec() - 7200,
    updated_at: nowSec() - 7000,
    message_count: 2,
  },
  {
    id: "demo-login",
    title: "修复登录 403 问题",
    workspace,
    created_at: nowSec() - 9000,
    updated_at: nowSec() - 8800,
    message_count: 6,
  },
];

const messagesById: Record<string, StoredMessage[]> = {
  "demo-pay": [
    { id: "p1", role: "user", content: "帮我看看支付模块的 createOrder 逻辑，顺便重构一下。", created_at: nowSec() - 5400 },
    {
      id: "p2",
      role: "assistant",
      content: "我先读取相关实现。",
      tool_calls: [{ id: "pt1", name: "read_file", arguments: '{"path":"src/pay.rs"}' }],
      created_at: nowSec() - 5390,
    },
    {
      id: "p3",
      role: "tool",
      tool_call_id: "pt1",
      name: "read_file",
      content: "fn createOrder(...) {\n    // 校验、计价、落库混在一起\n}",
      created_at: nowSec() - 5380,
    },
    {
      id: "p4",
      role: "assistant",
      content:
        "已将 `createOrder` 拆分为参数校验、金额计算、持久化三步，并把金额统一改为「分」为单位，最后补充了幂等键处理。",
      created_at: nowSec() - 5370,
    },
  ],
  "demo-readme": [
    { id: "r1", role: "user", content: "README 里 CLI 命令表格和实际实现对不上，帮我核对。", created_at: nowSec() - 7200 },
    { id: "r2", role: "assistant", content: "已逐条核对并补齐了 `sen web` 一节，表格现已与 CLI 实现一致。", created_at: nowSec() - 7100 },
  ],
  "demo-login": [
    { id: "l1", role: "user", content: "登录接口偶发 403，帮排查。", created_at: nowSec() - 9000 },
    {
      id: "l2",
      role: "assistant",
      content: "我查一下中间件的同源校验逻辑。",
      tool_calls: [{ id: "lt1", name: "grep", arguments: '{"pattern":"same_site"}' }],
      created_at: nowSec() - 8990,
    },
    {
      id: "l3",
      role: "tool",
      tool_call_id: "lt1",
      name: "grep",
      content: "crates/sen-server/src/routes.rs:63: fn same_site(origin: &str, host: &str) -> bool",
      created_at: nowSec() - 8980,
    },
    {
      id: "l4",
      role: "assistant",
      content: "原因是反向代理改写了 Host，`same_site` 判定失败；已放宽为「主机名等价」比较。",
      created_at: nowSec() - 8970,
    },
  ],
};

const plansBySession: Record<string, Plan[]> = {
  "demo-pay": [
    {
      id: "plan-demo-1",
      session_id: "demo-pay",
      content:
        "## 重构支付模块计划\n\n1. 抽取 `validate_order` 校验函数\n2. 金额统一使用「分」为单位（整数）\n3. 新增幂等键，写入前先校验\n4. 补充单元测试覆盖边界",
      status: "draft",
      created_at: nowSec() - 3000,
    },
  ],
};

let settings: EditableSettings = {
  config_path: `${DEMO_HOME}/.sen-agent/config.toml`,
  default_provider: "deepseek",
  context_window: 128000,
  max_tool_rounds: 25,
  system_prompt: null,
  providers: [
    {
      name: "deepseek",
      base_url: "https://api.deepseek.com/v1",
      model: "deepseek-chat",
      wire_api: "chat",
      api_key: null,
      api_key_set: true,
      max_tokens: null,
      temperature: null,
    },
    {
      name: "ollama",
      base_url: "http://localhost:11434/v1",
      model: "qwen3:8b",
      wire_api: "chat",
      api_key: null,
      api_key_set: false,
      max_tokens: null,
      temperature: null,
    },
  ],
};

function runtime(): RuntimeInfo {
  return {
    provider: settings.default_provider,
    model: DEMO_MODEL,
    wire_api: "chat",
    cwd: workspace,
    home: DEMO_HOME,
    skills: [
      { name: "git-helper", description: "处理 git 操作的最佳实践", group: "coding" },
      { name: "code-review", description: "代码审查清单与常见缺陷", group: "coding" },
      { name: "blog-post", description: "撰写技术博客的结构化流程", group: "writing" },
    ],
  };
}

/* ------------------------------ 文件树 ------------------------------ */

type Entry = [name: string, isDir: boolean, size?: number];

function buildTree(prefix: string, entries: Entry[]): FileNode[] {
  return entries.map(([name, isDir, size]) => ({
    name,
    path: prefix ? `${prefix}/${name}` : name,
    is_dir: isDir,
    size: size ?? 0,
    modified_at: nowSec() - 1200,
  }));
}

const FILE_TREE: Record<string, FileNode[]> = {
  "": buildTree("", [
    ["src", true],
    ["frontend", true],
    ["README.md", false, 7430],
    ["Cargo.toml", false, 512],
    ["package.json", false, 690],
  ]),
  src: buildTree("src", [
    ["main.rs", false, 2048],
    ["lib.rs", false, 5120],
    ["pay.rs", false, 8192],
  ]),
  frontend: buildTree("frontend", [
    ["src", true],
    ["package.json", false, 430],
    ["vite.config.ts", false, 640],
  ]),
  "frontend/src": buildTree("frontend/src", [
    ["App.tsx", false, 5320],
    ["api.ts", false, 3900],
    ["store.ts", false, 12040],
  ]),
};

const FILE_CONTENT: Record<string, string> = {
  "README.md":
    "# SenAgent（演示）\n\n跨平台个人 AI Agent：CLI / Web / 桌面三端同核，Rust 全栈。\n\n- **sen-core**：Agent 主循环、Provider 双协议、工具系统、技能\n- **sen-server**：axum 服务，WebSocket + REST + 前端托管\n- **frontend**：React 19 + TS + Vite + Tailwind 4 + Zustand\n\n> 当前为 GitHub Pages 静态预览，数据均为演示内容。\n",
  "Cargo.toml":
    "[workspace]\nmembers = [\n  \"crates/sen-core\",\n  \"crates/sen-server\",\n  \"crates/sen-cli\",\n]\nresolver = \"2\"\n\n[workspace.package]\nversion = \"0.1.5\"\nedition = \"2021\"\n",
  "src/main.rs":
    "fn main() {\n    let cwd = std::env::current_dir().unwrap();\n    println!(\"SenAgent demo in {}\", cwd.display());\n}\n",
  "src/pay.rs":
    "pub struct Order {\n    pub id: String,\n    pub amount_cents: i64,\n}\n\n// 演示：拆分后的下单流程\npub fn create_order(id: &str, amount_cents: i64) -> Order {\n    validate(id, amount_cents);\n    Order { id: id.to_string(), amount_cents }\n}\n\nfn validate(id: &str, amount_cents: i64) {\n    assert!(!id.is_empty(), \"id required\");\n    assert!(amount_cents > 0, \"amount must be positive\");\n}\n",
  "frontend/package.json":
    '{\n  "name": "senagent-frontend",\n  "version": "0.1.5",\n  "type": "module"\n}\n',
};

/* ------------------------------ 目录浏览 ------------------------------ */

const DIR_TREE: Record<string, string[]> = {
  "/": ["/home"],
  "/home": ["/home/demo"],
  "/home/demo": ["/home/demo/projects", "/home/demo/Documents"],
  "/home/demo/projects": [
    "/home/demo/projects/senagent-demo",
    "/home/demo/projects/pi-web-demo",
  ],
  "/home/demo/projects/senagent-demo": [
    "/home/demo/projects/senagent-demo/src",
    "/home/demo/projects/senagent-demo/frontend",
  ],
  "/home/demo/projects/pi-web-demo": ["/home/demo/projects/pi-web-demo/src"],
};

function dirListing(rawPath: string): DirListing {
  const path = rawPath && rawPath.trim() ? rawPath.replace(/\/+$/, "") || "/" : DEMO_HOME;
  const children = DIR_TREE[path] ?? [];
  const parent = path === "/" ? null : path.slice(0, path.lastIndexOf("/")) || "/";
  return {
    path,
    parent,
    dirs: children.map((p) => ({ name: p.slice(p.lastIndexOf("/") + 1), path: p })),
  };
}

/* ------------------------------ REST 路由 ------------------------------ */

/** 拦截单个 REST 请求，返回演示数据（方法/路径与真实后端一致）。 */
export function mockRequest<T>(path: string, init?: RequestInit): T {
  const url = new URL(path, "http://mock.local");
  const method = (init?.method ?? "GET").toUpperCase();
  const p = url.pathname;
  const body = init?.body ? (JSON.parse(String(init.body)) as Record<string, unknown>) : {};

  // /api/sessions 及其子资源
  if (p === "/api/sessions") {
    if (method === "GET") return sessions as unknown as T;
    if (method === "POST") {
      const now = nowSec();
      const s: Session = {
        id: `demo-${Date.now()}`,
        title: (body.title as string) || "新会话",
        workspace: (body.workspace as string) || workspace,
        created_at: now,
        updated_at: now,
        message_count: 0,
      };
      sessions = [s, ...sessions];
      messagesById[s.id] = [];
      return s as unknown as T;
    }
  }

  if (p.startsWith("/api/sessions/")) {
    const rest = p.slice("/api/sessions/".length);
    const [id, sub] = rest.split("/");
    if (sub === "plans" && method === "GET") {
      return (plansBySession[id] ?? []) as unknown as T;
    }
    if (sub === "workspace" && method === "PUT") {
      workspace = String(body.path ?? workspace);
      const s = sessions.find((x) => x.id === id);
      if (s) s.workspace = workspace;
      return runtime() as unknown as T;
    }
    if (!sub) {
      const s = sessions.find((x) => x.id === id);
      if (method === "GET") {
        return { session: s, messages: messagesById[id] ?? [] } as unknown as T;
      }
      if (method === "PATCH" && s) {
        s.title = String(body.title ?? s.title);
        s.updated_at = nowSec();
        return s as unknown as T;
      }
      if (method === "DELETE") {
        sessions = sessions.filter((x) => x.id !== id);
        delete messagesById[id];
        return { ok: true } as unknown as T;
      }
    }
  }

  if (p === "/api/config" && method === "GET") {
    return runtime() as unknown as T;
  }

  if (p === "/api/settings") {
    if (method === "GET") return settings as unknown as T;
    if (method === "PUT") {
      const draft = body as unknown as DraftSettings;
      settings = {
        ...settings,
        default_provider: draft.default_provider,
        context_window: draft.context_window,
        max_tool_rounds: draft.max_tool_rounds,
        system_prompt: draft.system_prompt,
        providers: draft.providers.map((d) => ({
          name: d.name,
          base_url: d.base_url,
          model: d.model,
          wire_api: d.wire_api,
          api_key: d.api_key && !d.api_key.startsWith("env:") ? null : d.api_key ?? null,
          api_key_set: Boolean(d.api_key),
          max_tokens: d.max_tokens ?? null,
          temperature: d.temperature ?? null,
        })),
      };
      return settings as unknown as T;
    }
  }

  if (p === "/api/models" && method === "POST") {
    return { models: ["deepseek-chat", "deepseek-reasoner", "gpt-4o", "qwen3:8b"] } as unknown as T;
  }

  if (p === "/api/files/tree" && method === "GET") {
    const dir = url.searchParams.get("path") ?? "";
    return (FILE_TREE[dir] ?? []) as unknown as T;
  }

  if (p === "/api/files/content" && method === "GET") {
    const file = url.searchParams.get("path") ?? "";
    return { path: file, content: FILE_CONTENT[file] ?? "（演示文件，暂无内容）" } as unknown as T;
  }

  if (p === "/api/fs/dirs" && method === "GET") {
    return dirListing(url.searchParams.get("path") ?? "") as unknown as T;
  }

  if (p === "/api/workspace" && method === "PUT") {
    workspace = String(body.path ?? workspace);
    return runtime() as unknown as T;
  }

  throw new Error(`mock 未实现的接口：${method} ${p}`);
}

/* ------------------------- WebSocket 模拟 ------------------------- */

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function emit(sock: Record<string, unknown>, ev: ServerEvent) {
  (sock.onmessage as ((e: { data: string }) => void) | null)?.({ data: JSON.stringify(ev) });
}

function chunkText(text: string): string[] {
  const out: string[] = [];
  for (let i = 0; i < text.length; ) {
    const n = 2 + Math.floor(Math.random() * 4);
    out.push(text.slice(i, i + n));
    i += n;
  }
  return out;
}

function addMockPlan(sessionId: string): Plan {
  const plan: Plan = {
    id: `plan-${Date.now()}`,
    session_id: sessionId,
    content:
      "## 重构支付模块计划\n\n1. 抽取 `validate_order` 校验函数\n2. 金额统一使用「分」为单位（整数）\n3. 新增幂等键，写入前先校验\n4. 补充单元测试覆盖边界",
    status: "draft",
    created_at: nowSec(),
  };
  plansBySession[sessionId] = [...(plansBySession[sessionId] ?? []), plan];
  return plan;
}

async function streamReply(
  sock: Record<string, unknown>,
  prompt: string,
  planMode: boolean,
  sessionId: string,
) {
  await sleep(220);
  emit(sock, { type: "reasoning", delta: "先理解需求，" });
  await sleep(280);
  emit(sock, { type: "reasoning", delta: "再查阅相关实现……\n" });
  await sleep(280);

  emit(sock, { type: "tool_call", name: "grep", arguments: '{"pattern":"createOrder"}' });
  await sleep(520);
  emit(sock, {
    type: "tool_result",
    name: "grep",
    output: "src/pay.rs:42: fn create_order(...)\nsrc/pay.rs:88: fn refund(...)",
    is_error: false,
  });
  await sleep(260);

  emit(sock, { type: "tool_call", name: "read_file", arguments: '{"path":"src/pay.rs"}' });
  await sleep(520);
  emit(sock, {
    type: "tool_result",
    name: "read_file",
    output: "fn create_order(id: &str, amount_cents: i64) -> Order { /* ... */ }",
    is_error: false,
  });
  await sleep(260);

  const reply = planMode
    ? "我已梳理实现路径，生成计划如下（见右侧「计划」面板）。"
    : `已根据「${prompt.slice(0, 40)}」完成分析：\n\n1. 抽离参数校验；\n2. 金额统一以「分」为单位；\n3. 写入前先落幂等键。\n\n（以上为演示数据）`;

  for (const chunk of chunkText(reply)) {
    emit(sock, { type: "token", delta: chunk });
    await sleep(45);
  }

  if (planMode) {
    const plan = addMockPlan(sessionId);
    emit(sock, { type: "plan", content: plan.content });
    await sleep(150);
  }

  emit(sock, { type: "done", usage: { input_tokens: 1284, output_tokens: 356 } });
}

function handleClient(sock: Record<string, unknown>, raw: string) {
  let msg: { type?: string; content?: string; mode?: string; session_id?: string };
  try {
    msg = JSON.parse(raw) as typeof msg;
  } catch {
    return;
  }
  switch (msg.type) {
    case "ping":
      emit(sock, { type: "pong" });
      break;
    case "chat":
      void streamReply(sock, msg.content ?? "", msg.mode === "plan", msg.session_id ?? "");
      break;
    case "execute_plan":
      void streamReply(sock, "请执行该计划。", false, msg.session_id ?? "");
      break;
    default:
      break;
  }
}

/** 构造一个行为兼容 WebSocket 的假连接（仅支持本项目用到的接口）。 */
export function createMockSocket(): WebSocket {
  const sock: Record<string, unknown> = {
    readyState: 0,
    onopen: null,
    onmessage: null,
    onclose: null,
    onerror: null,
    send: (data: string) => handleClient(sock, String(data)),
    close: () => {
      sock.readyState = 3;
      setTimeout(() => (sock.onclose as (() => void) | null)?.(), 0);
    },
  };

  setTimeout(() => {
    if (sock.readyState === 3) return;
    sock.readyState = 1;
    (sock.onopen as (() => void) | null)?.();
    emit(sock, { type: "ready", model: DEMO_MODEL, wire_api: "chat" });
  }, 260);

  return sock as unknown as WebSocket;
}
