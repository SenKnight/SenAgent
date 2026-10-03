/** 应用外壳：整体布局；编排标签页、设置面板、目录选择器；挂载快捷键与主题。 */

import { useEffect } from "react";

import { getRuntimeInfo } from "../api";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import { useTheme } from "../hooks/useTheme";
import { useStore } from "../store";
import { connectWs } from "../ws";
import { ChatWindow } from "./ChatWindow";
import { DirectoryPicker } from "./DirectoryPicker";
import { FileViewer } from "./FileViewer";
import { SessionSidebar } from "./SessionSidebar";
import { SettingsPanel } from "./SettingsPanel";
import { TabBar } from "./TabBar";
import { TopBar } from "./TopBar";

export function AppShell() {
  useTheme();
  useKeyboardShortcuts();

  const sidebarOpen = useStore((s) => s.sidebarOpen);
  const settingsOpen = useStore((s) => s.settingsOpen);
  const dirPickerOpen = useStore((s) => s.dirPickerOpen);

  useEffect(() => {
    connectWs();
    useStore.getState().refreshSessions().catch(() => {});
    useStore.getState().refreshPlans().catch(() => {});
    getRuntimeInfo()
      .then((i) => useStore.setState({ info: i }))
      .catch(() => {});
  }, []);

  return (
    <div className="flex h-full bg-surface text-ink">
      {sidebarOpen && <SessionSidebar />}

      <main className="flex-1 flex flex-col min-w-0 min-h-0">
        <TopBar />
        {settingsOpen ? <SettingsPanel /> : <MainTabs />}
      </main>

      {dirPickerOpen && <DirectoryPicker />}
    </div>
  );
}

/** 主区：标签栏 + 激活标签内容。 */
function MainTabs() {
  const tabs = useStore((s) => s.tabs);
  const activeTabId = useStore((s) => s.activeTabId);
  const active = tabs.find((t) => t.id === activeTabId) ?? null;

  return (
    <div className="flex-1 flex flex-col min-w-0 min-h-0">
      <TabBar />
      {!active ? (
        <EmptyMain />
      ) : active.kind === "chat" ? (
        <ChatWindow key={active.sessionId} sessionId={active.sessionId} />
      ) : (
        <FileViewer key={active.path} path={active.path} />
      )}
    </div>
  );
}

/** 无标签时的空状态：新建会话入口。 */
function EmptyMain() {
  const newSession = useStore((s) => s.newSession);
  return (
    <div className="flex-1 flex items-center justify-center">
      <button
        type="button"
        onClick={() => void newSession()}
        className="flex items-center gap-2 rounded-lg bg-primary text-primary-fg text-sm font-medium px-4 py-2 hover:bg-primary/90"
      >
        <span className="text-lg leading-none">+</span>
        <span>SenAgent</span>
      </button>
    </div>
  );
}
