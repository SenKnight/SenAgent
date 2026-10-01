/** 与 Rust 后端（sen-core / sen-server）对齐的类型定义。 */

export interface Session {
  id: string;
  title: string;
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
  | { type: "done"; usage?: Usage | null }
  | { type: "error"; message: string };

/** 客户端 → 服务端的 WS 消息。 */
export type ClientEvent =
  | { type: "chat"; session_id: string; content: string }
  | { type: "ping" };

export interface RuntimeInfo {
  provider: string;
  model: string;
  wire_api: string;
  cwd: string;
  skills: { name: string; description: string; group: string }[];
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
