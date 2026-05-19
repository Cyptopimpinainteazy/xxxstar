import React, { useState } from 'react';
import { Vote, Plus, MessageSquare, Clock, CheckCircle2, X, TrendingUp } from 'lucide-react';

interface Proposal {
  id: string;
  title: string;
  description: string;
  status: 'pending' | 'voting' | 'passed' | 'rejected' | 'executed';
  votes: { for: number; against: number; abstain: number };
  quorum: number;
  quorumRequired: number;
  endTime: string;
  creator: string;
  priority: 'low' | 'medium' | 'high';
}

export const GovernanceProposalsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'active' | 'history' | 'create'>('active');
  const [filterStatus, setFilterStatus] = useState<'all' | 'voting' | 'pending' | 'passed' | 'rejected'>('all');

  const proposals: Proposal[] = [
    {
      id: 'prop-101',
      title: 'Increase LM rewards to 5M X3/epoch',
      description: 'Proposal to boost liquidity mining rewards from 3M to 5M X3 per epoch for 6 months to accelerate TVL growth.',
      status: 'voting',
      votes: { for: 4250000, against: 850000, abstain: 300000 },
      quorum: 5400000,
      quorumRequired: 4000000,
      endTime: '2 days left',
      creator: 'alice.x3',
      priority: 'high',
    },
    {
      id: 'prop-100',
      title: 'Enable token-gated communities',
      description: 'Enable governance token holders to create private communities with NFT-gated access and custom moderation rules.',
      status: 'voting',
      votes: { for: 3100000, against: 420000, abstain: 180000 },
      quorum: 3700000,
      quorumRequired: 4000000,
      endTime: '4 days left',
      creator: 'bob.x3',
      priority: 'medium',
    },
    {
      id: 'prop-99',
      title: 'Deploy cross-chain bridge to Bitcoin',
      description: 'Proposal to integrate Bitcoin HTLC bridge for native BTC trading on X3 DEX without centralized wrapping.',
      status: 'passed',
      votes: { for: 5800000, against: 120000, abstain: 80000 },
      quorum: 6000000,
      quorumRequired: 4000000,
      endTime: 'Passed 5 days ago',
      creator: 'charlie.x3',
      priority: 'high',
    },
  ];

  const filteredProposals = proposals.filter(p => 
    filterStatus === 'all' || p.status === filterStatus
  );

  const getStatusColor = (status: Proposal['status']) => {
    const colors = {
      pending: 'bg-gray-500/20 text-gray-400',
      voting: 'bg-blue-500/20 text-blue-400',
      passed: 'bg-green-500/20 text-green-400',
      rejected: 'bg-red-500/20 text-red-400',
      executed: 'bg-purple-500/20 text-purple-400',
    };
    return colors[status];
  };

  const getVotePercentage = (votes: number, total: number) => {
    return total === 0 ? 0 : (votes / total) * 100;
  };

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-blue-500 to-cyan-500 rounded-lg">
            <Vote className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Governance Proposals</h1>
            <p className="text-xs text-gray-400">DAO voting, quorum tracking, proposal timeline</p>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['active', 'history', 'create'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-blue-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'active' && '📋 Active Proposals'}
            {tab === 'history' && '📜 History'}
            {tab === 'create' && '✍️ Create Proposal'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'active' && (
          <div className="p-4 space-y-4">
            {/* Filter */}
            <div className="flex gap-2 flex-wrap">
              {(['all', 'voting', 'pending', 'passed', 'rejected'] as const).map(status => (
                <button
                  key={status}
                  onClick={() => setFilterStatus(status)}
                  className={`px-3 py-1 rounded-lg text-xs font-medium transition ${
                    filterStatus === status
                      ? 'bg-blue-600 text-white'
                      : 'bg-[#1a1a2e] text-gray-400 hover:text-gray-200'
                  }`}
                >
                  {status.charAt(0).toUpperCase() + status.slice(1)}
                </button>
              ))}
            </div>

            {/* Proposals */}
            {filteredProposals.map(proposal => (
              <div key={proposal.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-blue-500/50 transition">
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h3 className="font-semibold text-white">{proposal.title}</h3>
                      <span className={`text-xs px-2 py-1 rounded font-medium ${getStatusColor(proposal.status)}`}>
                        {proposal.status.charAt(0).toUpperCase() + proposal.status.slice(1)}
                      </span>
                      <span className={`text-xs px-2 py-1 rounded ${
                        proposal.priority === 'high' ? 'bg-red-500/20 text-red-400' :
                        proposal.priority === 'medium' ? 'bg-yellow-500/20 text-yellow-400' :
                        'bg-green-500/20 text-green-400'
                      }`}>
                        {proposal.priority.charAt(0).toUpperCase() + proposal.priority.slice(1)} Priority
                      </span>
                    </div>
                    <p className="text-xs text-gray-400 line-clamp-2">{proposal.description}</p>
                  </div>
                </div>

                {/* Voting Stats */}
                {proposal.status === 'voting' && (
                  <div className="mb-3 space-y-2">
                    <div className="flex items-center justify-between text-xs mb-1">
                      <span className="text-gray-400">Voting Results</span>
                      <span className="text-gray-400">
                        Quorum: {(proposal.quorum / 1000000).toFixed(1)}M / {(proposal.quorumRequired / 1000000).toFixed(1)}M
                      </span>
                    </div>

                    {/* Vote bars */}
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="w-12 text-xs text-green-400">For</span>
                        <div className="flex-1 h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                          <div
                            className="h-2 bg-green-500"
                            style={{
                              width: `${getVotePercentage(proposal.votes.for, proposal.votes.for + proposal.votes.against + proposal.votes.abstain)}%`
                            }}
                          />
                        </div>
                        <span className="w-20 text-xs text-gray-400 text-right">
                          {(proposal.votes.for / 1000000).toFixed(1)}M
                        </span>
                      </div>

                      <div className="flex items-center gap-2">
                        <span className="w-12 text-xs text-red-400">Against</span>
                        <div className="flex-1 h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                          <div
                            className="h-2 bg-red-500"
                            style={{
                              width: `${getVotePercentage(proposal.votes.against, proposal.votes.for + proposal.votes.against + proposal.votes.abstain)}%`
                            }}
                          />
                        </div>
                        <span className="w-20 text-xs text-gray-400 text-right">
                          {(proposal.votes.against / 1000000).toFixed(1)}M
                        </span>
                      </div>

                      <div className="flex items-center gap-2">
                        <span className="w-12 text-xs text-gray-400">Abstain</span>
                        <div className="flex-1 h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                          <div
                            className="h-2 bg-gray-500"
                            style={{
                              width: `${getVotePercentage(proposal.votes.abstain, proposal.votes.for + proposal.votes.against + proposal.votes.abstain)}%`
                            }}
                          />
                        </div>
                        <span className="w-20 text-xs text-gray-400 text-right">
                          {(proposal.votes.abstain / 1000000).toFixed(1)}M
                        </span>
                      </div>
                    </div>
                  </div>
                )}

                <div className="flex items-center justify-between mb-3 text-xs">
                  <div className="flex items-center gap-4 text-gray-400">
                    <span>Proposed by {proposal.creator}</span>
                    <span className="flex items-center gap-1">
                      <Clock size={14} />
                      {proposal.endTime}
                    </span>
                  </div>
                </div>

                <button className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition text-sm">
                  {proposal.status === 'voting' ? 'Cast Your Vote' : 'View Details'}
                </button>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'history' && (
          <div className="p-4 space-y-3">
            <div className="text-center py-8 text-gray-400">
              <CheckCircle2 className="w-8 h-8 mx-auto mb-2 text-gray-500" />
              <p className="text-sm">View historical proposal outcomes</p>
            </div>
          </div>
        )}

        {activeTab === 'create' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3">Create New Proposal</h3>
              <div className="space-y-3">
                <div>
                  <label className="text-xs text-gray-400">Title</label>
                  <input
                    type="text"
                    placeholder="Enter proposal title"
                    className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-white placeholder-gray-600 focus:border-blue-500 outline-none"
                  />
                </div>
                <div>
                  <label className="text-xs text-gray-400">Description</label>
                  <textarea
                    placeholder="Detailed proposal description"
                    rows={4}
                    className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-white placeholder-gray-600 focus:border-blue-500 outline-none"
                  />
                </div>
                <div>
                  <label className="text-xs text-gray-400">Priority</label>
                  <select className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option>Low</option>
                    <option>Medium</option>
                    <option>High</option>
                  </select>
                </div>
                <button className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition">
                  Submit Proposal
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default GovernanceProposalsPanel;
