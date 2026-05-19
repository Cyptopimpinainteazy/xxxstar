/**
 * Desktop store — window management, layout, icon preferences.
 *
 * Persists window positions, icon order, and UI settings to localStorage.
 */
import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
  WindowState,
  WindowId,
  WindowPosition,
  WindowSize,
} from "@/types/window";
import {
  DEFAULT_WINDOW_SIZE,
  DEFAULT_MIN_WINDOW_SIZE,
  CASCADE_OFFSET,
  MAX_Z_INDEX,
} from "@/types/window";

export type IconSize = "small" | "medium" | "large";
export type DesktopLayout = "grid" | "list" | "custom";

export interface DesktopState {
  /* ── Window management ─────────────────────────────── */
  windows: WindowState[];
  activeWindowId: WindowId | null;
  nextZIndex: number;

  openWindow: (
    appId: string,
    title: string,
    accentColor?: string,
  ) => WindowId;
  closeWindow: (id: WindowId) => void;
  focusWindow: (id: WindowId) => void;
  minimizeWindow: (id: WindowId) => void;
  restoreWindow: (id: WindowId) => void;
  maximizeWindow: (id: WindowId) => void;
  moveWindow: (id: WindowId, pos: WindowPosition) => void;
  resizeWindow: (id: WindowId, size: WindowSize) => void;
  minimizeAll: () => void;
  clearAllWindows: () => void;

  /* ── Desktop preferences ───────────────────────────── */
  iconSize: IconSize;
  layout: DesktopLayout;
  iconOrder: string[]; // app IDs in display order
  iconSizes: Record<string, IconSize>; // individual icon sizes by app ID

  setIconSize: (size: IconSize) => void;
  setIconSizes: (sizes: Record<string, IconSize>) => void;
  setLayout: (layout: DesktopLayout) => void;
  setIconOrder: (order: string[]) => void;
}

let windowCounter = 0;

function nextWindowId(): WindowId {
  return `win_${++windowCounter}_${Date.now()}`;
}

/** Calculate cascaded position for a new window */
function cascadePosition(existingCount: number): WindowPosition {
  const offset = (existingCount % 12) * CASCADE_OFFSET;
  return { x: 80 + offset, y: 60 + offset };
}

export const useDesktopStore = create<DesktopState>()(
  persist(
    (set, get) => ({
      /* ── Initial state ──────────────────────────────── */
      windows: [],
      activeWindowId: null,
      nextZIndex: 1,
      iconSize: "medium",
      layout: "grid",
      iconOrder: [],
      iconSizes: {},

      /* ── Window actions ─────────────────────────────── */
      openWindow: (appId, title, accentColor) => {
        const state = get();
        // Don't open duplicate windows for the same app
        const existing = state.windows.find((w) => w.appId === appId);
        if (existing) {
          get().focusWindow(existing.id);
          return existing.id;
        }

        const id = nextWindowId();
        const position = cascadePosition(state.windows.length);
        const zIndex =
          state.nextZIndex >= MAX_Z_INDEX ? 1 : state.nextZIndex;

        const newWindow: WindowState = {
          id,
          appId,
          title,
          position,
          size: { ...DEFAULT_WINDOW_SIZE },
          minSize: { ...DEFAULT_MIN_WINDOW_SIZE },
          zIndex,
          isMinimized: false,
          isMaximized: false,
          isFocused: true,
          accentColor,
        };

        set((s) => ({
          windows: [
            ...s.windows.map((w) => ({ ...w, isFocused: false })),
            newWindow,
          ],
          activeWindowId: id,
          nextZIndex: zIndex + 1,
        }));

        return id;
      },

      closeWindow: (id) =>
        set((s) => {
          const remaining = s.windows.filter((w) => w.id !== id);
          const active =
            s.activeWindowId === id
              ? remaining.length > 0
                ? remaining[remaining.length - 1].id
                : null
              : s.activeWindowId;
          return {
            windows: remaining.map((w) => ({
              ...w,
              isFocused: w.id === active,
            })),
            activeWindowId: active,
          };
        }),

      focusWindow: (id) =>
        set((s) => {
          const zIndex =
            s.nextZIndex >= MAX_Z_INDEX ? 1 : s.nextZIndex;
          return {
            windows: s.windows.map((w) => ({
              ...w,
              isFocused: w.id === id,
              zIndex: w.id === id ? zIndex : w.zIndex,
              isMinimized: w.id === id ? false : w.isMinimized,
            })),
            activeWindowId: id,
            nextZIndex: zIndex + 1,
          };
        }),

      minimizeWindow: (id) =>
        set((s) => {
          const windows = s.windows.map((w) =>
            w.id === id ? { ...w, isMinimized: true, isFocused: false } : w,
          );
          // Focus the next non-minimized window
          const visible = windows.filter((w) => !w.isMinimized);
          const nextFocus =
            visible.length > 0 ? visible[visible.length - 1].id : null;
          return {
            windows: windows.map((w) => ({
              ...w,
              isFocused: w.id === nextFocus,
            })),
            activeWindowId: nextFocus,
          };
        }),

      restoreWindow: (id) => {
        get().focusWindow(id);
      },

      maximizeWindow: (id) =>
        set((s) => ({
          windows: s.windows.map((w) =>
            w.id === id ? { ...w, isMaximized: !w.isMaximized } : w,
          ),
        })),

      moveWindow: (id, pos) =>
        set((s) => ({
          windows: s.windows.map((w) =>
            w.id === id ? { ...w, position: pos } : w,
          ),
        })),

      resizeWindow: (id, size) =>
        set((s) => ({
          windows: s.windows.map((w) => {
            if (w.id !== id) return w;
            return {
              ...w,
              size: {
                width: Math.max(w.minSize.width, size.width),
                height: Math.max(w.minSize.height, size.height),
              },
            };
          }),
        })),

      minimizeAll: () =>
        set((s) => ({
          windows: s.windows.map((w) => ({
            ...w,
            isMinimized: true,
            isFocused: false,
          })),
          activeWindowId: null,
        })),

      clearAllWindows: () =>
        set(() => ({
          windows: [],
          activeWindowId: null,
        })),

      /* ── Desktop preference actions ─────────────────── */
      setIconSize: (iconSize) => set({ iconSize }),
      setIconSizes: (iconSizes) => set({ iconSizes }),
      setLayout: (layout) => set({ layout }),
      setIconOrder: (iconOrder) => set({ iconOrder }),
    }),
    {
      name: "x3-desktop-state",
      // Only persist a subset of state
      partialize: (state) => ({
        iconSize: state.iconSize,
        layout: state.layout,
        iconOrder: state.iconOrder,
        iconSizes: state.iconSizes,
        // Don't persist windows - start fresh each time
        // windows: state.windows.map(
        //   (w): PersistedWindowState => ({
        //     id: w.id,
        //     appId: w.appId,
        //     position: w.position,
        //     size: w.size,
        //     isMaximized: w.isMaximized,
        //   }),
        // ),
      }),
    },
  ),
);
