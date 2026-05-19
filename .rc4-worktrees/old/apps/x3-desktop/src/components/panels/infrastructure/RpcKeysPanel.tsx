import React, { useState } from 'react';
import { Key, Lock, TrendingUp, Copy, Trash2, Plus } from 'lucide-react';

interface APIKey {
  id: string;
  name: string;
  key: string;
  rateLimit: number;
  requestsToday: number;
  createdAt: string;
  lastUsed: string;
  permissions: string[];
  status: 'active' | 'revoked' | 'limited';
}

export const RpcKeysPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'keys' | 'usage' | 'permissions'>('keys');
  const [showNewKeyForm, setShowNewKeyForm] = useState(false);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const keys: APIKey[] = [
    {
      id: '1',
      name: 'App: X3 Trading Bot',
      key: 'x3_pk_live_4f9e2d8c7b1a5e3f_masked',
      rateLimit: 1000,
      requestsToday: 847,
      createdAt: '2026-01-15',
      lastUsed: '2 mins ago',
      permissions: ['read:chain', 'write:transactions', 'read:balance'],
      status: 'active',
    },
    {
      id: '2',
      name: 'Integration: The Graph Indexer',
      key: 'x3_pk_live_8d2f4c9e1b7a3e5c_masked',
      rateLimit: 500,
      requestsToday: 312,
      createdAt: '2026-02-01',
      lastUsed: '5 mins ago',
      permissions: ['read:chain', 'read:events'],
      status: 'active',
    },
    {
      id: '3',
      name: 'Dev: Local Testing',
      key: 'x3_pk_test_2a5f8c1d9e3b7f4c_masked',
      rateLimit: 100,
      requestsToday: 89,
      createdAt: '2026-02-10',
      lastUsed: '1 hour ago',
      permissions: ['read:chain', 'read:balance'],
      status: 'active',
    },
    {
      id: '4',
      name: 'Archive: Old Validator Setup',
      key: 'x3_pk_live_7c3f2e1a9d8b5e4f_masked',
      rateLimit: 2000,
      requestsToday: 0,
      createdAt: '2025-12-01',
      lastUsed: '30 days ago',
      permissions: ['read:chain', 'read:validator'],
      status: 'revoked',
    },
  ];

  const usageData = [
    { method: 'chain_getBlock', count: 245, avg_time: '12ms' },
    { method: 'system_events', count: 189, avg_time: '8ms' },
    { method: 'author_submitExtrinsic', count: 156, avg_time: '45ms' },
    { method: 'state_getStorage', count: 134, avg_time: '6ms' },
    { method: 'balance_free', count: 123, avg_time: '5ms' },
  ];

  const copyToClipboard = (key: string, id: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(id);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active':
        return 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30';
      case 'limited':
        return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
      default:
        return 'bg-red-500/20 text-red-400 border-red-500/30';
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-blue-500/20 to-purple-500/20">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-3">
            <Key className="w-5 h-5 text-blue-400" />
            <h1 className="text-lg font-bold text-white">RPC Access Keys</h1>
          </div>
          <button
            onClick={() => setShowNewKeyForm(!showNewKeyForm)}
            className="flex items-center gap-2 px-3 py-2 bg-blue-600 hover:bg-blue-700 rounded text-white text-sm font-semibold transition"
          >
            <Plus className="w-4 h-4" />
            New Key
          </button>
        </div>
        <p className="text-sm text-gray-400">Manage API keys with rate limiting and permission controls</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['keys', 'usage', 'permissions'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-blue-400 border-b-2 border-blue-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'keys' && 'API Keys'}
            {tab === 'usage' && 'Usage Stats'}
            {tab === 'permissions' && 'Permissions'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {showNewKeyForm && (
          <div className="p-6 border-b border-[#2a2a35] bg-blue-500/5">
            <div className="space-y-3">
              <input type="text" placeholder="Key name (e.g., 'My App')" className="w-full px-3 py-2 bg-[#0f0f15] border border-[#2a2a35] rounded text-white placeholder-gray-600 text-sm" />
              <select className="w-full px-3 py-2 bg-[#0f0f15] border border-[#2a2a35] rounded text-white text-sm">
                <option>Select permissions...</option>
                <option>read:chain</option>
                <option>read:balance</option>
                <option>write:transactions</option>
              </select>
              <div className="flex gap-2">
                <button className="flex-1 px-3 py-2 bg-emerald-600 hover:bg-emerald-700 rounded text-white text-sm font-semibold transition">
                  Create Key
                </button>
                <button onClick={() => setShowNewKeyForm(false)} className="flex-1 px-3 py-2 bg-[#2a2a35] hover:bg-[#3a3a45] rounded text-white text-sm font-semibold transition">
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'keys' && (
          <div className="p-6 space-y-3">
            {keys.map((key) => (
              <div
                key={key.id}
                onClick={() => setSelectedKey(key.id)}
                className={`p-4 border rounded-lg cursor-pointer transition ${
                  selectedKey === key.id
                    ? 'border-blue-500 bg-blue-500/10'
                    : 'border-[#2a2a35] hover:border-blue-500/50'
                }`}
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <h3 className="font-semibold text-white">{key.name}</h3>
                    <p className="text-xs text-gray-500 font-mono mt-1">{key.key}</p>
                  </div>
                  <span className={`px-2 py-1 text-xs rounded font-semibold ${getStatusColor(key.status)}`}>
                    {key.status.toUpperCase()}
                  </span>
                </div>
                <div className="grid grid-cols-3 gap-3 text-sm mb-3">
                  <div>
                    <span className="text-gray-500 text-xs">Rate Limit</span>
                    <p className="text-blue-400 font-semibold">{key.rateLimit}/min</p>
                  </div>
                  <div>
                    <span className="text-gray-500 text-xs">Today</span>
                    <p className="text-blue-400 font-semibold">{key.requestsToday} reqs</p>
                  </div>
                  <div>
                    <span className="text-gray-500 text-xs">Last Used</span>
                    <p className="text-blue-400 font-semibold">{key.lastUsed}</p>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex gap-1 flex-wrap">
                    {key.permissions.map((perm) => (
                      <span key={perm} className="px-2 py-1 text-xs bg-[#2a2a35] text-gray-300 rounded">
                        {perm}
                      </span>
                    ))}
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={() => copyToClipboard(key.key, key.id)}
                      className="p-2 hover:bg-[#2a2a35] rounded transition"
                    >
                      <Copy className="w-4 h-4 text-gray-400" />
                    </button>
                    {key.status !== 'revoked' && (
                      <button className="p-2 hover:bg-red-500/20 rounded transition">
                        <Trash2 className="w-4 h-4 text-red-400" />
                      </button>
                    )}
                  </div>
                </div>
                {copiedKey === key.id && <p className="text-xs text-emerald-400 mt-2">Copied to clipboard!</p>}
              </div>
            ))}
          </div>
        )}

        {activeTab === 'usage' && (
          <div className="p-6">
            <div className="space-y-3">
              {usageData.map((item, idx) => (
                <div key={idx} className="p-4 border border-[#2a2a35] rounded-lg hover:border-blue-500/30 transition">
                  <div className="flex justify-between items-center mb-2">
                    <h3 className="font-semibold text-white text-sm">{item.method}</h3>
                    <span className="text-blue-400 font-semibold">{item.count}</span>
                  </div>
                  <div className="w-full bg-[#2a2a35] rounded-full h-2">
                    <div
                      className="h-full rounded-full bg-gradient-to-r from-blue-500 to-purple-500"
                      style={{ width: `${(item.count / 250) * 100}%` }}
                    />
                  </div>
                  <div className="flex justify-between text-xs text-gray-500 mt-2">
                    <span>Avg: {item.avg_time}</span>
                    <span>{((item.count / 847) * 100).toFixed(1)}% of total</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'permissions' && (
          <div className="p-6 space-y-3">
            {['read:chain', 'read:balance', 'read:validator', 'write:transactions', 'read:events', 'write:contracts'].map(
              (perm) => (
                <div key={perm} className="p-4 border border-[#2a2a35] rounded-lg hover:border-blue-500/30 transition">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <Lock className="w-4 h-4 text-blue-400" />
                      <span className="text-white font-semibold text-sm">{perm}</span>
                    </div>
                    <span className="text-gray-500 text-xs">
                      {keys.filter((k) => k.permissions.includes(perm) && k.status === 'active').length} active keys
                    </span>
                  </div>
                </div>
              )
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default RpcKeysPanel;
