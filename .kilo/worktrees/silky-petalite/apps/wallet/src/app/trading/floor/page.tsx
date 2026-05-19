'use client';

import { useState, useEffect, useMemo } from 'react';
import type { FloorStats, ArbIntent } from '@/lib/x3/types';
import { IntentState } from '@/lib/x3/types';
import { getFloorStats, getIntents } from '@/lib/x3/services/api';
import { Button, ProgressBar, Badge } from '@/components/x3/UIComponents';
import {
  VolumeTrendChart,
  IntentStatePie,
  type TrendPoint,
  type StateDistribution,
} from '@/components/x3/Charts';

// Demo data for static rendering — replaced by API in production.
const DEMO_STATS: FloorStats = {
  activeAgents: 47,
  totalIntents: 12_849,
  totalVolume: '84,291,003.21',
  totalSlashes: 23,
  totalDisputes: 7,
  avgSuccessRate: 94.7,
  activeFlashloans: 3,
};

const DEMO_NOW = 1_700_000_120_000;

const DET_FEED_STATES = [
  IntentState.Executing,
  IntentState.Executed,
  IntentState.Slashed,
  IntentState.Finalized,
];

const DEMO_FEED: ArbIntent[] = [
  {
    id: '0xa3f1..8c02',
    agentId: 'agent-alpha',
    state: IntentState.Finalized,
    legs: [
      {
        chain: 'ETH',
        protocol: 'UniV3',
        tokenIn: 'WETH',
        tokenOut: 'USDC',
        amountIn: '10.0',
        expectedOut: '18,421.50',
      },
      {
        chain: 'ARB',
        protocol: 'Camelot',
        tokenIn: 'USDC',
        tokenOut: 'WETH',
        amountIn: '18,421.50',
        expectedOut: '10.04',
      },
    ],
    feeCap: 42.0,
    feeActual: 38.2,
    createdAt: DEMO_NOW - 12000,
    executedAt: DEMO_NOW - 8000,
    proofHash: 'e9c1a2b3d4f5...',
  },
  {
    id: '0xb7e2..1a4f',
    agentId: 'agent-bravo',
    state: IntentState.Executing,
    legs: [
      {
        chain: 'SOL',
        protocol: 'Raydium',
        tokenIn: 'SOL',
        tokenOut: 'USDC',
        amountIn: '500',
        expectedOut: '48,250.00',
      },
    ],
    feeCap: 25.0,
    feeActual: null,
    createdAt: DEMO_NOW - 3000,
    executedAt: null,
    proofHash: null,
  },
  {
    id: '0xd4c3..9f87',
    agentId: 'agent-delta',
    state: IntentState.Slashed,
    legs: [
      {
        chain: 'ETH',
        protocol: 'UniV3',
        tokenIn: 'USDC',
        tokenOut: 'DAI',
        amountIn: '50,000',
        expectedOut: '49,995',
      },
    ],
    feeCap: 12.0,
    feeActual: null,
    createdAt: DEMO_NOW - 60000,
    executedAt: null,
    proofHash: 'f8a1b2c3d4e5...',
  },
];

function stateColor(state: IntentState): 'green' | 'blue' | 'red' | 'amber' | 'muted' {
  switch (state) {
    case IntentState.Finalized:
      return 'green';
    case IntentState.Executing:
    case IntentState.Executed:
      return 'blue';
    case IntentState.Slashed:
      return 'red';
    case IntentState.Expired:
    case IntentState.Cancelled:
      return 'muted';
    default:
      return 'amber';
  }
}

function timeSince(ts: number): string {
  const seconds = Math.max(0, Math.floor((DEMO_NOW - ts) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

export default function FloorDashboard() {
  const [stats, setStats] = useState<FloorStats>(DEMO_STATS);
  const [feed, setFeed] = useState<ArbIntent[]>(DEMO_FEED);
  const [volumeSeries, setVolumeSeries] = useState<number[]>([
    84_291_003.21, 82_100_000, 80_500_000, 83_000_000, 84_291_003.21,
  ]);
  const [successSeries, setSuccessSeries] = useState<number[]>([
    92, 93, 94, 94.5, 94.7,
  ]);
  const [autoRefresh, setAutoRefresh] = useState(false);

  useEffect(() => {
    if (!autoRefresh) return;

    let mounted = true;

    async function fetchLive() {
      try {
        const s = await getFloorStats();
        const r = await getIntents(1, 10);
        if (!mounted) return;
        setStats(s);
        setFeed(r.items.slice(0, 10));
        setVolumeSeries((v) => [
          ...v.slice(-4),
          Number(s.totalVolume.replace(/[,]/g, '')),
        ]);
        setSuccessSeries((srs) => [...srs.slice(-4), s.avgSuccessRate]);
      } catch (e) {
        // Fallback to demo data
      }
    }

    fetchLive();
    const interval = setInterval(() => {
      if (mounted) {
        fetchLive();
        setFeed((prev) => {
          const next = prev.length + 1;
          const id = `0x${(next + 0x100).toString(16)}`;
          const state = DET_FEED_STATES[next % DET_FEED_STATES.length];
          const newIntent: ArbIntent = {
            id,
            agentId: `agent-${String((next * 37) % 1000).padStart(3, '0')}`,
            state,
            legs: [
              {
                chain: 'ETH',
                protocol: 'UniV3',
                tokenIn: 'WETH',
                tokenOut: 'USDC',
                amountIn: '1.0',
                expectedOut: '1842.10',
              },
            ],
            feeCap: 10,
            feeActual: null,
            createdAt: DEMO_NOW - next * 1000,
            executedAt: null,
            proofHash: null,
          };
          return [newIntent, ...prev].slice(0, 20);
        });
      }
    }, 3000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, [autoRefresh]);

  const volRounded = useMemo(() => stats.totalVolume, [stats.totalVolume]);

  const trendData: TrendPoint[] = useMemo(() => {
    return volumeSeries.map((vol, i) => ({
      timestamp: `${i}h ago`,
      volume: vol,
      successRate: successSeries[i] || 0,
    }));
  }, [volumeSeries, successSeries]);

  const stateDistribution: StateDistribution[] = useMemo(() => {
    const counts: Record<string, number> = {};
    feed.forEach((intent) => {
      counts[intent.state] = (counts[intent.state] || 0) + 1;
    });
    return Object.entries(counts).map(([name, value]) => ({
      name,
      value,
    }));
  }, [feed]);

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-baseline gap-4">
        <h1 className="text-3xl font-bold">X3 Floor</h1>
        <span className="text-gray-400">Arbitrage jurisdiction — live</span>
        <Button
          variant={autoRefresh ? 'success' : 'secondary'}
          size="sm"
          onClick={() => setAutoRefresh(!autoRefresh)}
          className="ml-auto"
        >
          {autoRefresh ? '🔄 Live' : '⏸ Paused'}
        </Button>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Active Agents</div>
          <div className="text-2xl font-bold text-green-400">{stats.activeAgents}</div>
        </div>

        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Total Intents</div>
          <div className="text-2xl font-bold">{stats.totalIntents.toLocaleString()}</div>
        </div>

        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Volume (USDC)</div>
          <div className="text-2xl font-bold">{volRounded}</div>
        </div>

        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Success Rate</div>
          <div className="text-2xl font-bold text-green-400">{stats.avgSuccessRate}%</div>
          <div className="mt-2">
            <ProgressBar
              value={stats.avgSuccessRate}
              max={100}
              color="green"
            />
          </div>
        </div>
      </div>

      {/* Charts Section */}
      <div className="grid grid-cols-3 gap-4">
        <div className="col-span-2 bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold mb-4">Volume & Success Trend</h2>
          <VolumeTrendChart data={trendData} />
        </div>

        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold mb-4">Intent State Distribution</h2>
          <IntentStatePie data={stateDistribution} />
        </div>
      </div>

      {/* Live Execution Feed */}
      <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-lg font-bold">Execution Feed</h2>
          <span className="text-xs text-gray-400">LIVE • {feed.length} items</span>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="text-xs text-gray-400 uppercase tracking-wider border-b border-x3-dark-gray">
              <tr>
                <th className="text-left py-2">Intent</th>
                <th className="text-left py-2">Agent</th>
                <th className="text-left py-2">State</th>
                <th className="text-left py-2">Route</th>
                <th className="text-left py-2">Fee</th>
                <th className="text-left py-2">Time</th>
              </tr>
            </thead>
            <tbody>
              {feed.map((intent) => (
                <tr key={intent.id} className="border-b border-x3-dark hover:bg-x3-dark-gray transition-colors">
                  <td className="py-2 font-mono text-xs">{intent.id}</td>
                  <td className="py-2 font-mono text-xs">{intent.agentId}</td>
                  <td className="py-2">
                    <Badge variant={stateColor(intent.state)}>
                      {intent.state}
                    </Badge>
                  </td>
                  <td className="py-2 text-xs">
                    {intent.legs.map((leg, i) => (
                      <span key={i}>
                        {i > 0 && ' → '}
                        <span className="text-gray-400">{leg.chain}</span>:
                        {leg.tokenIn}→{leg.tokenOut}
                      </span>
                    ))}
                  </td>
                  <td className="py-2 font-mono">
                    {intent.feeActual !== null ? (
                      <span className="text-green-400">{intent.feeActual.toFixed(1)}</span>
                    ) : (
                      <span className="text-gray-600">—</span>
                    )}
                    <span className="text-gray-600"> / {intent.feeCap.toFixed(1)}</span>
                  </td>
                  <td className="py-2 text-gray-400 text-xs">
                    {timeSince(intent.createdAt)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
