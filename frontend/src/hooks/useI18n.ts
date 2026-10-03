/** i18n Hook：从 store 读取当前语言，返回翻译函数 `t` 与切换方法。 */

import { useCallback } from "react";

import { translate, type Lang, type TKey } from "../i18n";
import { useStore } from "../store";

export function useI18n() {
  const lang = useStore((s) => s.lang);
  const setLang = useStore((s) => s.setLang);
  const t = useCallback(
    (key: TKey, vars?: Record<string, string | number>) =>
      translate(lang, key, vars),
    [lang],
  );
  return { lang: lang as Lang, setLang, t };
}
