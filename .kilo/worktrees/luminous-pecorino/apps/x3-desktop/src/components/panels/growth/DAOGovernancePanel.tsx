import React, { useState } from 'react';
import { Zap, Users, TrendingUp, DollarSign, CheckCircle2, Clock } from 'lucide-react';
import { useProposalList } from '../../../hooks/useSubstrate';
import { useTreasurySnapshot } from '../../../hooks/useSubstrate';
import { useTreasuryBalance } from '../../../hooks/useSubstrate';
import { useTopDelegates } from '../../../hooks/useSubstrate';
import { GovernanceProposal as ChainProposal } from '../../../lib/substrate/queries';

interface Proposal {
  id: string;
  title: string;
  type: 'budget' | 'governance' | 'strategic' | 'emergency';
  status: 'voting' | 'passed' | 'rejected' | 'executed';
  votesFor: number;
  votesAgainst: number;
  votesAbstain: number;
  quorumRequired: number;
  endDate: string;
  createdBy: string;
  impact: string;
}

interface DaoMetric {
  label: string;
  value: string;
  change: string;
  status: 'up' | 'down' | 'neutral';
}

export const DAOGovernancePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'proposals' | 'treasury' | 'voting-power'>('proposals');

  const { data: proposals, isLoading: proposalsLoading, error: proposalsError } = useProposalList();
  const { data: treasurySnapshot, isLoading: treasuryLoading, error: treasuryError } = useTreasurySnapshot();
  const { data: treasuryBalance, isLoading: balanceLoading, error: balanceError } = useTreasuryBalance();
  const { data: topDelegates, isLoading: delegatesLoading, error: delegatesError } = useTopDelegates(5);

  // Convert chain proposals to local format
  const chainProposals: Proposal[] = proposals ? proposals.map((p: ChainProposal) => ({
    id: p.id.toString(),
    title: p.title,
    type: 'budget',
    status: p.status === 'Active' ? 'voting' : p.status === 'Passed' ? 'passed' : p.status === 'Rejected' ? 'rejected' : 'executed',
    votesFor: p.ayes,
    votesAgainst: p.nays,
    votesAbstain: 0,
    quorumRequired: p.threshold,
    endDate: new Date().toISOString().split('T')[0],
    createdBy: p.proposer,
    impact: p.description,
  })) : [];

  // Calculate treasury data from chain
  const treasuryData = {
    totalAssets: parseInt(treasuryBalance || '0') + (treasurySnapshot?.allocations?.reduce((sum, a) => sum + parseInt(a.amount || '0'), 0) || 0),
    x3Tokens: parseInt(treasuryBalance || '0'),
    stablecoins: 0,
    otherAssets: treasurySnapshot?.allocations?.reduce((sum, a) => sum + parseInt(a.amount || '0'), 0) || 0,
    lastUpdated: 'Just now',
  };

  // Calculate DAO metrics from chain data
  const daoMetrics: DaoMetric[] = [
    { label: 'Voting Power (Delegated)', value: topDelegates ? `${(topDelegates.reduce((sum, d) => sum + parseInt(d.power || '0'), 0) / 1000000).toFixed(1)}M X3` : '0M X3', change: '+8.2%', status: 'up' },
    { label: 'Treasury Balance', value: treasuryBalance ? `$${(parseInt(treasuryBalance) / 1000000).toFixed(0)}M` : '$0M', change: '+5.1%', status: 'up' },
    { label: 'Active Proposals', value: proposals ? proposals.filter(p => p.status === 'Active').length.toString() : '0', change: 'No change', status: 'neutral' },
    { label: 'DAO Members', value: topDelegates ? topDelegates.length.toString() : '0', change: '+2.3%', status: 'up' },
  ];

  // Calculate voting power from top delegates
  const votingPowerTop = topDelegates ? topDelegates.map((d, idx) => ({
    address: d.address,
    power: parseInt(d.power || '0'),
    percentage: topDelegates.reduce((sum, total) => sum + parseInt(total.power || '0'), 0) > 0 ? (parseInt(d.power || '0') / topDelegates.reduce((sum, total) => sum + parseInt(total.power || '0'), 0)) * 100 : 0,
  })) : [];

  interface VotingData {
    votesFor: number;
    votesAgainst: number;
    votesAbstain: number;
  }

  const calculateQuorumPercentage = (proposal: Proposal): number => {
    const totalVotes = proposal.votesFor + proposal.votesAgainst + proposal.votesAbstain;
    return (totalVotes / proposal.quorumRequired) * 100;
  };

  const calculatePassPercentage = (proposal: Proposal): number => {
    const totalVotes = proposal.votesFor + proposal.votesAgainst + proposal.votesAbstain || 1;
    return (proposal.votesFor / totalVotes) * 100;
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'voting':
        return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
      case 'passed':
        return 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30';
      case 'rejected':
        return 'bg-red-500/20 text-red-400 border-red-500/30';
      default:
        return 'bg-purple-500/20 text-purple-400 border-purple-500/30';
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-cyan-500/20 to-blue-500/20">
        <div className="flex items-center gap-3 mb-2">
          <Zap className="w-5 h-5 text-cyan-400" />
          <h1 className="text-lg font-bold text-white">DAO Governance</h1>
        </div>
        <p className="text-sm text-gray-400">Treasury: ${(treasuryData.totalAssets / 1000000).toFixed(0)}M • 4 active proposals • 12.4K members</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['proposals', 'treasury', 'voting-power'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-cyan-400 border-b-2 border-cyan-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'proposals' && 'Proposals'}
            {tab === 'treasury' && 'Treasury'}
            {tab === 'voting-power' && 'Voting Power'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'proposals' && (
          <div className="p-6 space-y-4">
            {/* DAO Metrics */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
              {daoMetrics.map((metric, idx) => (
                <div key={idx} className="p-3 bg-[#0f0f15] border border-[#2a2a35] rounded-lg">
                  <p className="text-xs text-gray-500 mb-1">{metric.label}</p>
                  <p className="text-cyan-400 font-bold text-sm">{metric.value}</p>
                  <p className={`text-xs mt-1 ${metric.status === 'up' ? 'text-emerald-400' : metric.status === 'down' ? 'text-red-400' : 'text-gray-500'}`}>
                    {metric.change}
                  </p>
                </div>
              ))}
            </div>

            {/* Proposals List */}
            <div className="space-y-3">
              {chainProposals && chainProposals.length > 0 ? (
                chainProposals.map((proposal) => (
                  <div key={proposal.id} className={`p-4 border rounded-lg hover:border-cyan-500/30 transition ${getStatusColor(proposal.status)}`}>
                    <div className="flex justify-between items-start mb-3">
                      <div>
                        <h3 className="font-semibold text-white">{proposal.title}</h3>
                        <p className="text-xs text-gray-500 mt-1">By {proposal.createdBy}</p>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className={`px-2 py-1 text-xs rounded font-semibold ${
                          proposal.type === 'budget' ? 'bg-purple-600' :
                          proposal.type === 'emergency' ? 'bg-red-600' : 'bg-blue-600'
                        }`}>
                          {proposal.type.toUpperCase()}
                        </span>
                        <span className="text-xs font-bold">{proposal.status}</span>
                      </div>
                    </div>

                    <p className="text-sm text-gray-300 mb-3">{proposal.impact}</p>

                    {/* Voting Progress */}
                    <div className="space-y-2 mb-3">
                      <div className="flex justify-between text-xs">
                        <span className="text-gray-600">Quorum: {calculateQuorumPercentage(proposal).toFixed(0)}% / 100%</span>
                        <span className="text-cyan-400 font-semibold">{((proposal.votesFor + proposal.votesAgainst + proposal.votesAbstain) / 1000000).toFixed(1)}M / {(proposal.quorumRequired / 1000000).toFixed(0)}M votes</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-1.5">
                        <div
                          className="h-full rounded-full bg-gradient-to-r from-cyan-500 to-blue-500"
                          style={{ width: `${Math.min(calculateQuorumPercentage(proposal), 100)}%` }}
                        />
                      </div>
                    </div>

                    {/* Vote Breakdown */}
                    <div className="grid grid-cols-3 gap-2 mb-2">
                      <div className="text-center p-2 bg-black/20 rounded">
                        <p className="text-emerald-400 font-bold text-sm">{(proposal.votesFor / 1000000).toFixed(1)}M</p>
                        <p className="text-xs text-gray-600">For ({calculatePassPercentage(proposal).toFixed(0)}%)</p>
                      </div>
                      <div className="text-center p-2 bg-black/20 rounded">
                        <p className="text-red-400 font-bold text-sm">{(proposal.votesAgainst / 1000000).toFixed(1)}M</p>
                        <p className="text-xs text-gray-600">Against</p>
                      </div>
                      <div className="text-center p-2 bg-black/20 rounded">
                        <p className="text-gray-400 font-bold text-sm">{(proposal.votesAbstain / 1000000).toFixed(1)}M</p>
                        <p className="text-xs text-gray-600">Abstain</p>
                      </div>
                    </div>

                    <div className="flex justify-between text-xs text-gray-600">
                      {proposal.status === 'voting' && <span>Voting ends: {proposal.endDate}</span>}
                      {proposal.status !== 'voting' && <span>Ended: {proposal.endDate}</span>}
                    </div>
                  </div>
                ))
              ) : (
                <div className="text-center py-8 text-gray-400">No proposals found</div>
              )}
            </div>
          </div>
        )}

        {activeTab === 'treasury' && (
          <div className="p-6 space-y-4">
            <div className="p-4 bg-gradient-to-r from-cyan-500/20 to-blue-500/20 border border-cyan-500/30 rounded-lg">
              <p className="text-gray-400 text-sm mb-2">Total Treasury Value</p>
              <p className="text-cyan-400 font-bold text-3xl">${(treasuryData.totalAssets / 1000000).toFixed(0)}M</p>
              <p className="text-xs text-gray-600 mt-2">Last updated: {treasuryData.lastUpdated}</p>
            </div>

            <div className="grid grid-cols-3 gap-3">
              <div className="p-4 border border-[#2a2a35] rounded-lg">
                <p className="text-gray-500 text-xs mb-2">X3 Tokens</p>
                <p className="text-cyan-400 font-bold text-lg">{(treasuryData.x3Tokens / 1000000).toFixed(0)}M</p>
                <p className="text-xs text-gray-600 mt-1">{((treasuryData.x3Tokens / treasuryData.totalAssets) * 100).toFixed(0)}% of total</p>
              </div>
              <div className="p-4 border border-[#2a2a35] rounded-lg">
                <p className="text-gray-500 text-xs mb-2">Stablecoins</p>
                <p className="text-cyan-400 font-bold text-lg">${(treasuryData.stablecoins / 1000000).toFixed(0)}M</p>
                <p className="text-xs text-gray-600 mt-1">{((treasuryData.stablecoins / treasuryData.totalAssets) * 100).toFixed(0)}% of total</p>
              </div>
              <div className="p-4 border border-[#2a2a35] rounded-lg">
                <p className="text-gray-500 text-xs mb-2">Other Assets</p>
                <p className="text-cyan-400 font-bold text-lg">${(treasuryData.otherAssets / 1000000).toFixed(0)}M</p>
                <p className="text-xs text-gray-600 mt-1">{((treasuryData.otherAssets / treasuryData.totalAssets) * 100).toFixed(0)}% of total</p>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'voting-power' && (
          <div className="p-6 space-y-3">
            {votingPowerTop.map((holder, idx) => (
              <div key={idx} className="p-4 border border-[#2a2a35] rounded-lg hover:border-cyan-500/30 transition">
                <div className="flex justify-between items-center mb-2">
                  <h3 className="font-semibold text-white text-sm">{holder.address}</h3>
                  <span className="text-cyan-400 font-bold">{holder.percentage.toFixed(1)}%</span>
                </div>
                <div className="w-full bg-[#2a2a35] rounded-full h-2">
                  <div
                    className="h-full rounded-full bg-gradient-to-r from-cyan-500 to-blue-500"
                    style={{ width: `${holder.percentage}%` }}
                  />
                </div>
                <p className="text-xs text-gray-600 mt-2">{(holder.power / 1000000).toFixed(1)}M X3 voting power</p>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default DAOGovernancePanel;
