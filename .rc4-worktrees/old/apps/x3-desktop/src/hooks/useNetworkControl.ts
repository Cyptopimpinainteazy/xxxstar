import { useTauriPolling } from "./useTauriPolling";
import type { NetworkControlData } from "@/types/panelTelemetry";

export function useNetworkControl() {
  return useTauriPolling<NetworkControlData>("launch_network_control", 4000);
}
