/** 全局状态（Zustand）。 */

import { create } from "zustand";

import * as api from "./api";
import { sendWs } from "./ws";
import type {
  Plan,
  RuntimeInfo,
  ServerEvent,
  Session,
  StoredMessage,
  UiMessage,
} from "./types";

/** 主题模式。 */
export type Theme = "dark" | "light";

/** 右侧面板标签页。 */
export type RightPanelTab = "files" | "changes" | "plans";

const THEME_KEY = "sen-theme";

function applyTheme(theme: Theme) {
  document.documentElement.classList.toggle("light", theme === "light");
}

function initialTheme(): Theme {
  const saved = localStorage.getItem(THEME_KEY);
  if (saved === "light" || saved === "dark") return saved;
  const prefersLight =
    window.matchMedia?.("(prefers-color-scheme: light)")?.matches ?? false;
  return prefersLight ? "light" : "dark";
}

interface AppState {
  // 数据
  sessions: Session[];
  currentSessionId: string | null;
  messages: UiMessage[];
  info: RuntimeInfo | null;
  plans: Plan[];
  currentPlan: Plan | null;

  // UI 状态
  running: boolean;
  wsConnected: boolean;
  serverModel: string | null;
  sidebarOpen: boolean;
  /** 主区视图：对话 / 设置（整页，可与对话切换） */
  view: "chat" | "settings";
  dirPickerOpen: boolean;
  theme: Theme;
  rightPanelOpen: boolean;
  rightPanelTab: RightPanelTab;
  planMode: boolean;

  // 简单动作
  setWsState: (connected: boolean, model: string | null) => void;
  toggleSidebar: () => void;
  setView: (view: "chat" | "settings") => void;
  setDirPickerOpen: (open: boolean) => void;
  changeWorkspace: (path: string) => Promise<void>;
  setTheme: (theme: Theme) => void;
  toggleRightPanel: () => void;
  setRightPanelTab: (tab: RightPanelTab) => void;
  setPlanMode: (on: boolean) => void;

  // 会话动作
  refreshSessions: () => Promise<void>;
  newSession: () => Promise<Session>;
  openSession: (id: string) => Promise<void>;
  removeSession: (id: string) => Promise<void>;
  renameSession: (id: string, title: string) => Promise<void>;

  // 计划动作
  refreshPlans: () => Promise<void>;
  executePlan: (planId: string) => Promise<void>;

  // 对话动作
  appendUser: (content: string) => void;
  startAssistant: () => void;
  finishWithError: (message: string) => void;
  handleEvent: (ev: ServerEvent) => void;
}

export const useStore = create<AppState>((set, get) => ({
  sessions: [],
  currentSessionId: null,
  messages: [],
  info: null,
  plans: [],
  currentPlan: null,
  running: false,
  wsConnected: false,
  serverModel: null,
  sidebarOpen: true,
  view: "chat",
  dirPickerOpen: false,
  theme: initialTheme(),
  rightPanelOpen: false,
  rightPanelTab: "files",
  planMode: false,

  setWsState: (wsConnected, serverModel) => set({ wsConnected, serverModel }),
  toggleSidebar: () => set((st) => ({ sidebarOpen: !st.sidebarOpen })),
  setView: (view) => set({ view }),
  setDirPickerOpen: (dirPickerOpen) => set({ dirPickerOpen }),
  changeWorkspace: async (path) => {
    // 已打开会话时，切目录同时迁移该会话归属，保持「当前目录 == 当前会话目录」一致
    const id = get().currentSessionId;
    const info = id
      ? await api.setSessionWorkspace(id, path)
      : await api.setWorkspace(path);
    set({ info });
    if (id) get().refreshSessions().catch(() => {});
  },
  setTheme: (theme) => {
    localStorage.setItem(THEME_KEY, theme);
    applyTheme(theme);
    set({ theme });
  },
  toggleRightPanel: () => set((st) => ({ rightPanelOpen: !st.rightPanelOpen })),
  setRightPanelTab: (rightPanelTab) => set({ rightPanelTab, rightPanelOpen: true }),
  setPlanMode: (planMode) => set({ planMode }),

  refreshSessions: async () => {
    const sessions = await api.listSessions();
    set({ sessions });
  },

  newSession: async () => {
    const session = await api.createSession("", get().info?.cwd);
    await get().refreshSessions();
    set({
      currentSessionId: session.id,
      messages: [],
      plans: [],
      currentPlan: null,
      planMode: false,
    });
    return session;
  },

  openSession: async (id) => {
    const { session, messages } = await api.getSession(id);
    // 联动：若会话所属目录与当前不同，切换当前工作目录
    const ws = session.workspace;
    if (ws && ws !== get().info?.cwd) {
      try {
        const info = await api.setWorkspace(ws);
        set({ info });
      } catch {
        /* 目录不可用时仍打开会话，不阻断 */
      }
    }
    set({
      currentSessionId: session.id,
      messages: toUiMessages(messages),
      currentPlan: null,
    });
    // 切换会话时刷新标题等信息
    api.listSessions().then((sessions) => set({ sessions })).catch(() => {});
    get().refreshPlans().catch(() => {});
  },

  removeSession: async (id) => {
    await api.deleteSession(id);
    const { currentSessionId } = get();
    if (currentSessionId === id) {
      set({ currentSessionId: null, messages: [] });
    }
    await get().refreshSessions();
  },

  renameSession: async (id, title) => {
    await api.renameSession(id, title);
    await get().refreshSessions();
  },

  refreshPlans: async () => {
    const id = get().currentSessionId;
    if (!id) {
      set({ plans: [], currentPlan: null });
      return;
    }
    const plans = await api.listPlans(id);
    set({ plans });
  },

  executePlan: async (planId) => {
    if (get().running) return;
    const plan = get().plans.find((p) => p.id === planId) ?? get().currentPlan;
    let sessionId = get().currentSessionId;
    if (!sessionId) {
      sessionId = (await get().newSession()).id;
    }
    const content = plan
      ? `请按以下计划执行（用户已确认）：\n\n${plan.content}`
      : "请执行该计划。";
    get().appendUser(content);
    get().startAssistant();
    set({ running: true });
    if (!sendWs({ type: "execute_plan", session_id: sessionId, plan_id: planId })) {
      get().finishWithError("WebSocket 未连接，请稍候重试");
    }
  },

  appendUser: (content) =>
    set((st) => ({
      messages: [
        ...st.messages,
        { key: crypto.randomUUID(), role: "user" as const, content, tools: [] },
      ],
    })),

  startAssistant: () =>
    set((st) => ({
      messages: [
        ...st.messages,
        {
          key: crypto.randomUUID(),
          role: "assistant" as const,
          content: "",
          tools: [],
          streaming: true,
        },
      ],
    })),

  finishWithError: (message) => {
    const msgs = [...get().messages];
    const last = msgs[msgs.length - 1];
    if (last && last.role === "assistant") {
      msgs[msgs.length - 1] = {
        ...last,
        streaming: false,
        error: (last.error ? last.error + "\n" : "") + message,
      };
    } else {
      msgs.push({
        key: crypto.randomUUID(),
        role: "assistant",
        content: "",
        tools: [],
        error: message,
      });
    }
    set({ messages: msgs, running: false });
  },

  handleEvent: (ev) => {
    const setLast = (mutate: (m: UiMessage) => void) => {
      const msgs = [...get().messages];
      for (let i = msgs.length - 1; i >= 0; i -= 1) {
        if (msgs[i].role === "assistant") {
          const copy: UiMessage = { ...msgs[i], tools: [...msgs[i].tools] };
          mutate(copy);
          msgs[i] = copy;
          set({ messages: msgs });
          return;
        }
      }
    };

    switch (ev.type) {
      case "token":
        setLast((m) => {
          m.content += ev.delta;
        });
        break;
      case "reasoning":
        setLast((m) => {
          m.reasoning = (m.reasoning ?? "") + ev.delta;
        });
        break;
      case "tool_call":
        setLast((m) => {
          m.tools = [...m.tools, { name: ev.name, arguments: ev.arguments, done: false }];
        });
        break;
      case "tool_result":
        setLast((m) => {
          const idx = m.tools.findIndex((t) => !t.done && t.name === ev.name);
          const target = idx >= 0 ? idx : m.tools.length - 1;
          if (target >= 0) {
            m.tools = m.tools.map((t, i) =>
              i === target
                ? { ...t, output: ev.output, isError: ev.is_error, done: true }
                : t,
            );
          } else {
            m.tools = [
              ...m.tools,
              {
                name: ev.name,
                arguments: "",
                output: ev.output,
                isError: ev.is_error,
                done: true,
              },
            ];
          }
        });
        break;
      case "plan":
        set({ rightPanelOpen: true, rightPanelTab: "plans" });
        get()
          .refreshPlans()
          .then(() => {
            const plans = get().plans;
            const last = plans[plans.length - 1];
            if (last) set({ currentPlan: last });
          })
          .catch(() => {});
        break;
      case "done":
        setLast((m) => {
          m.streaming = false;
          m.usage = ev.usage ?? undefined;
        });
        set({ running: false });
        // 标题可能在首轮对话后生成，刷新侧栏
        get().refreshSessions().catch(() => {});
        break;
      case "error":
        setLast((m) => {
          m.streaming = false;
          m.error = (m.error ? m.error + "\n" : "") + ev.message;
        });
        set({ running: false });
        break;
      default:
        break;
    }
  },
}));

/** 数据库消息 → UI 消息（tool 结果按 tool_call_id 配对）。 */
export function toUiMessages(stored: StoredMessage[]): UiMessage[] {
  const out: UiMessage[] = [];
  for (const m of stored) {
    switch (m.role) {
      case "user":
        out.push({ key: m.id, role: "user", content: m.content, tools: [] });
        break;
      case "assistant": {
        out.push({
          key: m.id,
          role: "assistant",
          content: m.content,
          tools: (m.tool_calls ?? []).map((tc) => ({
            id: tc.id,
            name: tc.name,
            arguments: tc.arguments,
            done: false,
          })),
        });
        break;
      }
      case "tool": {
        const prev = out[out.length - 1];
        if (prev && prev.role === "assistant") {
          prev.tools = prev.tools.map((t) =>
            t.id && t.id === m.tool_call_id
              ? {
                  ...t,
                  output: m.content,
                  isError: m.content.startsWith("Error: "),
                  done: true,
                }
              : t,
          );
        }
        break;
      }
      default:
        break;
    }
  }
  return out;
}
