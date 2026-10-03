/** 左栏容器：会话树 + 文件树两个可折叠区块；底部设置入口。 */

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import { FileExplorer } from "./FileExplorer";
import { SessionTree } from "./SessionTree";

export function SessionSidebar() {
  const { t } = useI18n();
  const openSettings = useStore((s) => s.openSettings);
  const settingsOpen = useStore((s) => s.settingsOpen);

  return (
    <aside className="w-72 shrink-0 border-r border-line bg-panel flex flex-col min-h-0">
      <SessionTree />
      <FileExplorer />
      <div className="p-2 border-t border-line shrink-0">
        <button
          type="button"
          onClick={openSettings}
          className={`w-full text-left text-xs px-2 py-1.5 rounded hover:bg-elevated ${
            settingsOpen ? "text-ink bg-hover" : "text-ink-muted hover:text-ink"
          }`}
        >
          {t("top.settings")}
        </button>
      </div>
    </aside>
  );
}
