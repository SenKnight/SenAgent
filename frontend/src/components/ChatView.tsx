/** 对话主视图：消息列表 + 输入框 + 流式渲染。 */

import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { useStore } from "../store";
import { sendWs } from "../ws";
import { Markdown } from "./Markdown";
import { ToolCard } from "./ToolCard";

export function ChatView() {
  const messages = useStore((s) => s.messages);
  const running = useStore((s) => s.running);
  const wsConnected = useStore((s) => s.wsConnected);
  const currentSessionId = useStore((s) => s.currentSessionId);
  const appendUser = useStore((s) => s.appendUser);
  const startAssistant = useStore((s) => s.startAssistant);
  const finishWithError = useStore((s) => s.finishWithError);
  const setRunningState = useStore.setState;
  const newSession = useStore((s) => s.newSession);

  const [input, setInput] = useState("");
  const listRef = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);

  // 自动滚动（用户在底部附近时跟随）
  useEffect(() => {
    const el = listRef.current;
    if (el && stickToBottom.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages]);

  const onScroll = () => {
    const el = listRef.current;
    if (!el) return;
    stickToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
  };

  const send = async () => {
    const text = input.trim();
    if (!text || running) return;
    let sessionId = currentSessionId;
    try {
      if (!sessionId) {
        const session = await newSession();
        sessionId = session.id;
      }
      setInput("");
      appendUser(text);
      startAssistant();
      setRunningState({ running: true });
      stickToBottom.current = true;
      if (!sendWs({ type: "chat", session_id: sessionId, content: text })) {
        finishWithError("WebSocket 未连接，请稍候重试");
      }
    } catch (e) {
      finishWithError(e instanceof Error ? e.message : String(e));
    }
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div
        ref={listRef}
        onScroll={onScroll}
        className="flex-1 overflow-y-auto px-4 md:px-8 py-6 space-y-5"
      >
        {messages.length === 0 && <EmptyState />}
        {messages.map((m) => (
          <MessageRow key={m.key} message={m} />
        ))}
      </div>

      <div className="border-t border-zinc-800 p-3 md:p-4">
        <div className="max-w-3xl mx-auto flex items-end gap-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={onKeyDown}
            rows={Math.min(6, input.split("\n").length)}
            placeholder={
              wsConnected
                ? "输入消息，Enter 发送，Shift+Enter 换行"
                : "等待与后端建立连接…"
            }
            disabled={running}
            className="flex-1 resize-none rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-3 text-sm outline-none focus:border-zinc-500 disabled:opacity-60"
          />
          <button
            type="button"
            disabled={running || !input.trim()}
            onClick={() => void send()}
            className="rounded-xl bg-zinc-100 text-zinc-900 text-sm font-medium px-5 py-3 hover:bg-white disabled:opacity-40"
          >
            {running ? "生成中…" : "发送"}
          </button>
        </div>
      </div>
    </div>
  );
}

function EmptyState() {
  const info = useStore((s) => s.info);
  return (
    <div className="max-w-3xl mx-auto pt-16 text-center">
      <div className="text-2xl font-semibold mb-2">SenAgent</div>
      <div className="text-sm text-zinc-500 mb-8">
        跨平台个人 AI Agent · 工具调用 · 技能系统 · 会话记忆
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-left">
        {[
          "帮我看看当前目录里的项目结构，解释一下它是做什么的",
          "读取 README 并总结要点",
          "写一个 Python 脚本：扫描目录下所有重复文件",
          "创建一个技能：把我的周报写作流程固化下来",
        ].map((t) => (
          <ExampleCard key={t} text={t} />
        ))}
      </div>
      {info && (
        <div className="mt-8 text-xs text-zinc-600">
          当前模型: {info.model}（{info.wire_api} 协议） · 工作目录: {info.cwd}
        </div>
      )}
    </div>
  );
}

function ExampleCard({ text }: { text: string }) {
  const setInput = (v: string) => {
    const el = document.querySelector<HTMLTextAreaElement>("textarea");
    if (el) {
      const setter = Object.getOwnPropertyDescriptor(
        window.HTMLTextAreaElement.prototype,
        "value",
      )?.set;
      setter?.call(el, v);
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.focus();
    }
  };
  return (
    <button
      type="button"
      onClick={() => setInput(text)}
      className="rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 text-sm text-zinc-300 text-left hover:border-zinc-600"
    >
      {text}
    </button>
  );
}

function MessageRow({ message }: { message: import("../types").UiMessage }) {
  if (message.role === "user") {
    return (
      <div className="max-w-3xl mx-auto flex justify-end">
        <div className="max-w-[85%] rounded-2xl rounded-br-md bg-blue-600/90 text-white px-4 py-2.5 text-sm whitespace-pre-wrap">
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
        <span className="w-5 h-5 rounded-full bg-zinc-200 text-zinc-900 text-xs flex items-center justify-center font-bold">
          S
        </span>
        <span className="text-xs text-zinc-500">SenAgent</span>
        {message.streaming && (
          <span className="text-xs text-amber-400 animate-pulse">生成中…</span>
        )}
      </div>

      {message.reasoning && <ReasoningBlock text={message.reasoning} />}

      {hasTools && (
        <div className="space-y-2 mb-2">
          {message.tools.map((t, i) => (
            <ToolCard key={`${t.name}-${i}`} tool={t} />
          ))}
        </div>
      )}

      {message.content && (
        <div className="text-sm text-zinc-100">
          <Markdown content={message.content} />
        </div>
      )}

      {showStreamingDot && (
        <span className="inline-block w-2 h-4 bg-zinc-400 animate-pulse align-middle" />
      )}

      {message.error && (
        <div className="mt-2 rounded-lg border border-red-900 bg-red-950/40 px-3 py-2 text-xs text-red-300 whitespace-pre-wrap">
          {message.error}
        </div>
      )}

      {message.usage && (
        <div className="mt-2 text-xs text-zinc-600">
          用量: 输入 {message.usage.input_tokens} tokens / 输出{" "}
          {message.usage.output_tokens} tokens
        </div>
      )}
    </div>
  );
}

function ReasoningBlock({ text }: { text: string }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="mb-2">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="text-xs text-zinc-500 hover:text-zinc-300"
      >
        {open ? "收起推理过程" : "查看推理过程"}
      </button>
      {open && (
        <pre className="mt-1 text-xs text-zinc-500 bg-zinc-900/60 border border-zinc-800 rounded-lg p-3 whitespace-pre-wrap max-h-64 overflow-y-auto">
          {text}
        </pre>
      )}
    </div>
  );
}
