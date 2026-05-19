/**
 * Unit tests for the application registry and store.
 *
 * Tests application registration, process lifecycle, and running state queries.
 *
 * Invariants tested: INV-APP-001 (application store maintains valid process state)
 */
import { describe, it, expect, beforeEach } from "vitest";
import { useApplicationStore } from "../../src/stores/applicationStore";
import type { Application } from "../../src/types/application";
import { DEFAULT_APPLICATIONS } from "../../src/services/applicationService";

const mockApp: Application = {
  id: "test-app",
  name: "Test App",
  category: "utility",
  icon: { type: "placeholder", category: "utility", color: "#66bb6a" },
  launchCommand: { type: "tauri", target: "test_command" },
};

beforeEach(() => {
  useApplicationStore.setState({
    applications: [],
    runningProcesses: new Map(),
  });
});

describe("application registry", () => {
  it("sets applications list", () => {
    useApplicationStore.getState().setApplications([mockApp]);
    expect(useApplicationStore.getState().applications).toHaveLength(1);
    expect(useApplicationStore.getState().applications[0].id).toBe("test-app");
  });

  it("getApp returns correct application", () => {
    useApplicationStore.getState().setApplications([mockApp]);
    const app = useApplicationStore.getState().getApp("test-app");
    expect(app).toBeDefined();
    expect(app!.name).toBe("Test App");
  });

  it("getApp returns undefined for unknown ID", () => {
    const app = useApplicationStore.getState().getApp("nonexistent");
    expect(app).toBeUndefined();
  });

  it("DEFAULT_APPLICATIONS contains expected entries", () => {
    expect(DEFAULT_APPLICATIONS.length).toBeGreaterThan(10);
    const explorer = DEFAULT_APPLICATIONS.find((a) => a.id === "block-explorer");
    expect(explorer).toBeDefined();
    expect(explorer!.category).toBe("blockchain");
  });
});

describe("process lifecycle", () => {
  it("startProcess creates a process entry with 'starting' status", () => {
    useApplicationStore.getState().startProcess("test-app", 1234);
    const proc = useApplicationStore.getState().runningProcesses.get("test-app");
    expect(proc).toBeDefined();
    expect(proc!.status).toBe("starting");
    expect(proc!.pid).toBe(1234);
  });

  it("updateProcessStatus changes status", () => {
    useApplicationStore.getState().startProcess("test-app");
    useApplicationStore.getState().updateProcessStatus("test-app", "running");
    const proc = useApplicationStore.getState().runningProcesses.get("test-app");
    expect(proc!.status).toBe("running");
  });

  it("heartbeat updates lastHeartbeat and sets status to running", () => {
    useApplicationStore.getState().startProcess("test-app");
    useApplicationStore.getState().heartbeat("test-app");
    const proc = useApplicationStore.getState().runningProcesses.get("test-app");
    expect(proc!.status).toBe("running");
    expect(proc!.lastHeartbeat).toBeDefined();
  });

  it("removeProcess deletes the entry", () => {
    useApplicationStore.getState().startProcess("test-app");
    useApplicationStore.getState().removeProcess("test-app");
    expect(
      useApplicationStore.getState().runningProcesses.has("test-app"),
    ).toBe(false);
  });

  it("isRunning returns true for running/starting processes", () => {
    useApplicationStore.getState().startProcess("test-app");
    expect(useApplicationStore.getState().isRunning("test-app")).toBe(true);

    useApplicationStore.getState().updateProcessStatus("test-app", "running");
    expect(useApplicationStore.getState().isRunning("test-app")).toBe(true);
  });

  it("isRunning returns false for stopped/crashed/unknown processes", () => {
    expect(useApplicationStore.getState().isRunning("unknown")).toBe(false);

    useApplicationStore.getState().startProcess("test-app");
    useApplicationStore.getState().updateProcessStatus("test-app", "crashed");
    expect(useApplicationStore.getState().isRunning("test-app")).toBe(false);
  });
});
