/** 右侧产出物面板：文件树（只读预览）/ 修改记录 / 计划。 */

import { useEffect, useMemo, useState } from "react";

import { getFileContent, listFiles } from "../api";
import { useStore, type RightPanelTab } from "../store";
import type { FileNode, UiMessage } from "../types";
import { Markdown } from "./Markdown";

const TABS: { id: RightPanelTab; label: string }[] = [
  { id: "files", label: "文件" },
  { id: "changes", label: "修改记录" },
  { id: "plans", label: "计划" },
];

export function RightPanel() {
  const tab = useStore((s) => s.rightPanelTab);
  const setTab = useStore((s) => s.setRightPanelTab);
  const toggle = useStore((s) => s.toggleRightPanel);
  const cwd = useStore((s) => s.info?.cwd ?? "");

  return (
    <aside className="w-96 shrink-0 border-l border-line bg-panel flex flex-col">
      <div className="p-3 border-b border-line flex items-center justify-between">
        <div className="text-sm font-medium">产出物</div>
        <button
          type="button"
          onClick={toggle}
          className="text-ink-muted hover:text-ink text-sm"
        >
          关闭
        </button>
      </div>

      <div className="flex border-b border-line text-xs">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`flex-1 py-2 ${
              tab === t.id
                ? "text-ink border-b-2 border-accent"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto">
        {tab === "files" && <FilesTab key={cwd} />}
        {tab === "changes" && <ChangesTab />}
        {tab === "plans" && <PlansTab />}
      </div>
    </aside>
  );
}

/* ------------------------------- 文件 ------------------------------- */

function FilesTab() {
  const [children, setChildren] = useState<Record<string, FileNode[]>>({});
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<{
    path: string;
    content: string;
    error?: string;
  } | null>(null);
  const [loading, setLoading] = useState(false);

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

  const openFile = async (path: string) => {
    setLoading(true);
    try {
      const res = await getFileContent(path);
      setSelected({ path, content: res.content });
    } catch (e) {
      setSelected({
        path,
        content: "",
        error: e instanceof Error ? e.message : String(e),
      });
    } finally {
      setLoading(false);
    }
  };

  if (selected) {
    return (
      <div className="p-3">
        <button
          type="button"
          onClick={() => setSelected(null)}
          className="text-xs text-ink-muted hover:text-ink mb-2"
        >
          ← 返回文件树
        </button>
        <div className="text-xs text-ink-muted font-mono break-all mb-2">
          {selected.path}
        </div>
        {selected.error ? (
          <div className="text-xs text-red-400 break-all">{selected.error}</div>
        ) : isMarkdown(selected.path) ? (
          <div className="text-sm text-ink">
            <Markdown content={selected.content} />
          </div>
        ) : (
          <pre className="text-xs bg-surface rounded p-2 overflow-auto whitespace-pre-wrap break-all text-ink-muted">
            {selected.content}
          </pre>
        )}
      </div>
    );
  }

  const renderNodes = (nodes: FileNode[], depth: number) =>
    nodes.map((n) => {
      const isOpen = !!expanded[n.path];
      return (
        <div key={n.path}>
          <button
            type="button"
            style={{ paddingLeft: `${depth * 12 + 8}px` }}
            className="w-full flex items-center gap-1 text-left text-xs py-1 pr-2 hover:bg-hover rounded"
            onClick={() => (n.is_dir ? toggleDir(n.path) : void openFile(n.path))}
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
    <div className="p-2 text-xs">
      {loading && <div className="text-ink-faint px-2 py-1">加载中…</div>}
      {error ? (
        <div className="text-red-400 px-2 py-1 break-all">{error}</div>
      ) : children[""] ? (
        renderNodes(children[""], 0)
      ) : (
        <div className="text-ink-faint px-2 py-1">加载中…</div>
      )}
    </div>
  );
}

function isMarkdown(path: string): boolean {
  const lower = path.toLowerCase();
  return lower.endsWith(".md") || lower.endsWith(".markdown");
}

/* ----------------------------- 修改记录 ----------------------------- */

interface ChangeItem {
  path: string;
  op: string;
}

function deriveChanges(messages: UiMessage[]): ChangeItem[] {
  const map = new Map<string, string>();
  for (const m of messages) {
    for (const t of m.tools) {
      if (t.name === "write_file" || t.name === "edit_file") {
        let path = "";
        try {
          path = (JSON.parse(t.arguments) as { path?: string }).path ?? "";
        } catch {
          /* 忽略解析失败 */
        }
        if (path) map.set(path, t.name);
      }
    }
  }
  return [...map.entries()].map(([path, op]) => ({ path, op }));
}

function ChangesTab() {
  const messages = useStore((s) => s.messages);
  const changes = useMemo(() => deriveChanges(messages), [messages]);

  if (changes.length === 0) {
    return (
      <div className="p-3 text-xs text-ink-muted">
        当前会话暂无文件写入/修改记录。
      </div>
    );
  }

  return (
    <ul className="p-2 space-y-1">
      {changes.map((c) => (
        <li
          key={c.path}
          className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-hover"
        >
          <span
            className={`text-[10px] shrink-0 px-1.5 py-0.5 rounded ${
              c.op === "write_file"
                ? "bg-emerald-500/15 text-emerald-400"
                : "bg-amber-500/15 text-amber-400"
            }`}
          >
            {c.op === "write_file" ? "写入" : "编辑"}
          </span>
          <span className="text-xs text-ink-muted font-mono break-all">{c.path}</span>
        </li>
      ))}
    </ul>
  );
}

/* ------------------------------- 计划 ------------------------------- */

function PlansTab() {
  const plans = useStore((s) => s.plans);
  const currentPlan = useStore((s) => s.currentPlan);
  const executePlan = useStore((s) => s.executePlan);
  const running = useStore((s) => s.running);
  const [openId, setOpenId] = useState<string | null>(null);

  useEffect(() => {
    if (currentPlan) setOpenId(currentPlan.id);
  }, [currentPlan]);

  if (plans.length === 0) {
    return (
      <div className="p-3 text-xs text-ink-muted">
        当前会话暂无计划。切换到「计划模式」发送消息即可生成计划。
      </div>
    );
  }

  return (
    <div className="p-2 space-y-2">
      {plans.map((p) => {
        const open = openId === p.id;
        const draft = p.status === "draft";
        return (
          <div key={p.id} className="border border-line rounded-lg overflow-hidden">
            <div className="flex items-center gap-2 px-2 py-1.5 bg-elevated">
              <button
                type="button"
                onClick={() => setOpenId(open ? null : p.id)}
                className="flex-1 text-left text-xs text-ink-muted hover:text-ink"
              >
                <span
                  className={`inline-block text-[10px] px-1.5 py-0.5 rounded mr-2 ${
                    draft
                      ? "bg-accent/15 text-accent"
                      : "bg-emerald-500/15 text-emerald-400"
                  }`}
                >
                  {draft ? "草稿" : "已执行"}
                </span>
                {new Date(p.created_at).toLocaleString()}
              </button>
              {draft && (
                <button
                  type="button"
                  disabled={running}
                  onClick={() => void executePlan(p.id)}
                  className="text-[10px] px-2 py-0.5 rounded bg-primary text-primary-fg hover:opacity-90 disabled:opacity-50"
                >
                  执行
                </button>
              )}
            </div>
            {open && (
              <div className="border-t border-line p-2 text-sm text-ink max-h-96 overflow-y-auto">
                <Markdown content={p.content} />
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
