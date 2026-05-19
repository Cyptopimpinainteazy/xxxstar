/**
 * Window state and event types for the desktop window manager.
 */

/** Unique identifier for a managed window */
export type WindowId = string;

/** Position of a window on screen */
export interface WindowPosition {
  x: number;
  y: number;
}

/** Dimensions of a window */
export interface WindowSize {
  width: number;
  height: number;
}

/** Complete state of a managed desktop window */
export interface WindowState {
  /** Unique window identifier */
  id: WindowId;
  /** Associated application ID */
  appId: string;
  /** Display title in title bar */
  title: string;
  /** Position on screen */
  position: WindowPosition;
  /** Current dimensions */
  size: WindowSize;
  /** Minimum allowed dimensions */
  minSize: WindowSize;
  /** Stacking order (higher = on top) */
  zIndex: number;
  /** Whether the window is currently minimized */
  isMinimized: boolean;
  /** Whether the window is maximized */
  isMaximized: boolean;
  /** Whether the window is the active/focused window */
  isFocused: boolean;
  /** Accent color for the window title bar */
  accentColor?: string;
}

/** Serializable version of window state for localStorage persistence */
export interface PersistedWindowState {
  id: WindowId;
  appId: string;
  position: WindowPosition;
  size: WindowSize;
  isMaximized: boolean;
}

/** Actions dispatched to the window manager */
export type WindowAction =
  | { type: "OPEN"; payload: Omit<WindowState, "zIndex" | "isFocused"> }
  | { type: "CLOSE"; payload: { id: WindowId } }
  | { type: "FOCUS"; payload: { id: WindowId } }
  | { type: "MINIMIZE"; payload: { id: WindowId } }
  | { type: "RESTORE"; payload: { id: WindowId } }
  | { type: "MAXIMIZE"; payload: { id: WindowId } }
  | { type: "MOVE"; payload: { id: WindowId; position: WindowPosition } }
  | { type: "RESIZE"; payload: { id: WindowId; size: WindowSize } };

/** Default window dimensions */
export const DEFAULT_WINDOW_SIZE: WindowSize = { width: 800, height: 600 };
export const DEFAULT_MIN_WINDOW_SIZE: WindowSize = { width: 320, height: 240 };

/** Cascade offset for new windows */
export const CASCADE_OFFSET = 28;

/** Maximum z-index before reset */
export const MAX_Z_INDEX = 10000;
