/** Markdown 渲染（GFM + 数学公式 + 代码高亮）。由现 `Markdown.tsx` 演进。 */

import { isValidElement, useState } from "react";
import type { ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";

import { useI18n } from "../hooks/useI18n";

export function MarkdownBody({ content }: { content: string }) {
  return (
    <div className="markdown">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMath]}
        rehypePlugins={[rehypeHighlight, rehypeKatex]}
        components={components}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}

const components: Components = { pre: CodeBlock };

/** 代码块：语言标签 + 复制按钮。 */
function CodeBlock({ children }: { children?: ReactNode }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  const first = Array.isArray(children) ? children[0] : children;
  let lang = "";
  let text = "";
  if (isValidElement(first)) {
    const props = first.props as { className?: string; children?: ReactNode };
    lang = /language-([\w-]+)/.exec(props.className ?? "")?.[1] ?? "";
    text = extractText(props.children);
  }

  const copy = () => {
    navigator.clipboard
      ?.writeText(text)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1500);
      })
      .catch(() => {});
  };

  return (
    <div className="code-block">
      <div className="code-block-bar">
        <span>{lang || "text"}</span>
        <button type="button" className="code-copy" onClick={copy}>
          {copied ? t("common.copied") : t("common.copy")}
        </button>
      </div>
      <pre>{children}</pre>
    </div>
  );
}

function extractText(node: ReactNode): string {
  if (node === null || node === undefined || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(extractText).join("");
  if (isValidElement(node)) {
    return extractText((node.props as { children?: ReactNode }).children);
  }
  return "";
}
