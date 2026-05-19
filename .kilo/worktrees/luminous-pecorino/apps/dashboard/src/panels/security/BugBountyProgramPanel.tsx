import React, { useState } from 'react';
import { Gift, BarChart3, AlertCircle, CheckCircle, Clock, TrendingUp } from 'lucide-react';

interface BountyListing {
  id: string;
  title: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  reward: number;
  status: 'open' | 'claimed' | 'resolved';
  description: string;
  postedDate: string;
  submissions: number;
}

interface BountySubmission {
  id: string;
  submitter: string;
  bountyId: string;
  submissionDate: string;
  status: 'pending' | 'approved' | 'rejected' | 'paid';
  rewardClaimable: boolean;
}

interface BountyMetrics {
  totalBounties: number;
  totalAllocated: number;
  totalPaid: number;
  openBounties: number;
  avgReward: number;
  averagePayout: number;
}

export const BugBountyProgramPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'bounties' | 'submissions' | 'metrics'>('bounties');

  const [bounties] = useState<BountyListing[]>([
    {
      id: 'bounty-1',
      title: 'Critical: Integer Overflow in GPU Memory Allocation',
      severity: 'critical',
      reward: 50000,
      status: 'claimed',
      description: 'Potential integer overflow in GPU memory pool allocation logic',
      postedDate: '2024-01-15',
      submissions: 3,
    },
    {
      id: 'bounty-2',
      title: 'High: Cross-Chain Bridge Authentication Bypass',
      severity: 'high',
      reward: 25000,
      status: 'open',
      description: 'Potential security vulnerability in bridge validator authentication',
      postedDate: '2024-02-01',
      submissions: 1,
    },
    {
      id: 'bounty-3',
      title: 'Medium: Reentrancy in Token Staking',
      severity: 'medium',
      reward: 10000,
      status: 'claimed',
      description: 'Potential reentrancy vulnerability in staking contract',
      postedDate: '2024-01-20',
      submissions: 2,
    },
    {
      id: 'bounty-4',
      title: 'High: RPC Endpoint DoS Vector',
      severity: 'high',
      reward: 20000,
      status: 'open',
      description: 'Missing rate limiting on specific RPC methods',
      postedDate: '2024-02-10',
      submissions: 0,
    },
    {
      id: 'bounty-5',
      title: 'Medium: Input Validation in CLI',
      severity: 'medium',
      reward: 5000,
      status: 'open',
      description: 'Insufficient input validation in command-line interface',
      postedDate: '2024-02-15',
      submissions: 1,
    },
  ]);

  const [submissions] = useState<BountySubmission[]>([
    {
      id: 'sub-1',
      submitter: 'security_researcher_alpha',
      bountyId: 'bounty-1',
      submissionDate: '2024-01-22',
      status: 'paid',
      rewardClaimable: false,
    },
    {
      id: 'sub-2',
      submitter: 'bug_hunter_beta',
      bountyId: 'bounty-3',
      submissionDate: '2024-01-28',
      status: 'approved',
      rewardClaimable: true,
    },
    {
      id: 'sub-3',
      submitter: 'crypto_auditor_gamma',
      bountyId: 'bounty-2',
      submissionDate: '2024-02-05',
      status: 'pending',
      rewardClaimable: false,
    },
    {
      id: 'sub-4',
      submitter: 'infosec_delta',
      bountyId: 'bounty-5',
      submissionDate: '2024-02-18',
      status: 'pending',
      rewardClaimable: false,
    },
  ]);

  const metrics: BountyMetrics = {
    totalBounties: bounties.length,
    totalAllocated: bounties.reduce((sum, b) => sum + b.reward, 0),
    totalPaid: 56000,
    openBounties: bounties.filter((b) => b.status === 'open').length,
    avgReward: Math.round(bounties.reduce((sum, b) => sum + b.reward, 0) / bounties.length),
    averagePayout: 18666,
  };

  const criticalCount = bounties.filter((b) => b.severity === 'critical').length;
  const claimedCount = bounties.filter((b) => b.status === 'claimed' || b.status === 'resolved').length;

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-yellow-400 to-red-500 mb-2">
              Bug Bounty Program
            </h1>
            <p className="text-gray-400">Immunefi • Critical to Low • Submissions • Payouts</p>
          </div>
          <Gift className="w-12 h-12 text-yellow-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Allocated</div>
            <div className="text-2xl font-bold text-yellow-400">${(metrics.totalAllocated / 1000).toFixed(0)}K</div>
            <div className="text-xs text-gray-500 mt-2">Across {metrics.totalBounties} bounties</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Paid</div>
            <div className="text-2xl font-bold text-green-400">${(metrics.totalPaid / 1000).toFixed(0)}K</div>
            <div className="text-xs text-gray-500 mt-2">{claimedCount} resolved</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Open Bounties</div>
            <div className="text-2xl font-bold text-orange-400">{metrics.openBounties}</div>
            <div className="text-xs text-gray-500 mt-2">Waiting for submissions</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Critical Issues</div>
            <div className="text-2xl font-bold text-red-400">{criticalCount}</div>
            <div className="text-xs text-gray-500 mt-2">Max $50K reward</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['bounties', 'submissions', 'metrics'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-yellow-400 border-b-2 border-yellow-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'bounties' && 'Active Bounties'}
              {tab === 'submissions' && 'Submissions'}
              {tab === 'metrics' && 'Metrics'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'bounties' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Active Bounties</h3>
              {bounties.map((bounty) => (
                <div key={bounty.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{bounty.title}</h4>
                      <p className="text-sm text-gray-400 mt-1">{bounty.description}</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        bounty.status === 'open'
                          ? 'bg-blue-500/20 text-blue-400'
                          : bounty.status === 'claimed'
                          ? 'bg-yellow-500/20 text-yellow-400'
                          : 'bg-green-500/20 text-green-400'
                      }`}
                    >
                      {bounty.status.toUpperCase()}
                    </div>
                  </div>
                  <div className="grid grid-cols-6 gap-3 text-sm">
                    <div>
                      <div className="text-gray-400 text-xs">Reward</div>
                      <div className="text-yellow-400 font-bold">${bounty.reward.toLocaleString()}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Severity</div>
                      <div
                        className={`font-semibold capitalize ${
                          bounty.severity === 'critical'
                            ? 'text-red-400'
                            : bounty.severity === 'high'
                            ? 'text-orange-400'
                            : bounty.severity === 'medium'
                            ? 'text-yellow-400'
                            : 'text-green-400'
                        }`}
                      >
                        {bounty.severity}
                      </div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Posted</div>
                      <div className="text-white font-semibold">{bounty.postedDate}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Submissions</div>
                      <div className="text-cyan-400 font-semibold">{bounty.submissions}</div>
                    </div>
                    <div colSpan={2}>
                      <button className="px-3 py-1 bg-yellow-600 hover:bg-yellow-700 text-white text-xs rounded font-semibold transition">
                        View & Submit
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'submissions' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Recent Submissions</h3>
              {submissions.map((sub) => (
                <div key={sub.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{sub.submitter}</h4>
                      <p className="text-sm text-gray-400">Bounty: {sub.bountyId}</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        sub.status === 'paid'
                          ? 'bg-green-500/20 text-green-400'
                          : sub.status === 'approved'
                          ? 'bg-blue-500/20 text-blue-400'
                          : 'bg-yellow-500/20 text-yellow-400'
                      }`}
                    >
                      {sub.status.toUpperCase()}
                    </div>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <div className="text-gray-400">Submitted: {sub.submissionDate}</div>
                    {sub.rewardClaimable && (
                      <button className="px-3 py-1 bg-green-600 hover:bg-green-700 text-white text-xs rounded font-semibold transition">
                        Claim Reward
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'metrics' && (
            <div className="grid grid-cols-2 gap-4">
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <div className="text-gray-400 text-sm mb-3">Overall Program Health</div>
                <div className="space-y-3">
                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span className="text-gray-400">Bounties Resolved</span>
                      <span className="text-white font-semibold">{claimedCount}/5 (60%)</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div className="bg-green-500 h-2 rounded-full" style={{ width: '60%' }} />
                    </div>
                  </div>
                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span className="text-gray-400">Funds Distributed</span>
                      <span className="text-white font-semibold">
                        ${metrics.totalPaid}K / ${metrics.totalAllocated}K (61%)
                      </span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="bg-yellow-500 h-2 rounded-full"
                        style={{ width: `${(metrics.totalPaid / metrics.totalAllocated) * 100}%` }}
                      />
                    </div>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <div className="text-gray-400 text-sm mb-4">Reward Distribution</div>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Avg Reward (All)</span>
                    <span className="text-white font-semibold">${metrics.avgReward.toLocaleString()}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Avg Payout (Paid)</span>
                    <span className="text-white font-semibold">${metrics.averagePayout.toLocaleString()}</span>
                  </div>
                  <div className="flex justify-between pt-2 border-t border-[#2a2a35]">
                    <span className="text-gray-400">Total Submissions</span>
                    <span className="text-cyan-400 font-semibold">{submissions.length}</span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default BugBountyProgramPanel;
