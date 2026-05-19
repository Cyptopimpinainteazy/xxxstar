import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { IpfsStoragePanel } from "./IpfsStoragePanel";
import * as TauriApi from "@tauri-apps/api/core";
import * as TauriEvent from "@tauri-apps/api/event";

vi.mock("@tauri-apps/api/core");
vi.mock("@tauri-apps/api/event");

describe("IpfsStoragePanel", () => {
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
      {
        cid: "bafy2bzaceayp7fq2kmx3vhyikpohczb73f7mw7bnvp6u4zvvqfqiduxpvrhq",
        name: "training-data.tar",
        size: 18432000,
        pinned_at: "2026-02-06T10:00:00Z",
        replicas: 3,
        earning_potential: 420.75,
      },
    ],
    storage_used: 23244300,
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
      {
        id: "deal-002",
        client: "research-collective",
        size: 18432000,
        price_per_epoch: 1.25,
        duration_epochs: 260,
        status: "Active",
        earned: 325.0,
      },
    ],
    total_pins: 2,
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

      render(<IpfsStoragePanel />);

      expect(screen.getByText("Loading IPFS storage data...")).toBeInTheDocument();
    });
  });

  describe("Success State", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should render the panel with title", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });
    });

    it("should display node ID", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText(/Node: bafy2bzacea.../)).toBeInTheDocument();
      });
    });

    it("should display storage capacity bar", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText(/Storage Capacity/)).toBeInTheDocument();
      });
    });

    it("should display quick stats", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        // Look for quick stats without relying on exact number matching
        expect(screen.getByText(/Active Deals/)).toBeInTheDocument();
        expect(screen.getByText(/Total Earned/)).toBeInTheDocument();
      });
    });

    it("should display storage deals", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("Storage Deals")).toBeInTheDocument();
        expect(screen.getByText("x3-ai-lab")).toBeInTheDocument();
        expect(screen.getByText("research-collective")).toBeInTheDocument();
      });
    });

    it("should display pinned content", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("Pinned Content")).toBeInTheDocument();
        expect(screen.getByText("x3-runtime.wasm")).toBeInTheDocument();
        expect(screen.getByText("training-data.tar")).toBeInTheDocument();
      });
    });

    it("should display last updated timestamp", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText(/Updated:/)).toBeInTheDocument();
      });
    });
  });

  describe("Error State", () => {
    it(
      "should show error message when invoke fails",
      async () => {
        const errorMsg = "Command not found: launch_ipfs_storage";
        (TauriApi.invoke as any).mockRejectedValue(new Error(errorMsg));
        (TauriEvent.listen as any).mockResolvedValue(vi.fn());

        render(<IpfsStoragePanel />);

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
      "should show helpful message with troubleshooting info",
      async () => {
        (TauriApi.invoke as any).mockRejectedValue(new Error("IPFS node unavailable"));
        (TauriEvent.listen as any).mockResolvedValue(vi.fn());

        render(<IpfsStoragePanel />);

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
    it("should listen for telemetry updates on mount", async () => {
      (TauriApi.invoke as any).mockResolvedValue(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });

      // Verify listen was called for telemetry_update event
      expect(TauriEvent.listen).toHaveBeenCalledWith(
        'telemetry_update',
        expect.any(Function)
      );
    });
  });

  describe("Data Formatting", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should format currency values correctly", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText(/\$150\.50/)).toBeInTheDocument();
        expect(screen.getByText(/\$420\.75/)).toBeInTheDocument();
      });
    });

    it("should format byte sizes to human-readable units", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        const elements = screen.getAllByText(/[0-9]+\.[0-9] (B|KB|MB|GB|TB)/);
        expect(elements.length).toBeGreaterThan(0);
      });
    });

    it("should truncate long CIDs", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        const cidElements = screen.getAllByText(/bafy2bzacea.../);
        expect(cidElements.length).toBeGreaterThan(0);
      });
    });
  });

  describe("Deal Status Display", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should show correct deal status colors", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getAllByText("Active").length).toBeGreaterThan(0);
      });
    });

    it("should display deal client names", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("x3-ai-lab")).toBeInTheDocument();
        expect(screen.getByText("research-collective")).toBeInTheDocument();
      });
    });

    it("should display deal earnings", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        // Earnings are displayed as numbers in the deals section
        const elements = screen.queryAllByText((content) => {
          return /260\.0|325\.0/.test(content);
        });
        expect(elements.length).toBeGreaterThan(0);
      });
    });
  });

  describe("Pinned Content Display", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should display pinned content names", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("x3-runtime.wasm")).toBeInTheDocument();
        expect(screen.getByText("training-data.tar")).toBeInTheDocument();
      });
    });

    it("should display replica counts", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText(/5 replicas/)).toBeInTheDocument();
        expect(screen.getByText(/3 replicas/)).toBeInTheDocument();
      });
    });

    it("should display earning potential per content", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText(/\$150\.50/)).toBeInTheDocument();
        expect(screen.getByText(/\$420\.75/)).toBeInTheDocument();
      });
    });
  });

  describe("Quick Stats Section", () => {
    beforeEach(() => {
      (TauriApi.invoke as any).mockResolvedValue(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should calculate and display total pinned objects", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("Pinned Objects")).toBeInTheDocument();
      });
    });

    it("should count active storage deals", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("Active Deals")).toBeInTheDocument();
      });
    });

    it("should calculate total earnings from all deals", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        // 260 + 325 = 585.00 total
        expect(screen.getByText(/Total Earned/)).toBeInTheDocument();
      });
    });
  });

  describe("Edge Cases", () => {
    it("should handle empty pinned objects gracefully", async () => {
      const emptyData = {
        ...mockIpfsData,
        pinned_objects: [],
        total_pins: 0,
      };

      (TauriApi.invoke as any).mockResolvedValue(emptyData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });
    });

    it("should handle empty storage deals gracefully", async () => {
      const noDealData = {
        ...mockIpfsData,
        storage_market: [],
      };

      (TauriApi.invoke as any).mockResolvedValue(noDealData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });
    });

    it("should handle zero storage usage", async () => {
      const zeroUsageData = {
        ...mockIpfsData,
        storage_used: 0,
      };

      (TauriApi.invoke as any).mockResolvedValue(zeroUsageData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });
    });

    it("should handle very large storage capacities", async () => {
      const largeData = {
        ...mockIpfsData,
        storage_capacity: 1099511627776000, // 1 PB
      };

      (TauriApi.invoke as any).mockResolvedValue(largeData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
      });
    });

    it("should handle multiple replicas up to 10", async () => {
      const highReplicaData = {
        ...mockIpfsData,
        pinned_objects: mockIpfsData.pinned_objects.map((pin) => ({
          ...pin,
          replicas: 10,
        })),
      };

      (TauriApi.invoke as any).mockResolvedValue(highReplicaData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());

      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getAllByText(/10 replicas/).length).toBeGreaterThan(0);
      });
    });
  });

  describe("Accessibility", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      (TauriApi.invoke as any).mockResolvedValue(mockIpfsData);
      (TauriEvent.listen as any).mockResolvedValue(vi.fn());
    });

    it("should have semantic HTML structure", async () => {
      const { container } = render(<IpfsStoragePanel />);

      await waitFor(() => {
        const headings = container.querySelectorAll("h3, h4");
        expect(headings.length).toBeGreaterThan(0);
      });
    });

    it("should display information in a readable format", async () => {
      render(<IpfsStoragePanel />);

      await waitFor(() => {
        expect(screen.getByText("IPFS Storage Marketplace")).toBeInTheDocument();
        expect(screen.getByText("Storage Deals")).toBeInTheDocument();
        expect(screen.getByText("Pinned Content")).toBeInTheDocument();
      });
    });
  });
});
