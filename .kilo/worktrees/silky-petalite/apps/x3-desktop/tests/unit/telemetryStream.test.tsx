/**
 * Unit tests for telemetry stream hook.
 *
 * Invariants tested: FRONTEND-TELEMETRY-001 (telemetry stream updates hook state)
 */
import React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import { useTelemetryStream } from "../../src/hooks/useTelemetryStream";
import type { TelemetrySnapshot } from "../../src/types/panelTelemetry";
import {
  sampleSwarmHealthData,
  sampleNetworkControlData,
  sampleStorageMonitorData,
  sampleIdeTelemetryData,
} from "../../src/types/panelTelemetry";

let handler: ((payload: TelemetrySnapshot) => void) | null = null;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((command: string) => {
    switch (command) {
      case "launch_swarm_health":
        return Promise.resolve(sampleSwarmHealthData);
      case "launch_network_control":
        return Promise.resolve(sampleNetworkControlData);
      case "launch_storage_monitor":
        return Promise.resolve(sampleStorageMonitorData);
      case "launch_ide_ipc":
        return Promise.resolve(sampleIdeTelemetryData);
      default:
        return Promise.reject(new Error("Unknown command"));
    }
  }),
}));

vi.mock("../../src/services/ipcService", () => ({
  ipcListen: vi.fn(async (_event: string, next: (payload: TelemetrySnapshot) => void) => {
    handler = next;
    return () => {};
  }),
}));

const HookProbe: React.FC = () => {
  const { data, loading, error } = useTelemetryStream();
  return (
    <div>
      <div data-testid="loading">{loading ? "loading" : "ready"}</div>
      <div data-testid="error">{error ?? ""}</div>
      <div data-testid="avg">{data ? Math.round(data.swarm.summary.avgGpuUtil) : 0}</div>
    </div>
  );
};

beforeEach(() => {
  handler = null;
});

describe("useTelemetryStream", () => {
  it("loads initial snapshot and updates on telemetry events", async () => {
    render(<HookProbe />);

    await waitFor(() => {
      expect(screen.getByTestId("loading").textContent).toBe("ready");
    });

    await waitFor(() => {
      expect(screen.getByTestId("avg").textContent).toBe(
        String(Math.round(sampleSwarmHealthData.summary.avgGpuUtil)),
      );
    });

    const updated: TelemetrySnapshot = {
      swarm: {
        ...sampleSwarmHealthData,
        summary: { ...sampleSwarmHealthData.summary, avgGpuUtil: 91 },
        updatedAt: "2026-02-08T14:40:00Z",
      },
      network: sampleNetworkControlData,
      storage: sampleStorageMonitorData,
      ide: sampleIdeTelemetryData,
      updatedAt: "2026-02-08T14:40:00Z",
    };

    await act(async () => {
      handler?.(updated);
    });

    await waitFor(() => {
      expect(screen.getByTestId("avg").textContent).toBe("91");
    });
  });
});
