/** 与 Rust 后端（sen-core / sen-server）对齐的类型定义。 */

export interface Session {
  id: string;
  title: string;
  /** 所属项目目录（绝对路径；无项目时为用户主目录） */
  workspace: string;
  created_at: number;
  updated_at: number;
  message_count: number;
}

export interface StoredToolCall {
  id: string;
  name: string;
  arguments: string;
}

export interface StoredMessage {
  id: string;
  role: "system" | "user" | "assistant" | "tool";
  content: string;
  tool_calls?: StoredToolCall[];
  tool_call_id?: string;
  name?: string;
  created_at: number;
}

export interface Usage {
  input_tokens: number;
  output_tokens: number;
}

/** 服务端 → 客户端的 WS 事件（AgentEvent 的 JSON 形态，type 字段区分）。 */
export type ServerEvent =
  | { type: "ready"; model: string; wire_api: string }
  | { type: "pong" }
  | { type: "token"; delta: string }
  | { type: "reasoning"; delta: string }
  | { type: "tool_call"; name: string; arguments: string }
  | { type: "tool_result"; name: string; output: string; is_error: boolean }
  | { type: "plan"; content: string }
  | { type: "done"; usage?: Usage | null }
  | { type: "error"; message: string };

/** 客户端 → 服务端的 WS 消息。 */
export type ClientEvent =
  | { type: "chat"; session_id: string; content: string; mode?: "normal" | "plan" }
  | { type: "execute_plan"; session_id: string; plan_id: string }
  | { type: "ping" };

export interface RuntimeInfo {
  provider: string;
  model: string;
  wire_api: string;
  cwd: string;
  /** 用户主目录（用于「用户目录」分组标识） */
  home: string;
  skills: { name: string; description: string; group: string }[];
}

/** 设置页：可编辑 Provider（env: 引用原样回显；明文密钥不回显，以 api_key_set 标记）。 */
export interface EditableProvider {
  name: string;
  base_url: string;
  model: string;
  wire_api: "chat" | "responses";
  api_key: string | null;
  api_key_set: boolean;
  max_tokens: number | null;
  temperature: number | null;
}

/** 设置页：完整可编辑配置（GET / PUT /api/settings）。 */
export interface EditableSettings {
  config_path: string;
  default_provider: string;
  context_window: number;
  max_tool_rounds: number;
  system_prompt: string | null;
  providers: EditableProvider[];
}

/** 保存请求：api_key 缺省 = 保留原值，空串 = 清除。 */
export interface DraftProvider {
  name: string;
  base_url: string;
  model: string;
  wire_api: "chat" | "responses";
  api_key?: string;
  max_tokens: number | null;
  temperature: number | null;
}

export interface DraftSettings {
  default_provider: string;
  context_window: number;
  max_tool_rounds: number;
  system_prompt: string | null;
  providers: DraftProvider[];
}

/** 工作目录内的文件/目录项（文件树用）。 */
export interface FileNode {
  name: string;
  /** 相对工作目录的路径，使用 `/` 分隔符 */
  path: string;
  is_dir: boolean;
  size: number;
  modified_at: number;
}

/** 目录浏览器：某绝对目录下的子目录列表（用于选择项目目录）。 */
export interface DirListing {
  path: string;
  /** 上级目录绝对路径（根目录为 null） */
  parent: string | null;
  dirs: { name: string; path: string }[];
}

/** 计划模式下产出的计划。 */
export interface Plan {
  id: string;
  session_id: string;
  content: string;
  status: "draft" | "executed";
  created_at: number;
}

/** UI 层的工具调用状态。 */
export interface UiTool {
  /** 对应 assistant 消息中的 tool_call id（用于与 tool 结果配对） */
  id?: string;
  name: string;
  arguments: string;
  output?: string;
  isError?: boolean;
  done: boolean;
}

/** UI 层消息（渲染用）。 */
export interface UiMessage {
  key: string;
  role: "user" | "assistant";
  content: string;
  reasoning?: string;
  tools: UiTool[];
  usage?: Usage;
  error?: string;
  streaming?: boolean;
}
