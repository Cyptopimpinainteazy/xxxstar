/**
 * Taskbar — bottom panel showing running applications with window previews.
 *
 * Features:
 * - Clock display
 * - Running app indicators (click to focus/restore)
 * - System tray placeholder
 */
import React, { useEffect, useState } from "react";
import { useDesktopStore } from "@/stores/desktopStore";
import { useApplicationStore } from "@/stores/applicationStore";
import { CATEGORY_COLORS, CATEGORY_LABELS } from "@/types/application";

const Taskbar: React.FC = () => {
  const windows = useDesktopStore((s) => s.windows);
  const { focusWindow, restoreWindow } = useDesktopStore();
  const applications = useApplicationStore((s) => s.applications);

  // Clock
  const [time, setTime] = useState(new Date());
  useEffect(() => {
    const timer = setInterval(() => setTime(new Date()), 30_000);
    return () => clearInterval(timer);
  }, []);

  const timeStr = time.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  const dateStr = time.toLocaleDateString([], {
    month: "short",
    day: "numeric",
    year: "numeric",
  });

  return (
    <div
      className="absolute bottom-0 left-0 right-0 h-11 glass-panel flex items-center
        px-3 z-[9999]"
      role="toolbar"
      aria-label="Taskbar"
    >
      {/* Start / brand mark */}
      <button
        className="flex items-center gap-2 px-3 py-1 rounded hover:bg-white/5
          transition-colors mr-2"
        aria-label="X3 Chain"
      >
        <span className="text-accent-primary font-bold text-sm">⬡</span>
        <span className="text-xs font-medium text-text-primary hidden sm:inline">
          X3
        </span>
      </button>

      {/* Divider */}
      <div className="w-px h-5 bg-border-default mr-2" />

      {/* Window buttons */}
      <div className="flex items-center gap-1 flex-1 overflow-x-auto">
        {windows.map((win) => {
          const app = applications.find((a) => a.id === win.appId);
          const accent = win.accentColor ?? CATEGORY_COLORS[app?.category ?? "other"];
          const emoji = CATEGORY_LABELS[app?.category ?? "other"];

          return (
            <button
              key={win.id}
              className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs
                transition-all max-w-[160px] truncate
                ${
                  win.isFocused
                    ? "bg-white/10 text-text-primary"
                    : "text-text-secondary hover:bg-white/5"
                }
                ${win.isMinimized ? "opacity-60" : ""}`}
              style={{
                borderBottom: win.isFocused
                  ? `2px solid ${accent}`
                  : "2px solid transparent",
              }}
              onClick={() =>
                win.isMinimized ? restoreWindow(win.id) : focusWindow(win.id)
              }
              title={win.title}
            >
              <span className="text-[10px]">{emoji}</span>
              <span className="truncate">{win.title}</span>
            </button>
          );
        })}
      </div>

      {/* System tray area */}
      <div className="flex items-center gap-3 ml-2">
        {/* Network indicator */}
        <div
          className="w-2 h-2 rounded-full bg-green-500"
          title="Connected"
        />

        {/* Clock */}
        <div className="text-right">
          <div className="text-xs text-text-primary leading-tight">
            {timeStr}
          </div>
          <div className="text-[9px] text-text-secondary leading-tight">
            {dateStr}
          </div>
        </div>
      </div>
    </div>
  );
};

export default React.memo(Taskbar);
