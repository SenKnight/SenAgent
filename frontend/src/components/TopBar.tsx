/** 顶栏：品牌、项目目录入口、模型徽标、连接状态、语言 / 主题切换、侧栏折叠、设置。 */

import { useI18n } from "../hooks/useI18n";
import { useTheme } from "../hooks/useTheme";
import { MOCK } from "../mock";
import { useStore } from "../store";

export function TopBar() {
  const { t, lang, setLang } = useI18n();
  const { theme, toggleTheme } = useTheme();
  const sidebarOpen = useStore((s) => s.sidebarOpen);
  const toggleSidebar = useStore((s) => s.toggleSidebar);
  const info = useStore((s) => s.info);
  const wsConnected = useStore((s) => s.wsConnected);
  const serverModel = useStore((s) => s.serverModel);
  const setDirPickerOpen = useStore((s) => s.setDirPickerOpen);
  const openSettings = useStore((s) => s.openSettings);

  const dirName = info?.cwd
    ? (info.cwd.split("/").filter(Boolean).pop() ?? info.cwd)
    : null;
  const modelLabel = info ? `${info.provider} · ${info.model}` : serverModel ?? "";

  return (
    <header className="h-12 shrink-0 border-b border-line flex items-center gap-2 px-3">
      <button
        type="button"
        onClick={toggleSidebar}
        title={t("top.toggleSidebar")}
        className="text-ink-muted hover:text-ink text-sm px-1"
      >
        ☰
      </button>
      <div className="font-medium text-sm">{t("app.title")}</div>
      {MOCK && (
        <span
          className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-400"
          title={t("app.demo")}
        >
          {t("app.demo")}
        </span>
      )}
      <span className="text-xs text-ink-muted truncate hidden sm:inline">
        {modelLabel}
      </span>
      <span className="flex-1" />

      <span
        className={`w-2 h-2 rounded-full shrink-0 ${
          wsConnected ? "bg-emerald-400" : "bg-red-400"
        }`}
        title={
          wsConnected
            ? t("top.connected")
            : t("top.reconnecting")
        }
      />

      <button
        type="button"
        onClick={() => setDirPickerOpen(true)}
        title={t("top.chooseDir", { path: info?.cwd ?? t("top.noDir") })}
        className="text-ink-muted hover:text-ink text-xs max-w-[180px] truncate"
      >
        {t("top.project")}: {dirName ?? t("top.noDir")}
      </button>

      <button
        type="button"
        onClick={() => setLang(lang === "zh" ? "en" : "zh")}
        title={t("top.lang")}
        className="px-1.5 py-0.5 rounded border border-line text-xs text-ink-muted hover:text-ink hover:bg-elevated"
      >
        {lang === "zh" ? "中" : "EN"}
      </button>

      <button
        type="button"
        onClick={toggleTheme}
        title={theme === "dark" ? t("top.themeToLight") : t("top.themeToDark")}
        className="p-1 rounded hover:bg-elevated text-ink-muted hover:text-ink"
      >
        <ThemeIcon dark={theme === "dark"} />
      </button>

      <button
        type="button"
        onClick={openSettings}
        title={t("top.settings")}
        className="p-1 rounded hover:bg-elevated text-ink-muted hover:text-ink"
      >
        <GearIcon />
      </button>
    </header>
  );
}

/** 主题切换图标（深色显示月亮，浅色显示太阳）。 */
function ThemeIcon({ dark }: { dark: boolean }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {dark ? (
        <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
      ) : (
        <>
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
        </>
      )}
    </svg>
  );
}

/** 设置图标。 */
function GearIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}
