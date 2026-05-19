import { useTauriPolling } from "./useTauriPolling";
import type { SwarmHealthData } from "@/types/panelTelemetry";

export function useSwarmHealth() {
  return useTauriPolling<SwarmHealthData>("launch_swarm_health", 5000);
}
