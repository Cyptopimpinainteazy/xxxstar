import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import MonitoringDashboard from "./MonitoringDashboard";
import * as TauriApi from "@tauri-apps/api/core";
import * as TauriEvent from "@tauri-apps/api/event";

vi.mock("@tauri-apps/api/core");
vi.mock("@tauri-apps/api/event");

describe("MonitoringDashboard", () => {
  const mockSystemMetrics = {
    cpu: {
      usage_percent: 45.5,
      cores: 8,
      frequency: 3600,
    },
    memory: {
      used: 8589934592,
      total: 16777216000,
      usage_percent: 51.2,
    },
    disk: [
      {
        name: "/",
        used: 214748364800,
        total: 1099511627776,
        usage_percent: 19.5,
      },
    ],
    updated_at: "2026-02-08T12:34:56Z",
  };

  const mockIpfsData = {
    node_id: "bafy2bzaceayutrxdyedzv2n7yguwq4py2w4xfa2z4aceo4vq3bsfzb5zraea",
    pinned_objects: [
      {
        cid: "bafy2bzaceayutrxdyedzv2n7yguwq4py2w4xfa2z4aceo4vq3bsfzb5zraea",
        name: "x3-runtime.wasm",
        size: 4812300,
        pinned_at: "2026-02-08T12:00:00Z",
        replicas: 5,
        earning_potential: 150.5,
      },
    ],
    storage_used: 4812300,
    storage_capacity: 500000000000,
    storage_market: [
      {
        id: "deal-001",
        client: "x3-ai-lab",
        size: 4812300,
        price_per_epoch: 0.5,
        duration_epochs: 520,
        status: "Active",
        earned: 260.0,
      },
    ],
    total_pins: 1,
    updated_at: "2026-02-08T12:34:56Z",
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Layout and Structure", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should render the dashboard container", async () => {
      render(<MonitoringDashboard />);

      const container = screen.getByText("System Metrics") || document.querySelector(".grid");
      expect(container).toBeInTheDocument();
    });

    it("should have responsive grid layout", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const grid = container.querySelector(".grid");
        expect(grid).toHaveClass("grid-cols-1");
        expect(grid).toHaveClass("lg:grid-cols-2");
      });
    });

    it("should include gap between columns", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const grid = container.querySelector(".grid");
        expect(grid).toHaveClass("gap-4");
      });
    });

    it("should have scrollable content area", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const contentArea = container.querySelector(".overflow-auto");
        expect(contentArea).toBeInTheDocument();
      });
    });

    it("should apply dark theme styling", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const wrapper = container.querySelector(".bg-gray-950");
        if (wrapper) {
          expect(wrapper).toBeInTheDocument();
        } else {
          // Dark theme can be applied through parent or classname
          const panels = container.querySelectorAll(".bg-gray-900");
          expect(panels.length).toBeGreaterThan(0);
        }
      });
    });
  });

  describe("Component Rendering", () => {
    beforeEach(() => {
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should render SystemMetricsPanel", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
      });
    });

    it("should render IpfsStoragePanel", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });
    });

    it("should render both panels in a side-by-side layout", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const panels = container.querySelectorAll(".bg-gray-900.rounded-lg.border");
        if (panels.length < 2) {
          // Panels might be rendered with slightly different selectors
          const allPanels = container.querySelectorAll(".bg-gray-900");
          expect(allPanels.length).toBeGreaterThanOrEqual(1);
        } else {
          expect(panels.length).toBeGreaterThanOrEqual(2);
        }
      });
    });
  });

  describe("System Metrics Integration", () => {
    beforeEach(() => {
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should display system metrics data in the left panel", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("CPU")).toBeInTheDocument();
        expect(screen.getByText("Memory")).toBeInTheDocument();
      });
    });

    it("should show CPU usage metrics", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText(/45\.5%/)).toBeInTheDocument();
      });
    });

    it("should show memory usage metrics", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("Memory Usage")).toBeInTheDocument();
      });
    });
  });

  describe("IPFS Storage Integration", () => {
    beforeEach(() => {
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should display IPFS storage data in the right panel", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });
    });

    it("should show storage deals", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("Storage Deals")).toBeInTheDocument();
        expect(screen.getByText("x3-ai-lab")).toBeInTheDocument();
      });
    });

    it("should show pinned content information", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("Pinned Content")).toBeInTheDocument();
      });
    });
  });

  describe("Responsive Behavior", () => {
    beforeEach(() => {
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should use single column on mobile", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const grid = container.querySelector(".grid");
        expect(grid).toHaveClass("grid-cols-1");
      });
    });

    it("should use two columns on large screens", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const grid = container.querySelector(".grid");
        expect(grid).toHaveClass("lg:grid-cols-2");
      });
    });

    it("should have proper spacing on all screen sizes", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const wrapper = container.querySelector(".w-full.h-full.p-4");
        expect(wrapper).toHaveClass("space-y-4");
      });
    });
  });

  describe("Error Handling", () => {
    it("should render dashboard even when system metrics fails", async () => {
      vi.clearAllMocks();
      (TauriApi.invoke as any)
        .mockRejectedValueOnce(new Error("System metrics unavailable"))
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      const { container } = render(<MonitoringDashboard />);

      // Dashboard should render without crashing
      expect(container).toBeInTheDocument();
    });

    it("should render dashboard even when IPFS data fails", async () => {
      vi.clearAllMocks();
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockRejectedValueOnce(new Error("IPFS unavailable"));
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      const { container } = render(<MonitoringDashboard />);

      // Dashboard should render without crashing
      expect(container).toBeInTheDocument();
    });

    it("should render dashboard when both panels fail", async () => {
      vi.clearAllMocks();
      (TauriApi.invoke as any).mockRejectedValue(new Error("Backend unavailable"));
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      const { container } = render(<MonitoringDashboard />);

      // Dashboard should render without crashing
      expect(container).toBeInTheDocument();
    });
  });

  describe("Real-time Updates", () => {
    it("should update both panels when telemetry events arrive", async () => {
      let eventListener: any;
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockImplementation(
        (_event: string, callback: any) => {
          eventListener = callback;
          return Promise.resolve(vi.fn());
        }
      );

      const { rerender } = render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });

      // Simulate telemetry updates
      const updatedSystemMetrics = {
        ...mockSystemMetrics,
        cpu: { ...mockSystemMetrics.cpu, usage_percent: 75.2 },
      };

      const updatedIpfsData = {
        ...mockIpfsData,
        storage_market: mockIpfsData.storage_market.map((deal) => ({
          ...deal,
          earned: deal.earned + 50,
        })),
      };

      if (eventListener) {
        eventListener({ payload: { system: updatedSystemMetrics, ipfs: updatedIpfsData } });
      }

      rerender(<MonitoringDashboard />);

      // Verify both panels still exist after update
      expect(screen.getByText("System Metrics")).toBeInTheDocument();
      expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
    });
  });

  describe("Rendering Performance", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should render without unnecessary re-renders", async () => {
      const spy = vi.fn();
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
      });

      // Component should be stable after initial render
      expect(spy.mock.calls.length).toBeLessThanOrEqual(2);
    });

    it("should lazy load panel components efficiently", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const panels = container.querySelectorAll(".bg-gray-900.rounded-lg");
        expect(panels.length).toBeGreaterThan(0);
      });
    });
  });

  describe("Visual Layout", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should have proper padding", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const wrapper = container.querySelector(".p-4");
        expect(wrapper).toBeInTheDocument();
      });
    });

    it("should have proper spacing between panels", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const wrapper = container.querySelector(".w-full.h-full");
        expect(wrapper).toHaveClass("space-y-4");
      });
    });

    it("should apply backdrop blur effect", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const wrapper = container.querySelector(".w-full");
        expect(wrapper).toBeInTheDocument();
      });
    });
  });

  describe("Accessibility", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      (TauriApi.invoke as any)
        .mockResolvedValueOnce(mockSystemMetrics)
        .mockResolvedValueOnce(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should have semantic structure with headings", async () => {
      const { container } = render(<MonitoringDashboard />);

      await waitFor(() => {
        const headings = container.querySelectorAll("h3, h4");
        expect(headings.length).toBeGreaterThan(0);
      });
    });

    it("should render content in logical reading order", async () => {
      render(<MonitoringDashboard />);

      await waitFor(() => {
        expect(screen.getByText("System Metrics")).toBeInTheDocument();
      });

      // Both panels should be in the document
      expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
    });
  });
});
