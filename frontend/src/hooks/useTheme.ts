/** 主题 Hook：把 store 中的主题同步到 `html.light` 类，并提供切换方法。 */

import { useEffect } from "react";

import { useStore } from "../store";

export function useTheme() {
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);

  useEffect(() => {
    document.documentElement.classList.toggle("light", theme === "light");
  }, [theme]);

  const toggleTheme = () => setTheme(theme === "dark" ? "light" : "dark");

  return { theme, setTheme, toggleTheme };
}
