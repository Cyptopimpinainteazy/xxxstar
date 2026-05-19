import { useTauriPolling } from "./useTauriPolling";
import type { IdeTelemetryData } from "@/types/panelTelemetry";

export function useIdeTelemetry() {
  return useTauriPolling<IdeTelemetryData>("launch_ide_ipc", 5000);
}
