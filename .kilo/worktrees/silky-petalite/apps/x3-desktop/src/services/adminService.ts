// Lazy guarded invoke to avoid accessing Tauri internals in browser dev
async function tauriInvoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
} 

export interface AllowedCommand {
  id: string;
  label: string;
}

export interface ServiceHealth {
  name: string;
  port: number;
  healthy: boolean;
  status_code: number;
  latency_ms: number;
}

export interface AdminSystemOverview {
  hostname: string;
  kernel: string;
  uptime: string;
  platform: string;
  arch: string;
}

/** Run an allowlisted system command by its ID */
export const runSystemCommand = async (cmd: string): Promise<string> => {
  return await tauriInvoke<string>("run_system_command", { cmd });
};

/** Get list of allowed command IDs */
export const listAdminCommands = async (): Promise<AllowedCommand[]> => {
  return await tauriInvoke<AllowedCommand[]>("admin_list_commands");
};

/** Get system overview (hostname, kernel, uptime, platform) */
export const getSystemOverview = async (): Promise<AdminSystemOverview> => {
  return await tauriInvoke<AdminSystemOverview>("admin_system_overview");
};

/** Check health of local infrastructure services */
export const checkServices = async (): Promise<ServiceHealth[]> => {
  return await tauriInvoke<ServiceHealth[]>("admin_check_services");
};
