/** 会话树：按项目（目录）分组展示会话；新建 / 切换 / 重命名 / 删除。
 *
 * 迁移自现 `Sidebar.tsx`，切换会话改为打开主区对话标签。
 */

import { useMemo, useState } from "react";

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import { formatTs } from "../utils";

export function SessionTree() {
  const { t } = useI18n();
  const sessions = useStore((s) => s.sessions);
  const currentId = useStore((s) => s.currentSessionId);
  const openSession = useStore((s) => s.openSession);
  const newSession = useStore((s) => s.newSession);
  const removeSession = useStore((s) => s.removeSession);
  const renameSession = useStore((s) => s.renameSession);
  const openChatTab = useStore((s) => s.openChatTab);
  const running = useStore((s) => s.running);
  const info = useStore((s) => s.info);

  const currentWs = info?.cwd ?? "";
  const home = info?.home ?? "";
  const [open, setOpen] = useState(true);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const projectLabel = (ws: string) => {
    if (!ws || (home && ws === home)) return t("sidebar.userDir");
    return ws.split("/").filter(Boolean).pop() ?? ws;
  };

  // 按项目目录分组；当前项目置顶，其余按最近更新时间降序
  const groups = useMemo(() => {
    const map = new Map<string, typeof sessions>();
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
    const title = window.prompt(t("sidebar.renamePrompt"), current);
    if (title && title.trim() && title !== current) {
      void renameSession(id, title.trim());
    }
  };

  const handleDelete = (id: string, title: string) => {
    if (
      window.confirm(
        t("sidebar.deleteConfirm", { title: title || t("sidebar.untitled") }),
      )
    ) {
      void removeSession(id);
    }
  };

  const handleOpen = (id: string) => {
    if (id === currentId) {
      openChatTab(id);
      return;
    }
    if (running) return;
    void openSession(id);
  };

  return (
    <section className="flex-1 min-h-0 flex flex-col">
      <div className="p-2 border-b border-line shrink-0">
        <div className="flex items-center gap-1.5 px-1">
          <button
            type="button"
            onClick={() => setOpen((v) => !v)}
            className="text-ink-faint text-[10px] w-2"
          >
            {open ? "▾" : "▸"}
          </button>
          <span className="text-xs font-medium text-ink-muted uppercase tracking-wide">
            {t("sidebar.sessions")}
          </span>
        </div>
        <button
          type="button"
          disabled={running}
          onClick={() => void newSession()}
          className="mt-2 w-full rounded-lg bg-primary text-primary-fg text-sm font-medium py-1.5 hover:bg-primary/90 disabled:opacity-50"
        >
          {t("sidebar.newSession")}
        </button>
      </div>

      {open && (
        <div className="flex-1 overflow-y-auto py-2 min-h-0">
          {sessions.length === 0 && (
            <div className="px-4 py-6 text-xs text-ink-muted">
              {t("sidebar.noSessions")}
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
                    {projectLabel(g.ws)}
                  </span>
                  <span className="text-[10px] text-ink-faint">
                    · {g.list.length}
                  </span>
                  {isCurrent && (
                    <span className="ml-auto text-[10px] text-accent">
                      {t("sidebar.current")}
                    </span>
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
                        onClick={() => handleOpen(s.id)}
                      >
                        <div className="flex items-center gap-2">
                          <div className="flex-1 min-w-0">
                            <div className="text-sm truncate text-ink">
                              {s.title || t("sidebar.untitled")}
                            </div>
                            <div className="text-xs text-ink-muted">
                              {formatTs(s.updated_at)} ·{" "}
                              {t("sidebar.messageCount", { n: s.message_count })}
                            </div>
                          </div>
                          <div className="hidden group-hover:flex items-center gap-1">
                            <button
                              type="button"
                              title={t("sidebar.rename")}
                              className="text-ink-muted hover:text-ink text-xs px-1"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleRename(s.id, s.title);
                              }}
                            >
                              {t("sidebar.rename")}
                            </button>
                            <button
                              type="button"
                              title={t("sidebar.delete")}
                              className="text-ink-muted hover:text-red-400 text-xs px-1"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleDelete(s.id, s.title);
                              }}
                            >
                              {t("sidebar.delete")}
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
      )}
    </section>
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
