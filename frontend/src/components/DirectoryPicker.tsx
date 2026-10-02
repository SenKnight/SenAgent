/** 项目目录选择弹窗：服务端目录浏览器（逐级浏览并选择目录）。 */

import { useEffect, useState } from "react";

import { listDirs } from "../api";
import { useStore } from "../store";
import type { DirListing } from "../types";

export function DirectoryPicker() {
  const info = useStore((s) => s.info);
  const changeWorkspace = useStore((s) => s.changeWorkspace);
  const setDirPickerOpen = useStore((s) => s.setDirPickerOpen);

  const [listing, setListing] = useState<DirListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);

  const load = async (path: string) => {
    setLoading(true);
    try {
      const l = await listDirs(path);
      setListing(l);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  // 打开时从当前工作目录开始
  useEffect(() => {
    void load(info?.cwd ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setDirPickerOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setDirPickerOpen]);

  const apply = async () => {
    if (!listing) return;
    setApplying(true);
    try {
      await changeWorkspace(listing.path);
      setDirPickerOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
  };

  // 面包屑：把绝对路径按 "/" 拆成可点击层级
  const segments = (listing?.path ?? "").split("/").filter(Boolean);
  const crumbs = segments.map((name, i) => ({
    name,
    path: "/" + segments.slice(0, i + 1).join("/"),
  }));

  return (
    <div
      className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4"
      onClick={() => setDirPickerOpen(false)}
    >
      <div
        className="w-full max-w-xl max-h-[80vh] bg-panel border border-line rounded-xl shadow-2xl flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="p-3 border-b border-line flex items-center justify-between">
          <div className="text-sm font-medium">选择项目目录</div>
          <button
            type="button"
            onClick={() => setDirPickerOpen(false)}
            className="text-ink-muted hover:text-ink text-sm"
          >
            关闭
          </button>
        </div>

        <div className="px-3 py-2 border-b border-line text-xs text-ink-muted">
          <div className="flex items-center gap-1 flex-wrap font-mono">
            <button
              type="button"
              onClick={() => void load("/")}
              className="hover:text-ink"
            >
              /
            </button>
            {crumbs.map((c, i) => (
              <span key={c.path} className="flex items-center gap-1">
                {i > 0 && <span className="text-ink-faint">/</span>}
                <button
                  type="button"
                  onClick={() => void load(c.path)}
                  className="hover:text-ink break-all"
                >
                  {c.name}
                </button>
              </span>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-2">
          {listing?.parent && (
            <button
              type="button"
              onClick={() => void load(listing.parent!)}
              className="w-full text-left text-xs px-2 py-1.5 rounded hover:bg-hover text-ink-muted"
            >
              ↑ 上一级
            </button>
          )}
          {error && <div className="text-xs text-red-400 px-2 py-1 break-all">{error}</div>}
          {loading && !listing && (
            <div className="text-xs text-ink-faint px-2 py-1">加载中…</div>
          )}
          {listing && listing.dirs.length === 0 && (
            <div className="text-xs text-ink-faint px-2 py-1">（无子目录）</div>
          )}
          {listing?.dirs.map((d) => (
            <button
              key={d.path}
              type="button"
              onClick={() => void load(d.path)}
              className="w-full flex items-center gap-2 text-left text-xs px-2 py-1.5 rounded hover:bg-hover"
            >
              <span className="text-ink-faint">▸</span>
              <span className="text-ink break-all">{d.name}</span>
            </button>
          ))}
        </div>

        <div className="p-3 border-t border-line flex items-center gap-3">
          <div className="flex-1 text-xs text-ink-muted font-mono break-all">
            当前: {listing?.path ?? "…"}
          </div>
          <button
            type="button"
            onClick={() => setDirPickerOpen(false)}
            className="text-xs px-3 py-1.5 rounded border border-line text-ink-muted hover:text-ink"
          >
            取消
          </button>
          <button
            type="button"
            disabled={!listing || applying}
            onClick={() => void apply()}
            className="text-xs px-3 py-1.5 rounded bg-primary text-primary-fg hover:opacity-90 disabled:opacity-50"
          >
            {applying ? "应用中…" : "选择此目录"}
          </button>
        </div>
      </div>
    </div>
  );
}
