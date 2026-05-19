import "@testing-library/jest-dom";
import { vi } from "vitest";

// Mock window.__TAURI_INTERNALS__ to satisfy components with private runtime checks
if (typeof window !== "undefined") {
  (window as any).__TAURI_INTERNALS__ = {
    invoke: vi.fn(),
  };
}

// Global mock for @tauri-apps/api/core
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Global mock for lucide-react (prevents potential rendering issues with icons)
vi.mock("lucide-react", async () => {
  const actual = await vi.importActual("lucide-react");
  return {
    ...actual,
    // Add specific mocks if needed
  };
});
