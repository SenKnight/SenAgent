/** 会话列表（对齐 pi-web）：仅展示当前项目（目录）下的会话；选择 / 重命名 / 删除。 */

import { useMemo } from "react";

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import { formatTs } from "../utils";

export function SessionTree() {
  const { t } = useI18n();
  const sessions = useStore((s) => s.sessions);
  const currentId = useStore((s) => s.currentSessionId);
  const openSession = useStore((s) => s.openSession);
  const removeSession = useStore((s) => s.removeSession);
  const renameSession = useStore((s) => s.renameSession);
  const running = useStore((s) => s.running);
  const info = useStore((s) => s.info);

  const cwd = info?.cwd ?? "";
  const home = info?.home ?? "";

  // 仅展示当前项目下的会话（无 workspace 的历史会话归入用户主目录），按最近更新降序
  const list = useMemo(() => {
    const owned = sessions.filter((s) => (s.workspace || home) === cwd);
    return [...owned].sort((a, b) => b.updated_at - a.updated_at);
  }, [sessions, cwd, home]);

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
    if (id === currentId || running) return;
    void openSession(id);
  };

  return (
    <section className="flex-1 min-h-0 flex flex-col">
      <div className="px-3 pt-2 pb-1 shrink-0">
        <span className="text-[10px] font-medium text-ink-faint uppercase tracking-wide">
          {t("sidebar.sessions")}
        </span>
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-2 min-h-0">
        {list.length === 0 && (
          <div className="px-2 py-6 text-xs text-ink-muted">
            {t("sidebar.noSessions")}
          </div>
        )}
        {list.map((s) => {
          const active = s.id === currentId;
          return (
            <div
              key={s.id}
              className={`group mb-0.5 rounded-lg px-3 py-1.5 cursor-pointer ${
                active ? "bg-hover" : "hover:bg-elevated"
              }`}
              onClick={() => handleOpen(s.id)}
            >
              <div className="flex items-center gap-2">
                <div className="flex-1 min-w-0">
                  <div className="text-[13px] truncate text-ink">
                    {s.title || t("sidebar.untitled")}
                  </div>
                  <div className="text-[11px] text-ink-muted">
                    {formatTs(s.updated_at)} ·{" "}
                    {t("sidebar.messageCount", { n: s.message_count })}
                  </div>
                </div>
                <div className="hidden group-hover:flex items-center gap-1">
                  <button
                    type="button"
                    title={t("sidebar.rename")}
                    className="text-ink-muted hover:text-ink text-[11px] px-1"
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
                    className="text-ink-muted hover:text-red-400 text-[11px] px-1"
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
    </section>
  );
}
