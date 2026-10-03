import { useEffect } from "react";

import { getRuntimeInfo } from "./api";
import { MOCK } from "./mock";
import { ChatView } from "./components/ChatView";
import { DirectoryPicker } from "./components/DirectoryPicker";
import { RightPanel } from "./components/RightPanel";
import { SettingsView } from "./components/SettingsView";
import { Sidebar } from "./components/Sidebar";
import { useStore } from "./store";
import { connectWs } from "./ws";

export default function App() {
  const sidebarOpen = useStore((s) => s.sidebarOpen);
  const view = useStore((s) => s.view);
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);
  const rightPanelOpen = useStore((s) => s.rightPanelOpen);
  const toggleSidebar = useStore((s) => s.toggleSidebar);
  const toggleRightPanel = useStore((s) => s.toggleRightPanel);
  const info = useStore((s) => s.info);
  const wsConnected = useStore((s) => s.wsConnected);
  const serverModel = useStore((s) => s.serverModel);
  const dirPickerOpen = useStore((s) => s.dirPickerOpen);
  const setDirPickerOpen = useStore((s) => s.setDirPickerOpen);
  const dirName = info?.cwd
    ? info.cwd.split("/").filter(Boolean).pop() ?? info.cwd
    : "选择目录";

  useEffect(() => {
    document.documentElement.classList.toggle("light", theme === "light");
    connectWs();
    useStore.getState().refreshSessions().catch(() => {});
    useStore.getState().refreshPlans().catch(() => {});
    getRuntimeInfo()
      .then((i) => useStore.setState({ info: i }))
      .catch(() => {});
    // 仅在挂载时执行一次；theme 用于初始化 html 类名
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex h-full bg-surface text-ink">
      {sidebarOpen && <Sidebar />}

      <main className="flex-1 flex flex-col min-w-0">
        <header className="h-12 shrink-0 border-b border-line flex items-center gap-3 px-4">
          <button
            type="button"
            onClick={toggleSidebar}
            title={sidebarOpen ? "收起侧栏" : "展开侧栏"}
            className="text-ink-muted hover:text-ink text-sm"
          >
            ☰
          </button>
          <div className="font-medium text-sm">SenAgent</div>
          {MOCK && (
            <span
              className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-400"
              title="GitHub Pages 静态预览：数据均为演示内容，未连接后端"
            >
              演示数据
            </span>
          )}
          <span className="text-xs text-ink-muted">
            {info ? `${info.provider} · ${info.model}` : serverModel ?? ""}
          </span>
          <span className="flex-1" />
          <span
            className={`w-2 h-2 rounded-full ${
              wsConnected ? "bg-emerald-400" : "bg-red-400"
            }`}
            title={wsConnected ? "已连接" : "未连接"}
          />
          <button
            type="button"
            onClick={() => setDirPickerOpen(true)}
            title={`项目目录：${info?.cwd ?? "未设置"}`}
            className="text-ink-muted hover:text-ink text-xs max-w-[200px] truncate"
          >
            目录：{dirName}
          </button>
          <button
            type="button"
            onClick={toggleRightPanel}
            title={rightPanelOpen ? "收起产出物面板" : "展开产出物面板"}
            className={`p-1 rounded hover:bg-elevated ${
              rightPanelOpen ? "text-accent" : "text-ink-muted hover:text-ink"
            }`}
          >
            <PanelRightIcon />
          </button>
          <button
            type="button"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            title={theme === "dark" ? "切换到浅色主题" : "切换到深色主题"}
            className="p-1 rounded hover:bg-elevated text-ink-muted hover:text-ink"
          >
            <ThemeIcon dark={theme === "dark"} />
          </button>
        </header>

        {view === "settings" ? <SettingsView /> : <ChatView />}
      </main>

      {rightPanelOpen && <RightPanel />}
      {dirPickerOpen && <DirectoryPicker />}
    </div>
  );
}

/** 右侧产出物面板开关图标。 */
function PanelRightIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <line x1="15" y1="3" x2="15" y2="21" />
    </svg>
  );
}

/** 主题切换图标（深色显示月亮，浅色显示太阳）。 */
function ThemeIcon({ dark }: { dark: boolean }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {dark ? (
        <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
      ) : (
        <>
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
        </>
      )}
    </svg>
  );
}
