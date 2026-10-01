/** REST 客户端（dev 走 Vite proxy，生产与后端同源）。 */

import type { RuntimeInfo, Session, StoredMessage } from "./types";

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

export const createSession = (title = "") =>
  req<Session>("/api/sessions", {
    method: "POST",
    body: JSON.stringify({ title }),
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

export function wsUrl(): string {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${location.host}/api/ws`;
}
