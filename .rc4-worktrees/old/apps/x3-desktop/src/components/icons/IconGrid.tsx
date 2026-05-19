/**
 * IconGrid — responsive grid layout of application icons.
 *
 * Supports category-based launcher views:
 * - Main: one canonical app per category
 * - Per-category: full category app set
 * - All Apps: everything available
 */
import React, { useCallback, useMemo, useState } from "react";
import ApplicationIcon from "./ApplicationIcon";
import type { Application } from "@/types/application";
import { useApplicationStore } from "@/stores/applicationStore";
import { useDesktopStore, type IconSize } from "@/stores/desktopStore";
import {
  DESKTOP_LAUNCHER_GROUPS,
  type DesktopLauncherCategory,
} from "@/config/appstore.config";

/** App ID that gets its own featured card (excluded from the grid) */
export const FEATURED_APP_ID = "blockchain-connector";

export interface IconGridProps {
  /** Applications to display */
  applications: Application[];
  /** Called when an application is launched */
  onLaunch: (appId: string) => void;
}

/**
 * 3-column layout: icons arranged in 3 columns, flowing top→bottom, then left→right
 * On smaller screens we allow overflow-y scroll.
 */
const COLS_MAP: Record<IconSize, string> = {
  small: "grid-cols-3",
  medium: "grid-cols-3",
  large: "grid-cols-3",
};

type LauncherView = "main" | "all" | DesktopLauncherCategory;

const tabClass = (active: boolean): string =>
  `px-2 py-1 text-[11px] rounded-md font-medium whitespace-nowrap ${
    active
      ? "bg-[#1a9fb5]/20 text-[#1a9fb5] border border-[#1a9fb5]/40"
      : "text-[#9ca3af] border border-transparent hover:border-white/20 hover:text-[#d1d5db]"
  }`;

const IconGrid: React.FC<IconGridProps> = ({ applications, onLaunch }) => {
  const [activeView, setActiveView] = useState<LauncherView>("main");
  const iconSize = useDesktopStore((s) => s.iconSize);
  const iconSizes = useDesktopStore((s) => s.iconSizes);
  const setIconSizes = useDesktopStore((s) => s.setIconSizes);
  const isRunning = useApplicationStore((s) => s.isRunning);

  const handleResize = useCallback(
    (appId: string, newSize: "small" | "medium" | "large") => {
      setIconSizes({ ...iconSizes, [appId]: newSize });
    },
    [iconSizes, setIconSizes],
  );

  const handleLaunch = useCallback(
    (appId: string) => {
      onLaunch(appId);
    },
    [onLaunch],
  );

  // Exclude the featured app — it renders as a separate hero card
  const gridApps = useMemo(
    () => applications.filter((a) => a.id !== FEATURED_APP_ID),
    [applications],
  );

  const appById = useMemo(() => {
    const map = new Map<string, Application>();
    gridApps.forEach((app) => map.set(app.id, app));
    return map;
  }, [gridApps]);

  const canonicalApps = useMemo(
    () =>
      DESKTOP_LAUNCHER_GROUPS
        .map((group) => appById.get(group.primaryAppId))
        .filter((app): app is Application => app != null),
    [appById],
  );

  const categoryApps = useMemo(() => {
    const map = new Map<DesktopLauncherCategory, Application[]>();
    DESKTOP_LAUNCHER_GROUPS.forEach((group) => {
      const apps = group.appIds
        .map((id) => appById.get(id))
        .filter((app): app is Application => app != null);
      map.set(group.id, apps);
    });
    return map;
  }, [appById]);

  const allApps = useMemo(() => {
    const seen = new Set<string>();
    const ordered: Application[] = [];

    DESKTOP_LAUNCHER_GROUPS.forEach((group) => {
      group.appIds.forEach((id) => {
        const app = appById.get(id);
        if (app && !seen.has(app.id)) {
          seen.add(app.id);
          ordered.push(app);
        }
      });
    });

    const remaining = gridApps
      .filter((app) => !seen.has(app.id))
      .sort((a, b) => a.name.localeCompare(b.name));

    return [...ordered, ...remaining];
  }, [appById, gridApps]);

  const visibleApps = useMemo(() => {
    if (activeView === "main") return canonicalApps;
    if (activeView === "all") return allApps;
    return categoryApps.get(activeView) ?? [];
  }, [activeView, canonicalApps, allApps, categoryApps]);

  if (applications.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary text-sm">
        No applications registered
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-1 px-3 pt-3 pb-2 overflow-x-auto border-b border-white/10">
        <button
          className={tabClass(activeView === "main")}
          onClick={() => setActiveView("main")}
          type="button"
        >
          Main
        </button>
        {DESKTOP_LAUNCHER_GROUPS.map((group) => (
          <button
            key={group.id}
            className={tabClass(activeView === group.id)}
            onClick={() => setActiveView(group.id)}
            type="button"
          >
            {group.label}
          </button>
        ))}
        <button
          className={tabClass(activeView === "all")}
          onClick={() => setActiveView("all")}
          type="button"
        >
          All Apps
        </button>
      </div>

      <div
        className={`grid ${COLS_MAP[iconSize]} gap-3 p-4 overflow-y-auto max-h-full auto-rows-min overflow-visible`}
        role="list"
        aria-label="Application launcher"
      >
        {visibleApps.map((app) => (
          <div key={app.id} role="listitem">
            <ApplicationIcon
              app={app}
              isRunning={isRunning(app.id)}
              onLaunch={handleLaunch}
              size={iconSizes[app.id] ?? iconSize}
              onResize={handleResize}
            />
          </div>
        ))}
        {visibleApps.length === 0 && (
          <div className="col-span-3 text-xs text-[#666] px-1 py-2">
            No applications in this view.
          </div>
        )}
      </div>
    </div>
  );
};

export default React.memo(IconGrid);
