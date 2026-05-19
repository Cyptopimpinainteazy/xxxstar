import { useEffect, useState } from 'react';
import ValidatorGlobe from '@/components/ValidatorGlobe';

type ChainStatus = {
  block: number | null;
  finalized: string | null;
  rpc: string;
  online: boolean;
  error: string | null;
};

const App = () => {
  const rpc = (import.meta.env.VITE_X3_RPC_HTTP as string) || 'https://rpc.x3star.net/rpc';
  const [chainStatus, setChainStatus] = useState<ChainStatus>({
    block: null,
    finalized: null,
    rpc,
    online: false,
    error: null,
  });

  useEffect(() => {
    let active = true;

    const pollChain = async () => {
      try {
        const [headerRes, finalizedRes] = await Promise.all([
          fetch(rpc, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: 1, jsonrpc: '2.0', method: 'chain_getHeader', params: [] }),
          }),
          fetch(rpc, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: 2, jsonrpc: '2.0', method: 'chain_getFinalizedHead', params: [] }),
          }),
        ]);

        const header = await headerRes.json();
        const finalized = await finalizedRes.json();
        const numberHex = header?.result?.number;
        const blockNumber = typeof numberHex === 'string' ? parseInt(numberHex, 16) : null;

        if (!active) return;
        setChainStatus({
          block: Number.isFinite(blockNumber as number) ? blockNumber : null,
          finalized: finalized?.result ?? null,
          rpc,
          online: true,
          error: null,
        });
      } catch (error) {
        if (!active) return;
        setChainStatus((prev) => ({
          ...prev,
          online: false,
          error: error instanceof Error ? error.message : 'Unknown RPC error',
        }));
      }
    };

    pollChain();
    const interval = window.setInterval(pollChain, 10000);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [rpc]);

  return (
    <div className="w-full h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-black">
      <div className="absolute top-4 left-4 z-20 rounded-lg border border-slate-700 bg-slate-950/80 px-4 py-2 text-xs text-slate-200 backdrop-blur">
        <div className="font-semibold">Main Chain Integration</div>
        <div className="mt-1">RPC: {chainStatus.rpc}</div>
        <div>Status: {chainStatus.online ? 'Online' : 'Offline'}</div>
        <div>Best Block: {chainStatus.block ?? 'N/A'}</div>
        <div>Finalized Head: {chainStatus.finalized ?? 'N/A'}</div>
        {chainStatus.error ? <div className="text-red-400">Error: {chainStatus.error}</div> : null}
      </div>
      <ValidatorGlobe />
    </div>
  );
};

export default App;
