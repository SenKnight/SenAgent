/** 单条消息渲染：用户气泡 / 助手块（推理折叠、工具卡、Markdown、用量、错误）。 */

import { useState } from "react";

import { useI18n } from "../hooks/useI18n";
import type { UiMessage } from "../types";
import { MarkdownBody } from "./MarkdownBody";
import { ToolCallCard } from "./ToolCallCard";

export function MessageView({ message }: { message: UiMessage }) {
  const { t } = useI18n();

  if (message.role === "user") {
    return (
      <div className="max-w-3xl mx-auto flex justify-end">
        <div className="max-w-[85%] rounded-2xl rounded-br-md bg-user text-white px-4 py-2.5 text-sm whitespace-pre-wrap">
          {message.content}
        </div>
      </div>
    );
  }

  const hasTools = message.tools.length > 0;
  const showStreamingDot = message.streaming && !message.content && !hasTools;

  return (
    <div className="max-w-3xl mx-auto">
      <div className="flex items-center gap-2 mb-1.5">
        <span className="w-5 h-5 rounded-full bg-primary text-primary-fg text-xs flex items-center justify-center font-bold">
          S
        </span>
        <span className="text-xs text-ink-muted">{t("app.title")}</span>
        {message.streaming && (
          <span className="text-xs text-amber-400 animate-pulse">
            {t("chat.generating")}
          </span>
        )}
      </div>

      {message.reasoning && <ReasoningBlock text={message.reasoning} />}

      {hasTools && (
        <div className="space-y-2 mb-2">
          {message.tools.map((tool, i) => (
            <ToolCallCard key={`${tool.name}-${i}`} tool={tool} />
          ))}
        </div>
      )}

      {message.content && (
        <div className="text-sm text-ink">
          <MarkdownBody content={message.content} />
        </div>
      )}

      {showStreamingDot && (
        <span className="inline-block w-2 h-4 bg-ink-faint animate-pulse align-middle" />
      )}

      {message.error && (
        <div className="mt-2 rounded-lg border border-red-900 bg-red-950/40 px-3 py-2 text-xs text-red-300 whitespace-pre-wrap">
          {message.error}
        </div>
      )}

      {message.usage && (
        <div className="mt-2 text-xs text-ink-faint">
          {t("chat.usage", {
            input: message.usage.input_tokens,
            output: message.usage.output_tokens,
          })}
        </div>
      )}
    </div>
  );
}

function ReasoningBlock({ text }: { text: string }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  return (
    <div className="mb-2">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="text-xs text-ink-muted hover:text-ink"
      >
        {open ? t("chat.reasoningHide") : t("chat.reasoningShow")}
      </button>
      {open && (
        <pre className="mt-1 text-xs text-ink-muted bg-elevated/60 border border-line rounded-lg p-3 whitespace-pre-wrap max-h-64 overflow-y-auto">
          {text}
        </pre>
      )}
    </div>
  );
}
