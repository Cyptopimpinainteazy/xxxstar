/**
 * Unit tests for LiveTelemetryPanel rendering states.
 *
 * Invariants tested: FRONTEND-TELEMETRY-002 (panel renders loading and error states)
 */
import React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import LiveTelemetryPanel from "../../src/components/panels/LiveTelemetryPanel";
import type { TelemetrySnapshot } from "../../src/types/panelTelemetry";
import {
  sampleSwarmHealthData,
  sampleNetworkControlData,
  sampleStorageMonitorData,
  sampleIdeTelemetryData,
} from "../../src/types/panelTelemetry";

const mockSnapshot: TelemetrySnapshot = {
  swarm: sampleSwarmHealthData,
  network: sampleNetworkControlData,
  storage: sampleStorageMonitorData,
  ide: sampleIdeTelemetryData,
  updatedAt: "2026-02-08T14:38:31Z",
};
let mockState = {
  data: mockSnapshot,
  loading: false,
  error: null,
};

vi.mock("../../src/hooks/useTelemetryStream", () => ({
  useTelemetryStream: () => mockState,
}));

describe("LiveTelemetryPanel", () => {
  it("renders heatmap and storage graph when data is available", () => {
    mockState = { data: mockSnapshot, loading: false, error: null };
    render(<LiveTelemetryPanel />);

    expect(screen.getByText("Utilization Heatmap")).toBeInTheDocument();
    expect(screen.getByText("Utilization Graph")).toBeInTheDocument();
  });

  it("renders loading state", async () => {
    mockState = { data: null, loading: true, error: null };
    const { unmount } = render(<LiveTelemetryPanel />);
    expect(screen.getByText("Streaming telemetry...")).toBeInTheDocument();
    unmount();
  });

  it("renders error state", async () => {
    mockState = { data: null, loading: false, error: "boom" };
    const { unmount } = render(<LiveTelemetryPanel />);
    expect(screen.getByText(/Could not load panel data/i)).toBeInTheDocument();
    unmount();
  });
});
