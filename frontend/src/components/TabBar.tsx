/** 右侧文件面板的标签栏：文件标签的激活与关闭。 */

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";

export function TabBar() {
  const { t } = useI18n();
  const fileTabs = useStore((s) => s.fileTabs);
  const fileActiveTabId = useStore((s) => s.fileActiveTabId);
  const activateFileTab = useStore((s) => s.activateFileTab);
  const closeFileTab = useStore((s) => s.closeFileTab);

  if (fileTabs.length === 0) {
    return (
      <div className="h-10 flex items-center px-3 text-xs text-ink-faint">
        {t("files.noneOpen")}
      </div>
    );
  }

  return (
    <div className="h-10 flex items-stretch overflow-x-auto">
      {fileTabs.map((tab) => {
        const active = tab.id === fileActiveTabId;
        return (
          <div
            key={tab.id}
            onClick={() => activateFileTab(tab.id)}
            title={tab.path}
            className={`group flex items-center gap-2 pl-3 pr-1.5 border-r border-line cursor-pointer max-w-[220px] ${
              active ? "bg-elevated text-ink" : "text-ink-muted hover:bg-hover"
            }`}
          >
            <span className="text-[10px] text-ink-faint shrink-0">▤</span>
            <span className="text-xs truncate">{tab.title}</span>
            <button
              type="button"
              title={t("tab.close")}
              onClick={(e) => {
                e.stopPropagation();
                closeFileTab(tab.id);
              }}
              className="text-ink-faint hover:text-ink text-sm leading-none px-1 shrink-0 rounded group-hover:bg-hover"
            >
              ×
            </button>
          </div>
        );
      })}
    </div>
  );
}
