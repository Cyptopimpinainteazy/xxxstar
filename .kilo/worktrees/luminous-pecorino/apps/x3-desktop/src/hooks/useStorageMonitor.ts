import { useTauriPolling } from "./useTauriPolling";
import type { StorageMonitorData } from "@/types/panelTelemetry";

export function useStorageMonitor() {
  return useTauriPolling<StorageMonitorData>("launch_storage_monitor", 6000);
}
