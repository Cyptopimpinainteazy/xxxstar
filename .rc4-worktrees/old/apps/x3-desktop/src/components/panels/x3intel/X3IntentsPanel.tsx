import React, { useState } from 'react';
import {
  ResponsiveContainer, PieChart, Pie, Cell, Tooltip,
} from 'recharts';

type IntentState = 'Submitted' | 'RouteBound' | 'Executing' | 'Executed' | 'Finalized' | 'Slashed' | 'Cancelled' | 'Expired';

interface Leg { chain: string; protocol: string; tokenIn: string; tokenOut: string; amountIn: string; amountOut: string }
interface Intent {
  id: string; agent: string; state: IntentState; legs: Leg[];
  feeCap: string; feeActual: string; proofHash: string; created: string;
}

const stateColor: Record<IntentState, string> = {
  Submitted: '#4488ff', RouteBound: '#4488ff', Executing: '#ffaa00', Executed: '#00d4aa',
  Finalized: '#00d4aa', Slashed: '#ff4444', Cancelled: '#666', Expired: '#888',
};

const pieData = [
  { name: 'Finalized', value: 14502, color: '#00d4aa' },
  { name: 'Executing', value: 234, color: '#ffaa00' },
  { name: 'Submitted', value: 588, color: '#4488ff' },
  { name: 'Slashed', value: 23, color: '#ff4444' },
  { name: 'Expired', value: 300, color: '#888' },
  { name: 'Cancelled', value: 200, color: '#666' },
];

const mkLegs = (n: number): Leg[] => Array.from({ length: n }, (_, i) => ({
  chain: i % 2 === 0 ? 'Ethereum' : 'Solana',
  protocol: i % 2 === 0 ? 'Uniswap V3' : 'Raydium',
  tokenIn: i === 0 ? 'USDC' : 'WETH',
  tokenOut: i === 0 ? 'WETH' : 'SOL',
  amountIn: (1000 + i * 500).toLocaleString(),
  amountOut: (0.5 + i * 0.25).toFixed(4),
}));

const intents: Intent[] = [
  { id: '0xab12…f3e1', agent: 'agent-alpha',   state: 'Finalized',  legs: mkLegs(3), feeCap: '12.5 USDC',  feeActual: '8.2 USDC',  proofHash: '0xproof1…aa', created: '2026-02-10 09:12' },
  { id: '0xcd34…a7b2', agent: 'agent-beta',    state: 'Executing',  legs: mkLegs(2), feeCap: '8.0 USDC',   feeActual: '—',         proofHash: '—',            created: '2026-02-10 09:14' },
  { id: '0xef56…c9d3', agent: 'agent-gamma',   state: 'Submitted',  legs: mkLegs(4), feeCap: '20.0 USDC',  feeActual: '—',         proofHash: '—',            created: '2026-02-10 09:15' },
  { id: '0x1a78…e1f4', agent: 'agent-delta',   state: 'Slashed',    legs: mkLegs(2), feeCap: '15.0 USDC',  feeActual: '15.0 USDC', proofHash: '0xproof4…dd', created: '2026-02-10 08:44' },
  { id: '0x2b89…f2a5', agent: 'agent-alpha',   state: 'Finalized',  legs: mkLegs(3), feeCap: '11.2 USDC',  feeActual: '7.8 USDC',  proofHash: '0xproof5…ee', created: '2026-02-10 08:30' },
  { id: '0x3c9a…a3b6', agent: 'agent-epsilon', state: 'Expired',    legs: mkLegs(5), feeCap: '25.0 USDC',  feeActual: '—',         proofHash: '—',            created: '2026-02-10 07:55' },
  { id: '0x4da1…b4c7', agent: 'agent-beta',    state: 'Cancelled',  legs: mkLegs(1), feeCap: '5.0 USDC',   feeActual: '—',         proofHash: '—',            created: '2026-02-10 07:20' },
  { id: '0x5eb2…c5d8', agent: 'agent-gamma',   state: 'Executed',   legs: mkLegs(3), feeCap: '18.0 USDC',  feeActual: '12.1 USDC', proofHash: '0xproof8…hh', created: '2026-02-10 06:58' },
  { id: '0x6fc3…d6e9', agent: 'agent-zeta',    state: 'Finalized',  legs: mkLegs(2), feeCap: '9.5 USDC',   feeActual: '6.3 USDC',  proofHash: '0xproof9…ii', created: '2026-02-10 06:30' },
  { id: '0x70d4…e7fa', agent: 'agent-theta',   state: 'RouteBound', legs: mkLegs(3), feeCap: '14.0 USDC',  feeActual: '—',         proofHash: '—',            created: '2026-02-10 06:10' },
];

const statCards = [
  { label: 'Total Intents', value: '15,847' },
  { label: 'Active', value: '234' },
  { label: 'Finalized', value: '14,502' },
  { label: 'Slashed', value: '23' },
];

const tooltipStyle = { backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' };
const filterTabs: (IntentState | 'All')[] = ['All', 'Submitted', 'Executing', 'Finalized', 'Slashed'];

export default function X3IntentsPanel() {
  const [tab, setTab] = useState<IntentState | 'All'>('All');
  const [expanded, setExpanded] = useState<string | null>(null);

  const filtered = tab === 'All' ? intents : intents.filter(i => i.state === tab);

  return (
    <div className="min-h-full bg-[#0a0a0f] text-gray-300 p-6 space-y-6 overflow-auto">
      <h1 className="text-xl font-semibold text-white">Intent Explorer</h1>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-3">
        {statCards.map(s => (
          <div key={s.label} className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
            <p className="text-xs text-gray-500 uppercase tracking-wide">{s.label}</p>
            <p className="text-2xl font-mono text-white mt-1">{s.value}</p>
          </div>
        ))}
      </div>

      {/* Pie + Filters */}
      <div className="grid grid-cols-3 gap-4">
        <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
          <p className="text-sm text-gray-400 mb-2">State Distribution</p>
          <ResponsiveContainer width="100%" height={180}>
            <PieChart>
              <Pie data={pieData} dataKey="value" nameKey="name" cx="50%" cy="50%" outerRadius={65} strokeWidth={0}>
                {pieData.map((d, i) => <Cell key={i} fill={d.color} />)}
              </Pie>
              <Tooltip contentStyle={tooltipStyle} />
            </PieChart>
          </ResponsiveContainer>
          <div className="flex flex-wrap gap-x-3 gap-y-1 mt-1 justify-center">
            {pieData.map(d => (
              <span key={d.name} className="flex items-center gap-1 text-xs text-gray-400">
                <span className="w-2 h-2 rounded-full inline-block" style={{ background: d.color }} /> {d.name}
              </span>
            ))}
          </div>
        </div>

        <div className="col-span-2 flex flex-col gap-3">
          <div className="flex gap-2">
            {filterTabs.map(t => (
              <button key={t} onClick={() => setTab(t)}
                className={`px-3 py-1 rounded text-xs border transition ${tab === t ? 'border-[#00d4aa] text-[#00d4aa] bg-[#00d4aa11]' : 'border-[#222] text-gray-500 hover:text-gray-300'}`}>
                {t}
              </button>
            ))}
          </div>
          <p className="text-xs text-gray-500">{filtered.length} intent{filtered.length !== 1 ? 's' : ''} shown</p>
        </div>
      </div>

      {/* Intent Table */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[#222] text-gray-500 text-xs uppercase">
              <th className="text-left p-3 w-6"></th>
              <th className="text-left p-3">ID</th><th className="text-left p-3">Agent</th>
              <th className="text-left p-3">State</th><th className="text-right p-3">Legs</th>
              <th className="text-right p-3">Fee Cap</th><th className="text-right p-3">Fee Actual</th>
              <th className="text-right p-3">Proof Hash</th><th className="text-right p-3">Created</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map(i => (
              <React.Fragment key={i.id}>
                <tr className="border-b border-[#1a1a1a] hover:bg-[#ffffff04] cursor-pointer"
                  onClick={() => setExpanded(expanded === i.id ? null : i.id)}>
                  <td className="p-3 text-gray-500 text-xs">{expanded === i.id ? '▾' : '▸'}</td>
                  <td className="p-3 font-mono text-xs">{i.id}</td>
                  <td className="p-3">{i.agent}</td>
                  <td className="p-3">
                    <span className="px-2 py-0.5 rounded text-xs font-medium" style={{ background: stateColor[i.state] + '22', color: stateColor[i.state] }}>
                      {i.state}
                    </span>
                  </td>
                  <td className="p-3 text-right font-mono">{i.legs.length}</td>
                  <td className="p-3 text-right font-mono">{i.feeCap}</td>
                  <td className="p-3 text-right font-mono">{i.feeActual}</td>
                  <td className="p-3 text-right font-mono text-xs text-gray-500">{i.proofHash}</td>
                  <td className="p-3 text-right text-gray-500 font-mono text-xs">{i.created}</td>
                </tr>
                {expanded === i.id && (
                  <tr>
                    <td colSpan={9} className="bg-[#0d0d12] px-8 py-3 border-b border-[#1a1a1a]">
                      <p className="text-xs text-gray-500 uppercase mb-2">Leg Details</p>
                      <div className="grid grid-cols-3 gap-2">
                        {i.legs.map((l, li) => (
                          <div key={li} className="bg-[#111116] border border-[#222] rounded p-3 text-xs space-y-1">
                            <p className="text-gray-400"><span className="text-gray-500">Chain:</span> {l.chain}</p>
                            <p className="text-gray-400"><span className="text-gray-500">Protocol:</span> {l.protocol}</p>
                            <p className="font-mono text-gray-300">{l.tokenIn} → {l.tokenOut}</p>
                            <p className="font-mono text-gray-500">{l.amountIn} → {l.amountOut}</p>
                          </div>
                        ))}
                      </div>
                    </td>
                  </tr>
                )}
              </React.Fragment>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
