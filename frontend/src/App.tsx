import { useEffect } from "react";

import { getRuntimeInfo } from "./api";
import { ChatView } from "./components/ChatView";
import { SettingsPanel } from "./components/SettingsPanel";
import { Sidebar } from "./components/Sidebar";
import { useStore } from "./store";
import { connectWs } from "./ws";

export default function App() {
  const sidebarOpen = useStore((s) => s.sidebarOpen);
  const settingsOpen = useStore((s) => s.settingsOpen);
  const toggleSidebar = useStore((s) => s.toggleSidebar);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);
  const info = useStore((s) => s.info);
  const wsConnected = useStore((s) => s.wsConnected);
  const serverModel = useStore((s) => s.serverModel);

  useEffect(() => {
    connectWs();
    useStore.getState().refreshSessions().catch(() => {});
    getRuntimeInfo()
      .then((i) => useStore.setState({ info: i }))
      .catch(() => {});
  }, []);

  return (
    <div className="flex h-full">
      {sidebarOpen && <Sidebar />}

      <main className="flex-1 flex flex-col min-w-0">
        <header className="h-12 shrink-0 border-b border-zinc-800 flex items-center gap-3 px-4">
          <button
            type="button"
            onClick={toggleSidebar}
            title={sidebarOpen ? "收起侧栏" : "展开侧栏"}
            className="text-zinc-400 hover:text-zinc-100 text-sm"
          >
            ☰
          </button>
          <div className="font-medium text-sm">SenAgent</div>
          <span className="text-xs text-zinc-500">
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
            onClick={() => setSettingsOpen(!settingsOpen)}
            className="text-zinc-400 hover:text-zinc-100 text-sm"
          >
            设置
          </button>
        </header>

        <ChatView />
      </main>

      {settingsOpen && <SettingsPanel />}
    </div>
  );
}
