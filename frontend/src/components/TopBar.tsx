/** 顶栏：侧栏折叠、会话状态（模型 / token 用量）、连接状态、语言 / 主题切换、设置、文件面板开关。 */

import { useI18n } from "../hooks/useI18n";
import { useTheme } from "../hooks/useTheme";
import { MOCK } from "../mock";
import { useStore } from "../store";
import { ChatStatus } from "./ChatStatus";

const ICON_BUTTON =
  "flex h-9 w-9 items-center justify-center rounded-md text-ink-muted hover:text-ink hover:bg-elevated";

export function TopBar() {
  const { t, lang, setLang } = useI18n();
  const { theme, toggleTheme } = useTheme();
  const toggleSidebar = useStore((s) => s.toggleSidebar);
  const wsConnected = useStore((s) => s.wsConnected);
  const settingsOpen = useStore((s) => s.settingsOpen);
  const openSettings = useStore((s) => s.openSettings);
  const rightPanelOpen = useStore((s) => s.rightPanelOpen);
  const toggleRightPanel = useStore((s) => s.toggleRightPanel);
  const hasFileTabs = useStore((s) => s.fileTabs.length > 0);

  return (
    <header className="h-12 shrink-0 border-b border-line flex items-center gap-1.5 px-3">
      <button
        type="button"
        onClick={toggleSidebar}
        title={t("top.toggleSidebar")}
        className={ICON_BUTTON}
      >
        <MenuIcon />
      </button>

      {MOCK && (
        <span
          className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-400 shrink-0"
          title={t("app.demo")}
        >
          {t("app.demo")}
        </span>
      )}
      <div className="min-w-0 flex-1 px-1">
        <ChatStatus />
      </div>

      <span
        className={`w-2 h-2 rounded-full shrink-0 mr-1 ${
          wsConnected ? "bg-emerald-400" : "bg-red-400"
        }`}
        title={wsConnected ? t("top.connected") : t("top.reconnecting")}
      />

      <button
        type="button"
        onClick={() => setLang(lang === "zh" ? "en" : "zh")}
        title={t("top.lang")}
        className="h-9 px-2 rounded-md border border-line text-xs text-ink-muted hover:text-ink hover:bg-elevated"
      >
        {lang === "zh" ? "中" : "EN"}
      </button>

      <button
        type="button"
        onClick={toggleTheme}
        title={theme === "dark" ? t("top.themeToLight") : t("top.themeToDark")}
        className={ICON_BUTTON}
      >
        <ThemeIcon dark={theme === "dark"} />
      </button>

      <button
        type="button"
        onClick={openSettings}
        title={t("top.settings")}
        aria-pressed={settingsOpen}
        className={`${ICON_BUTTON} ${settingsOpen ? "text-ink bg-elevated" : ""}`}
      >
        <GearIcon />
      </button>

      <button
        type="button"
        onClick={toggleRightPanel}
        title={t("top.toggleFilePanel")}
        aria-pressed={rightPanelOpen}
        className={`${ICON_BUTTON} ${
          rightPanelOpen ? "text-ink bg-elevated" : ""
        }`}
      >
        <FilePanelIcon filled={hasFileTabs} />
      </button>
    </header>
  );
}

/** 侧栏折叠图标。 */
function MenuIcon() {
  return (
    <svg
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <line x1="9" y1="3" x2="9" y2="21" />
    </svg>
  );
}

/** 主题切换图标（深色显示月亮，浅色显示太阳）。 */
function ThemeIcon({ dark }: { dark: boolean }) {
  return (
    <svg
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
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
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="12" cy="12" r="3.2" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}

/** 文件面板图标（面板形状，有标签时半透明填充）。 */
function FilePanelIcon({ filled }: { filled: boolean }) {
  return (
    <svg
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="3" y="3" width="18" height="18" rx="2.5" />
      <line x1="15" y1="3" x2="15" y2="21" />
      {filled && (
        <rect
          x="15"
          y="3"
          width="6"
          height="18"
          fill="currentColor"
          opacity="0.35"
          stroke="none"
        />
      )}
    </svg>
  );
}
