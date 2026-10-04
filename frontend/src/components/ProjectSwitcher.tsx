/** 项目切换器（对齐 pi-web）：按钮显示当前项目，下拉切换最近项目 / 默认目录 / 自定义路径。 */

import { useEffect, useMemo, useRef, useState } from "react";

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";

/** 把用户主目录前缀折叠为 `~`。 */
function displayPath(path: string, home: string): string {
  if (home && path.startsWith(home)) return "~" + path.slice(home.length);
  return path;
}

export function ProjectSwitcher() {
  const { t } = useI18n();
  const info = useStore((s) => s.info);
  const sessions = useStore((s) => s.sessions);
  const changeWorkspace = useStore((s) => s.changeWorkspace);
  const setDirPickerOpen = useStore((s) => s.setDirPickerOpen);

  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const boxRef = useRef<HTMLDivElement>(null);

  const cwd = info?.cwd ?? "";
  const home = info?.home ?? "";

  // 项目来源：当前目录 + 历史会话目录 + 用户主目录
  const projects = useMemo(() => {
    const set = new Set<string>();
    if (cwd) set.add(cwd);
    for (const s of sessions) if (s.workspace) set.add(s.workspace);
    if (home) set.add(home);
    return [...set];
  }, [cwd, home, sessions]);

  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return q ? projects.filter((p) => p.toLowerCase().includes(q)) : projects;
  }, [projects, filter]);

  // 点击组件外部时收起
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!boxRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  const pick = (path: string) => {
    setOpen(false);
    setFilter("");
    if (path && path !== cwd) void changeWorkspace(path);
  };

  return (
    <div ref={boxRef} className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        title={cwd}
        className={`w-full flex items-center gap-2 rounded-[7px] border px-2.5 py-1.5 text-left text-xs font-mono ${
          cwd ? "bg-hover border-line text-ink" : "border-accent/40 text-ink-faint"
        }`}
      >
        <span className="flex-1 truncate">
          {cwd ? displayPath(cwd, home) : t("sidebar.selectProject")}
        </span>
        <ChevronIcon />
      </button>

      {open && (
        <div className="absolute left-0 right-0 top-full z-40 mt-1 overflow-hidden rounded-lg border border-line bg-panel shadow-xl">
          <div className="border-b border-line p-1.5">
            <input
              value={filter}
              autoFocus
              onChange={(e) => setFilter(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  setFilter("");
                  setOpen(false);
                }
              }}
              placeholder={t("sidebar.filterProjects")}
              className="w-full rounded border border-line bg-surface px-2 py-1 text-[11px] font-mono outline-none"
            />
          </div>

          <div className="max-h-64 overflow-y-auto">
            {visible.map((p) => (
              <button
                key={p}
                type="button"
                onClick={() => pick(p)}
                title={p}
                className="flex w-full items-center gap-1.5 border-b border-line px-2.5 py-2 text-left text-[11px] font-mono last:border-b-0 hover:bg-hover"
              >
                <span className="w-2.5 shrink-0">{p === cwd && <CheckIcon />}</span>
                <span className="flex-1 truncate text-ink-muted">{displayPath(p, home)}</span>
              </button>
            ))}
            {visible.length === 0 && (
              <div className="px-2.5 py-2 text-[11px] text-ink-faint">
                {t("sidebar.noMatchingProjects")}
              </div>
            )}
          </div>

          <button
            type="button"
            onClick={() => pick(home)}
            className="flex w-full items-center gap-1.5 border-t border-line px-2.5 py-2 text-left text-[11px] text-ink-muted hover:bg-hover"
          >
            <FolderIcon />
            <span>{t("sidebar.useDefaultDirectory")}</span>
          </button>
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              setDirPickerOpen(true);
            }}
            className="flex w-full items-center gap-1.5 px-2.5 py-2 text-left text-[11px] text-ink-muted hover:bg-hover"
          >
            <PlusIcon />
            <span>{t("sidebar.customPath")}</span>
          </button>
        </div>
      )}
    </div>
  );
}

function ChevronIcon() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 10 10"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="shrink-0 opacity-70"
    >
      <polyline points="2 4 5 7 8 4" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg
      width="10"
      height="10"
      viewBox="0 0 10 10"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="text-accent"
    >
      <polyline points="1.5 5 4 7.5 8.5 2.5" />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="shrink-0"
    >
      <path d="M3 7a2 2 0 0 1 2-2h3.4l1.9 1.9H19a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 10 10"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      className="shrink-0"
    >
      <line x1="5" y1="1" x2="5" y2="9" />
      <line x1="1" y1="5" x2="9" y2="5" />
    </svg>
  );
}
