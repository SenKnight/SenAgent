/** 工具调用可折叠卡片。 */

import { useState } from "react";

import type { UiTool } from "../types";
import { Markdown } from "./Markdown";

function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

export function ToolCard({ tool }: { tool: UiTool }) {
  const [open, setOpen] = useState(false);
  const [preview, setPreview] = useState(false);

  const status = tool.done ? (tool.isError ? "error" : "ok") : "running";
  const statusMeta = {
    running: { dot: "bg-amber-400 animate-pulse", text: "执行中" },
    ok: { dot: "bg-emerald-400", text: "完成" },
    error: { dot: "bg-red-400", text: "失败" },
  }[status];

  return (
    <div className="border border-line rounded-lg bg-elevated overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left text-sm hover:bg-hover"
      >
        <span className={`w-2 h-2 rounded-full shrink-0 ${statusMeta.dot}`} />
        <span className="font-mono text-ink">{tool.name}</span>
        <span className="text-xs text-ink-muted">{statusMeta.text}</span>
        <span className="flex-1" />
        <span className="text-xs text-ink-muted">{open ? "收起" : "展开"}</span>
      </button>

      {open && (
        <div className="border-t border-line px-3 py-2 space-y-2">
          <div>
            <div className="text-xs text-ink-muted mb-1">参数</div>
            <pre className="text-xs bg-surface rounded p-2 overflow-x-auto max-h-48 text-ink-muted">
              {prettyJson(tool.arguments)}
            </pre>
          </div>
          {tool.output !== undefined && (
            <div>
              <div className="flex items-center gap-2 mb-1">
                <div className="text-xs text-ink-muted">
                  {tool.isError ? "错误" : "结果"}
                </div>
                {!tool.isError && (
                  <div className="flex-1 text-right">
                    <button
                      type="button"
                      onClick={() => setPreview((v) => !v)}
                      className="text-xs text-ink-muted hover:text-ink"
                    >
                      {preview ? "源码" : "预览"}
                    </button>
                  </div>
                )}
              </div>
              {preview && !tool.isError ? (
                <div className="text-sm rounded p-2 overflow-auto max-h-96 bg-surface text-ink">
                  <Markdown content={tool.output} />
                </div>
              ) : (
                <pre
                  className={`text-xs rounded p-2 overflow-auto max-h-72 whitespace-pre-wrap break-all ${
                    tool.isError
                      ? "bg-red-950/40 text-red-300"
                      : "bg-surface text-ink-muted"
                  }`}
                >
                  {tool.output}
                </pre>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
