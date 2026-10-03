/** WebSocket 客户端：自动重连 + 事件分发到 store。 */

import { wsUrl } from "./api";
import { MOCK, createMockSocket } from "./mock";
import { useStore } from "./store";
import type { ClientEvent } from "./types";

let socket: WebSocket | null = null;
let intentionalClose = false;
let retry = 0;

export function connectWs(): void {
  if (
    socket &&
    (socket.readyState === WebSocket.OPEN ||
      socket.readyState === WebSocket.CONNECTING)
  ) {
    return;
  }
  intentionalClose = false;
  const ws: WebSocket = MOCK ? createMockSocket() : new WebSocket(wsUrl());
  socket = ws;

  ws.onopen = () => {
    retry = 0;
  };

  ws.onmessage = (e: MessageEvent<string>) => {
    let ev: unknown;
    try {
      ev = JSON.parse(e.data);
    } catch {
      return;
    }
    const store = useStore.getState();
    const type = (ev as { type?: string }).type;
    if (type === "ready") {
      const ready = ev as { model: string };
      store.setWsState(true, ready.model);
      return;
    }
    store.handleEvent(ev as import("./types").ServerEvent);
  };

  ws.onclose = () => {
    socket = null;
    useStore.getState().setWsState(false, null);
    if (!intentionalClose) {
      retry += 1;
      window.setTimeout(connectWs, Math.min(1000 * retry, 5000));
    }
  };

  ws.onerror = () => {
    ws.close();
  };
}

export function sendWs(msg: ClientEvent): boolean {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    return false;
  }
  socket.send(JSON.stringify(msg));
  return true;
}

export function closeWs(): void {
  intentionalClose = true;
  socket?.close();
  socket = null;
}

/** 关闭并立即重连：后端无 cancel 消息，据此取消当前生成轮次。 */
export function reconnectWs(): void {
  closeWs();
  retry = 0;
  connectWs();
}
