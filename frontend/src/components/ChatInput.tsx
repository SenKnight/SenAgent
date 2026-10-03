/** 输入区：计划模式开关 + 发送 / 停止。Enter 发送、Shift+Enter 换行。 */

import { useCallback, useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { useI18n } from "../hooks/useI18n";
import { SEND_EVENT } from "../hooks/useKeyboardShortcuts";
import { useStore } from "../store";
import { sendWs } from "../ws";

/** 空状态示例问题「填入输入框」事件名（由 ChatWindow 派发）。 */
export const SET_INPUT_EVENT = "sen-set-input";

export function ChatInput({ sessionId }: { sessionId: string }) {
  const { t } = useI18n();
  const running = useStore((s) => s.running);
  const wsConnected = useStore((s) => s.wsConnected);
  const planMode = useStore((s) => s.planMode);
  const setPlanMode = useStore((s) => s.setPlanMode);
  const stopGeneration = useStore((s) => s.stopGeneration);

  const [input, setInput] = useState("");
  const taRef = useRef<HTMLTextAreaElement>(null);
  const inputRef = useRef(input);
  inputRef.current = input;

  const send = useCallback(() => {
    const st = useStore.getState();
    const text = inputRef.current.trim();
    if (!text || st.running) return;
    setInput("");
    st.appendUser(text);
    st.startAssistant();
    useStore.setState({ running: true });
    if (
      !sendWs({
        type: "chat",
        session_id: sessionId,
        content: text,
        mode: st.planMode ? "plan" : "normal",
      })
    ) {
      st.finishWithError(t("chat.wsDisconnected"));
    }
  }, [sessionId, t]);

  // 快捷键 / 示例问题事件
  useEffect(() => {
    const onSend = () => send();
    const onSetInput = (e: Event) => {
      setInput((e as CustomEvent<string>).detail ?? "");
      taRef.current?.focus();
    };
    window.addEventListener(SEND_EVENT, onSend);
    window.addEventListener(SET_INPUT_EVENT, onSetInput);
    return () => {
      window.removeEventListener(SEND_EVENT, onSend);
      window.removeEventListener(SET_INPUT_EVENT, onSetInput);
    };
  }, [send]);

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      send();
    }
  };

  return (
    <div className="border-t border-line p-3 md:p-4">
      <div className="max-w-3xl mx-auto">
        <div className="flex items-center gap-2 mb-2">
          <button
            type="button"
            onClick={() => setPlanMode(!planMode)}
            title={t("chat.planTitle")}
            className={`text-xs rounded-full px-2.5 py-1 border ${
              planMode
                ? "border-accent text-accent bg-accent/10"
                : "border-line text-ink-muted hover:text-ink"
            }`}
          >
            {planMode ? t("chat.planOn") : t("chat.planOff")}
          </button>
          {planMode && (
            <span className="text-xs text-ink-faint">{t("chat.planHint")}</span>
          )}
        </div>
        <div className="flex items-end gap-2">
          <textarea
            ref={taRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={onKeyDown}
            rows={Math.min(6, input.split("\n").length)}
            placeholder={
              wsConnected ? t("chat.placeholder") : t("chat.placeholderDisconnected")
            }
            className="flex-1 resize-none rounded-xl border border-line-strong bg-elevated px-4 py-3 text-sm outline-none focus:border-ink-faint disabled:opacity-60"
          />
          {running ? (
            <button
              type="button"
              onClick={stopGeneration}
              title={t("chat.stop")}
              className="rounded-xl border border-line-strong text-sm font-medium px-5 py-3 hover:bg-hover"
            >
              {t("chat.stop")}
            </button>
          ) : (
            <button
              type="button"
              disabled={!input.trim()}
              onClick={send}
              className="rounded-xl bg-primary text-primary-fg text-sm font-medium px-5 py-3 hover:bg-primary/90 disabled:opacity-40"
            >
              {t("chat.send")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
