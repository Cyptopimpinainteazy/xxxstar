import {
  ResponsiveContainer, BarChart, Bar, Cell, XAxis, YAxis, CartesianGrid, Tooltip,
} from 'recharts';

type Severity = 'Minor' | 'Moderate' | 'Major' | 'Critical';

interface SlashEvent {
  id: string; agent: string; severity: Severity;
  reason: string; amount: string; proofHash: string; timestamp: string;
}

const severityColor: Record<Severity, string> = {
  Minor: '#ffaa00',
  Moderate: '#ff8800',
  Major: '#ff4444',
  Critical: '#cc0000',
};

const severityChart = [
  { severity: 'Minor',    count: 12, fill: '#ffaa00' },
  { severity: 'Moderate', count: 6,  fill: '#ff8800' },
  { severity: 'Major',    count: 4,  fill: '#ff4444' },
  { severity: 'Critical', count: 1,  fill: '#cc0000' },
];

const slashEvents: SlashEvent[] = [
  { id: 'sl-001', agent: 'agent-delta',   severity: 'Major',    reason: 'Missed execution deadline',      amount: '3,000 USDC', proofHash: '0xslash1…aa', timestamp: '2026-02-10 08:44' },
  { id: 'sl-002', agent: 'agent-eta',     severity: 'Critical', reason: 'Invalid proof submitted',        amount: '12,000 USDC', proofHash: '0xslash2…bb', timestamp: '2026-02-09 22:10' },
  { id: 'sl-003', agent: 'agent-delta',   severity: 'Minor',    reason: 'Partial fill below threshold',   amount: '750 USDC',   proofHash: '0xslash3…cc', timestamp: '2026-02-09 18:33' },
  { id: 'sl-004', agent: 'agent-eta',     severity: 'Moderate', reason: 'Route deviation exceeded 5%',    amount: '2,000 USDC', proofHash: '0xslash4…dd', timestamp: '2026-02-09 14:05' },
  { id: 'sl-005', agent: 'agent-theta',   severity: 'Minor',    reason: 'Stale oracle price reference',   amount: '500 USDC',   proofHash: '0xslash5…ee', timestamp: '2026-02-08 20:12' },
  { id: 'sl-006', agent: 'agent-delta',   severity: 'Major',    reason: 'Double-spend on Solana leg',     amount: '8,000 USDC', proofHash: '0xslash6…ff', timestamp: '2026-02-08 11:45' },
  { id: 'sl-007', agent: 'agent-eta',     severity: 'Moderate', reason: 'Timeout on verification step',   amount: '1,500 USDC', proofHash: '0xslash7…gg', timestamp: '2026-02-07 09:22' },
  { id: 'sl-008', agent: 'agent-beta',    severity: 'Minor',    reason: 'Gas estimation exceeded 120%',   amount: '250 USDC',   proofHash: '0xslash8…hh', timestamp: '2026-02-07 06:55' },
];

const summaryStats = [
  { label: 'Total Slashes', value: '23' },
  { label: 'Total Amount Slashed', value: '$45,000' },
  { label: 'Minor', value: '12' },
  { label: 'Moderate', value: '6' },
  { label: 'Major', value: '4' },
  { label: 'Critical', value: '1' },
];

const tooltipStyle = { backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' };

export default function X3SlashingPanel() {
  return (
    <div className="min-h-full bg-[#0a0a0f] text-gray-300 p-6 space-y-6 overflow-auto">
      <h1 className="text-xl font-semibold text-white">Slashing Events</h1>

      {/* Summary Stats */}
      <div className="grid grid-cols-6 gap-3">
        {summaryStats.map(s => (
          <div key={s.label} className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
            <p className="text-xs text-gray-500 uppercase tracking-wide">{s.label}</p>
            <p className="text-xl font-mono text-white mt-1">{s.value}</p>
          </div>
        ))}
      </div>

      {/* Severity Distribution */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
        <p className="text-sm text-gray-400 mb-3">Severity Distribution</p>
        <ResponsiveContainer width="100%" height={200}>
          <BarChart data={severityChart}>
            <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
            <XAxis dataKey="severity" stroke="#666" tick={{ fontSize: 11 }} />
            <YAxis stroke="#666" tick={{ fontSize: 11 }} />
            <Tooltip contentStyle={tooltipStyle} />
            <Bar dataKey="count" radius={[4, 4, 0, 0]}>
              {severityChart.map((entry, i) => (
                <Cell key={i} fill={entry.fill} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* Events Table */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[#222] text-gray-500 text-xs uppercase">
              <th className="text-left p-3">ID</th><th className="text-left p-3">Agent</th>
              <th className="text-left p-3">Severity</th><th className="text-left p-3">Reason</th>
              <th className="text-right p-3">Amount</th><th className="text-right p-3">Proof Hash</th>
              <th className="text-right p-3">Timestamp</th>
            </tr>
          </thead>
          <tbody>
            {slashEvents.map(e => (
              <tr key={e.id} className="border-b border-[#1a1a1a] hover:bg-[#ffffff04]">
                <td className="p-3 font-mono text-xs">{e.id}</td>
                <td className="p-3">{e.agent}</td>
                <td className="p-3">
                  <span className="px-2 py-0.5 rounded text-xs font-medium"
                    style={{ background: severityColor[e.severity] + '22', color: severityColor[e.severity] }}>
                    {e.severity}
                  </span>
                </td>
                <td className="p-3 text-gray-400">{e.reason}</td>
                <td className="p-3 text-right font-mono">{e.amount}</td>
                <td className="p-3 text-right font-mono text-xs text-gray-500">{e.proofHash}</td>
                <td className="p-3 text-right text-gray-500 font-mono text-xs">{e.timestamp}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
