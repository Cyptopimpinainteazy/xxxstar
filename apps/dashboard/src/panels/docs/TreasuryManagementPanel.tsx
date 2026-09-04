import React, { useState } from 'react';
import { Wallet, Send, Lock, Eye, BarChart3, History, Plus, CheckCircle2, Clock } from 'lucide-react';

interface TreasuryTransaction {
  id: string;
  type: 'in' | 'out';
  description: string;
  amount: number;
  token: string;
  recipient: string;
  status: 'pending' | 'approved' | 'executed' | 'rejected';
  approvals: number;
  approvalThreshold: number;
  timestamp: string;
}

interface BudgetAllocation {
  category: string;
  amount: number;
  percentage: number;
  spent: number;
  remaining: number;
}

export const TreasuryManagementPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'overview' | 'transactions' | 'budget'>('overview');

  const treasuryBalance = 8750000;
  const treasuryToken = 'X3';

  const transactions: TreasuryTransaction[] = [
    {
      id: 'tx-1',
      type: 'out',
      description: 'LM Rewards Distribution - Epoch 245',
      amount: 3000000,
      token: 'X3',
      recipient: 'AMM Pools',
      status: 'executed',
      approvals: 5,
      approvalThreshold: 5,
      timestamp: '2 hours ago',
    },
    {
      id: 'tx-2',
      type: 'out',
      description: 'Developer Grant - AI Agents Framework',
      amount: 250000,
      token: 'X3',
      recipient: 'dev.grants.x3.dao',
      status: 'approved',
      approvals: 4,
      approvalThreshold: 5,
      timestamp: '5 hours ago',
    },
    {
      id: 'tx-3',
      type: 'in',
      description: 'Protocol Revenue - Trading Fees',
      amount: 125000,
      token: 'X3',
      recipient: 'treasury.x3.dao',
      status: 'executed',
      approvals: 0,
      approvalThreshold: 0,
      timestamp: '1 day ago',
    },
  ];

  const budgetAllocations: BudgetAllocation[] = [
    { category: 'Liquidity Mining', amount: 5000000, percentage: 57, spent: 3000000, remaining: 2000000 },
    { category: 'Developer Grants', amount: 1500000, percentage: 17, spent: 850000, remaining: 650000 },
    { category: 'Marketing & Growth', amount: 800000, percentage: 9, spent: 320000, remaining: 480000 },
    { category: 'Infrastructure', amount: 600000, percentage: 7, spent: 200000, remaining: 400000 },
    { category: 'Security & Audits', amount: 400000, percentage: 5, spent: 180000, remaining: 220000 },
    { category: 'Ecosystem Reserve', amount: 1450000, percentage: 17, spent: 0, remaining: 1450000 },
  ];

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-emerald-500 to-teal-500 rounded-lg">
            <Wallet className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Treasury Management</h1>
            <p className="text-xs text-gray-400">Multi-sig wallet, budget allocation, spending history</p>
          </div>
        </div>

        {/* Balance Card */}
        <div className="bg-gradient-to-br from-emerald-500/20 to-teal-500/20 border border-emerald-500/30 rounded-lg p-4">
          <p className="text-xs text-gray-400 mb-1">Treasury Balance</p>
          <div className="flex items-baseline gap-2 mb-2">
            <span className="text-3xl font-bold text-emerald-400">{(treasuryBalance / 1000000).toFixed(2)}M</span>
            <span className="text-lg font-semibold text-gray-400">{treasuryToken}</span>
          </div>
          <p className="text-xs text-gray-500">≈ ${(treasuryBalance * 1.25 / 1000000).toFixed(2)}M USD</p>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['overview', 'transactions', 'budget'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-emerald-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'overview' && '📊 Overview'}
            {tab === 'transactions' && '💸 Transactions'}
            {tab === 'budget' && '📈 Budget'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'overview' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3">Multi-Sig Status</h3>
              <div className="grid grid-cols-3 gap-3">
                <div className="bg-[#0a0a0f] rounded p-3 border border-[#2a2a35]">
                  <p className="text-xs text-gray-500 mb-1">Signatories</p>
                  <p className="text-lg font-bold text-cyan-400">5 of 7</p>
                </div>
                <div className="bg-[#0a0a0f] rounded p-3 border border-[#2a2a35]">
                  <p className="text-xs text-gray-500 mb-1">Threshold</p>
                  <p className="text-lg font-bold text-cyan-400">4 of 5</p>
                </div>
                <div className="bg-[#0a0a0f] rounded p-3 border border-[#2a2a35]">
                  <p className="text-xs text-gray-500 mb-1">Time Lock</p>
                  <p className="text-lg font-bold text-cyan-400">2 days</p>
                </div>
              </div>
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3 flex items-center gap-2">
                <Clock size={16} />
                Pending Approvals
              </h3>
              {transactions
                .filter(t => t.status === 'approved')
                .map(tx => (
                  <div key={tx.id} className="bg-[#0a0a0f] rounded p-3 border border-[#2a2a35] mb-2 last:mb-0">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm text-white">{tx.description}</span>
                      <span className="text-sm font-semibold text-cyan-400">{tx.amount.toLocaleString()} {tx.token}</span>
                    </div>
                    <div className="flex items-center gap-2 mb-2">
                      <div className="flex-1 h-2 bg-gray-700 rounded-full overflow-hidden">
                        <div
                          className="h-2 bg-blue-500"
                          style={{ width: `${(tx.approvals / tx.approvalThreshold) * 100}%` }}
                        />
                      </div>
                      <span className="text-xs text-gray-400">{tx.approvals}/{tx.approvalThreshold}</span>
                    </div>
                    <button className="px-3 py-1 text-xs bg-blue-600 hover:bg-blue-700 text-white rounded transition">
                      Approve
                    </button>
                  </div>
                ))}
            </div>
          </div>
        )}

        {activeTab === 'transactions' && (
          <div className="p-4 space-y-3">
            <button className="w-full px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded font-medium transition flex items-center justify-center gap-2 mb-4">
              <Plus size={16} />
              New Transaction
            </button>

            {transactions.map(tx => (
              <div key={tx.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-emerald-500/50 transition">
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <div className={`p-2 rounded-lg ${tx.type === 'out' ? 'bg-red-500/20' : 'bg-green-500/20'}`}>
                      {tx.type === 'out' ? (
                        <Send className={`w-4 h-4 ${tx.type === 'out' ? 'text-red-400' : 'text-green-400'}`} />
                      ) : (
                        <Send className={`w-4 h-4 text-green-400`} />
                      )}
                    </div>
                    <div>
                      <p className="font-medium text-white">{tx.description}</p>
                      <p className="text-xs text-gray-500">{tx.recipient}</p>
                    </div>
                  </div>
                  <span className={`text-sm font-semibold ${tx.type === 'out' ? 'text-red-400' : 'text-green-400'}`}>
                    {tx.type === 'out' ? '-' : '+'}{tx.amount.toLocaleString()} {tx.token}
                  </span>
                </div>

                <div className="flex items-center justify-between text-xs">
                  <span className="text-gray-500">{tx.timestamp}</span>
                  <span className={`px-2 py-1 rounded ${
                    tx.status === 'executed' ? 'bg-green-500/20 text-green-400' :
                    tx.status === 'approved' ? 'bg-blue-500/20 text-blue-400' :
                    'bg-yellow-500/20 text-yellow-400'
                  }`}>
                    {tx.status.charAt(0).toUpperCase() + tx.status.slice(1)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'budget' && (
          <div className="p-4 space-y-3">
            <div className="text-sm text-gray-400 mb-4">
              Annual Budget: <span className="text-white font-semibold">{(budgetAllocations.reduce((sum, b) => sum + b.amount, 0) / 1000000).toFixed(1)}M {treasuryToken}</span>
            </div>

            {budgetAllocations.map(budget => (
              <div key={budget.category} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-emerald-500/50 transition">
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <p className="font-medium text-white">{budget.category}</p>
                    <p className="text-xs text-gray-500">${(budget.amount / 1000000).toFixed(2)}M allocated</p>
                  </div>
                  <span className="text-xs px-2 py-1 bg-emerald-500/20 text-emerald-400 rounded font-medium">
                    {budget.percentage}%
                  </span>
                </div>

                <div className="mb-2">
                  <div className="flex items-center justify-between mb-1 text-xs">
                    <span className="text-gray-500">Spent</span>
                    <span className="text-gray-300">${(budget.spent / 1000000).toFixed(2)}M / ${(budget.amount / 1000000).toFixed(2)}M</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div
                      className="h-2 bg-gradient-to-r from-emerald-500 to-teal-500"
                      style={{ width: `${(budget.spent / budget.amount) * 100}%` }}
                    />
                  </div>
                </div>

                <p className="text-xs text-gray-400">
                  Remaining: ${(budget.remaining / 1000000).toFixed(2)}M
                </p>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default TreasuryManagementPanel;
