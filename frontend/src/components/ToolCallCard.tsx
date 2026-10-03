/** 工具调用卡片：状态、参数、结果、复制。由现 `ToolCard.tsx` 演进。 */

import { useState } from "react";

import { useI18n } from "../hooks/useI18n";
import type { UiTool } from "../types";
import { MarkdownBody } from "./MarkdownBody";

function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

export function ToolCallCard({ tool }: { tool: UiTool }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [preview, setPreview] = useState(false);
  const [copied, setCopied] = useState(false);

  const status = tool.done ? (tool.isError ? "error" : "ok") : "running";
  const statusMeta = {
    running: { dot: "bg-amber-400 animate-pulse", text: t("tool.running") },
    ok: { dot: "bg-emerald-400", text: t("tool.done") },
    error: { dot: "bg-red-400", text: t("tool.failed") },
  }[status];

  const copy = () => {
    navigator.clipboard
      ?.writeText(tool.output ?? "")
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1500);
      })
      .catch(() => {});
  };

  return (
    <div className="border border-line rounded-lg bg-tool overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left text-sm hover:bg-hover"
      >
        <span className={`w-2 h-2 rounded-full shrink-0 ${statusMeta.dot}`} />
        <span className="font-mono text-ink">{tool.name}</span>
        <span className="text-xs text-ink-muted">{statusMeta.text}</span>
        <span className="flex-1" />
        <span className="text-xs text-ink-muted">
          {open ? t("tool.collapse") : t("tool.expand")}
        </span>
      </button>

      {open && (
        <div className="border-t border-line px-3 py-2 space-y-2">
          <div>
            <div className="text-xs text-ink-muted mb-1">{t("tool.arguments")}</div>
            <pre className="text-xs bg-surface rounded p-2 overflow-x-auto max-h-48 text-ink-muted">
              {prettyJson(tool.arguments)}
            </pre>
          </div>
          {tool.output !== undefined && (
            <div>
              <div className="flex items-center gap-2 mb-1">
                <div className="text-xs text-ink-muted">
                  {tool.isError ? t("tool.error") : t("tool.result")}
                </div>
                <div className="flex-1 text-right flex items-center justify-end gap-3">
                  {!tool.isError && (
                    <button
                      type="button"
                      onClick={() => setPreview((v) => !v)}
                      className="text-xs text-ink-muted hover:text-ink"
                    >
                      {preview ? t("file.source") : t("file.preview")}
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={copy}
                    className="text-xs text-ink-muted hover:text-ink"
                  >
                    {copied ? t("common.copied") : t("common.copy")}
                  </button>
                </div>
              </div>
              {preview && !tool.isError ? (
                <div className="text-sm rounded p-2 overflow-auto max-h-96 bg-surface text-ink">
                  <MarkdownBody content={tool.output} />
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
