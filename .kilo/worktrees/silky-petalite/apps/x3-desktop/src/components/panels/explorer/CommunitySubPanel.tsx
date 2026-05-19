import React, { useState } from 'react';
import { Users, Calendar, MessageSquare, Gift, ExternalLink, Clock, MessageCircle, ArrowRight } from 'lucide-react';

type Tab = 'ecosystem' | 'events' | 'forum' | 'grants';

const projects = [
  { name: 'AtlaSwap', category: 'DeFi', description: 'Cross-VM AMM with concentrated liquidity and atomic routing across EVM and SVM.', logo: '🔄' },
  { name: 'SphereQuest', category: 'Gaming', description: 'Fully on-chain RPG with SVM combat engine and EVM-based marketplace.', logo: '🎮' },
  { name: 'SwarmML', category: 'AI', description: 'Distributed model training across GPU swarm with verifiable proof-of-compute.', logo: '🤖' },
  { name: 'X3 Wallet', category: 'Wallet', description: 'Official browser extension wallet with unified EVM/SVM balance view.', logo: '👛' },
  { name: 'PropertyChain', category: 'RWA', description: 'Tokenized real estate with fractional ownership and yield distribution.', logo: '🏢' },
  { name: 'OracleAI', category: 'Oracle', description: 'AI-powered price oracle with transparent inference verification.', logo: '🔮' },
  { name: 'ComitBridge', category: 'Infrastructure', description: 'Cross-chain bridge leveraging Comit atomicity for trustless transfers.', logo: '🌉' },
  { name: 'StarDAO', category: 'Governance', description: 'Community governance platform for STAR token holders and proposal management.', logo: '⭐' },
];

const events = [
  { date: 'Feb 15, 2026', title: 'X3 Developer Summit', type: 'Conference', location: 'San Francisco, CA', link: '#' },
  { date: 'Feb 22, 2026', title: 'Cross-VM Hackathon', type: 'Hackathon', location: 'Virtual', link: '#' },
  { date: 'Mar 05, 2026', title: 'GPU Swarm Workshop', type: 'Workshop', location: 'Berlin, Germany', link: '#' },
  { date: 'Mar 18, 2026', title: 'X3 Community Call #24', type: 'Community', location: 'Discord / YouTube', link: '#' },
  { date: 'Apr 02, 2026', title: 'DeFi on Dual-VM Bootcamp', type: 'Workshop', location: 'Virtual', link: '#' },
];

const topics = [
  { title: 'Best practices for cross-VM DeFi composability', category: 'Development', replies: 42, lastActivity: '2h ago' },
  { title: 'Proposal: Increase validator set to 24', category: 'Governance', replies: 87, lastActivity: '15m ago' },
  { title: 'GPU Swarm node setup guide for RTX 4090', category: 'Operations', replies: 23, lastActivity: '1h ago' },
  { title: 'X3 tokenomics discussion - staking rewards adjustment', category: 'Tokenomics', replies: 156, lastActivity: '30m ago' },
  { title: 'Bug: EVM contract not visible on SVM side after bridge', category: 'Bug Report', replies: 8, lastActivity: '4h ago' },
  { title: 'Introducing ComitBridge v2 with batched transfers', category: 'Announcements', replies: 34, lastActivity: '6h ago' },
  { title: 'Tutorial request: ZK proofs on X3 Chain', category: 'Requests', replies: 19, lastActivity: '12h ago' },
  { title: 'Performance benchmarks: X3 vs Solana vs Ethereum L2s', category: 'Research', replies: 67, lastActivity: '3h ago' },
];

const grants = [
  { title: 'Core Infrastructure Grant', amount: '$50,000 – $250,000', deadline: 'Mar 31, 2026', requirements: 'Open-source tooling, SDKs, or core protocol improvements that benefit the entire X3 ecosystem.', status: 'Open' },
  { title: 'DeFi Innovation Grant', amount: '$25,000 – $100,000', deadline: 'Apr 15, 2026', requirements: 'Novel DeFi protocols leveraging dual-VM architecture and Comit-based atomic execution.', status: 'Open' },
  { title: 'Community Education Grant', amount: '$5,000 – $25,000', deadline: 'Ongoing', requirements: 'Tutorials, documentation, video content, or educational programs for the X3 developer community.', status: 'Open' },
  { title: 'GPU Swarm Node Subsidy', amount: '$10,000 – $50,000', deadline: 'Feb 28, 2026', requirements: 'Expand the GPU swarm network by deploying and maintaining high-performance compute nodes.', status: 'Closing Soon' },
];

const eventTypeColors: Record<string, string> = {
  Conference: 'bg-purple-500/10 text-purple-400',
  Hackathon: 'bg-blue-500/10 text-blue-400',
  Workshop: 'bg-green-500/10 text-green-400',
  Community: 'bg-yellow-500/10 text-yellow-400',
};

const categoryColors: Record<string, string> = {
  Development: 'text-blue-400',
  Governance: 'text-purple-400',
  Operations: 'text-green-400',
  Tokenomics: 'text-yellow-400',
  'Bug Report': 'text-red-400',
  Announcements: 'text-[#ff6b35]',
  Requests: 'text-cyan-400',
  Research: 'text-indigo-400',
};

const CommunitySubPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<Tab>('ecosystem');

  const tabs: { key: Tab; label: string; icon: React.ReactNode }[] = [
    { key: 'ecosystem', label: 'Ecosystem', icon: <Users size={14} /> },
    { key: 'events', label: 'Events', icon: <Calendar size={14} /> },
    { key: 'forum', label: 'Forum', icon: <MessageSquare size={14} /> },
    { key: 'grants', label: 'Grants', icon: <Gift size={14} /> },
  ];

  return (
    <div className="flex flex-col h-full bg-[#0a0a0f] text-gray-300">
      <div className="flex items-center gap-4 px-5 py-3 border-b border-[#1a1a1a]">
        <Users size={18} className="text-[#ff6b35]" />
        <h1 className="text-lg font-semibold text-white">Community</h1>
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
        {activeTab === 'ecosystem' && (
          <div className="grid grid-cols-2 gap-3">
            {projects.map((p, i) => (
              <div key={i} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg hover:border-[#ff6b35]/20 transition-colors">
                <div className="flex items-center gap-3 mb-2">
                  <div className="w-10 h-10 bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg flex items-center justify-center text-xl">{p.logo}</div>
                  <div>
                    <h3 className="text-sm font-semibold text-white">{p.name}</h3>
                    <span className="text-xs text-gray-500">{p.category}</span>
                  </div>
                </div>
                <p className="text-xs text-gray-400">{p.description}</p>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'events' && (
          <div className="space-y-3 max-w-3xl">
            <h2 className="text-lg font-bold text-white mb-3">Upcoming Events</h2>
            {events.map((ev, i) => (
              <div key={i} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg hover:border-[#ff6b35]/20 transition-colors">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <div className="flex items-center gap-1.5 text-xs text-gray-500">
                      <Calendar size={12} /> {ev.date}
                    </div>
                    <span className={`text-xs px-2 py-0.5 rounded-full ${eventTypeColors[ev.type] || 'bg-gray-500/10 text-gray-400'}`}>{ev.type}</span>
                  </div>
                  <ExternalLink size={12} className="text-gray-500" />
                </div>
                <h3 className="text-sm font-semibold text-white mb-1">{ev.title}</h3>
                <p className="text-xs text-gray-500">{ev.location}</p>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'forum' && (
          <div className="max-w-3xl">
            <h2 className="text-lg font-bold text-white mb-3">Recent Topics</h2>
            <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden divide-y divide-[#1a1a1a]">
              {topics.map((t, i) => (
                <div key={i} className="px-4 py-3 hover:bg-white/[0.02] cursor-pointer transition-colors">
                  <div className="flex items-center justify-between">
                    <div className="flex-1">
                      <h4 className="text-sm text-white hover:text-[#ff6b35] transition-colors">{t.title}</h4>
                      <span className={`text-xs ${categoryColors[t.category] || 'text-gray-500'}`}>{t.category}</span>
                    </div>
                    <div className="flex items-center gap-4 text-xs text-gray-500 flex-shrink-0 ml-4">
                      <span className="flex items-center gap-1"><MessageCircle size={10} /> {t.replies}</span>
                      <span className="flex items-center gap-1"><Clock size={10} /> {t.lastActivity}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'grants' && (
          <div className="max-w-3xl">
            <h2 className="text-lg font-bold text-white mb-3">Grant Programs</h2>
            <div className="space-y-3">
              {grants.map((g, i) => (
                <div key={i} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="text-sm font-semibold text-white">{g.title}</h3>
                    <span className={`text-xs px-2 py-0.5 rounded-full ${g.status === 'Closing Soon' ? 'bg-red-500/10 text-red-400' : 'bg-green-500/10 text-green-400'}`}>{g.status}</span>
                  </div>
                  <p className="text-xs text-gray-400 mb-3">{g.requirements}</p>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4 text-xs text-gray-500">
                      <span>Amount: <span className="text-[#ff6b35] font-semibold">{g.amount}</span></span>
                      <span>Deadline: <span className="text-gray-300">{g.deadline}</span></span>
                    </div>
                    <button className="flex items-center gap-1 text-xs text-[#ff6b35] hover:text-[#ff6b35]/80 transition-colors">
                      Apply <ArrowRight size={10} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default CommunitySubPanel;
