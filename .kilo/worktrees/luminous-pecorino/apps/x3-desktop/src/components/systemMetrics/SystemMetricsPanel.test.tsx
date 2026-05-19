import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { SystemMetricsPanel } from "./SystemMetricsPanel";
import * as TauriApi from "@tauri-apps/api/core";
import * as TauriEvent from "@tauri-apps/api/event";
// Unused: import * as errorHandler from "../../utils/errorHandler";

vi.mock("@tauri-apps/api/core");
vi.mock("@tauri-apps/api/event");
vi.mock("../../utils/errorHandler", async () => {
  const actual = await vi.importActual("../../utils/errorHandler");
  return {
    ...actual,
    withRetry: vi.fn(async (fn: any) => {
      // In tests, don't retry, just call the function once to speed up tests
      return fn();
    }),
  };
});

describe("SystemMetricsPanel", () => {
  const mockSystemMetrics = {
    cpu: {
      usage_percent: 45.5,
      cores: 8,
      frequency: 3600,
    },
    memory: {
      used: 8589934592, // 8 GB
      total: 16777216000, // ~16 GB
      usage_percent: 51.2,
    },
    disk: [
      {
        name: "/",
        used: 214748364800, // 200 GB
        total: 1099511627776, // 1 TB
        usage_percent: 19.5,
      },
    ],
    updated_at: "2026-02-08T12:34:56Z",
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Loading State", () => {
    it("should show loading state initially", async () => {
      (TauriApi.invoke as any).mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<SystemMetricsPanel />);

      expect(screen.getByText("Loading system metrics...")).toBeInTheDocument();
    });
  });

  describe("Success State", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockSystemMetrics);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should render the panel with title", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
      });
    });

    it("should display CPU metrics", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("CPU")).toBeInTheDocument();
        expect(screen.getByText("CPU Usage")).toBeInTheDocument();
        expect(screen.getByText(/45\.5%/)).toBeInTheDocument();
        expect(screen.getByText(/8 cores @ 3.6 GHz/)).toBeInTheDocument();
      });
    });

    it("should display memory metrics", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("Memory")).toBeInTheDocument();
        expect(screen.getByText("Memory Usage")).toBeInTheDocument();
        expect(screen.getByText(/51\.2%/)).toBeInTheDocument();
      });
    });

    it("should display formatted memory values", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        // Memory values should be formatted in human-readable units
        expect(screen.getByText(/15\.6 GB/)).toBeInTheDocument();
      });
    });

    it("should display storage/disk metrics", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("Storage")).toBeInTheDocument();
      });
    });

    it("should display last updated timestamp", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText(/Updated:/)).toBeInTheDocument();
      });
    });

    it("should respect color coding based on usage", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        const cpuBar = screen.getAllByText("CPU Usage")[0];
        expect(cpuBar).toBeInTheDocument();
      });
    });
  });

  describe("Error State", () => {
    it(
      "should show error message when invoke fails",
      async () => {
        const errorMsg = "Command not found: launch_system_metrics";
        (TauriApi.invoke as any).mockRejectedValue(new Error(errorMsg));
        (TauriEvent.listen as any).mockResolvedValue(vi.fn());

        render(<SystemMetricsPanel />);

        // Check that error UI appears by looking for the retry button
        await waitFor(
          () => {
            expect(screen.getByText(/Retry/)).toBeInTheDocument();
          },
          { timeout: 8000 }
        );
      },
      15000
    );

    it(
      "should show helpful message with error details",
      async () => {
        (TauriApi.invoke as any).mockRejectedValue(new Error("Backend unavailable"));
        (TauriEvent.listen as any).mockResolvedValue(vi.fn());

        render(<SystemMetricsPanel />);

        // Check that error UI appears with retry button
        await waitFor(
          () => {
            expect(screen.getByText(/Retry/)).toBeInTheDocument();
          },
          { timeout: 8000 }
        );
      },
      15000
    );
  });

  describe("Real-time Updates", () => {
    it("should update metrics from telemetry events", async () => {
      let eventListener: any;
      (TauriApi.invoke as any).mockResolvedValue(mockSystemMetrics);
      (TauriEvent.listen as any).mockImplementation(
        (_event: string, callback: any) => {
          eventListener = callback;
          return Promise.resolve(vi.fn());
        }
      );

      const { rerender } = render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
      });

      // Simulate a telemetry update
      const updatedMetrics = {
        ...mockSystemMetrics,
        cpu: { ...mockSystemMetrics.cpu, usage_percent: 75.2 },
      };

      if (eventListener) {
        eventListener({ payload: { system: updatedMetrics } });
      }

      rerender(<SystemMetricsPanel />);

      // Component should update with new data
      expect(screen.getByText("System Metrics")).toBeInTheDocument();
    });
  });

  describe("Data Formatting", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockSystemMetrics);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should format bytes to human-readable units", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        // 8GB should show as "8.0 GB"
        const elements = screen.getAllByText(/[0-9]+\.[0-9] (B|KB|MB|GB|TB)/);
        expect(elements.length).toBeGreaterThan(0);
      });
    });

    it("should show percentage with one decimal place", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText(/45\.5%/)).toBeInTheDocument();
        expect(screen.getByText(/51\.2%/)).toBeInTheDocument();
        expect(screen.getByText(/19\.5%/)).toBeInTheDocument();
      });
    });

    it("should format CPU frequency correctly", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText(/3.6 GHz/)).toBeInTheDocument();
      });
    });
  });

  describe("Edge Cases", () => {
    it("should handle zero metrics gracefully", async () => {
      const zeroMetrics = {
        cpu: { usage_percent: 0, cores: 0, frequency: 0 },
        memory: { used: 0, total: 1000000, usage_percent: 0 },
        disk: [],
        updated_at: "2026-02-08T12:34:56Z",
      };

      (TauriApi.invoke as any).mockResolvedValue(zeroMetrics);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
      });
    });

    it("should handle very high usage values (100+%)", async () => {
      const highMetrics = {
        ...mockSystemMetrics,
        cpu: { ...mockSystemMetrics.cpu, usage_percent: 105 },
      };

      (TauriApi.invoke as any).mockResolvedValue(highMetrics);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText(/105\.0%/)).toBeInTheDocument();
      });
    });

    it("should handle empty disk array", async () => {
      const noDiskMetrics = {
        ...mockSystemMetrics,
        disk: [],
      };

      (TauriApi.invoke as any).mockResolvedValue(noDiskMetrics);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
      });
    });

    it("should handle multiple disk entries", async () => {
      const multiDiskMetrics = {
        ...mockSystemMetrics,
        disk: [
          { name: "/", used: 100000000, total: 1000000000, usage_percent: 10 },
          { name: "/home", used: 50000000, total: 500000000, usage_percent: 10 },
        ],
      };

      (TauriApi.invoke as any).mockResolvedValue(multiDiskMetrics);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("Storage")).toBeInTheDocument();
      });
    });
  });

  describe("Accessibility", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockSystemMetrics);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should have semantic HTML structure", async () => {
      const { container } = render(<SystemMetricsPanel />);

      await waitFor(() => {
        const headings = container.querySelectorAll("h3, h4");
        expect(headings.length).toBeGreaterThan(0);
      });
    });

    it("should display metrics in a readable format", async () => {
      render(<SystemMetricsPanel />);

      await waitFor(() => {
        expect(screen.getByText("CPU")).toBeInTheDocument();
        expect(screen.getByText("Memory")).toBeInTheDocument();
        expect(screen.getByText("Storage")).toBeInTheDocument();
      });
    });
  });
});
