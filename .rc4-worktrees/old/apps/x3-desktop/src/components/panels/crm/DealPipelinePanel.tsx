import React, { useState } from 'react';
import { TrendingUp, Plus, GripVertical, CheckCircle2, Clock, DollarSign } from 'lucide-react';

interface Deal {
  id: string;
  name: string;
  company: string;
  value: number;
  stage: 'lead' | 'proposal' | 'negotiation' | 'won' | 'lost';
  probability: number;
  daysInStage: number;
  owner: string;
  nextAction: string;
}

export const DealPipelinePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'kanban' | 'analytics' | 'forecast'>('kanban');

  const deals: Deal[] = [
    {
      id: '1',
      name: 'Enterprise Node Cluster',
      company: 'TechCorp Inc',
      value: 250000,
      stage: 'negotiation',
      probability: 75,
      daysInStage: 8,
      owner: 'Sarah Chen',
      nextAction: 'Final contract review',
    },
    {
      id: '2',
      name: 'DEX Integration',
      company: 'SwapHub DAO',
      value: 180000,
      stage: 'proposal',
      probability: 60,
      daysInStage: 14,
      owner: 'Alex Rodriguez',
      nextAction: 'Technical specification',
    },
    {
      id: '3',
      name: 'GPU Validator Setup',
      company: 'BlockSecure LLC',
      value: 120000,
      stage: 'lead',
      probability: 35,
      daysInStage: 3,
      owner: 'Jordan Lee',
      nextAction: 'Initial discovery call',
    },
    {
      id: '4',
      name: 'Multi-Chain Bridge',
      company: 'PolyBridge Collective',
      value: 350000,
      stage: 'negotiation',
      probability: 80,
      daysInStage: 5,
      owner: 'Sarah Chen',
      nextAction: 'Signature pending',
    },
    {
      id: '5',
      name: 'API Gateway Service',
      company: 'DataFlow Systems',
      value: 95000,
      stage: 'proposal',
      probability: 55,
      daysInStage: 21,
      owner: 'Mike Torres',
      nextAction: 'Pricing negotiation',
    },
    {
      id: '6',
      name: 'Enterprise Cloud Setup',
      company: 'FortressNet Inc',
      value: 520000,
      stage: 'won',
      probability: 100,
      daysInStage: 120,
      owner: 'Sarah Chen',
      nextAction: 'Implementation phase',
    },
  ];

  const stageColors = {
    lead: 'bg-blue-500/20 border-blue-500/30',
    proposal: 'bg-purple-500/20 border-purple-500/30',
    negotiation: 'bg-yellow-500/20 border-yellow-500/30',
    won: 'bg-emerald-500/20 border-emerald-500/30',
    lost: 'bg-red-500/20 border-red-500/30',
  };

  const stageLabels = {
    lead: 'Lead',
    proposal: 'Proposal',
    negotiation: 'Negotiation',
    won: 'Won',
    lost: 'Lost',
  };

  const dealsByStage = {
    lead: deals.filter((d) => d.stage === 'lead'),
    proposal: deals.filter((d) => d.stage === 'proposal'),
    negotiation: deals.filter((d) => d.stage === 'negotiation'),
    won: deals.filter((d) => d.stage === 'won'),
    lost: deals.filter((d) => d.stage === 'lost'),
  };

  const piplineTotal = Object.values(dealsByStage)
    .flat()
    .reduce((sum, d) => sum + d.value * (d.probability / 100), 0);

  const actualWon = dealsByStage.won.reduce((sum, d) => sum + d.value, 0);

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-green-500/20 to-emerald-500/20">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-3">
            <TrendingUp className="w-5 h-5 text-green-400" />
            <h1 className="text-lg font-bold text-white">Deal Pipeline (Kanban)</h1>
          </div>
          <button className="flex items-center gap-2 px-3 py-2 bg-green-600 hover:bg-green-700 rounded text-white text-sm font-semibold transition">
            <Plus className="w-4 h-4" />
            New Deal
          </button>
        </div>
        <p className="text-sm text-gray-400">Sales pipeline: ${(piplineTotal / 1000000).toFixed(2)}M forecast, ${(actualWon / 1000000).toFixed(2)}M won</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['kanban', 'analytics', 'forecast'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-green-400 border-b-2 border-green-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'kanban' && 'Kanban Board'}
            {tab === 'analytics' && 'Win Rate'}
            {tab === 'forecast' && 'Forecast'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-x-auto">
        {activeTab === 'kanban' && (
          <div className="p-6 flex gap-6 min-w-max">
            {(['lead', 'proposal', 'negotiation', 'won', 'lost'] as const).map((stage) => (
              <div key={stage} className="flex-shrink-0 w-80">
                <div className="mb-4">
                  <h3 className="font-semibold text-white mb-1">{stageLabels[stage]}</h3>
                  <p className="text-xs text-gray-500">{dealsByStage[stage].length} deals • ${dealsByStage[stage].reduce((sum, d) => sum + d.value * (d.probability / 100), 0) / 1000}K forecast</p>
                </div>
                <div className={`border rounded-lg p-3 min-h-96 space-y-3 ${stageColors[stage]}`}>
                  {dealsByStage[stage].map((deal) => (
                    <div key={deal.id} className="p-3 bg-[#0f0f15] border border-[#2a2a35] rounded cursor-grab hover:border-green-500/30 transition">
                      <div className="flex justify-between items-start mb-2">
                        <h4 className="font-semibold text-white text-sm">{deal.name}</h4>
                        <span className="text-green-400 text-xs font-bold">${(deal.value / 1000).toFixed(0)}K</span>
                      </div>
                      <p className="text-xs text-gray-500 mb-2">{deal.company}</p>
                      <div className="space-y-2 text-xs">
                        <div className="flex justify-between">
                          <span className="text-gray-600">Probability</span>
                          <span className="text-gray-400">{deal.probability}%</span>
                        </div>
                        <div className="w-full bg-[#2a2a35] rounded-full h-1.5">
                          <div
                            className="h-full rounded-full bg-gradient-to-r from-green-500 to-emerald-500"
                            style={{ width: `${deal.probability}%` }}
                          />
                        </div>
                        <div className="flex justify-between pt-2 border-t border-[#2a2a35]">
                          <span className="text-gray-600">{deal.owner}</span>
                          <span className="text-gray-600">{deal.daysInStage}d</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'analytics' && (
          <div className="p-6 grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="border border-[#2a2a35] rounded-lg p-4">
              <h3 className="text-sm font-semibold text-white mb-4">Win Rate by Stage</h3>
              {['lead', 'proposal', 'negotiation', 'won'].map((stage) => {
                const stageName = stageLabels[stage as keyof typeof stageLabels];
                const count = dealsByStage[stage as keyof typeof dealsByStage].length;
                return (
                  <div key={stage} className="mb-4">
                    <div className="flex justify-between text-xs mb-1">
                      <span className="text-gray-400">{stageName}</span>
                      <span className="text-gray-500">{count} deals</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="h-full rounded-full bg-gradient-to-r from-green-500 to-emerald-500"
                        style={{ width: `${Math.random() * 100}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>

            <div className="border border-[#2a2a35] rounded-lg p-4">
              <h3 className="text-sm font-semibold text-white mb-4">Sales Team Performance</h3>
              {['Sarah Chen', 'Alex Rodriguez', 'Jordan Lee', 'Mike Torres'].map((owner) => {
                const ownerDeals = deals.filter((d) => d.owner === owner);
                const totalValue = ownerDeals.reduce((sum, d) => sum + d.value, 0);
                return (
                  <div key={owner} className="mb-4">
                    <div className="flex justify-between text-xs mb-1">
                      <span className="text-gray-400">{owner}</span>
                      <span className="text-green-400 font-semibold">${(totalValue / 1000).toFixed(0)}K</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="h-full rounded-full bg-gradient-to-r from-green-500 to-emerald-500"
                        style={{ width: `${(totalValue / 520000) * 100}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {activeTab === 'forecast' && (
          <div className="p-6">
            <div className="space-y-6">
              <div className="border border-[#2a2a35] rounded-lg p-4">
                <h3 className="text-sm font-semibold text-white mb-4">Revenue Forecast</h3>
                <div className="space-y-4">
                  <div>
                    <div className="flex justify-between text-sm mb-2">
                      <span className="text-gray-400">Weighted Pipeline</span>
                      <span className="text-green-400 font-semibold">${(piplineTotal / 1000).toFixed(0)}K</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-3">
                      <div className="h-full rounded-full bg-gradient-to-r from-green-500 to-emerald-500" style={{ width: '65%' }} />
                    </div>
                  </div>
                  <div>
                    <div className="flex justify-between text-sm mb-2">
                      <span className="text-gray-400">Actual Won (YTD)</span>
                      <span className="text-emerald-400 font-semibold">${(actualWon / 1000).toFixed(0)}K</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-3">
                      <div className="h-full rounded-full bg-gradient-to-r from-emerald-500 to-cyan-500" style={{ width: '100%' }} />
                    </div>
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div className="border border-[#2a2a35] rounded-lg p-4 text-center">
                  <p className="text-gray-500 text-xs mb-1">Avg Deal Size</p>
                  <p className="text-green-400 font-bold text-lg">$215K</p>
                </div>
                <div className="border border-[#2a2a35] rounded-lg p-4 text-center">
                  <p className="text-gray-500 text-xs mb-1">Sales Cycle</p>
                  <p className="text-green-400 font-bold text-lg">34 days</p>
                </div>
                <div className="border border-[#2a2a35] rounded-lg p-4 text-center">
                  <p className="text-gray-500 text-xs mb-1">Win Rate</p>
                  <p className="text-green-400 font-bold text-lg">67%</p>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default DealPipelinePanel;
