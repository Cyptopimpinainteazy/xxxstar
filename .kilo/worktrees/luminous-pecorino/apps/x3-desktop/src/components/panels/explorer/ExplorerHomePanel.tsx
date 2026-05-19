import React, { useState } from 'react';
import { Search, Box, ArrowRightLeft, BarChart3, Shield, Wallet, Coins, Vote, Clock, CheckCircle } from 'lucide-react';

import { useEffect } from 'react';
import { useRecentBlocks, useNewHeads } from '@/hooks/useSubstrate';

const latestBlocksFallback = [
  { number: 1284520, hash: '0xa4f8e2d9...3456', extrinsics: 24, time: '6s ago' },
  { number: 1284519, hash: '0x3b5c91e7...7890', extrinsics: 18, time: '12s ago' },
  { number: 1284518, hash: '0xd7e2f8a4...2345', extrinsics: 31, time: '18s ago' },
  { number: 1284517, hash: '0x8c1a3f5e...ef01', extrinsics: 12, time: '24s ago' },
  { number: 1284516, hash: '0xf2b8e4c7...2345', extrinsics: 27, time: '30s ago' },
  { number: 1284515, hash: '0x5e9a1c3f...2345', extrinsics: 20, time: '36s ago' },
];

const latestExtrinsics = [
  { hash: '0xe8f2a1...c4d5', block: 1284520, call: 'balances.transfer', status: 'success' },
  { hash: '0x7b3c9d...e1f2', block: 1284520, call: 'evm.call', status: 'success' },
  { hash: '0x1a5f8e...6789', block: 1284519, call: 'svm.invoke', status: 'success' },
  { hash: '0xd4c2b7...ab01', block: 1284519, call: 'comit.execute', status: 'success' },
  { hash: '0x9e6f3a...2345', block: 1284518, call: 'staking.bond', status: 'success' },
  { hash: '0x2c8d5f...6789', block: 1284518, call: 'balances.transfer', status: 'failed' },
];

const activityData = [
  { label: '12h', txns: 1200 }, { label: '11h', txns: 1450 }, { label: '10h', txns: 1100 },
  { label: '9h', txns: 1680 }, { label: '8h', txns: 2100 }, { label: '7h', txns: 1950 },
  { label: '6h', txns: 2400 }, { label: '5h', txns: 2200 }, { label: '4h', txns: 1800 },
  { label: '3h', txns: 2600 }, { label: '2h', txns: 2350 }, { label: '1h', txns: 2800 },
];
const maxTxns = Math.max(...activityData.map(d => d.txns));

const quickLinks = [
  { label: 'Validators', icon: <Shield size={16} />, desc: '12 active' },
  { label: 'Accounts', icon: <Wallet size={16} />, desc: '148,234' },
  { label: 'Tokens', icon: <Coins size={16} />, desc: '86 tokens' },
  { label: 'Governance', icon: <Vote size={16} />, desc: '5 proposals' },
];

const ExplorerHomePanel: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');

  // Live blocks (updates via SWR hook)
  const { data: liveBlocks, mutate: mutateBlocks } = useRecentBlocks(6);
  const { data: latestHead } = useNewHeads();

  // Revalidate recent blocks on new head
  useEffect(() => {
    if (latestHead) mutateBlocks?.();
  }, [latestHead, mutateBlocks]);

  // helpers
  const shortHash = (h?: string) => (h ? (h.length > 14 ? `${h.slice(0, 10)}…${h.slice(-6)}` : h) : '—');
  const timeAgo = (ts?: number) => {
    if (!ts) return '—';
    const diff = Math.max(0, Math.floor((Date.now() - Number(ts)) / 1000));
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    return `${Math.floor(diff / 3600)}h ago`;
  };

  const blocksToShow = (liveBlocks && liveBlocks.length > 0)
    ? liveBlocks.map((b) => ({ number: b.number, hash: shortHash(b.hash), extrinsics: b.extrinsicsCount ?? 0, time: timeAgo(b.timestamp) }))
    : latestBlocksFallback;

  return (
    <div className="flex flex-col h-full bg-[#0a0a0f] text-gray-300">
      {/* Hero */}
      <div className="px-6 pt-6 pb-4 border-b border-[#1a1a1a]">
        <h1 className="text-2xl font-bold text-white mb-1">X3 Block Explorer</h1>
        <p className="text-sm text-gray-500 mb-4">Search blocks, transactions, accounts, and more across the X3 Chain network.</p>
        <div className="relative max-w-2xl">
          <Search size={16} className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-500" />
          <input
            type="text"
            placeholder="Search by block number, transaction hash, or account address..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            className="w-full pl-11 pr-4 py-2.5 bg-[#111118] border border-[#1a1a1a] rounded-lg text-sm text-gray-300 focus:outline-none focus:border-[#ff6b35]/50"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-5 space-y-5">
        {/* Network Stats */}
        <div className="grid grid-cols-4 gap-3">
          {[
            { label: 'Latest Block', value: '#1,284,520', icon: <Box size={14} /> },
            { label: 'Finalized Block', value: '#1,284,516', icon: <CheckCircle size={14} /> },
            { label: 'Total Extrinsics', value: '24,891,402', icon: <ArrowRightLeft size={14} /> },
            { label: 'Total Accounts', value: '148,234', icon: <Wallet size={14} /> },
          ].map((s, i) => (
            <div key={i} className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
              <div className="flex items-center gap-1.5 mb-1">
                <span className="text-[#ff6b35]">{s.icon}</span>
                <span className="text-xs text-gray-500">{s.label}</span>
              </div>
              <p className="text-lg font-bold text-white">{s.value}</p>
            </div>
          ))}
        </div>

        {/* 3-Column Layout */}
        <div className="grid grid-cols-3 gap-4">
          {/* Latest Blocks */}
          <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
            <div className="flex items-center gap-2 px-4 py-2.5 border-b border-[#1a1a1a]">
              <Box size={14} className="text-[#ff6b35]" />
              <h3 className="text-sm font-semibold text-white">Latest Blocks</h3>
            </div>
            <div className="divide-y divide-[#1a1a1a]/50">
              {blocksToShow.map((b, i) => (
                <div key={i} className="px-4 py-2.5 hover:bg-white/[0.02] transition-colors">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="text-sm font-mono text-[#ff6b35]">#{Number(b.number).toLocaleString()}</span>
                      <p className="text-xs font-mono text-gray-500 mt-0.5">{b.hash}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-xs text-gray-300">{b.extrinsics} extrinsics</p>
                      <p className="text-xs text-gray-500 flex items-center gap-1 justify-end"><Clock size={9} /> {b.time}</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Latest Extrinsics */}
          <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
            <div className="flex items-center gap-2 px-4 py-2.5 border-b border-[#1a1a1a]">
              <ArrowRightLeft size={14} className="text-[#ff6b35]" />
              <h3 className="text-sm font-semibold text-white">Latest Extrinsics</h3>
            </div>
            <div className="divide-y divide-[#1a1a1a]/50">
              {latestExtrinsics.map((ex, i) => (
                <div key={i} className="px-4 py-2.5 hover:bg-white/[0.02] transition-colors">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="text-xs font-mono text-[#ff6b35]">{ex.hash}</span>
                      <p className="text-xs text-gray-400 mt-0.5">{ex.call}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-xs font-mono text-gray-500">#{ex.block.toLocaleString()}</p>
                      <span className={`text-xs ${ex.status === 'success' ? 'text-green-400' : 'text-red-400'}`}>{ex.status}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Network Activity Chart */}
          <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
            <div className="flex items-center gap-2 px-4 py-2.5 border-b border-[#1a1a1a]">
              <BarChart3 size={14} className="text-[#ff6b35]" />
              <h3 className="text-sm font-semibold text-white">Network Activity</h3>
            </div>
            <div className="p-4">
              <div className="flex items-end gap-1.5 h-40">
                {activityData.map((d, i) => (
                  <div key={i} className="flex-1 flex flex-col items-center justify-end h-full">
                    <div
                      className="w-full rounded-t transition-all duration-300 hover:opacity-80"
                      style={{
                        height: `${(d.txns / maxTxns) * 100}%`,
                        background: `linear-gradient(to top, #ff6b35, rgba(255,107,53,0.3))`,
                      }}
                    />
                    <span className="text-[9px] text-gray-600 mt-1">{d.label}</span>
                  </div>
                ))}
              </div>
              <div className="flex items-center justify-between mt-3 text-xs text-gray-500">
                <span>Txns/hour (last 12h)</span>
                <span>Peak: {maxTxns.toLocaleString()}</span>
              </div>
            </div>
          </div>
        </div>

        {/* Quick Links */}
        <div className="grid grid-cols-4 gap-3">
          {quickLinks.map((ql, i) => (
            <button key={i} className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg hover:border-[#ff6b35]/30 transition-colors text-left group">
              <div className="flex items-center gap-2 mb-1">
                <span className="text-[#ff6b35] group-hover:text-[#ff6b35]">{ql.icon}</span>
                <span className="text-sm font-semibold text-white">{ql.label}</span>
              </div>
              <p className="text-xs text-gray-500">{ql.desc}</p>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};

export default ExplorerHomePanel;
