/** 会话状态：模型 + 累计 token 用量（顶栏与输入框下方共用同一份展示）。 */

import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";

/** 紧凑数字格式：1200 → 1.2k。 */
const fmt = (n: number) =>
  n >= 1000 ? `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k` : String(n);

export function ChatStatus() {
  const { t } = useI18n();
  const info = useStore((s) => s.info);
  const serverModel = useStore((s) => s.serverModel);
  const messages = useStore((s) => s.messages);

  let input = 0;
  let output = 0;
  for (const m of messages) {
    if (m.usage) {
      input += m.usage.input_tokens;
      output += m.usage.output_tokens;
    }
  }

  const model = info ? `${info.provider} · ${info.model}` : serverModel ?? "";

  return (
    <div className="flex items-center gap-3 min-w-0 text-[11px] text-ink-muted font-mono">
      {model && <span className="truncate">{model}</span>}
      {input > 0 && (
        <span className="flex items-center gap-1 shrink-0" title={t("chat.tokensIn")}>
          <TokenIcon dir="up" />
          {fmt(input)}
        </span>
      )}
      {output > 0 && (
        <span className="flex items-center gap-1 shrink-0" title={t("chat.tokensOut")}>
          <TokenIcon dir="down" />
          {fmt(output)}
        </span>
      )}
    </div>
  );
}

/** 输入/输出 token 指示箭头。 */
function TokenIcon({ dir }: { dir: "up" | "down" }) {
  return (
    <svg
      width="10"
      height="10"
      viewBox="0 0 10 10"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.3"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {dir === "up" ? (
        <>
          <line x1="5" y1="8.5" x2="5" y2="1.5" />
          <polyline points="2 4 5 1.5 8 4" />
        </>
      ) : (
        <>
          <line x1="5" y1="1.5" x2="5" y2="8.5" />
          <polyline points="2 6 5 8.5 8 6" />
        </>
      )}
    </svg>
  );
}
