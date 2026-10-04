/** 右侧文件面板（对齐 pi-web）：文件标签栏 + 内容查看；左侧边缘可拖动调宽，可折叠。 */

import { useCallback, useEffect, useRef, useState } from "react";

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import { FileViewer } from "./FileViewer";
import { TabBar } from "./TabBar";

const WIDTH_KEY = "sen-file-panel-width";
const DEFAULT_WIDTH = 460;
const MIN_WIDTH = 280;
const MIN_CENTER = 360;

export function RightPanel() {
  const { t } = useI18n();
  const fileTabs = useStore((s) => s.fileTabs);
  const fileActiveTabId = useStore((s) => s.fileActiveTabId);
  const setRightPanelOpen = useStore((s) => s.setRightPanelOpen);

  const [width, setWidth] = useState(() => {
    const saved = Number(localStorage.getItem(WIDTH_KEY));
    return Number.isFinite(saved) && saved >= MIN_WIDTH ? saved : DEFAULT_WIDTH;
  });
  const dragging = useRef(false);

  const onMove = useCallback((e: MouseEvent) => {
    if (!dragging.current) return;
    const max = Math.max(MIN_WIDTH, window.innerWidth - MIN_CENTER);
    const next = Math.min(max, Math.max(MIN_WIDTH, window.innerWidth - e.clientX));
    setWidth(next);
    localStorage.setItem(WIDTH_KEY, String(next));
  }, []);

  useEffect(() => {
    const stop = () => {
      dragging.current = false;
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", stop);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", stop);
    };
  }, [onMove]);

  const active = fileTabs.find((tb) => tb.id === fileActiveTabId) ?? null;

  return (
    <aside
      className="relative shrink-0 flex flex-col min-h-0 border-l border-line bg-surface"
      style={{ width }}
    >
      <div
        onMouseDown={() => {
          dragging.current = true;
        }}
        title={t("files.resizeHint")}
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-accent/40 z-10"
      />

      <div className="h-10 shrink-0 border-b border-line flex items-stretch bg-panel">
        <div className="flex-1 min-w-0 overflow-hidden">
          <TabBar />
        </div>
        <button
          type="button"
          onClick={() => setRightPanelOpen(false)}
          title={t("files.hidePanel")}
          className="w-9 shrink-0 flex items-center justify-center border-l border-line text-ink-muted hover:text-ink"
        >
          <svg
            width="15"
            height="15"
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
        </button>
      </div>

      <div className="flex-1 min-h-0 overflow-hidden">
        {active ? (
          <FileViewer key={active.path} path={active.path} />
        ) : (
          <div className="h-full flex items-center justify-center text-xs text-ink-faint">
            {t("files.noneOpen")}
          </div>
        )}
      </div>
    </aside>
  );
}
