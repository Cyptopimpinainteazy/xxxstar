import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { Plus, Droplets, TrendingUp, Search, Filter, Loader2, Wifi, WifiOff } from 'lucide-react';
import clsx from 'clsx';
import x3Chain, { type LiquidityPool } from '@/services/x3ChainService';

type PoolFilter = 'all' | 'yours' | 'gainers';

interface PoolUI {
  id: string;
  tokenA: string;
  tokenB: string;
  iconA: string;
  iconB: string;
  tvl: string;
  apr: number;
  volume24h: string;
  volume7d: string;
  yourLiquidity: string | null;
  isReal: boolean;
}

const TOKEN_ICONS: Record<string, string> = {
  X3: '🔵',
  ETH: '⟠',
  SOL: '◎',
  USDC: '💲',
  WETH: '⟠',
};

const MOCK_POOLS: PoolUI[] = [
  { id: 'm1', tokenA: 'X3', tokenB: 'USDC', iconA: '🔵', iconB: '💲', tvl: '$32.1M', apr: 24.5, volume24h: '$4.2M', volume7d: '$28.7M', yourLiquidity: '$12,450', isReal: false },
  { id: 'm2', tokenA: 'ETH', tokenB: 'X3', iconA: '⟠', iconB: '🔵', tvl: '$18.4M', apr: 18.2, volume24h: '$2.8M', volume7d: '$19.1M', yourLiquidity: '$5,200', isReal: false },
  { id: 'm3', tokenA: 'SOL', tokenB: 'X3', iconA: '◎', iconB: '🔵', tvl: '$12.7M', apr: 31.8, volume24h: '$1.9M', volume7d: '$13.4M', yourLiquidity: '$820', isReal: false },
];

const DexPoolsPanel: React.FC = () => {
  const [filter, setFilter] = useState<PoolFilter>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [realPools, setRealPools] = useState<LiquidityPool[]>([]);
  const [loading, setLoading] = useState(true);
  const [chainConnected, setChainConnected] = useState(x3Chain.isConnected);
  const [showAnalytics, setShowAnalytics] = useState<string | null>(null);

  const loadPools = useCallback(async () => {
    try {
      const p = await x3Chain.getLiquidityPools();
      setRealPools(p);
    } catch (err) {
      console.warn('[DexPools] Failed to fetch live pools:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPools();
    const unsub = x3Chain.onConnectionChange(setChainConnected);
    const iv = setInterval(loadPools, 10000);
    return () => {
      if (typeof unsub === 'function') unsub();
      clearInterval(iv);
    };
  }, [loadPools]);

  const uiPools: PoolUI[] = useMemo(() => {
    const live = realPools.map((p) => {
      const symA = x3Chain.getAssetSymbol(p.tokenA);
      const symB = x3Chain.getAssetSymbol(p.tokenB);
      const decA = x3Chain.getAssetDecimals(p.tokenA);
      const decB = x3Chain.getAssetDecimals(p.tokenB);

      const resA = Number(x3Chain.fromChainUnits(p.reserveA, decA));
      const resB = Number(x3Chain.fromChainUnits(p.reserveB, decB));

      return {
        id: p.poolId,
        tokenA: symA,
        tokenB: symB,
        iconA: TOKEN_ICONS[symA] ?? '🪙',
        iconB: TOKEN_ICONS[symB] ?? '🪙',
        tvl: `$${(resA * 1.25 + resB).toLocaleString(undefined, { maximumFractionDigits: 0 })}`,
        apr: (p.feeBps / 100) * 12, 
        volume24h: '$0',
        volume7d: '$0',
        yourLiquidity: null,
        isReal: true,
      };
    });

    return live.length > 0 ? live : MOCK_POOLS;
  }, [realPools]);

  const filteredPools = uiPools.filter((pool) => {
    const markets = `${pool.tokenA}/${pool.tokenB}`.toLowerCase();
    const matchesSearch = !searchQuery || markets.includes(searchQuery.toLowerCase());
    if (filter === 'yours') return matchesSearch && pool.yourLiquidity !== null;
    if (filter === 'gainers') return matchesSearch && pool.apr > 20;
    return matchesSearch;
  });

  const totalTvlVal = uiPools.reduce((acc, p) => {
    const val = parseFloat(p.tvl.replace(/[$,M]/g, ''));
    const mult = p.tvl.includes('M') ? 1000000 : 1;
    return acc + (isNaN(val) ? 0 : val * mult);
  }, 0);

  const aprColor = (apr: number) => (apr > 20 ? 'text-green-400' : apr > 10 ? 'text-blue-400' : 'text-white');

  const renderAnalytics = (pool: PoolUI) => {
    const feesEarned = pool.yourLiquidity ? parseFloat(pool.yourLiquidity.replace(/[$,]/g, '')) * (pool.apr / 100) : 0;
    const fee24h = parseFloat(pool.volume24h.replace(/[$,M]/g, '')) * 0.003; // 0.3% fee
    const fee7d = parseFloat(pool.volume7d.replace(/[$,M]/g, '')) * 0.003;

    return (
      <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
        <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl max-h-96 overflow-auto">
          <h3 className="font-bold text-white mb-4">{pool.tokenA}/{pool.tokenB} Analytics</h3>
          
          <div className="space-y-4 text-sm">
            <div className="bg-[#0a0a0f] rounded-lg p-3">
              <div className="text-gray-500 mb-1">Pool TVL</div>
              <div className="text-lg font-bold text-white">{pool.tvl}</div>
            </div>

            <div className="bg-[#0a0a0f] rounded-lg p-3">
              <div className="text-gray-500 mb-1">Your Share & Fees (Annual)</div>
              <div className="flex justify-between items-center">
                <span className="text-white font-semibold">{pool.yourLiquidity || '—'}</span>
                <span className="text-green-400 font-semibold">{feesEarned.toFixed(2)} X3</span>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-2">
              <div className="bg-[#0a0a0f] rounded-lg p-3">
                <div className="text-gray-500 text-xs mb-1">24h Fees Generated</div>
                <div className="font-bold text-blue-400">${fee24h.toFixed(3)}K</div>
              </div>
              <div className="bg-[#0a0a0f] rounded-lg p-3">
                <div className="text-gray-500 text-xs mb-1">7d Fees Generated</div>
                <div className="font-bold text-blue-400">${fee7d.toFixed(2)}K</div>
              </div>
            </div>

            <div className="bg-[#0a0a0f] rounded-lg p-3">
              <div className="text-gray-500 mb-1">APY Breakdown</div>
              <div className="space-y-1 text-xs">
                <div className="flex justify-between">
                  <span>Base APR</span>
                  <span className="text-green-400">{pool.apr}% annual</span>
                </div>
                <div className="flex justify-between">
                  <span>Swap Fees</span>
                  <span className="text-green-400">{(pool.apr * 0.7).toFixed(1)}%</span>
                </div>
                <div className="flex justify-between">
                  <span>LM Rewards</span>
                  <span className="text-green-400">{(pool.apr * 0.3).toFixed(1)}%</span>
                </div>
              </div>
            </div>
          </div>

          <button
            onClick={() => setShowAnalytics(null)}
            className="w-full mt-4 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-500 to-blue-600 text-white font-semibold"
          >
            Close
          </button>
        </div>
      </div>
    );
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Droplets size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">Liquidity Pools</h1>
          <div className={clsx(
            'flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded border',
            chainConnected ? 'text-green-400 border-green-500/30 bg-green-500/10' : 'text-gray-500 border-[#1a1a1a] bg-[#111111]',
          )}>
            {chainConnected ? <Wifi size={8} /> : <WifiOff size={8} />}
            {chainConnected ? 'Live' : 'Offline'}
          </div>
        </div>
        <div className="flex items-center gap-3">
          <div className="relative">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
            <input
              type="text"
              placeholder="Search pools..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="bg-[#111111] border border-[#1a1a1a] rounded-lg pl-9 pr-3 py-2 text-xs text-white placeholder-gray-600 outline-none focus:border-blue-500/40 w-48"
            />
          </div>
          <button className="flex items-center gap-2 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-400 hover:to-blue-500 text-white text-xs font-semibold px-4 py-2 rounded-lg transition-all shadow-lg shadow-blue-500/20">
            <Plus size={14} /> New Position
          </button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-3 px-5 py-4">
        {[
          { label: 'Network TVL', value: `$${(totalTvlVal / 1000000).toFixed(1)}M` },
          { label: '24h Volume', value: realPools.length > 0 ? '$0' : '$12.4M' },
          { label: 'Pools Active', value: uiPools.length.toString() },
          { label: 'Your Assets', value: '3' },
        ].map((s) => (
          <div key={s.label} className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition-colors">
            <div className="text-xs text-gray-500">{s.label}</div>
            <div className="text-lg font-bold text-white mt-0.5">{s.value}</div>
          </div>
        ))}
      </div>

      {/* Filter tabs */}
      <div className="px-5 flex items-center justify-between mb-3">
        <div className="flex items-center gap-1">
          <Filter size={12} className="text-gray-500 mr-1" />
          {(['all', 'yours', 'gainers'] as const).map((key) => (
            <button
              key={key}
              onClick={() => setFilter(key)}
              className={clsx(
                'px-3 py-1.5 rounded-md text-xs font-medium transition-colors capitalize',
                filter === key ? 'bg-blue-500/20 text-blue-400' : 'text-gray-500 hover:text-white',
              )}
            >
              {key === 'all' ? 'All Pools' : key === 'yours' ? 'My Positions' : 'Top APR'}
            </button>
          ))}
        </div>
        {loading && <Loader2 size={12} className="animate-spin text-blue-400" />}
      </div>

      {/* Pool table */}
      <div className="flex-1 px-5 pb-5 overflow-auto">
        <div className="bg-[#111111] rounded-xl border border-[#1a1a1a] overflow-hidden shadow-2xl">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[#1a1a1a] text-gray-500 text-xs">
                <th className="text-left p-3">Pool</th>
                <th className="text-right p-3">TVL</th>
                <th className="text-right p-3">APR</th>
                <th className="text-right p-3">24h Volume</th>
                <th className="text-right p-3">7d Volume</th>
                <th className="text-right p-3">Your Stake</th>
              </tr>
            </thead>
            <tbody>
              {filteredPools.map((pool) => (
                <tr
                  key={pool.id}
                  onClick={() => setShowAnalytics(pool.id)}
                  className="border-b border-[#1a1a1a] last:border-0 hover:bg-[#0f0f14] transition-colors cursor-pointer group"
                >
                  <td className="p-3">
                    <div className="flex items-center gap-2">
                      <div className="flex -space-x-1.5">
                        <span className="text-base bg-[#0a0a0f] rounded-full p-0.5">{pool.iconA}</span>
                        <span className="text-base bg-[#0a0a0f] rounded-full p-0.5">{pool.iconB}</span>
                      </div>
                      <div className="flex flex-col">
                        <span className="font-medium text-white group-hover:text-blue-400 transition-colors">
                          {pool.tokenA}/{pool.tokenB}
                        </span>
                        {!pool.isReal && <span className="text-[9px] text-orange-500/70 font-mono tracking-tighter">DEMO MODE</span>}
                      </div>
                    </div>
                  </td>
                  <td className="p-3 text-right text-white font-mono">{pool.tvl}</td>
                  <td className={clsx('p-3 text-right font-medium', aprColor(pool.apr))}>
                    <div className="flex items-center justify-end gap-1 font-mono">
                      <TrendingUp size={12} />
                      {pool.apr.toFixed(1)}%
                    </div>
                  </td>
                  <td className="p-3 text-right text-gray-400 font-mono">{pool.volume24h}</td>
                  <td className="p-3 text-right text-gray-400 font-mono">{pool.volume7d}</td>
                  <td className="p-3 text-right">
                    {pool.yourLiquidity ? (
                      <span className="text-white font-medium font-mono">{pool.yourLiquidity}</span>
                    ) : (
                      <span className="text-gray-600">—</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {filteredPools.length === 0 && (
            <div className="p-12 text-center text-gray-500 text-sm">
              <Droplets size={32} className="mx-auto mb-2 opacity-20" />
              No pools match your search criteria.
            </div>
          )}
        </div>
      </div>

      {/* Analytics Modal */}
      {showAnalytics && renderAnalytics(filteredPools.find(p => p.id === showAnalytics)!)}
    </div>
  );
};

export default DexPoolsPanel;
