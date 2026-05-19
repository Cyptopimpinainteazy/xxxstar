/**
 * Unit tests for the desktop window manager.
 *
 * Tests window lifecycle: open, close, focus, minimize, maximize, move, resize.
 * Tests cascade positioning, z-index management, and state persistence.
 *
 * Invariants tested: INV-UI-002 (window manager maintains consistent state)
 */
import { describe, it, expect, beforeEach } from "vitest";
import { useDesktopStore } from "../../src/stores/desktopStore";

// Reset the store before each test
beforeEach(() => {
  useDesktopStore.setState({
    windows: [],
    activeWindowId: null,
    nextZIndex: 1,
    iconSize: "medium",
    layout: "grid",
    iconOrder: [],
  });
});

describe("openWindow", () => {
  it("creates a new window with correct properties", () => {
    const id = useDesktopStore.getState().openWindow("test-app", "Test App");
    const state = useDesktopStore.getState();

    expect(state.windows).toHaveLength(1);
    expect(state.windows[0].id).toBe(id);
    expect(state.windows[0].appId).toBe("test-app");
    expect(state.windows[0].title).toBe("Test App");
    expect(state.windows[0].isFocused).toBe(true);
    expect(state.activeWindowId).toBe(id);
  });

  it("does not open duplicate windows for the same app", () => {
    const id1 = useDesktopStore.getState().openWindow("app-1", "App 1");
    const id2 = useDesktopStore.getState().openWindow("app-1", "App 1");
    const state = useDesktopStore.getState();

    expect(state.windows).toHaveLength(1);
    expect(id1).toBe(id2);
  });

  it("cascades window positions", () => {
    useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().openWindow("app-2", "App 2");
    const state = useDesktopStore.getState();

    const win1 = state.windows[0];
    const win2 = state.windows[1];
    // Positions should differ by CASCADE_OFFSET (28px)
    expect(win2.position.x - win1.position.x).toBe(28);
    expect(win2.position.y - win1.position.y).toBe(28);
  });

  it("unfocuses previous windows when a new one opens", () => {
    useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().openWindow("app-2", "App 2");
    const state = useDesktopStore.getState();

    expect(state.windows[0].isFocused).toBe(false);
    expect(state.windows[1].isFocused).toBe(true);
  });
});

describe("closeWindow", () => {
  it("removes the window from state", () => {
    const id = useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().closeWindow(id);
    const state = useDesktopStore.getState();

    expect(state.windows).toHaveLength(0);
    expect(state.activeWindowId).toBeNull();
  });

  it("focuses the previous window when active window is closed", () => {
    useDesktopStore.getState().openWindow("app-1", "App 1");
    const id2 = useDesktopStore.getState().openWindow("app-2", "App 2");
    useDesktopStore.getState().closeWindow(id2);
    const state = useDesktopStore.getState();

    expect(state.windows).toHaveLength(1);
    expect(state.windows[0].isFocused).toBe(true);
    expect(state.activeWindowId).toBe(state.windows[0].id);
  });
});

describe("focusWindow", () => {
  it("sets the focused window and updates z-index", () => {
    const id1 = useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().openWindow("app-2", "App 2");
    useDesktopStore.getState().focusWindow(id1);
    const state = useDesktopStore.getState();

    const win1 = state.windows.find((w) => w.id === id1)!;
    const win2 = state.windows.find((w) => w.appId === "app-2")!;

    expect(win1.isFocused).toBe(true);
    expect(win2.isFocused).toBe(false);
    expect(win1.zIndex).toBeGreaterThan(win2.zIndex);
  });

  it("restores minimized window when focused", () => {
    const id = useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().minimizeWindow(id);
    useDesktopStore.getState().focusWindow(id);
    const state = useDesktopStore.getState();

    const win = state.windows.find((w) => w.id === id)!;
    expect(win.isMinimized).toBe(false);
    expect(win.isFocused).toBe(true);
  });
});

describe("minimizeWindow", () => {
  it("sets isMinimized and removes focus", () => {
    const id = useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().minimizeWindow(id);
    const state = useDesktopStore.getState();

    const win = state.windows.find((w) => w.id === id)!;
    expect(win.isMinimized).toBe(true);
    expect(win.isFocused).toBe(false);
  });
});

describe("maximizeWindow", () => {
  it("toggles isMaximized", () => {
    const id = useDesktopStore.getState().openWindow("app-1", "App 1");

    useDesktopStore.getState().maximizeWindow(id);
    expect(
      useDesktopStore.getState().windows.find((w) => w.id === id)!.isMaximized,
    ).toBe(true);

    useDesktopStore.getState().maximizeWindow(id);
    expect(
      useDesktopStore.getState().windows.find((w) => w.id === id)!.isMaximized,
    ).toBe(false);
  });
});

describe("moveWindow", () => {
  it("updates the window position", () => {
    const id = useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().moveWindow(id, { x: 200, y: 300 });
    const state = useDesktopStore.getState();

    const win = state.windows.find((w) => w.id === id)!;
    expect(win.position).toEqual({ x: 200, y: 300 });
  });
});

describe("resizeWindow", () => {
  it("updates the window size", () => {
    const id = useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().resizeWindow(id, { width: 1000, height: 700 });
    const state = useDesktopStore.getState();

    const win = state.windows.find((w) => w.id === id)!;
    expect(win.size).toEqual({ width: 1000, height: 700 });
  });

  it("enforces minimum window size", () => {
    const id = useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().resizeWindow(id, { width: 100, height: 100 });
    const state = useDesktopStore.getState();

    const win = state.windows.find((w) => w.id === id)!;
    expect(win.size.width).toBeGreaterThanOrEqual(320);
    expect(win.size.height).toBeGreaterThanOrEqual(240);
  });
});

describe("minimizeAll", () => {
  it("minimizes all windows", () => {
    useDesktopStore.getState().openWindow("app-1", "App 1");
    useDesktopStore.getState().openWindow("app-2", "App 2");
    useDesktopStore.getState().minimizeAll();
    const state = useDesktopStore.getState();

    expect(state.windows.every((w) => w.isMinimized)).toBe(true);
    expect(state.activeWindowId).toBeNull();
  });
});

describe("desktop preferences", () => {
  it("updates icon size", () => {
    useDesktopStore.getState().setIconSize("large");
    expect(useDesktopStore.getState().iconSize).toBe("large");
  });

  it("updates layout", () => {
    useDesktopStore.getState().setLayout("list");
    expect(useDesktopStore.getState().layout).toBe("list");
  });

  it("updates icon order", () => {
    const order = ["app-3", "app-1", "app-2"];
    useDesktopStore.getState().setIconOrder(order);
    expect(useDesktopStore.getState().iconOrder).toEqual(order);
  });
});
