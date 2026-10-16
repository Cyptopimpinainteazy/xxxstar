/**
 * @module ipc/tauri
 * Typed Tauri IPC invoke wrappers for all backend commands.
 *
 * Every function maps 1:1 to a #[tauri::command] in src-tauri/src/commands.rs.
 * Falls back to no-op stubs when running outside Tauri (plain browser dev).
 */

import type { ChainStatus, Block } from "../types/chain";
import type { VoyageState } from "../types/game";
import type { SceneExport } from "../types/scene";

// -------------------------------------------------------------------------
// Dynamic import of @tauri-apps/api (only available inside Tauri runtime)
// -------------------------------------------------------------------------

interface TauriInvoke {
  invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown>;
}

let tauriCore: TauriInvoke | null = null;

async function getTauriCore(): Promise<TauriInvoke> {
  if (tauriCore) return tauriCore;
  try {
    const mod = await import("@tauri-apps/api/core");
    tauriCore = mod;
    return tauriCore;
  } catch {
    // Running in plain browser — return stub
    console.warn("[IPC] @tauri-apps/api not available — using stubs");
    tauriCore = {
      invoke: async (cmd: string, _args?: Record<string, unknown>) => {
        console.info(`[IPC stub] ${cmd}`);
        return null;
      },
    };
    return tauriCore;
  }
}

// -------------------------------------------------------------------------
// Chain commands
// -------------------------------------------------------------------------

export async function connectChain(chainId: string, rpcUrl: string): Promise<void> {
  const t = await getTauriCore();
  await t.invoke("connect_chain", { chainId, rpcUrl });
}

export async function disconnectChain(chainId: string): Promise<void> {
  const t = await getTauriCore();
  await t.invoke("disconnect_chain", { chainId });
}

export async function fetchChainStatus(chainId: string): Promise<ChainStatus> {
  const t = await getTauriCore();
  const result = await t.invoke("fetch_chain_status", { chainId });
  return result as ChainStatus;
}

export async function fetchBlocks(chainId: string, count: number): Promise<Block[]> {
  const t = await getTauriCore();
  const result = await t.invoke("fetch_blocks", { chainId, count });
  return result as Block[];
}

// -------------------------------------------------------------------------
// Signing / accounts
// -------------------------------------------------------------------------

export async function signAndSendTx(
  chainId: string,
  from: string,
  to: string,
  method: string,
  args: Record<string, unknown>,
  password: string,
): Promise<string> {
  const t = await getTauriCore();
  const result = await t.invoke("sign_and_send_tx", { chainId, from, to, method, args, password });
  return result as string;
}

export async function createAccount(label: string, password: string): Promise<string> {
  const t = await getTauriCore();
  const result = await t.invoke("create_account", { label, password });
  return result as string;
}

export async function listAccounts(): Promise<string[]> {
  const t = await getTauriCore();
  const result = await t.invoke("list_accounts");
  return result as string[];
}

// -------------------------------------------------------------------------
// Scene
// -------------------------------------------------------------------------

export async function exportScene(entities: SceneExport): Promise<void> {
  const t = await getTauriCore();
  await t.invoke("export_scene", { entities });
}

// -------------------------------------------------------------------------
// Voyage persistence
// -------------------------------------------------------------------------

export async function saveVoyage(state: VoyageState): Promise<void> {
  const t = await getTauriCore();
  await t.invoke("save_voyage", { state });
}

export async function loadVoyage(): Promise<VoyageState | null> {
  const t = await getTauriCore();
  const result = await t.invoke("load_voyage");
  return result as VoyageState | null;
}
