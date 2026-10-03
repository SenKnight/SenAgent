/** 文件树（只读）+「修改记录」可折叠区块；点击文件 → 打开文件标签。 */

import { useEffect, useMemo, useState } from "react";

import { listFiles } from "../api";
import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import type { FileNode, UiMessage } from "../types";

export function FileExplorer() {
  const { t } = useI18n();
  const cwd = useStore((s) => s.info?.cwd ?? "");
  const [open, setOpen] = useState(true);

  return (
    <section className="shrink-0 border-t border-line flex flex-col max-h-[45%]">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1.5 px-3 py-2 text-left"
      >
        <span className="text-ink-faint text-[10px] w-2">{open ? "▾" : "▸"}</span>
        <span className="text-xs font-medium text-ink-muted uppercase tracking-wide">
          {t("sidebar.files")}
        </span>
      </button>
      {open && (
        <div className="flex-1 min-h-0 flex flex-col">
          <FileTree key={cwd} />
          <ChangesBlock />
        </div>
      )}
    </section>
  );
}

function FileTree() {
  const { t } = useI18n();
  const openFileTab = useStore((s) => s.openFileTab);
  const [children, setChildren] = useState<Record<string, FileNode[]>>({});
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);

  const load = async (path: string) => {
    try {
      const nodes = await listFiles(path);
      setChildren((c) => ({ ...c, [path]: nodes }));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  useEffect(() => {
    void load("");
    // 仅在挂载时加载根目录
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const toggleDir = (path: string) => {
    setExpanded((e) => ({ ...e, [path]: !e[path] }));
    if (!children[path]) void load(path);
  };

  const renderNodes = (nodes: FileNode[], depth: number) =>
    nodes.map((n) => {
      const isOpen = !!expanded[n.path];
      return (
        <div key={n.path}>
          <button
            type="button"
            style={{ paddingLeft: `${depth * 12 + 8}px` }}
            className="w-full flex items-center gap-1 text-left text-xs py-1 pr-2 hover:bg-hover rounded"
            onClick={() => (n.is_dir ? toggleDir(n.path) : openFileTab(n.path))}
          >
            <span className="text-ink-faint w-3 shrink-0">
              {n.is_dir ? (isOpen ? "▾" : "▸") : ""}
            </span>
            <span className={n.is_dir ? "text-ink" : "text-ink-muted break-all"}>
              {n.name}
            </span>
          </button>
          {n.is_dir && isOpen && children[n.path] && (
            <div>{renderNodes(children[n.path], depth + 1)}</div>
          )}
        </div>
      );
    });

  return (
    <div className="flex-1 overflow-y-auto p-2 text-xs min-h-0">
      {error ? (
        <div className="text-red-400 px-2 py-1 break-all">{error}</div>
      ) : children[""] ? (
        renderNodes(children[""], 0)
      ) : (
        <div className="text-ink-faint px-2 py-1">{t("file.loading")}</div>
      )}
    </div>
  );
}

/* ----------------------------- 修改记录 ----------------------------- */

interface ChangeItem {
  path: string;
  op: string;
}

function deriveChanges(messages: UiMessage[]): ChangeItem[] {
  const map = new Map<string, string>();
  for (const m of messages) {
    for (const tool of m.tools) {
      if (tool.name === "write_file" || tool.name === "edit_file") {
        let path = "";
        try {
          path = (JSON.parse(tool.arguments) as { path?: string }).path ?? "";
        } catch {
          /* 忽略解析失败 */
        }
        if (path) map.set(path, tool.name);
      }
    }
  }
  return [...map.entries()].map(([path, op]) => ({ path, op }));
}

function ChangesBlock() {
  const { t } = useI18n();
  const messages = useStore((s) => s.messages);
  const changes = useMemo(() => deriveChanges(messages), [messages]);
  const [open, setOpen] = useState(false);

  return (
    <div className="shrink-0 border-t border-line">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-1.5 px-3 py-1.5 text-left"
      >
        <span className="text-ink-faint text-[10px] w-2">{open ? "▾" : "▸"}</span>
        <span className="text-[10px] uppercase tracking-wide text-ink-faint">
          {t("changes.title")}
        </span>
        {changes.length > 0 && (
          <span className="text-[10px] text-ink-faint">· {changes.length}</span>
        )}
      </button>
      {open && (
        <div className="max-h-40 overflow-y-auto px-2 pb-2">
          {changes.length === 0 ? (
            <div className="text-[11px] text-ink-muted px-2 py-1">
              {t("changes.empty")}
            </div>
          ) : (
            <ul className="space-y-1">
              {changes.map((c) => (
                <li
                  key={c.path}
                  className="flex items-center gap-2 px-2 py-1 rounded hover:bg-hover"
                >
                  <span
                    className={`text-[10px] shrink-0 px-1.5 py-0.5 rounded ${
                      c.op === "write_file"
                        ? "bg-emerald-500/15 text-emerald-400"
                        : "bg-amber-500/15 text-amber-400"
                    }`}
                  >
                    {c.op === "write_file"
                      ? t("changes.write")
                      : t("changes.edit")}
                  </span>
                  <span className="text-xs text-ink-muted font-mono break-all">
                    {c.path}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
