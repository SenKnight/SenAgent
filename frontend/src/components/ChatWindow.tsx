/** 对话窗口：消息列表 + 自动滚动 + 内联计划卡片。 */

import { useEffect, useRef } from "react";

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import type { Plan } from "../types";
import { ChatInput, SET_INPUT_EVENT } from "./ChatInput";
import { MarkdownBody } from "./MarkdownBody";
import { MessageView } from "./MessageView";

export function ChatWindow({ sessionId }: { sessionId: string }) {
  const { t } = useI18n();
  const messages = useStore((s) => s.messages);
  const currentSessionId = useStore((s) => s.currentSessionId);
  const currentPlan = useStore((s) => s.currentPlan);

  const listRef = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);

  useEffect(() => {
    const el = listRef.current;
    if (el && stickToBottom.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages, currentPlan]);

  const onScroll = () => {
    const el = listRef.current;
    if (!el) return;
    stickToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
  };

  // 会话切换中：消息尚未加载，避免渲染到另一个会话
  if (sessionId !== currentSessionId) {
    return (
      <div className="flex-1 flex items-center justify-center text-sm text-ink-muted">
        {t("file.loading")}
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div
        ref={listRef}
        onScroll={onScroll}
        className="flex-1 overflow-y-auto px-4 md:px-8 py-6 space-y-5"
      >
        {messages.length === 0 && <EmptyState />}
        {messages.map((m) => (
          <MessageView key={m.key} message={m} />
        ))}
        {currentPlan && currentPlan.status === "draft" && (
          <PlanCard plan={currentPlan} />
        )}
      </div>
      <ChatInput sessionId={sessionId} />
    </div>
  );
}

/** 空状态：品牌 + 示例问题（点击填入输入框）。 */
function EmptyState() {
  const { t } = useI18n();
  const info = useStore((s) => s.info);
  const examples = [
    t("chat.example1"),
    t("chat.example2"),
    t("chat.example3"),
    t("chat.example4"),
  ];

  return (
    <div className="max-w-3xl mx-auto pt-16 text-center">
      <div className="text-2xl font-semibold mb-2">{t("app.title")}</div>
      <div className="text-sm text-ink-muted mb-8">{t("chat.emptySubtitle")}</div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-left">
        {examples.map((ex) => (
          <button
            key={ex}
            type="button"
            onClick={() =>
              window.dispatchEvent(new CustomEvent(SET_INPUT_EVENT, { detail: ex }))
            }
            className="rounded-xl border border-line bg-elevated/60 px-4 py-3 text-sm text-ink text-left hover:border-line-strong"
          >
            {ex}
          </button>
        ))}
      </div>
      {info && (
        <div className="mt-8 text-xs text-ink-faint">
          {t("chat.modelLine", {
            model: info.model,
            wire: info.wire_api,
            cwd: info.cwd,
          })}
        </div>
      )}
    </div>
  );
}

/** 计划卡片：内联在对话中，含「执行」按钮。 */
function PlanCard({ plan }: { plan: Plan }) {
  const { t } = useI18n();
  const executePlan = useStore((s) => s.executePlan);
  const running = useStore((s) => s.running);

  return (
    <div className="max-w-3xl mx-auto">
      <div className="border border-line rounded-xl overflow-hidden bg-elevated">
        <div className="flex items-center gap-2 px-3 py-2 border-b border-line">
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-accent/15 text-accent">
            {t("plan.draft")}
          </span>
          <span className="text-sm font-medium">{t("plan.title")}</span>
          <span className="flex-1" />
          <button
            type="button"
            disabled={running}
            onClick={() => void executePlan(plan.id)}
            className="text-xs px-3 py-1 rounded bg-primary text-primary-fg hover:opacity-90 disabled:opacity-50"
          >
            {t("plan.execute")}
          </button>
        </div>
        <div className="p-3 text-sm text-ink max-h-96 overflow-y-auto">
          <MarkdownBody content={plan.content} />
        </div>
      </div>
    </div>
  );
}
