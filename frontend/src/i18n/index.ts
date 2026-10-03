/** 轻量 i18n（无第三方依赖）。
 *
 * - 词典真源为 `zh.ts`，`Dict` 由 `keyof typeof zh` 派生，`en.ts` 必须覆盖同一组 key；
 * - 语言解析顺序：localStorage("sen-lang") → navigator.language → 默认 "zh"；
 * - 本模块**不依赖 store**，避免循环引用（store 单向依赖 i18n）。
 */

import { zh } from "./zh";
import { en } from "./en";

export type Lang = "zh" | "en";

/** 文案 key（由中文词典派生）。 */
export type TKey = keyof typeof zh;

/** 词典类型：key 与 zh 一致，value 为任意字符串。 */
export type Dict = Record<TKey, string>;

export const LANG_KEY = "sen-lang";

const dictionaries: Record<Lang, Dict> = { zh, en };

/** 解析初始语言：localStorage → 浏览器语言 → 默认 zh。 */
export function detectLang(): Lang {
  const saved = localStorage.getItem(LANG_KEY);
  if (saved === "zh" || saved === "en") return saved;
  const nav = (navigator.language || "").toLowerCase();
  if (nav.startsWith("zh")) return "zh";
  if (nav.startsWith("en")) return "en";
  return "zh";
}

/** 翻译并做 `{var}` 占位替换。 */
export function translate(
  lang: Lang,
  key: TKey,
  vars?: Record<string, string | number>,
): string {
  const dict = dictionaries[lang] ?? dictionaries.zh;
  let text = dict[key] ?? dictionaries.zh[key] ?? key;
  if (vars) {
    for (const k of Object.keys(vars)) {
      text = text.replace(new RegExp(`\\{${k}\\}`, "g"), String(vars[k]));
    }
  }
  return text;
}
