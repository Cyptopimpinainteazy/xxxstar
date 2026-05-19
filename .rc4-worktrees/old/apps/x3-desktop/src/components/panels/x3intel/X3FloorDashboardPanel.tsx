import { useState } from 'react';
import {
  ResponsiveContainer, AreaChart, Area, PieChart, Pie, Cell,
  XAxis, YAxis, CartesianGrid, Tooltip,
} from 'recharts';

enum IntentState {
  Submitted = 'Submitted',
  RouteBound = 'RouteBound',
  Executing = 'Executing',
  Executed = 'Executed',
  Finalized = 'Finalized',
  Slashed = 'Slashed',
  Cancelled = 'Cancelled',
  Expired = 'Expired',
}

interface StatCard { label: string; value: string; sub?: string }
interface VolumePoint { time: string; volume: number }
interface PieSlice { name: string; value: number; color: string }
interface Intent {
  id: string; agent: string; state: IntentState;
  legs: number; feeCap: string; created: string;
}

const stateColor: Record<IntentState, string> = {
  [IntentState.Submitted]: '#4488ff',
  [IntentState.RouteBound]: '#4488ff',
  [IntentState.Executing]: '#ffaa00',
  [IntentState.Executed]: '#00d4aa',
  [IntentState.Finalized]: '#00d4aa',
  [IntentState.Slashed]: '#ff4444',
  [IntentState.Cancelled]: '#666',
  [IntentState.Expired]: '#888',
};

const stats: StatCard[] = [
  { label: 'Active Agents', value: '42' },
  { label: 'Total Intents', value: '15,847' },
  { label: 'Volume', value: '$2.4M', sub: '24h' },
  { label: 'Slashes', value: '23' },
  { label: 'Avg Success Rate', value: '94.7%' },
];

const volumeData: VolumePoint[] = [
  { time: '00:00', volume: 120000 }, { time: '02:00', volume: 185000 },
  { time: '04:00', volume: 95000 },  { time: '06:00', volume: 210000 },
  { time: '08:00', volume: 310000 }, { time: '10:00', volume: 275000 },
  { time: '12:00', volume: 390000 }, { time: '14:00', volume: 420000 },
  { time: '16:00', volume: 355000 }, { time: '18:00', volume: 290000 },
  { time: '20:00', volume: 340000 }, { time: '22:00', volume: 260000 },
];

const pieData: PieSlice[] = [
  { name: 'Finalized', value: 14502, color: '#00d4aa' },
  { name: 'Executing', value: 234, color: '#ffaa00' },
  { name: 'Expired', value: 788, color: '#888' },
  { name: 'Slashed', value: 23, color: '#ff4444' },
  { name: 'Cancelled', value: 300, color: '#666' },
];

const intents: Intent[] = [
  { id: '0xab12…f3e1', agent: 'agent-alpha', state: IntentState.Finalized, legs: 3, feeCap: '12.5 USDC', created: '2026-02-10 09:12' },
  { id: '0xcd34…a7b2', agent: 'agent-beta',  state: IntentState.Executing, legs: 2, feeCap: '8.0 USDC',  created: '2026-02-10 09:14' },
  { id: '0xef56…c9d3', agent: 'agent-gamma', state: IntentState.Submitted, legs: 4, feeCap: '20.0 USDC', created: '2026-02-10 09:15' },
  { id: '0x1a78…e1f4', agent: 'agent-delta', state: IntentState.Slashed,   legs: 2, feeCap: '15.0 USDC', created: '2026-02-10 08:44' },
  { id: '0x2b89…f2a5', agent: 'agent-alpha', state: IntentState.Finalized, legs: 3, feeCap: '11.2 USDC', created: '2026-02-10 08:30' },
  { id: '0x3c9a…a3b6', agent: 'agent-epsilon', state: IntentState.Expired, legs: 5, feeCap: '25.0 USDC', created: '2026-02-10 07:55' },
  { id: '0x4da1…b4c7', agent: 'agent-beta',  state: IntentState.Cancelled, legs: 1, feeCap: '5.0 USDC',  created: '2026-02-10 07:20' },
  { id: '0x5eb2…c5d8', agent: 'agent-gamma', state: IntentState.Executed,  legs: 3, feeCap: '18.0 USDC', created: '2026-02-10 06:58' },
];

const tooltipStyle = { backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' };

export default function X3FloorDashboardPanel() {
  const [filter, setFilter] = useState<IntentState | 'All'>('All');
  const filtered = filter === 'All' ? intents : intents.filter(i => i.state === filter);

  return (
    <div className="min-h-full bg-[#0a0a0f] text-gray-300 p-6 space-y-6 overflow-auto">
      <h1 className="text-xl font-semibold text-white">Trading Floor</h1>

      {/* Stats Grid */}
      <div className="grid grid-cols-5 gap-3">
        {stats.map(s => (
          <div key={s.label} className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
            <p className="text-xs text-gray-500 uppercase tracking-wide">{s.label}</p>
            <p className="text-2xl font-mono text-white mt-1">{s.value}
              {s.sub && <span className="text-xs text-gray-500 ml-1">{s.sub}</span>}
            </p>
          </div>
        ))}
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-3 gap-4">
        <div className="col-span-2 bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
          <p className="text-sm text-gray-400 mb-3">Volume Trend (24h)</p>
          <ResponsiveContainer width="100%" height={200}>
            <AreaChart data={volumeData}>
              <defs>
                <linearGradient id="volGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#00d4aa" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#00d4aa" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
              <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} />
              <YAxis stroke="#666" tick={{ fontSize: 11 }} tickFormatter={v => `$${(v / 1000).toFixed(0)}k`} />
              <Tooltip contentStyle={tooltipStyle} formatter={(v: any) => [`$${Number(v).toLocaleString()}`, 'Volume']} />
              <Area type="monotone" dataKey="volume" stroke="#00d4aa" fill="url(#volGrad)" strokeWidth={2} />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
          <p className="text-sm text-gray-400 mb-3">Intent Distribution</p>
          <ResponsiveContainer width="100%" height={200}>
            <PieChart>
              <Pie data={pieData} dataKey="value" nameKey="name" cx="50%" cy="50%" outerRadius={70} strokeWidth={0}>
                {pieData.map((d, i) => <Cell key={i} fill={d.color} />)}
              </Pie>
              <Tooltip contentStyle={tooltipStyle} />
            </PieChart>
          </ResponsiveContainer>
          <div className="flex flex-wrap gap-x-3 gap-y-1 mt-2 justify-center">
            {pieData.map(d => (
              <span key={d.name} className="flex items-center gap-1 text-xs text-gray-400">
                <span className="w-2 h-2 rounded-full inline-block" style={{ background: d.color }} />
                {d.name}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* Filter Tabs */}
      <div className="flex gap-2">
        {['All', IntentState.Submitted, IntentState.Executing, IntentState.Finalized, IntentState.Slashed].map(t => (
          <button key={t} onClick={() => setFilter(t as any)}
            className={`px-3 py-1 rounded text-xs border transition ${filter === t ? 'border-[#00d4aa] text-[#00d4aa] bg-[#00d4aa11]' : 'border-[#222] text-gray-500 hover:text-gray-300'}`}>
            {t}
          </button>
        ))}
      </div>

      {/* Intents Table */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[#222] text-gray-500 text-xs uppercase">
              <th className="text-left p-3">ID</th><th className="text-left p-3">Agent</th>
              <th className="text-left p-3">State</th><th className="text-right p-3">Legs</th>
              <th className="text-right p-3">Fee Cap</th><th className="text-right p-3">Created</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map(i => (
              <tr key={i.id} className="border-b border-[#1a1a1a] hover:bg-[#ffffff04]">
                <td className="p-3 font-mono text-xs">{i.id}</td>
                <td className="p-3">{i.agent}</td>
                <td className="p-3">
                  <span className="px-2 py-0.5 rounded text-xs font-medium" style={{ background: stateColor[i.state] + '22', color: stateColor[i.state] }}>
                    {i.state}
                  </span>
                </td>
                <td className="p-3 text-right font-mono">{i.legs}</td>
                <td className="p-3 text-right font-mono">{i.feeCap}</td>
                <td className="p-3 text-right text-gray-500 font-mono text-xs">{i.created}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
