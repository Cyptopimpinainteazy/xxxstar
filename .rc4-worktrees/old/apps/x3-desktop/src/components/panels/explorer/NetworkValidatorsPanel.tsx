import React, { useState } from 'react';
import { Shield, Server, ArrowUpDown, Globe, CheckCircle, AlertCircle, DollarSign } from 'lucide-react';

type Tab = 'validators' | 'rpc' | 'ramps';

const validators = [
  { name: 'X3 Foundation', address: '5GrwvaEF5zXb26Fz9rcQpDWS57C', staked: 8200000, commission: 3, uptime: 99.98, status: 'active' },
  { name: 'StakeNode Pro', address: '5FHneW46xGXgs5mUiveU4sbT', staked: 6500000, commission: 5, uptime: 99.95, status: 'active' },
  { name: 'Chorus One', address: '5DAAnrj7VHTznn2AWBemMuyB', staked: 5800000, commission: 4, uptime: 99.90, status: 'active' },
  { name: 'Figment', address: '5HGjWAeFDfFCWPsjFQdVV2M', staked: 4200000, commission: 6, uptime: 99.85, status: 'active' },
  { name: 'P2P Validator', address: '5CiPPseXPECbkjWCa6MnjNok', staked: 3900000, commission: 5, uptime: 99.92, status: 'active' },
  { name: 'Everstake', address: '5HYYeCa1Hae5YYGJ2pHskH', staked: 3500000, commission: 7, uptime: 99.80, status: 'active' },
  { name: 'Blockdaemon', address: '5Ck5SLSHYac6WFt5EYERp', staked: 3200000, commission: 4, uptime: 99.88, status: 'active' },
  { name: 'Bison Trails', address: '5GNJqTPyNqANBkUVMN1LPP', staked: 2800000, commission: 5, uptime: 99.75, status: 'active' },
  { name: 'Infstones', address: '5FLSigC9HGRKVhB9FiEo4Y', staked: 2500000, commission: 8, uptime: 99.70, status: 'active' },
  { name: 'Allnodes', address: '5CRmqmsiNFExV6VtdIhcsFG', staked: 2100000, commission: 6, uptime: 99.65, status: 'active' },
  { name: 'NodeFactory', address: '5EYCAe2iJMFd5R3wx6cjgi', staked: 1600000, commission: 10, uptime: 98.50, status: 'waiting' },
  { name: 'SphereStake', address: '5HpG9w8EBLe5XCrbczpwq5', staked: 900000, commission: 12, uptime: 97.20, status: 'waiting' },
];

const rpcProviders = [
  { name: 'X3 Public RPC', endpoint: 'https://rpc.x3.network', latency: 45, requestsDay: 2400000, status: 'healthy' },
  { name: 'Alchemy X3', endpoint: 'https://x3.g.alchemy.com/v2/...', latency: 32, requestsDay: 8500000, status: 'healthy' },
  { name: 'Infura X3', endpoint: 'https://x3.infura.io/v3/...', latency: 38, requestsDay: 6200000, status: 'healthy' },
  { name: 'QuickNode', endpoint: 'https://x3.quiknode.pro/...', latency: 28, requestsDay: 4100000, status: 'healthy' },
  { name: 'Ankr', endpoint: 'https://rpc.ankr.com/x3', latency: 55, requestsDay: 1800000, status: 'degraded' },
  { name: 'BlastAPI', endpoint: 'https://x3.blastapi.io/...', latency: 42, requestsDay: 950000, status: 'healthy' },
];

const rampProviders = [
  { name: 'MoonPay', currencies: ['USD', 'EUR', 'GBP'], fees: '1.5-3.5%', minLimit: '$30', maxLimit: '$50,000', supports: ['on', 'off'] as string[] },
  { name: 'Transak', currencies: ['USD', 'EUR', 'INR', 'BRL'], fees: '1.0-5.0%', minLimit: '$15', maxLimit: '$25,000', supports: ['on', 'off'] as string[] },
  { name: 'Ramp Network', currencies: ['USD', 'EUR', 'GBP', 'PLN'], fees: '0.5-2.9%', minLimit: '$5', maxLimit: '$10,000', supports: ['on'] as string[] },
  { name: 'Banxa', currencies: ['USD', 'AUD', 'EUR'], fees: '1.0-3.0%', minLimit: '$20', maxLimit: '$100,000', supports: ['on', 'off'] as string[] },
];

const NetworkValidatorsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<Tab>('validators');
  const [rampType, setRampType] = useState<'on' | 'off'>('on');

  const totalStaked = validators.reduce((s, v) => s + v.staked, 0);
  const avgCommission = Math.round(validators.reduce((s, v) => s + v.commission, 0) / validators.length);
  const activeCount = validators.filter(v => v.status === 'active').length;

  const tabs: { key: Tab; label: string; icon: React.ReactNode }[] = [
    { key: 'validators', label: 'Validators', icon: <Shield size={14} /> },
    { key: 'rpc', label: 'RPC Providers', icon: <Server size={14} /> },
    { key: 'ramps', label: 'On/Off Ramps', icon: <ArrowUpDown size={14} /> },
  ];

  return (
    <div className="flex flex-col h-full bg-[#0a0a0f] text-gray-300">
      <div className="flex items-center gap-4 px-5 py-3 border-b border-[#1a1a1a]">
        <Globe size={18} className="text-[#ff6b35]" />
        <h1 className="text-lg font-semibold text-white">Network</h1>
        <div className="flex gap-1 ml-4">
          {tabs.map(t => (
            <button key={t.key} onClick={() => setActiveTab(t.key)}
              className={`flex items-center gap-1.5 px-3 py-1.5 text-sm rounded transition-colors ${activeTab === t.key ? 'bg-[#ff6b35]/10 text-[#ff6b35]' : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}`}>
              {t.icon} {t.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-5">
        {activeTab === 'validators' && (
          <div>
            <div className="grid grid-cols-3 gap-3 mb-5">
              {[{ label: 'Active Validators', value: activeCount, sub: `${validators.length} total` },
                { label: 'Total Staked', value: `$${(totalStaked / 1e6).toFixed(1)}M`, sub: 'X3' },
                { label: 'Avg Commission', value: `${avgCommission}%`, sub: 'across all validators' }
              ].map((s, i) => (
                <div key={i} className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                  <p className="text-xs text-gray-500">{s.label}</p>
                  <p className="text-xl font-bold text-white mt-1">{s.value}</p>
                  <p className="text-xs text-gray-500 mt-0.5">{s.sub}</p>
                </div>
              ))}
            </div>
            <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-[#1a1a1a] text-gray-500 text-xs uppercase tracking-wider">
                    <th className="text-left px-4 py-2.5">Validator</th>
                    <th className="text-left px-4 py-2.5">Address</th>
                    <th className="text-right px-4 py-2.5">Staked</th>
                    <th className="text-right px-4 py-2.5">Commission</th>
                    <th className="text-right px-4 py-2.5">Uptime</th>
                    <th className="text-center px-4 py-2.5">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {validators.map((v, i) => (
                    <tr key={i} className="border-b border-[#1a1a1a]/50 hover:bg-white/[0.02]">
                      <td className="px-4 py-2 text-white font-medium">{v.name}</td>
                      <td className="px-4 py-2 font-mono text-xs text-gray-500">{v.address.slice(0, 18)}...</td>
                      <td className="px-4 py-2 text-right">${(v.staked / 1e6).toFixed(2)}M</td>
                      <td className="px-4 py-2 text-right">{v.commission}%</td>
                      <td className="px-4 py-2 text-right">{v.uptime}%</td>
                      <td className="px-4 py-2 text-center">
                        <span className={`inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full ${v.status === 'active' ? 'bg-green-500/10 text-green-400' : 'bg-yellow-500/10 text-yellow-400'}`}>
                          {v.status === 'active' ? <CheckCircle size={10} /> : <AlertCircle size={10} />}
                          {v.status}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {activeTab === 'rpc' && (
          <div className="grid grid-cols-2 gap-3">
            {rpcProviders.map((p, i) => (
              <div key={i} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <h3 className="text-sm font-semibold text-white">{p.name}</h3>
                  <span className={`inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full ${p.status === 'healthy' ? 'bg-green-500/10 text-green-400' : 'bg-yellow-500/10 text-yellow-400'}`}>
                    {p.status === 'healthy' ? <CheckCircle size={10} /> : <AlertCircle size={10} />}
                    {p.status}
                  </span>
                </div>
                <p className="text-xs font-mono text-gray-500 truncate mb-3">{p.endpoint}</p>
                <div className="flex items-center justify-between text-xs text-gray-400">
                  <span>Latency: <span className="text-white">{p.latency}ms</span></span>
                  <span>Requests/day: <span className="text-white">{(p.requestsDay / 1e6).toFixed(1)}M</span></span>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'ramps' && (
          <div>
            <div className="flex items-center gap-2 mb-4">
              {(['on', 'off'] as const).map(t => (
                <button key={t} onClick={() => setRampType(t)}
                  className={`px-4 py-1.5 text-sm rounded transition-colors ${rampType === t ? 'bg-[#ff6b35]/10 text-[#ff6b35] border border-[#ff6b35]/30' : 'text-gray-400 border border-[#1a1a1a] hover:border-gray-600'}`}>
                  {t === 'on' ? 'On-Ramp (Fiat → Crypto)' : 'Off-Ramp (Crypto → Fiat)'}
                </button>
              ))}
            </div>
            <div className="grid grid-cols-2 gap-3">
              {rampProviders.filter(p => p.supports.includes(rampType)).map((p, i) => (
                <div key={i} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                  <div className="flex items-center gap-2 mb-3">
                    <DollarSign size={16} className="text-[#ff6b35]" />
                    <h3 className="text-sm font-semibold text-white">{p.name}</h3>
                  </div>
                  <div className="space-y-2 text-xs">
                    <div className="flex justify-between"><span className="text-gray-500">Currencies</span><span className="text-gray-300">{p.currencies.join(', ')}</span></div>
                    <div className="flex justify-between"><span className="text-gray-500">Fees</span><span className="text-gray-300">{p.fees}</span></div>
                    <div className="flex justify-between"><span className="text-gray-500">Min</span><span className="text-gray-300">{p.minLimit}</span></div>
                    <div className="flex justify-between"><span className="text-gray-500">Max</span><span className="text-gray-300">{p.maxLimit}</span></div>
                  </div>
                  <button className="w-full mt-3 py-1.5 bg-[#ff6b35]/10 text-[#ff6b35] text-xs rounded hover:bg-[#ff6b35]/20 transition-colors">
                    {rampType === 'on' ? 'Buy X3' : 'Sell X3'}
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default NetworkValidatorsPanel;
