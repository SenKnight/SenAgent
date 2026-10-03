/** 全局键盘快捷键。
 *
 * - `Ctrl/Cmd+N` 新会话；
 * - `Ctrl/Cmd+B` 折叠 / 展开侧栏；
 * - `Ctrl/Cmd+Enter` 发送（通过 `sen-send` 事件通知 ChatInput）；
 * - `Esc` 依次：关闭设置 → 关闭目录弹层 → 停止生成。
 */

import { useEffect } from "react";

import { useStore } from "../store";

/** ChatInput 监听的「发送当前输入」事件名。 */
export const SEND_EVENT = "sen-send";

export function useKeyboardShortcuts() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const store = useStore.getState();

      if (e.key === "Escape") {
        if (store.settingsOpen) {
          store.closeSettings();
          return;
        }
        if (store.dirPickerOpen) {
          store.setDirPickerOpen(false);
          return;
        }
        if (store.running) {
          store.stopGeneration();
        }
        return;
      }

      const mod = e.ctrlKey || e.metaKey;
      if (!mod) return;

      if (e.key === "Enter") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent(SEND_EVENT));
        return;
      }

      const k = e.key.toLowerCase();
      if (k === "n") {
        e.preventDefault();
        if (!store.running) void store.newSession();
      } else if (k === "b") {
        e.preventDefault();
        store.toggleSidebar();
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
