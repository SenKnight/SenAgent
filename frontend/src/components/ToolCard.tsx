/** 工具调用可折叠卡片。 */

import { useState } from "react";

import type { UiTool } from "../types";

function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

export function ToolCard({ tool }: { tool: UiTool }) {
  const [open, setOpen] = useState(false);

  const status = tool.done ? (tool.isError ? "error" : "ok") : "running";
  const statusMeta = {
    running: { dot: "bg-amber-400 animate-pulse", text: "执行中" },
    ok: { dot: "bg-emerald-400", text: "完成" },
    error: { dot: "bg-red-400", text: "失败" },
  }[status];

  return (
    <div className="border border-zinc-800 rounded-lg bg-zinc-900/60 overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left text-sm hover:bg-zinc-800/50"
      >
        <span className={`w-2 h-2 rounded-full shrink-0 ${statusMeta.dot}`} />
        <span className="font-mono text-zinc-200">{tool.name}</span>
        <span className="text-xs text-zinc-500">{statusMeta.text}</span>
        <span className="flex-1" />
        <span className="text-xs text-zinc-500">{open ? "收起" : "展开"}</span>
      </button>

      {open && (
        <div className="border-t border-zinc-800 px-3 py-2 space-y-2">
          <div>
            <div className="text-xs text-zinc-500 mb-1">参数</div>
            <pre className="text-xs bg-zinc-950 rounded p-2 overflow-x-auto max-h-48 text-zinc-300">
              {prettyJson(tool.arguments)}
            </pre>
          </div>
          {tool.output !== undefined && (
            <div>
              <div className="text-xs text-zinc-500 mb-1">
                {tool.isError ? "错误" : "结果"}
              </div>
              <pre
                className={`text-xs rounded p-2 overflow-auto max-h-72 whitespace-pre-wrap break-all ${
                  tool.isError
                    ? "bg-red-950/40 text-red-300"
                    : "bg-zinc-950 text-zinc-300"
                }`}
              >
                {tool.output}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
