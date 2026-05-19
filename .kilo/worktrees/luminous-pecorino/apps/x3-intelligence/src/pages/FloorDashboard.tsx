// Floor Dashboard — live stats and execution feed with professional charts

import { useState, useEffect, useMemo } from "react";
import type { FloorStats, ArbIntent } from "../types";
import { IntentState } from "../types";
import { getFloorStats, getIntents } from "../services/api";
import { Button, Metric, ProgressBar, Badge, Loading } from "../components/UIComponents";
import HelpModal from "../components/HelpModal";
import { dataIntegrity } from "../services/dataIntegrity";
import { useWebSocket } from "../hooks/useWebSocket";
import {
  VolumeTrendChart,
  IntentStatePie,
  TrendPoint,
  StateDistribution,
} from "../components/Charts";

// Demo data for static rendering — replaced by API in production.
const DEMO_STATS: FloorStats = {
  activeAgents: 47,
  totalIntents: 12_849,
  totalVolume: "84,291,003.21",
  totalSlashes: 23,
  totalDisputes: 7,
  avgSuccessRate: 94.7,
  activeFlashloans: 3,
};

const DEMO_FEED: ArbIntent[] = [
  {
    id: "0xa3f1..8c02",
    agentId: "agent-alpha",
    state: IntentState.Finalized,
    legs: [
      {
        chain: "ETH",
        protocol: "UniV3",
        tokenIn: "WETH",
        tokenOut: "USDC",
        amountIn: "10.0",
        expectedOut: "18,421.50",
      },
      {
        chain: "ARB",
        protocol: "Camelot",
        tokenIn: "USDC",
        tokenOut: "WETH",
        amountIn: "18,421.50",
        expectedOut: "10.04",
      },
    ],
    feeCap: 42.0,
    feeActual: 38.2,
    createdAt: Date.now() - 12000,
    executedAt: Date.now() - 8000,
    proofHash: "e9c1a2b3d4f5...",
  },
  {
    id: "0xb7e2..1a4f",
    agentId: "agent-bravo",
    state: IntentState.Executing,
    legs: [
      {
        chain: "SOL",
        protocol: "Raydium",
        tokenIn: "SOL",
        tokenOut: "USDC",
        amountIn: "500",
        expectedOut: "48,250.00",
      },
    ],
    feeCap: 25.0,
    feeActual: null,
    createdAt: Date.now() - 3000,
    executedAt: null,
    proofHash: null,
  },
  {
    id: "0xd4c3..9f87",
    agentId: "agent-delta",
    state: IntentState.Slashed,
    legs: [
      {
        chain: "ETH",
        protocol: "UniV3",
        tokenIn: "USDC",
        tokenOut: "DAI",
        amountIn: "50,000",
        expectedOut: "49,995",
      },
    ],
    feeCap: 12.0,
    feeActual: null,
    createdAt: Date.now() - 60000,
    executedAt: null,
    proofHash: "f8a1b2c3d4e5...",
  },
];

function stateColor(state: IntentState): string {
  switch (state) {
    case IntentState.Finalized:
      return "badge-green";
    case IntentState.Executing:
    case IntentState.Executed:
      return "badge-blue";
    case IntentState.Slashed:
      return "badge-red";
    case IntentState.Expired:
    case IntentState.Cancelled:
      return "badge-muted";
    default:
      return "badge-amber";
  }
}

function timeSince(ts: number): string {
  const seconds = Math.floor((Date.now() - ts) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

// Small SVG sparkline for visuals (no external deps)
function Sparkline({ data, color = "#00d4aa" }: { data: number[]; color?: string }) {
  const w = 220;
  const h = 48;
  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const points = data.map((v, i) => {
    const x = (i / Math.max(1, data.length - 1)) * w;
    const y = h - ((v - min) / (max - min || 1)) * h;
    return `${x},${y}`;
  });
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`}>
      <polyline
        fill="none"
        stroke={color}
        strokeWidth={2}
        points={points.join(" ")}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function FloorDashboard() {
  const [stats, setStats] = useState<FloorStats>(DEMO_STATS);
  const [feed, setFeed] = useState<ArbIntent[]>(DEMO_FEED);
  const [volumeSeries, setVolumeSeries] = useState<number[]>([
    84_291_003.21, 82_100_000, 80_500_000, 83_000_000, 84_291_003.21,
  ]);
  const [successSeries, setSuccessSeries] = useState<number[]>([
    92, 93, 94, 94.5, 94.7,
  ]);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  const [helpOpen, setHelpOpen] = useState(false);

  // Fetch live data when available; if not, simulate
  useEffect(() => {
    let mounted = true;

    async function fetchLive() {
      setLoading(true);
      try {
        const s = await getFloorStats();
        const r = await getIntents(1, 10);
        if (!mounted) return;
        setStats(s);
        setFeed(r.items.slice(0, 20));
        setVolumeSeries((v) => [
          ...v.slice(-4),
          Number(s.totalVolume.replace(/[,]/g, "")),
        ]);
        setSuccessSeries((srs) => [...srs.slice(-4), s.avgSuccessRate]);
      } catch (e) {
        // No backend — keep demo data, but raise the integrity flag.
        dataIntegrity.reportDemoFallback(
          "FloorDashboard",
          e instanceof Error ? e.message : String(e),
        );
      } finally {
        setLoading(false);
      }
    }

    if (autoRefresh) {
      fetchLive().finally(() => setInitialLoading(false));
    }

    const interval = setInterval(() => {
      if (autoRefresh) {
        fetchLive();

        // Simulate new intent for lively demo
        setFeed((prev) => {
          const id = `0x${Math.random().toString(16).slice(2, 8)}`;
          const newIntent: ArbIntent = {
            id,
            agentId: `agent-${Math.random().toString(36).slice(2, 7)}`,
            state: Math.random() > 0.85 ? IntentState.Slashed : IntentState.Executing,
            legs: [
              {
                chain: "ETH",
                protocol: "UniV3",
                tokenIn: "WETH",
                tokenOut: "USDC",
                amountIn: "1.0",
                expectedOut: "1842.10",
              },
            ],
            feeCap: 10,
            feeActual: null,
            createdAt: Date.now(),
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

  // WebSocket live updates (optional backend push)
  const WS_URL = import.meta.env.VITE_X3_WS || import.meta.env.VITE_RPC_WS || "wss://ws.x3star.net/ws";
  useWebSocket(WS_URL, (msg: any) => {
    if (!msg || !msg.type) return;
    if (msg.type === "intent:new") {
      setFeed((prev) => [msg.payload, ...prev].slice(0, 20));
    }
    if (msg.type === "stats:update") {
      setStats((s) => ({ ...s, ...msg.payload }));
      if (msg.payload.totalVolume) {
        setVolumeSeries((v) => [...v.slice(-4), Number(msg.payload.totalVolume)]);
      }
    }
  });

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
    <div className="page">
      {initialLoading && (
        <div className="loading-overlay">
          <Loading size="lg" />
        </div>
      )}

      <div className="page-header">
        <h1>X3 Floor</h1>
        <span className="subtitle">Arbitrage jurisdiction — live</span>

        <div style={{ marginLeft: 'auto', display: 'flex', gap: 8, alignItems: 'center' }}>
          <Button
            variant={autoRefresh ? "success" : "secondary"}
            size="sm"
            onClick={() => setAutoRefresh(!autoRefresh)}
            loading={loading}
          >
            {autoRefresh ? "🔄 Live" : "⏸ Paused"}
          </Button>

          <Button variant="secondary" size="sm" onClick={() => setHelpOpen(true)}>❓ Help</Button>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-label">Active Agents</div>
          <div className="stat-value green">{stats.activeAgents}</div>
        </div>

        <div className="stat-card">
          <div className="stat-label">Total Intents</div>
          <div className="stat-value">{stats.totalIntents.toLocaleString()}</div>
        </div>

        <div className="stat-card">
          <div className="stat-label">Volume (USDC)</div>
          <div className="stat-value">{volRounded}</div>
        </div>

        <div className="stat-card">
          <div className="stat-label">Success Rate</div>
          <div className="stat-value green">{stats.avgSuccessRate}%</div>
          <div style={{ margin: "8px 0" }}>
            <ProgressBar
              value={stats.avgSuccessRate}
              max={100}
              color="green"
            />
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-label">Total Slashes</div>
          <div className="stat-value red">{stats.totalSlashes}</div>
        </div>

        <div className="stat-card">
          <div className="stat-label">Disputes</div>
          <div className="stat-value amber">{stats.totalDisputes}</div>
        </div>

        <div className="stat-card">
          <div className="stat-label">Active Flashloans</div>
          <div className="stat-value blue">{stats.activeFlashloans}</div>
        </div>
      </div>

      {/* Charts Section */}
      <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: 16, marginBottom: 24 }}>
        <div className="card">
          <div className="card-header">
            <h2>Volume & Success Trend</h2>
            <div style={{ marginLeft: 'auto' }}>
              <Button size="sm" variant="secondary" onClick={() => setVolumeSeries((v)=>[...v, v[v.length-1]])}>+ Expand</Button>
            </div>
          </div>
          <VolumeTrendChart data={trendData} />
        </div>

        <div className="card">
          <div className="card-header">
            <h2>Intent State Distribution</h2>
          </div>
          <IntentStatePie data={stateDistribution} />
        </div>
      </div>

      {/* Live Execution Feed */}
      <div className="card">
        <div className="card-header">
          <h2>Execution Feed</h2>
          <span className="secondary mono" style={{ fontSize: 11 }}>
            LIVE • {feed.length} items
          </span>
        </div>
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>Intent</th>
                <th>Agent</th>
                <th>State</th>
                <th>Route</th>
                <th>Fee</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {feed.map((intent) => (
                <tr key={intent.id}>
                  <td className="mono hash">{intent.id}</td>
                  <td className="mono" style={{ fontSize: 12 }}>{intent.agentId}</td>
                  <td>
                    <Badge variant={stateColor(intent.state) as any}>
                      {intent.state}
                    </Badge>
                  </td>
                  <td style={{ fontSize: 12 }}>
                    {intent.legs.map((leg, i) => (
                      <span key={i}>
                        {i > 0 && " → "}
                        <span className="secondary">{leg.chain}</span>:
                        {leg.tokenIn}→{leg.tokenOut}
                      </span>
                    ))}
                  </td>
                  <td className="mono">
                    {intent.feeActual !== null ? (
                      <span className="green">{intent.feeActual.toFixed(1)}</span>
                    ) : (
                      <span className="muted">—</span>
                    )}
                    <span className="muted"> / {intent.feeCap.toFixed(1)}</span>
                  </td>
                  <td className="secondary" style={{ fontSize: 12 }}>
                    {timeSince(intent.createdAt)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
      {initialLoading && (
        <div style={{ position: 'absolute', top: 0, left: 0, right: 0, bottom: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', pointerEvents: 'none' }}>
          <div style={{ pointerEvents: 'auto' }}>
            <Loading />
          </div>
        </div>
      )}

      <HelpModal open={helpOpen} onClose={() => setHelpOpen(false)} />
    </div>
  );
}
