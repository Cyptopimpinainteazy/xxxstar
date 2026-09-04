import type { ChainState } from '../types';
import { useChainStore, useSettingsStore } from '../store';

let pollInterval: ReturnType<typeof setInterval> | null = null;

export async function pollChainStatus(): Promise<ChainState | null> {
  const rpcUrl = useSettingsStore.getState().chainRpcUrl;
  if (!rpcUrl) return null;

  const start = Date.now();
  try {
    const [blockResult, chainIdResult] = await Promise.all([
      window.x3studio.chain.rpcCall(rpcUrl, 'eth_blockNumber', []),
      window.x3studio.chain.rpcCall(rpcUrl, 'eth_chainId', []),
    ]);
    const latency = Date.now() - start;

    const chain: ChainState = {
      connected: !blockResult.error,
      chainId: chainIdResult.result ? String(parseInt(chainIdResult.result, 16)) : 'unknown',
      blockNumber: blockResult.result ? parseInt(blockResult.result, 16) : 0,
      latency,
      lastChecked: new Date().toISOString(),
      rpcUrl,
    };
    useChainStore.getState().setChain(chain);
    return chain;
  } catch {
    const chain: ChainState = {
      connected: false,
      chainId: 'unknown',
      blockNumber: 0,
      latency: Date.now() - start,
      lastChecked: new Date().toISOString(),
      rpcUrl,
    };
    useChainStore.getState().setChain(chain);
    return chain;
  }
}

export function startChainPolling(intervalMs: number = 5000) {
  stopChainPolling();
  pollChainStatus();
  pollInterval = setInterval(pollChainStatus, intervalMs);
}

export function stopChainPolling() {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
}
