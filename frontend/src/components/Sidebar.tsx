/** 会话侧栏：新建 / 切换 / 重命名 / 删除。 */

import { useStore } from "../store";
import { formatTs } from "../utils";

export function Sidebar() {
  const sessions = useStore((s) => s.sessions);
  const currentId = useStore((s) => s.currentSessionId);
  const openSession = useStore((s) => s.openSession);
  const newSession = useStore((s) => s.newSession);
  const removeSession = useStore((s) => s.removeSession);
  const renameSession = useStore((s) => s.renameSession);
  const running = useStore((s) => s.running);

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
    <aside className="w-72 shrink-0 border-r border-zinc-800 bg-zinc-950 flex flex-col">
      <div className="p-3 border-b border-zinc-800">
        <button
          type="button"
          disabled={running}
          onClick={() => void newSession()}
          className="w-full rounded-lg bg-zinc-100 text-zinc-900 text-sm font-medium py-2 hover:bg-white disabled:opacity-50"
        >
          + 新会话
        </button>
      </div>

      <div className="flex-1 overflow-y-auto py-2">
        {sessions.length === 0 && (
          <div className="px-4 py-6 text-sm text-zinc-500">
            暂无会话，点击「新会话」开始对话。
          </div>
        )}
        {sessions.map((s) => {
          const active = s.id === currentId;
          return (
            <div
              key={s.id}
              className={`group mx-2 mb-1 rounded-lg px-3 py-2 cursor-pointer ${
                active ? "bg-zinc-800" : "hover:bg-zinc-900"
              }`}
              onClick={() => {
                if (!running && s.id !== currentId) void openSession(s.id);
              }}
            >
              <div className="flex items-center gap-2">
                <div className="flex-1 min-w-0">
                  <div className="text-sm truncate text-zinc-200">
                    {s.title || "未命名会话"}
                  </div>
                  <div className="text-xs text-zinc-500">
                    {formatTs(s.updated_at)} · {s.message_count} 条
                  </div>
                </div>
                <div className="hidden group-hover:flex items-center gap-1">
                  <button
                    type="button"
                    title="重命名"
                    className="text-zinc-500 hover:text-zinc-200 text-xs px-1"
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
                    className="text-zinc-500 hover:text-red-400 text-xs px-1"
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

      <div className="p-3 border-t border-zinc-800 text-xs text-zinc-600">
        SenAgent · 数据存储于 ~/.sen-agent
      </div>
    </aside>
  );
}
