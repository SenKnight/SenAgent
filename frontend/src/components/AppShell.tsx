/** 应用外壳（对齐 pi-web）：左会话栏 + 中聊天 + 右文件面板；编排设置面板与目录选择器。 */

import { useEffect } from "react";

import { getRuntimeInfo } from "../api";
import { useI18n } from "../hooks/useI18n";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import { useTheme } from "../hooks/useTheme";
import { useStore } from "../store";
import { connectWs } from "../ws";
import { ChatWindow } from "./ChatWindow";
import { DirectoryPicker } from "./DirectoryPicker";
import { RightPanel } from "./RightPanel";
import { SessionSidebar } from "./SessionSidebar";
import { SettingsPanel } from "./SettingsPanel";
import { TopBar } from "./TopBar";

export function AppShell() {
  useTheme();
  useKeyboardShortcuts();

  const sidebarOpen = useStore((s) => s.sidebarOpen);
  const settingsOpen = useStore((s) => s.settingsOpen);
  const rightPanelOpen = useStore((s) => s.rightPanelOpen);
  const dirPickerOpen = useStore((s) => s.dirPickerOpen);
  const currentSessionId = useStore((s) => s.currentSessionId);

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
        {settingsOpen ? (
          <SettingsPanel />
        ) : currentSessionId ? (
          <ChatWindow key={currentSessionId} sessionId={currentSessionId} />
        ) : (
          <EmptyMain />
        )}
      </main>

      {!settingsOpen && rightPanelOpen && <RightPanel />}

      {dirPickerOpen && <DirectoryPicker />}
    </div>
  );
}

/** 未选择会话时的空状态：新建会话入口。 */
function EmptyMain() {
  const { t } = useI18n();
  const newSession = useStore((s) => s.newSession);
  return (
    <div className="flex-1 flex items-center justify-center">
      <button
        type="button"
        onClick={() => void newSession()}
        className="flex items-center gap-2 rounded-lg bg-primary text-primary-fg text-sm font-medium px-4 py-2 hover:bg-primary/90"
      >
        <span className="text-lg leading-none">+</span>
        <span>{t("sidebar.newSession")}</span>
      </button>
    </div>
  );
}
