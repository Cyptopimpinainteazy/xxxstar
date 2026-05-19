/**
 * useWindowManager — convenience hook for window management actions.
 */
import { useCallback } from "react";
import { useDesktopStore } from "@/stores/desktopStore";
import { useApplicationStore } from "@/stores/applicationStore";
import { launchApplication, stopApplication } from "@/services/applicationService";
import { CATEGORY_COLORS } from "@/types/application";

export function useWindowManager() {
  const openWindow = useDesktopStore((s) => s.openWindow);
  const closeWindow = useDesktopStore((s) => s.closeWindow);
  const getApp = useApplicationStore((s) => s.getApp);

  /**
   * Launch an application: start the process and open a window.
   */
  const launch = useCallback(
    async (appId: string) => {
      const app = getApp(appId);
      if (!app) {
        console.warn(`[WM] Unknown application: ${appId}`);
        return;
      }

      try {
        await launchApplication(app);
        openWindow(
          app.id,
          app.name,
          app.icon.color ?? CATEGORY_COLORS[app.category],
        );
      } catch (err) {
        console.error(`[WM] Failed to launch ${app.name}:`, err);
        // Still open the window to show the error state
        openWindow(
          app.id,
          `${app.name} (Error)`,
          "#ef5350",
        );
      }
    },
    [getApp, openWindow],
  );

  /**
   * Stop an application and close its window.
   */
  const stop = useCallback(
    async (appId: string) => {
      const windows = useDesktopStore.getState().windows;
      const win = windows.find((w) => w.appId === appId);
      if (win) closeWindow(win.id);
      await stopApplication(appId);
    },
    [closeWindow],
  );

  return { launch, stop };
}
