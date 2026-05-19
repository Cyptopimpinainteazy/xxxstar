/**
 * Application store — registry of available applications and running processes.
 *
 * Provides actions to register apps, launch/stop processes, and query status.
 */
import { create } from "zustand";
import type {
  Application,
  ProcessInfo,
  ProcessStatus,
} from "@/types/application";

export interface ApplicationState {
  /** All registered applications */
  applications: Application[];
  /** Running process info keyed by app ID */
  runningProcesses: Map<string, ProcessInfo>;

  /** Replace entire registry (called after backend fetch) */
  setApplications: (apps: Application[]) => void;
  /** Get application by ID */
  getApp: (id: string) => Application | undefined;

  /** Mark a process as started */
  startProcess: (appId: string, pid?: number) => void;
  /** Update process status */
  updateProcessStatus: (appId: string, status: ProcessStatus) => void;
  /** Record a heartbeat */
  heartbeat: (appId: string) => void;
  /** Remove process entry (stopped/crashed) */
  removeProcess: (appId: string) => void;

  /** Check if an application is currently running */
  isRunning: (appId: string) => boolean;
}

export const useApplicationStore = create<ApplicationState>()((set, get) => ({
  applications: [],
  runningProcesses: new Map(),

  setApplications: (apps) => set({ applications: apps }),

  getApp: (id) => get().applications.find((a) => a.id === id),

  startProcess: (appId, pid) =>
    set((s) => {
      const next = new Map(s.runningProcesses);
      next.set(appId, {
        appId,
        pid,
        status: "starting",
        startedAt: new Date().toISOString(),
      });
      return { runningProcesses: next };
    }),

  updateProcessStatus: (appId, status) =>
    set((s) => {
      const next = new Map(s.runningProcesses);
      const existing = next.get(appId);
      if (existing) {
        next.set(appId, { ...existing, status });
      }
      return { runningProcesses: next };
    }),

  heartbeat: (appId) =>
    set((s) => {
      const next = new Map(s.runningProcesses);
      const existing = next.get(appId);
      if (existing) {
        next.set(appId, {
          ...existing,
          status: "running",
          lastHeartbeat: new Date().toISOString(),
        });
      }
      return { runningProcesses: next };
    }),

  removeProcess: (appId) =>
    set((s) => {
      const next = new Map(s.runningProcesses);
      next.delete(appId);
      return { runningProcesses: next };
    }),

  isRunning: (appId) => {
    const proc = get().runningProcesses.get(appId);
    return proc != null && (proc.status === "running" || proc.status === "starting");
  },
}));
