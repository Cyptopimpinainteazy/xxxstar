/**
 * WindowManager — renders and manages all open application windows.
 *
 * Each window is a draggable, resizable floating container with a title bar,
 * minimize/maximize/close controls, and focus-on-click behaviour.
 */
import React, { useCallback, useRef, useEffect, lazy, Suspense } from "react";
import type { WindowState } from "@/types/window";
import { useDesktopStore } from "@/stores/desktopStore";
import { useApplicationStore } from "@/stores/applicationStore";
import { getPanelForApp } from "@/components/panels/panelRegistry";

const IframePanel = lazy(() => import("@/components/panels/IframePanel"));

/* ── Window Title Bar ──────────────────────────────────────── */
interface TitleBarProps {
  window: WindowState;
  onClose: () => void;
  onMinimize: () => void;
  onMaximize: () => void;
  onDragStart: (e: React.MouseEvent) => void;
}

const TitleBar: React.FC<TitleBarProps> = ({
  window: win,
  onClose,
  onMinimize,
  onMaximize,
  onDragStart,
}) => (
  <div
    className="flex items-center h-9 px-3 cursor-grab active:cursor-grabbing
      select-none shrink-0 rounded-t-lg"
    style={{
      background: `linear-gradient(90deg, ${win.accentColor ?? "#1a9fb5"}22 0%, transparent 40%)`,
      borderBottom: "1px solid rgba(255,255,255,0.06)",
    }}
    onMouseDown={onDragStart}
    role="toolbar"
    aria-label={`${win.title || "Window"} controls`}
  >
    {/* Title */}
    <span className="flex-1 text-xs font-medium text-text-primary truncate mr-2">
      {win.title || "Untitled"}
    </span>

    {/* Window controls */}
    <div className="flex items-center gap-1" onMouseDown={(e) => e.stopPropagation()}>
      <button
        className="title-bar-btn hover:bg-white/10"
        onClick={onMinimize}
        aria-label="Minimize"
        title="Minimize"
      >
        <svg width="10" height="10" viewBox="0 0 10 10">
          <rect y="4" width="10" height="2" fill="#a8a8a8" rx="1" />
        </svg>
      </button>
      <button
        className="title-bar-btn hover:bg-white/10"
        onClick={onMaximize}
        aria-label="Maximize"
        title="Maximize"
      >
        <svg width="10" height="10" viewBox="0 0 10 10">
          <rect
            x="1"
            y="1"
            width="8"
            height="8"
            fill="none"
            stroke="#a8a8a8"
            strokeWidth="1.5"
            rx="1"
          />
        </svg>
      </button>
      <button
        className="title-bar-btn hover:bg-red-500/30"
        onClick={onClose}
        aria-label="Close"
        title="Close"
      >
        <svg width="10" height="10" viewBox="0 0 10 10">
          <path d="M1 1L9 9M9 1L1 9" stroke="#a8a8a8" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      </button>
    </div>
  </div>
);

/* ── Single Window Container ───────────────────────────────── */
interface WindowContainerProps {
  window: WindowState;
}

const WindowContainer: React.FC<WindowContainerProps> = ({
  window: win,
}) => {
  const { closeWindow, focusWindow, minimizeWindow, maximizeWindow, moveWindow, resizeWindow } =
    useDesktopStore();

  const dragRef = useRef<{ startX: number; startY: number; origX: number; origY: number } | null>(
    null,
  );
  const resizeRef = useRef<{
    startX: number;
    startY: number;
    origW: number;
    origH: number;
  } | null>(null);

  // ── Drag handling ──────────────────────────────────────────
  const onDragStart = useCallback(
    (e: React.MouseEvent) => {
      if (win.isMaximized) return;
      e.preventDefault();
      dragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        origX: win.position.x,
        origY: win.position.y,
      };

      const onMove = (ev: MouseEvent) => {
        if (!dragRef.current) return;
        moveWindow(win.id, {
          x: dragRef.current.origX + (ev.clientX - dragRef.current.startX),
          y: Math.max(
            0,
            dragRef.current.origY + (ev.clientY - dragRef.current.startY),
          ),
        });
      };

      const onUp = () => {
        dragRef.current = null;
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [win.id, win.isMaximized, win.position, moveWindow],
  );

  // ── Resize handling (bottom-right corner) ──────────────────
  const onResizeStart = useCallback(
    (e: React.MouseEvent) => {
      if (win.isMaximized) return;
      e.preventDefault();
      e.stopPropagation();

      resizeRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        origW: win.size.width,
        origH: win.size.height,
      };

      const onMove = (ev: MouseEvent) => {
        if (!resizeRef.current) return;
        resizeWindow(win.id, {
          width: resizeRef.current.origW + (ev.clientX - resizeRef.current.startX),
          height: resizeRef.current.origH + (ev.clientY - resizeRef.current.startY),
        });
      };

      const onUp = () => {
        resizeRef.current = null;
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [win.id, win.isMaximized, win.size, resizeWindow],
  );

  if (win.isMinimized) return null;

  const isMax = win.isMaximized;
  const style: React.CSSProperties = isMax
    ? { inset: 0, width: "100%", height: "100%", zIndex: win.zIndex }
    : {
        left: win.position.x,
        top: win.position.y,
        width: win.size.width,
        height: win.size.height,
        zIndex: win.zIndex,
      };

  return (
    <div
      className={`absolute flex flex-col glass-panel rounded-lg shadow-window
        animate-fade-in ${win.isFocused ? "ring-1 ring-accent-primary/30" : ""}
        ${isMax ? "rounded-none" : ""}`}
      style={style}
      onMouseDown={() => focusWindow(win.id)}
      role="dialog"
      aria-label={win.title || "Application Window"}
    >
      <TitleBar
        window={win}
        onClose={() => closeWindow(win.id)}
        onMinimize={() => minimizeWindow(win.id)}
        onMaximize={() => maximizeWindow(win.id)}
        onDragStart={onDragStart}
      />

      {/* Content area */}
      <div className="flex-1 overflow-hidden bg-bg-primary/80 rounded-b-lg">
        <WindowContent appId={win.appId} title={win.title} />
      </div>

      {/* Resize handle */}
      {!isMax && (
        <div
          className="absolute bottom-0 right-0 w-6 h-6 cursor-se-resize bg-accent-primary/10 hover:bg-accent-primary/20 rounded-tl-lg transition-colors"
          onMouseDown={onResizeStart}
          role="separator"
          aria-label="Resize window"
        >
          <svg
            className="absolute bottom-1 right-1 opacity-60"
            width="12"
            height="12"
            viewBox="0 0 8 8"
          >
            <path d="M7 1L1 7M7 4L4 7M7 7L7 7" stroke="#888" strokeWidth="1" strokeLinecap="round" />
          </svg>
        </div>
      )}
    </div>
  );
};

/* ── Window Content — resolves what to render inside each window ── */
const WindowContent: React.FC<{ appId: string; title?: string }> = ({ appId, title }) => {
  // 1. Check if there's a dedicated panel
  const panel = getPanelForApp(appId);
  if (panel) return <>{panel}</>;

  // 2. Check if the app is a URL-type — render via iframe
  const app = useApplicationStore.getState().getApp(appId);
  if (app?.launchCommand.type === "url") {
    return (
      <Suspense fallback={
        <div className="flex items-center justify-center h-full bg-[#0a0a0f]">
          <div className="inline-block w-5 h-5 border-2 border-[#1a9fb5]/30 border-t-[#1a9fb5] rounded-full animate-spin" />
        </div>
      }>
        <IframePanel url={app.launchCommand.target} title={app.name} />
      </Suspense>
    );
  }

  // 3. Fallback placeholder
  return (
    <div className="flex items-center justify-center h-full text-text-secondary text-sm">
      <div className="text-center">
        <div className="text-3xl mb-2 opacity-40">
          {(title || "?").charAt(0).toUpperCase()}
        </div>
        <div className="text-xs text-text-secondary/60">{title || "Untitled Window"}</div>
      </div>
    </div>
  );
};

/* ── Window Manager Container ──────────────────────────────── */
const WindowManager: React.FC = () => {
  const windows = useDesktopStore((s) => s.windows);

  // ── Keyboard shortcuts ─────────────────────────────────────
  const {
    focusWindow,
    minimizeAll,
    closeWindow,
    activeWindowId,
  } = useDesktopStore();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Alt+Tab — cycle windows
      if (e.altKey && e.key === "Tab") {
        e.preventDefault();
        const visible = windows.filter((w) => !w.isMinimized);
        if (visible.length === 0) return;
        const currentIdx = visible.findIndex((w) => w.id === activeWindowId);
        const nextIdx = e.shiftKey
          ? (currentIdx - 1 + visible.length) % visible.length
          : (currentIdx + 1) % visible.length;
        focusWindow(visible[nextIdx].id);
      }

      // Alt+F4 — close active window
      if (e.altKey && e.key === "F4") {
        e.preventDefault();
        if (activeWindowId) closeWindow(activeWindowId);
      }

      // Meta+D — minimize all (show desktop)
      if ((e.metaKey || e.ctrlKey) && e.key === "d") {
        e.preventDefault();
        minimizeAll();
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [windows, activeWindowId, focusWindow, minimizeAll, closeWindow]);

  return (
    <>
      {windows.map((win) => (
        <WindowContainer key={win.id} window={win} />
      ))}
    </>
  );
};

export default WindowManager;
