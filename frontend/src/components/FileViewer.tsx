/** 文件查看标签内容：Markdown 渲染 / 源码预览。 */

import { useEffect, useState } from "react";

import { getFileContent } from "../api";
import { useI18n } from "../hooks/useI18n";
import { MarkdownBody } from "./MarkdownBody";

function isMarkdown(path: string): boolean {
  const lower = path.toLowerCase();
  return lower.endsWith(".md") || lower.endsWith(".markdown");
}

export function FileViewer({ path }: { path: string }) {
  const { t } = useI18n();
  const [content, setContent] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const md = isMarkdown(path);
  const [mode, setMode] = useState<"source" | "preview">(
    md ? "preview" : "source",
  );

  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    setContent(null);
    setMode(isMarkdown(path) ? "preview" : "source");
    getFileContent(path)
      .then((res) => {
        if (alive) setContent(res.content);
      })
      .catch((e: unknown) => {
        if (alive) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [path]);

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="h-9 shrink-0 border-b border-line flex items-center gap-3 px-4">
        <span className="text-xs text-ink-muted font-mono truncate">{path}</span>
        <span className="flex-1" />
        {md && (
          <div className="flex items-center gap-1.5 text-xs shrink-0">
            <button
              type="button"
              onClick={() => setMode("source")}
              className={
                mode === "source"
                  ? "text-ink"
                  : "text-ink-muted hover:text-ink"
              }
            >
              {t("file.source")}
            </button>
            <span className="text-ink-faint">/</span>
            <button
              type="button"
              onClick={() => setMode("preview")}
              className={
                mode === "preview"
                  ? "text-ink"
                  : "text-ink-muted hover:text-ink"
              }
            >
              {t("file.preview")}
            </button>
          </div>
        )}
      </div>
      <div className="flex-1 overflow-auto p-4">
        {loading ? (
          <div className="text-xs text-ink-muted">{t("file.loading")}</div>
        ) : error ? (
          <div className="text-xs text-red-400 break-all">
            {t("file.notFound")}: {error}
          </div>
        ) : md && mode === "preview" ? (
          <div className="text-sm text-ink max-w-3xl">
            <MarkdownBody content={content ?? ""} />
          </div>
        ) : (
          <pre className="text-xs font-mono whitespace-pre-wrap break-all text-ink-muted">
            {content}
          </pre>
        )}
      </div>
    </div>
  );
}
