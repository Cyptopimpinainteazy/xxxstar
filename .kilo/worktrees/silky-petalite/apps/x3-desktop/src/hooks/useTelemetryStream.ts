import { useEffect, useState } from "react";

// Lazy/guarded Tauri invoke to avoid browser crashes when Tauri is not available
async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(command, args);
}

import { ipcListen } from "@/services/ipcService";
import type {
  TelemetrySnapshot,
  SwarmHealthData,
  NetworkControlData,
  StorageMonitorData,
  IdeTelemetryData,
} from "@/types/panelTelemetry";
import { TELEMETRY_EVENT } from "@/types/panelTelemetry";

async function loadSnapshot(): Promise<TelemetrySnapshot> {
  const [swarm, network, storage, ide] = await Promise.all([
    tauriInvoke<SwarmHealthData>("launch_swarm_health"),
    tauriInvoke<NetworkControlData>("launch_network_control"),
    tauriInvoke<StorageMonitorData>("launch_storage_monitor"),
    tauriInvoke<IdeTelemetryData>("launch_ide_ipc"),
  ]);

  return {
    swarm,
    network,
    storage,
    ide,
    updatedAt: swarm.updatedAt,
  };
}

export type TelemetryStreamState = {
  data: TelemetrySnapshot | null;
  loading: boolean;
  error: string | null;
};

export function useTelemetryStream(): TelemetryStreamState {
  const [data, setData] = useState<TelemetrySnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;

    const init = async () => {
      try {
        const snapshot = await loadSnapshot();
        if (!mounted) return;
        setData(snapshot);
        setError(null);
      } catch (err) {
        if (!mounted) return;
        const message = typeof err === "string" ? err : (err as Record<string, string>)?.message ?? "Unknown error";
        setError(message);
      } finally {
        if (mounted) setLoading(false);
      }
    };

    const listen = async () => {
      unlisten = await ipcListen<TelemetrySnapshot>(TELEMETRY_EVENT, (payload) => {
        if (!mounted) return;
        setData(payload);
      });
    };

    init();
    listen();

    return () => {
      mounted = false;
      if (unlisten) unlisten();
    };
  }, []);

  return { data, loading, error };
}
