/** REST 客户端（dev 走 Vite proxy，生产与后端同源）。 */

import type {
  DirListing,
  DraftSettings,
  EditableSettings,
  FileNode,
  Plan,
  RuntimeInfo,
  Session,
  StoredMessage,
} from "./types";

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`;
    try {
      const body = (await res.json()) as { error?: string };
      if (body?.error) message = body.error;
    } catch {
      /* 保留 HTTP 状态信息 */
    }
    throw new Error(message);
  }
  return (await res.json()) as T;
}

export const listSessions = () => req<Session[]>("/api/sessions");

export const createSession = (title = "", workspace?: string) =>
  req<Session>("/api/sessions", {
    method: "POST",
    body: JSON.stringify({ title, workspace }),
  });

export const getSession = (id: string) =>
  req<{ session: Session; messages: StoredMessage[] }>(
    `/api/sessions/${encodeURIComponent(id)}`,
  );

export const renameSession = (id: string, title: string) =>
  req<Session>(`/api/sessions/${encodeURIComponent(id)}`, {
    method: "PATCH",
    body: JSON.stringify({ title }),
  });

export const deleteSession = (id: string) =>
  req<{ ok: boolean }>(`/api/sessions/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });

export const getRuntimeInfo = () => req<RuntimeInfo>("/api/config");

export const getSettings = () => req<EditableSettings>("/api/settings");

/** 自动发现 provider 可用模型（设置页「模型服务」用，OpenAI 兼容 /models）。 */
export const listModels = (provider: {
  name?: string;
  base_url: string;
  api_key?: string;
  wire_api?: "chat" | "responses";
}) =>
  req<{ models: string[] }>("/api/models", {
    method: "POST",
    body: JSON.stringify(provider),
  });

/** 列出工作目录内某目录的直接子项（只读文件树）。path 缺省为工作目录根。 */
export const listFiles = (path = "") =>
  req<FileNode[]>(`/api/files/tree?path=${encodeURIComponent(path)}`);

/** 读取工作目录内文本文件内容（只读预览）。 */
export const getFileContent = (path: string) =>
  req<{ path: string; content: string }>(
    `/api/files/content?path=${encodeURIComponent(path)}`,
  );

/** 列出某会话的计划（计划模式产出）。 */
export const listPlans = (sessionId: string) =>
  req<Plan[]>(`/api/sessions/${encodeURIComponent(sessionId)}/plans`);

/** 浏览文件系统目录（用于选择项目目录）。path 缺省为主目录。 */
export const listDirs = (path = "") =>
  req<DirListing>(`/api/fs/dirs?path=${encodeURIComponent(path)}`);

/** 切换并持久化项目工作目录（作用于文件树与 Agent 后续回合）。 */
export const setWorkspace = (path: string) =>
  req<RuntimeInfo>("/api/workspace", {
    method: "PUT",
    body: JSON.stringify({ path }),
  });

/** 切换某会话所属项目目录（已打开会话时联动切换当前工作目录）。 */
export const setSessionWorkspace = (id: string, path: string) =>
  req<RuntimeInfo>(`/api/sessions/${encodeURIComponent(id)}/workspace`, {
    method: "PUT",
    body: JSON.stringify({ path }),
  });

/** 保存配置并热生效（服务端校验通过后原子替换运行态 Agent）。 */
export const saveSettings = (settings: DraftSettings) =>
  req<EditableSettings>("/api/settings", {
    method: "PUT",
    body: JSON.stringify(settings),
  });

export function wsUrl(): string {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${location.host}/api/ws`;
}
