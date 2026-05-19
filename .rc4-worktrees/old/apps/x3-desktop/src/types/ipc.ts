/**
 * IPC message and response types for communication with the Tauri Rust backend.
 */

/** Standard IPC response envelope */
export interface IpcResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: IpcError;
  timestamp: string;
}

/** Structured IPC error */
export interface IpcError {
  code: string;
  message: string;
  details?: string;
}

/** Application launch result from the backend */
export interface LaunchResult {
  pid: number;
  status: "ok" | "error";
  message?: string;
}

/** Backend service status */
export interface ServiceStatus {
  id: string;
  name: string;
  running: boolean;
  uptime?: number;
  endpoint?: string;
}

/** Blockchain state summary */
export interface BlockchainState {
  chainId: string;
  blockHeight: number;
  peerCount: number;
  syncStatus: "syncing" | "synced" | "offline";
  lastBlockTime: string;
}

/** RPC endpoint configuration */
export interface RpcEndpoint {
  url: string;
  protocol: "ws" | "http";
  chainId: string;
  label: string;
}

/** Known IPC command names */
export type IpcCommand =
  | "get_services"
  | "launch_app"
  | "stop_app"
  | "get_blockchain_state"
  | "get_rpc_endpoints"
  | "get_app_registry"
  | "health_check"
  | "get_system_info"
  | "launch_swarm_health"
  | "launch_network_control"
  | "launch_storage_monitor"
  | "launch_ide_ipc";

export {}; // ensure module
