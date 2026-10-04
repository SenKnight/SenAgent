/** 全局状态（Zustand）。 */

import { create } from "zustand";

import * as api from "./api";
import { detectLang, translate, LANG_KEY, type Lang } from "./i18n";
import { reconnectWs, sendWs } from "./ws";
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

/** 右侧文件面板标签：按文件路径唯一。 */
export interface FileTab {
  id: string;
  path: string;
  title: string;
}

const THEME_KEY = "sen-theme";

const fileTabId = (path: string) => `file:${path}`;

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

  // 右侧文件面板标签
  fileTabs: FileTab[];
  fileActiveTabId: string | null;

  // UI 状态
  running: boolean;
  wsConnected: boolean;
  serverModel: string | null;
  sidebarOpen: boolean;
  /** 右侧文件面板是否展开 */
  rightPanelOpen: boolean;
  /** 设置面板是否打开（覆盖主区） */
  settingsOpen: boolean;
  dirPickerOpen: boolean;
  theme: Theme;
  lang: Lang;
  planMode: boolean;

  // 简单动作
  setWsState: (connected: boolean, model: string | null) => void;
  toggleSidebar: () => void;
  setDirPickerOpen: (open: boolean) => void;
  changeWorkspace: (path: string) => Promise<void>;
  setTheme: (theme: Theme) => void;
  setLang: (lang: Lang) => void;
  setPlanMode: (on: boolean) => void;
  openSettings: () => void;
  closeSettings: () => void;

  // 右侧文件面板动作
  openFileTab: (path: string) => void;
  closeFileTab: (id: string) => void;
  activateFileTab: (id: string) => void;
  toggleRightPanel: () => void;
  setRightPanelOpen: (open: boolean) => void;

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
  stopGeneration: () => void;
  handleEvent: (ev: ServerEvent) => void;
}

export const useStore = create<AppState>((set, get) => ({
  sessions: [],
  currentSessionId: null,
  messages: [],
  info: null,
  plans: [],
  currentPlan: null,
  fileTabs: [],
  fileActiveTabId: null,
  running: false,
  wsConnected: false,
  serverModel: null,
  sidebarOpen: true,
  rightPanelOpen: false,
  settingsOpen: false,
  dirPickerOpen: false,
  theme: initialTheme(),
  lang: detectLang(),
  planMode: false,

  setWsState: (wsConnected, serverModel) => set({ wsConnected, serverModel }),
  toggleSidebar: () => set((st) => ({ sidebarOpen: !st.sidebarOpen })),
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
  setLang: (lang) => {
    localStorage.setItem(LANG_KEY, lang);
    set({ lang });
  },
  setPlanMode: (planMode) => set({ planMode }),
  openSettings: () => set({ settingsOpen: true }),
  closeSettings: () => set({ settingsOpen: false }),

  openFileTab: (path) => {
    const id = fileTabId(path);
    if (!get().fileTabs.some((t) => t.id === id)) {
      const title = path.split("/").filter(Boolean).pop() ?? path;
      set((st) => ({ fileTabs: [...st.fileTabs, { id, path, title }] }));
    }
    set({ fileActiveTabId: id, rightPanelOpen: true, settingsOpen: false });
  },

  closeFileTab: (id) => {
    const { fileTabs, fileActiveTabId } = get();
    const idx = fileTabs.findIndex((t) => t.id === id);
    if (idx < 0) return;
    const next = fileTabs.filter((t) => t.id !== id);
    let nextActive = fileActiveTabId;
    if (fileActiveTabId === id) {
      const neighbor = next[idx] ?? next[idx - 1] ?? null;
      nextActive = neighbor ? neighbor.id : null;
    }
    // 关闭最后一个文件时自动收起右侧面板
    set({ fileTabs: next, fileActiveTabId: nextActive, rightPanelOpen: next.length > 0 });
  },

  activateFileTab: (id) => set({ fileActiveTabId: id, settingsOpen: false }),

  toggleRightPanel: () => set((st) => ({ rightPanelOpen: !st.rightPanelOpen })),
  setRightPanelOpen: (rightPanelOpen) => set({ rightPanelOpen }),

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
    get().refreshSessions().catch(() => {});
    get().refreshPlans().catch(() => {});
  },

  removeSession: async (id) => {
    await api.deleteSession(id);
    if (get().currentSessionId === id) {
      set({ currentSessionId: null, messages: [], plans: [], currentPlan: null });
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

  /** 停止当前生成：后端无 cancel 消息，采用「重连 WS」取消本轮（ws.rs 发送失败即中断）。 */
  stopGeneration: () => {
    const msgs = [...get().messages];
    const last = msgs[msgs.length - 1];
    if (last && last.role === "assistant" && last.streaming) {
      msgs[msgs.length - 1] = {
        ...last,
        streaming: false,
        error:
          (last.error ? last.error + "\n" : "") +
          translate(get().lang, "chat.stopped"),
      };
      set({ messages: msgs });
    }
    set({ running: false });
    reconnectWs();
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
        // 计划内联展示在对话中：刷新后取最新一条作为当前计划卡片
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
