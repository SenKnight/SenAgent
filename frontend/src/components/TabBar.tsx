/** 主区标签栏：对话 / 文件标签的激活与关闭。 */

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";

export function TabBar() {
  const { t } = useI18n();
  const tabs = useStore((s) => s.tabs);
  const activeTabId = useStore((s) => s.activeTabId);
  const activateTab = useStore((s) => s.activateTab);
  const closeTab = useStore((s) => s.closeTab);

  if (tabs.length === 0) {
    return <div className="h-10 shrink-0 border-b border-line" />;
  }

  return (
    <div className="h-10 shrink-0 border-b border-line flex items-stretch overflow-x-auto">
      {tabs.map((tab) => {
        const active = tab.id === activeTabId;
        const title =
          tab.kind === "chat" ? tab.title || t("tab.untitled") : tab.title;
        return (
          <div
            key={tab.id}
            onClick={() => activateTab(tab.id)}
            className={`group flex items-center gap-2 pl-3 pr-1.5 border-r border-line cursor-pointer max-w-[220px] ${
              active ? "bg-elevated text-ink" : "text-ink-muted hover:bg-hover"
            }`}
          >
            <span className="text-[10px] text-ink-faint shrink-0">
              {tab.kind === "chat" ? "●" : "▤"}
            </span>
            <span className="text-xs truncate">{title}</span>
            <button
              type="button"
              title={t("tab.close")}
              onClick={(e) => {
                e.stopPropagation();
                closeTab(tab.id);
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
