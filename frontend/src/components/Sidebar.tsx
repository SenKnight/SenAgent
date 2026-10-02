/** 会话侧栏：按项目（目录）分组展示会话；新建 / 切换 / 重命名 / 删除。
 *
 * 底部放置「设置」与「主题切换」入口（左下角）。
 */

import { useMemo, useState } from "react";

import { useStore } from "../store";
import type { Session } from "../types";
import { formatTs } from "../utils";

/** 项目目录展示名：用户主目录显示为「用户目录」，其余取末级目录名。 */
function projectLabel(ws: string, home: string): string {
  if (!ws || (home && ws === home)) return "用户目录";
  const seg = ws.split("/").filter(Boolean).pop();
  return seg ?? ws;
}

export function Sidebar() {
  const sessions = useStore((s) => s.sessions);
  const currentId = useStore((s) => s.currentSessionId);
  const openSession = useStore((s) => s.openSession);
  const newSession = useStore((s) => s.newSession);
  const removeSession = useStore((s) => s.removeSession);
  const renameSession = useStore((s) => s.renameSession);
  const running = useStore((s) => s.running);
  const info = useStore((s) => s.info);
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);

  const currentWs = info?.cwd ?? "";
  const home = info?.home ?? "";
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  // 按项目目录分组；当前项目置顶，其余按最近更新时间降序
  const groups = useMemo(() => {
    const map = new Map<string, Session[]>();
    for (const s of sessions) {
      const ws = s.workspace || home;
      const list = map.get(ws);
      if (list) list.push(s);
      else map.set(ws, [s]);
    }
    return [...map.entries()]
      .map(([ws, list]) => ({
        ws,
        list: [...list].sort((a, b) => b.updated_at - a.updated_at),
      }))
      .sort((a, b) => {
        if (a.ws === currentWs) return -1;
        if (b.ws === currentWs) return 1;
        return (b.list[0]?.updated_at ?? 0) - (a.list[0]?.updated_at ?? 0);
      });
  }, [sessions, currentWs, home]);

  const handleRename = (id: string, current: string) => {
    const title = window.prompt("重命名会话", current);
    if (title && title.trim() && title !== current) {
      void renameSession(id, title.trim());
    }
  };

  const handleDelete = (id: string, title: string) => {
    if (window.confirm(`删除会话「${title || "未命名"}」？该操作不可恢复。`)) {
      void removeSession(id);
    }
  };

  return (
    <aside className="w-72 shrink-0 border-r border-line bg-panel flex flex-col">
      <div className="p-3 border-b border-line">
        <button
          type="button"
          disabled={running}
          onClick={() => {
            setView("chat");
            void newSession();
          }}
          className="w-full rounded-lg bg-primary text-primary-fg text-sm font-medium py-2 hover:bg-primary/90 disabled:opacity-50"
        >
          + 新会话
        </button>
        {info && (
          <div
            className="mt-2 text-[11px] text-ink-faint truncate"
            title={currentWs}
          >
            当前项目：{projectLabel(currentWs, home)}
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto py-2">
        {sessions.length === 0 && (
          <div className="px-4 py-6 text-sm text-ink-muted">
            暂无会话，点击「新会话」开始对话。
          </div>
        )}
        {groups.map((g) => {
          const isCurrent = g.ws === currentWs;
          const isCollapsed = !!collapsed[g.ws];
          return (
            <div key={g.ws} className="mb-1">
              <button
                type="button"
                title={g.ws}
                onClick={() =>
                  setCollapsed((c) => ({ ...c, [g.ws]: !c[g.ws] }))
                }
                className="w-full flex items-center gap-1.5 px-3 pt-2 pb-1 text-left"
              >
                <span className="text-ink-faint text-[10px] w-2">
                  {isCollapsed ? "▸" : "▾"}
                </span>
                <FolderIcon
                  className={isCurrent ? "text-accent" : "text-ink-faint"}
                />
                <span
                  className={`text-xs truncate ${
                    isCurrent ? "text-ink font-medium" : "text-ink-muted"
                  }`}
                >
                  {projectLabel(g.ws, home)}
                </span>
                <span className="text-[10px] text-ink-faint">
                  · {g.list.length}
                </span>
                {isCurrent && (
                  <span className="ml-auto text-[10px] text-accent">当前</span>
                )}
              </button>

              {!isCollapsed &&
                g.list.map((s) => {
                  const active = s.id === currentId;
                  return (
                    <div
                      key={s.id}
                      className={`group mx-2 mb-1 rounded-lg px-3 py-2 cursor-pointer ${
                        active ? "bg-hover" : "hover:bg-elevated"
                      }`}
                      onClick={() => {
                        setView("chat");
                        if (!running && s.id !== currentId) {
                          void openSession(s.id);
                        }
                      }}
                    >
                      <div className="flex items-center gap-2">
                        <div className="flex-1 min-w-0">
                          <div className="text-sm truncate text-ink">
                            {s.title || "未命名会话"}
                          </div>
                          <div className="text-xs text-ink-muted">
                            {formatTs(s.updated_at)} · {s.message_count} 条
                          </div>
                        </div>
                        <div className="hidden group-hover:flex items-center gap-1">
                          <button
                            type="button"
                            title="重命名"
                            className="text-ink-muted hover:text-ink text-xs px-1"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleRename(s.id, s.title);
                            }}
                          >
                            改名
                          </button>
                          <button
                            type="button"
                            title="删除"
                            className="text-ink-muted hover:text-red-400 text-xs px-1"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDelete(s.id, s.title);
                            }}
                          >
                            删除
                          </button>
                        </div>
                      </div>
                    </div>
                  );
                })}
            </div>
          );
        })}
      </div>

      <div className="p-2 border-t border-line">
        <button
          type="button"
          onClick={() => setView("settings")}
          className={`w-full text-left text-xs px-2 py-1.5 rounded hover:bg-elevated ${
            view === "settings" ? "text-ink bg-hover" : "text-ink-muted hover:text-ink"
          }`}
        >
          设置
        </button>
      </div>
    </aside>
  );
}

function FolderIcon({ className }: { className?: string }) {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`shrink-0 ${className ?? ""}`}
    >
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    </svg>
  );
}
