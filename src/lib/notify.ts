/**
 * Уведомления Windows через tauri-plugin-notification.
 * Показываются, даже если окно менеджера свёрнуто. В браузерном
 * превью (не Tauri) — тихо игнорируются.
 */

import { isTauri } from "./bridge";

export async function showNotification(title: string, body: string): Promise<void> {
  if (!isTauri) return;
  try {
    const mod = await import("@tauri-apps/plugin-notification");
    let granted = await mod.isPermissionGranted();
    if (!granted) {
      granted = (await mod.requestPermission()) === "granted";
    }
    if (granted) {
      mod.sendNotification({ title, body });
    }
  } catch {
    /* плагин недоступен — тихо пропускаем */
  }
}
