'use client';

import React, { useState, useMemo } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import {
  Search,
  Blocks,
  ArrowRightLeft,
  Users,
  Activity,
  RefreshCw,
} from 'lucide-react';
import { HexagonCluster } from '../../components/frontend/ui/Logo';
import {
  useNetworkStats,
  useRecentBlocks,
  useRecentExtrinsics,
  useNewHeads,
  useAuthorities,
} from '@/hooks/useSubstrate';
import type { BlockInfo, ExtrinsicInfo } from '@/lib/substrate';

// VM type detection helper
const getVmType = (section: string): 'Comit' | 'EVM' | 'SVM' | 'Native' => {
  if (section === 'atlasKernel') return 'Comit';
  if (section === 'evm' || section === 'ethereum') return 'EVM';
  if (section === 'svm' || section === 'solana') return 'SVM';
  return 'Native';
};

const getVmStyle = (type: string) => ({
  bg: type === 'Comit' ? 'bg-orange-500/20' :
      type === 'EVM' ? 'bg-blue-500/20' :
      type === 'SVM' ? 'bg-emerald-500/20' : 'bg-gray-500/20',
  text: type === 'Comit' ? 'text-orange-400' :
        type === 'EVM' ? 'text-blue-400' :
        type === 'SVM' ? 'text-emerald-400' : 'text-gray-400',
  badge: type === 'Comit' ? 'badge-fire' :
         type === 'EVM' ? 'badge-info' :
         type === 'SVM' ? 'badge-success' : 'bg-gray-500/20 text-gray-400',
});

export default function ExplorerPage() {
  const router = useRouter();
  const [activeTab, setActiveTab] = useState<'overview' | 'blocks' | 'transactions' | 'accounts'>('overview');
  const [searchQuery, setSearchQuery] = useState('');
  const [isLive, setIsLive] = useState(true);

  // Real blockchain data hooks
  const { data: stats, error: statsError, isLoading: statsLoading } = useNetworkStats();
  const { data: blocks, error: blocksError, mutate: refreshBlocks } = useRecentBlocks(10);
  const { data: extrinsics, mutate: refreshExtrinsics } = useRecentExtrinsics(10);
  const { data: newHead } = useNewHeads();
  const { data: authorities } = useAuthorities();

  // Auto-refresh on new blocks if live mode enabled
  React.useEffect(() => {
    if (newHead && isLive) {
      refreshBlocks();
      refreshExtrinsics();
    }
  }, [newHead, isLive, refreshBlocks, refreshExtrinsics]);

  // Calculate VM statistics from real data
  const vmStats = useMemo(() => {
    if (!extrinsics) return { evm: 0, svm: 0, comit: 0, native: 0 };
    
    return extrinsics.reduce((acc, ext) => {
      const type = getVmType(ext.section);
      if (type === 'EVM') acc.evm++;
      else if (type === 'SVM') acc.svm++;
      else if (type === 'Comit') acc.comit++;
      else acc.native++;
      return acc;
    }, { evm: 0, svm: 0, comit: 0, native: 0 });
  }, [extrinsics]);

  // Network stats with real data
  const networkStatsData = [
    { 
      label: 'Block Height', 
      value: stats?.blockNumber?.toLocaleString() || '...', 
      icon: <Blocks className="w-5 h-5" />,
      change: isLive && newHead ? '+1' : ''
    },
    { 
      label: 'Validators', 
      value: authorities?.length?.toString() || '...', 
      icon: <Users className="w-5 h-5" />,
      change: ''
    },
    { 
      label: 'Block Time', 
      value: '6s', 
      icon: <Activity className="w-5 h-5" />,
      change: 'Aura'
    },
    { 
      label: 'Recent Txns', 
      value: extrinsics?.length?.toString() || '...', 
      icon: <ArrowRightLeft className="w-5 h-5" />,
      change: ''
    },
  ];

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchQuery.trim()) return;
    
    const query = searchQuery.trim();
    
    // Detect search type
    if (query.startsWith('0x') && query.length === 66) {
      // Block or transaction hash
      router.push(`/explorer/tx/${query}`);
    } else if (query.startsWith('5') && query.length >= 47) {
      // Substrate SS58 address
      router.push(`/explorer/account/${query}`);
    } else if (query.startsWith('0x') && query.length === 42) {
      // EVM address
      router.push(`/explorer/account/${query}`);
    } else if (/^\d+$/.test(query)) {
      // Block number
      router.push(`/explorer/block/${query}`);
    } else {
      // Try as block number or hash
      router.push(`/explorer/search?q=${encodeURIComponent(query)}`);
    }
  };

  const formatTime = (timestamp: number | undefined) => {
    if (!timestamp) return 'Just now';
    const diff = Date.now() - timestamp;
    if (diff < 1000) return 'Just now';
    if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return `${Math.floor(diff / 3600000)}h ago`;
  };

  const formatHash = (hash: string) => {
    if (!hash) return '';
    return `${hash.slice(0, 8)}...${hash.slice(-6)}`;
  };

  // Show connection error
  if (statsError || blocksError) {
    return (
      <div className="min-h-screen pt-20 flex items-center justify-center">
        <div className="text-center glass-card p-8 max-w-md">
          <div className="w-16 h-16 bg-red-500/20 rounded-full flex items-center justify-center mx-auto mb-4">
            <Blocks className="w-8 h-8 text-red-400" />
          </div>
          <h2 className="text-xl font-bold text-white mb-2">Connection Error</h2>
          <p className="text-gray-400 mb-4">
            Could not connect to the Atlas Sphere node. Make sure it&apos;s running.
          </p>
          <code className="block bg-black/50 rounded p-2 text-sm text-gray-300 mb-4">
            ./run-dev-node.sh
          </code>
          <button
            onClick={() => window.location.reload()}
            className="px-4 py-2 bg-orange-500 hover:bg-orange-600 text-white rounded-lg"
          >
            Retry Connection
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero / Search Section with unique Explorer header */}
      <section className="py-12 relative overflow-hidden border-b border-[#1a1a1a] page-header-explorer">
        <div className="absolute inset-0 bg-gradient-to-br from-cyan-950/20 via-black to-blue-950/10" />
        <div className="absolute inset-0 mesh-gradient opacity-10" />
        <div className="absolute right-0 top-0 w-96 h-96 opacity-10">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="flex items-center justify-between mb-6">
            <div>
              <div className="inline-flex items-center px-3 py-1 rounded-full bg-cyan-500/10 border border-cyan-500/30 mb-4">
                <Blocks className="w-4 h-4 mr-2 text-cyan-400" />
                <span className="text-sm text-cyan-300">Blockchain Explorer</span>
              </div>
              <h1 className="text-3xl md:text-4xl font-bold text-white mb-2">
                X3scan <span className="text-cyan-400">Explorer</span>
              </h1>
              <p className="text-gray-500">
                Explore the X3 STAR Atlas Sphere blockchain in real-time
              </p>
            </div>
            <div className="flex items-center space-x-2">
              <span className={`w-2 h-2 rounded-full ${isLive ? 'bg-emerald-500 animate-pulse' : 'bg-gray-600'}`} />
              <span className="text-sm text-gray-500">{isLive ? 'Live' : 'Paused'}</span>
              <button
                onClick={() => setIsLive(!isLive)}
                className="p-2 rounded-lg hover:bg-[#1a1a1a] transition-colors"
                title={isLive ? 'Pause live updates' : 'Resume live updates'}
              >
                <RefreshCw className={`w-4 h-4 text-gray-500 ${isLive ? 'animate-spin' : ''}`} style={{ animationDuration: '3s' }} />
              </button>
            </div>
          </div>

          {/* Search */}
          <form onSubmit={handleSearch} className="relative max-w-3xl">
            <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-600" />
            <input
              type="text"
              placeholder="Search by Address / Tx Hash / Block Number"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-12 pr-4 py-4 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-600 focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
            />
            <button
              type="submit"
              className="absolute right-2 top-1/2 -translate-y-1/2 px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg font-medium transition-colors"
            >
              Search
            </button>
          </form>
        </div>
      </section>

      {/* Stats */}
      <section className="py-8 border-b border-[#1a1a1a] bg-black">
        <div className="container-wide">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {networkStatsData.map((stat) => (
              <div key={stat.label} className="glass-card p-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-cyan-400">{stat.icon}</span>
                  {stat.change && <span className="text-xs text-emerald-400">{stat.change}</span>}
                </div>
                <div className="text-2xl font-bold text-white">
                  {statsLoading ? (
                    <span className="animate-pulse bg-[#1a1a1a] rounded h-8 w-20 inline-block" />
                  ) : (
                    stat.value
                  )}
                </div>
                <div className="text-sm text-gray-500">{stat.label}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Tabs */}
      <section className="border-b border-[#1a1a1a] bg-black">
        <div className="container-wide">
          <div className="flex space-x-1">
            {[
              { id: 'overview', label: 'Overview', icon: <Activity className="w-4 h-4" /> },
              { id: 'blocks', label: 'Blocks', icon: <Blocks className="w-4 h-4" /> },
              { id: 'transactions', label: 'Transactions', icon: <ArrowRightLeft className="w-4 h-4" /> },
              { id: 'accounts', label: 'Accounts', icon: <Users className="w-4 h-4" /> },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as typeof activeTab)}
                className={`flex items-center px-4 py-4 border-b-2 font-medium text-sm transition-colors ${
                  activeTab === tab.id
                    ? 'border-cyan-500 text-cyan-400'
                    : 'border-transparent text-gray-500 hover:text-white hover:border-[#333333]'
                }`}
              >
                <span className="mr-2">{tab.icon}</span>
                {tab.label}
              </button>
            ))}
          </div>
        </div>
      </section>

      {/* Content */}
      <section className="py-8 bg-black">
        <div className="container-wide">
          {activeTab === 'overview' && (
            <div className="grid lg:grid-cols-2 gap-8">
              {/* Latest Blocks */}
              <div className="glass-card overflow-hidden">
                <div className="p-4 border-b border-[#1a1a1a] flex items-center justify-between">
                  <h3 className="font-semibold text-white flex items-center">
                    <Blocks className="w-5 h-5 mr-2 text-orange-400" />
                    Latest Blocks
                  </h3>
                  <Link href="/explorer?tab=blocks" className="text-sm text-orange-400 hover:text-orange-300">
                    View All
                  </Link>
                </div>
                <div className="divide-y divide-[#1a1a1a]">
                  {blocks && blocks.length > 0 ? (
                    blocks.slice(0, 5).map((block: BlockInfo) => (
                      <div key={block.hash} className="p-4 hover:bg-[#0a0a0a] transition-colors">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center space-x-3">
                            <div className="w-10 h-10 rounded-lg bg-orange-500/20 flex items-center justify-center">
                              <Blocks className="w-5 h-5 text-orange-400" />
                            </div>
                            <div>
                              <Link href={`/explorer/block/${block.number}`} className="font-mono text-white hover:text-orange-400">
                                #{block.number.toLocaleString()}
                              </Link>
                              <div className="text-xs text-gray-600">{formatTime(block.timestamp)}</div>
                            </div>
                          </div>
                          <div className="text-right">
                            <div className="text-sm text-gray-400">{block.extrinsicsCount} extrinsics</div>
                            <div className="text-xs text-gray-600 font-mono">
                              {block.author ? formatHash(block.author) : 'Unknown'}
                            </div>
                          </div>
                        </div>
                      </div>
                    ))
                  ) : (
                    <div className="p-8 text-center text-gray-600">
                      {statsLoading ? 'Loading blocks...' : 'No blocks yet'}
                    </div>
                  )}
                </div>
              </div>

              {/* Latest Transactions */}
              <div className="glass-card overflow-hidden">
                <div className="p-4 border-b border-[#1a1a1a] flex items-center justify-between">
                  <h3 className="font-semibold text-white flex items-center">
                    <ArrowRightLeft className="w-5 h-5 mr-2 text-cyan-400" />
                    Latest Transactions
                  </h3>
                  <Link href="/explorer?tab=transactions" className="text-sm text-cyan-400 hover:text-cyan-300">
                    View All
                  </Link>
                </div>
                <div className="divide-y divide-[#1a1a1a]">
                  {extrinsics && extrinsics.length > 0 ? (
                    extrinsics.slice(0, 5).map((ext: ExtrinsicInfo, index: number) => {
                      const vmType = getVmType(ext.section);
                      const vmStyle = getVmStyle(vmType);
                      
                      return (
                        <div key={`${ext.hash}-${index}`} className="p-4 hover:bg-[#0a0a0a] transition-colors">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center space-x-3">
                              <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${vmStyle.bg}`}>
                                <span className={`text-xs font-bold ${vmStyle.text}`}>
                                  {vmType === 'Comit' ? '⚛' : vmType[0]}
                                </span>
                              </div>
                              <div>
                                <div className="flex items-center space-x-2">
                                  <Link href={`/explorer/tx/${ext.hash}`} className="font-mono text-sm text-white hover:text-cyan-400">
                                    {formatHash(ext.hash)}
                                  </Link>
                                  <span className={`badge text-xs ${vmStyle.badge}`}>
                                    {vmType}
                                  </span>
                                </div>
                                <div className="text-xs text-gray-600">
                                  {ext.section}.{ext.method}
                                </div>
                              </div>
                            </div>
                            <div className="text-right">
                              <div className={`text-sm ${ext.success ? 'text-green-400' : 'text-red-400'}`}>
                                {ext.success ? '✓ Success' : '✗ Failed'}
                              </div>
                              <div className="text-xs text-gray-600">{formatTime(ext.timestamp)}</div>
                            </div>
                          </div>
                        </div>
                      );
                    })
                  ) : (
                    <div className="p-8 text-center text-gray-600">
                      {statsLoading ? 'Loading transactions...' : 'No transactions yet'}
                    </div>
                  )}
                </div>
              </div>

              {/* VM Statistics */}
              <div className="lg:col-span-2">
                <h3 className="font-semibold text-white mb-4">Virtual Machine Statistics</h3>
                <div className="grid md:grid-cols-3 gap-4">
                  <div className="glass-card p-6 border-l-4 border-l-blue-500">
                    <div className="flex items-center justify-between mb-4">
                      <h4 className="text-lg font-bold text-blue-400">EVM</h4>
                      <span className="badge badge-info">{vmStats.evm} recent txns</span>
                    </div>
                    <div className="space-y-2 text-sm">
                      <div className="flex justify-between">
                        <span className="text-gray-500">Type</span>
                        <span className="text-white">Frontier-based</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-500">Compatibility</span>
                        <span className="text-white">Ethereum</span>
                      </div>
                    </div>
                  </div>
                  
                  <div className="glass-card p-6 border-l-4 border-l-emerald-500">
                    <div className="flex items-center justify-between mb-4">
                      <h4 className="text-lg font-bold text-emerald-400">SVM</h4>
                      <span className="badge badge-success">{vmStats.svm} recent txns</span>
                    </div>
                    <div className="space-y-2 text-sm">
                      <div className="flex justify-between">
                        <span className="text-gray-500">Type</span>
                        <span className="text-white">Solana Programs</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-500">Compatibility</span>
                        <span className="text-white">Solana</span>
                      </div>
                    </div>
                  </div>
                  
                  <div className="glass-card p-6 border-l-4 border-l-orange-500">
                    <div className="flex items-center justify-between mb-4">
                      <h4 className="text-lg font-bold text-orange-400">Cross-VM (Comits)</h4>
                      <span className="badge badge-fire">{vmStats.comit} recent</span>
                    </div>
                    <div className="space-y-2 text-sm">
                      <div className="flex justify-between">
                        <span className="text-gray-500">Type</span>
                        <span className="text-white">Atomic Dual-VM</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-500">Execution</span>
                        <span className="text-white">Parallel EVM+SVM</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'blocks' && (
            <div className="glass-card overflow-hidden">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-[#1a1a1a]">
                    <th className="table-header">Block</th>
                    <th className="table-header">Hash</th>
                    <th className="table-header">Extrinsics</th>
                    <th className="table-header">Author</th>
                    <th className="table-header">Time</th>
                  </tr>
                </thead>
                <tbody>
                  {blocks && blocks.length > 0 ? (
                    blocks.map((block: BlockInfo) => (
                      <tr key={block.hash} className="border-b border-[#111111] hover:bg-[#0a0a0a]">
                        <td className="table-cell">
                          <Link href={`/explorer/block/${block.number}`} className="text-orange-400 hover:text-orange-300 font-mono">
                            #{block.number.toLocaleString()}
                          </Link>
                        </td>
                        <td className="table-cell font-mono text-gray-500">{formatHash(block.hash)}</td>
                        <td className="table-cell text-white">{block.extrinsicsCount}</td>
                        <td className="table-cell font-mono text-gray-500">
                          {block.author ? formatHash(block.author) : 'Unknown'}
                        </td>
                        <td className="table-cell text-gray-600">{formatTime(block.timestamp)}</td>
                      </tr>
                    ))
                  ) : (
                    <tr>
                      <td colSpan={5} className="table-cell text-center text-gray-600 py-8">
                        No blocks found
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )}

          {activeTab === 'transactions' && (
            <div className="glass-card overflow-hidden">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-[#1a1a1a]">
                    <th className="table-header">Tx Hash</th>
                    <th className="table-header">Type</th>
                    <th className="table-header">Method</th>
                    <th className="table-header">From</th>
                    <th className="table-header">Status</th>
                    <th className="table-header">Time</th>
                  </tr>
                </thead>
                <tbody>
                  {extrinsics && extrinsics.length > 0 ? (
                    extrinsics.map((ext: ExtrinsicInfo, index: number) => {
                      const vmType = getVmType(ext.section);
                      const vmStyle = getVmStyle(vmType);
                      
                      return (
                        <tr key={`${ext.hash}-${index}`} className="border-b border-[#111111] hover:bg-[#0a0a0a]">
                          <td className="table-cell">
                            <Link href={`/explorer/tx/${ext.hash}`} className="text-cyan-400 hover:text-cyan-300 font-mono">
                              {formatHash(ext.hash)}
                            </Link>
                          </td>
                          <td className="table-cell">
                            <span className={`badge text-xs ${vmStyle.badge}`}>
                              {vmType}
                            </span>
                          </td>
                          <td className="table-cell text-gray-400">{ext.section}.{ext.method}</td>
                          <td className="table-cell font-mono text-gray-500">
                            {ext.signer ? formatHash(ext.signer) : 'Unsigned'}
                          </td>
                          <td className="table-cell">
                            <span className={ext.success ? 'text-green-400' : 'text-red-400'}>
                              {ext.success ? '✓ Success' : '✗ Failed'}
                            </span>
                          </td>
                          <td className="table-cell text-gray-600">{formatTime(ext.timestamp)}</td>
                        </tr>
                      );
                    })
                  ) : (
                    <tr>
                      <td colSpan={6} className="table-cell text-center text-gray-600 py-8">
                        No transactions found
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )}

          {activeTab === 'accounts' && (
            <div className="text-center py-16">
              <Users className="w-16 h-16 text-gray-700 mx-auto mb-4" />
              <h3 className="text-xl font-semibold text-white mb-2">Account Explorer</h3>
              <p className="text-gray-500 mb-6">Search for an account address to view details</p>
              <form onSubmit={(e) => {
                e.preventDefault();
                const input = (e.target as HTMLFormElement).elements.namedItem('address') as HTMLInputElement;
                if (input.value.trim()) {
                  router.push(`/explorer/account/${input.value.trim()}`);
                }
              }} className="max-w-md mx-auto">
                <input
                  type="text"
                  name="address"
                  placeholder="Enter account address (5xxx... or 0x...)"
                  className="w-full pl-4 pr-4 py-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-600 focus:outline-none focus:border-cyan-500"
                />
                <button
                  type="submit"
                  className="mt-4 px-6 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg font-medium transition-colors"
                >
                  View Account
                </button>
              </form>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
