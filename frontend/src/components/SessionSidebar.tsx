/** 左栏容器（对齐 pi-web）：标题 + 新建会话 + 项目切换；下方会话列表 + 文件树。 */

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import { FileExplorer } from "./FileExplorer";
import { ProjectSwitcher } from "./ProjectSwitcher";
import { SessionTree } from "./SessionTree";

export function SessionSidebar() {
  const { t } = useI18n();
  const newSession = useStore((s) => s.newSession);
  const running = useStore((s) => s.running);

  return (
    <aside className="w-72 shrink-0 border-r border-line bg-panel flex flex-col min-h-0">
      <div className="p-2 border-b border-line shrink-0">
        <div className="flex items-center justify-between gap-2 px-0.5 mb-2">
          <span className="text-sm font-semibold text-ink">{t("app.title")}</span>
          <button
            type="button"
            disabled={running}
            onClick={() => void newSession()}
            title={t("sidebar.newSession")}
            className="flex items-center gap-1 h-8 rounded-[7px] border border-line bg-hover px-2 text-xs text-ink-muted hover:text-ink disabled:opacity-50"
          >
            <span className="text-sm leading-none">+</span>
            {t("sidebar.new")}
          </button>
        </div>
        <ProjectSwitcher />
      </div>

      <SessionTree />
      <FileExplorer />
    </aside>
  );
}
