import React, { useState } from 'react';
import { BookOpen, Layers, Coins, GraduationCap, Cpu, Zap, Database, GitBranch, Boxes, Clock } from 'lucide-react';

type Tab = 'architecture' | 'concepts' | 'tokenomics' | 'tutorials';

const concepts = [
  { icon: <Layers size={18} />, title: 'Dual-VM Architecture', description: 'X3 Chain executes both EVM and SVM runtimes within a single Substrate-based blockchain. The X3 Kernel manages VM scheduling, state isolation, and cross-VM message passing. Developers deploy Solidity or Rust contracts and they coexist on the same chain.' },
  { icon: <Zap size={18} />, title: 'Comits', description: 'Composable Micro-Transactions (Comits) bundle cross-VM instructions into atomic execution units. A single Comit can call an EVM contract and an SVM program in one block, with all-or-nothing execution guarantees enforced at the consensus layer.' },
  { icon: <Database size={18} />, title: 'Canonical Ledger', description: 'A unified state root spanning both VM state trees. All balances, contract storage, and account data are committed to a single Merkle-Patricia trie, enabling efficient state proofs and light-client verification across both execution environments.' },
  { icon: <GitBranch size={18} />, title: 'Atomic Execution', description: 'Cross-VM transactions execute atomically: all instructions succeed or all revert. This protocol-level guarantee enables DeFi composability across VMs — flash loans, arbitrage, and complex multi-step operations all execute in a single block.' },
  { icon: <Cpu size={18} />, title: 'GPU Swarm', description: 'A decentralized network of GPU nodes that provide off-chain compute for AI inference, proof generation, and heavy computation. Jobs are auctioned on-chain and results are verified through proof-of-compute before settlement.' },
];

const distribution = [
  { label: 'Community', pct: 40, color: '#ff6b35' },
  { label: 'Ecosystem', pct: 25, color: '#ff8f65' },
  { label: 'Treasury', pct: 20, color: '#ffb395' },
  { label: 'Team', pct: 15, color: '#ffd7c5' },
];

const utilities = [
  { title: 'Gas Fees', description: 'X3 is used to pay transaction fees on both EVM and SVM execution.' },
  { title: 'Staking', description: 'Validators stake X3 to participate in block production and earn rewards.' },
  { title: 'Governance', description: 'STAR tokens are used for on-chain governance voting and proposal submission.' },
  { title: 'Swarm Rewards', description: 'GPU swarm node operators earn X3 for completing computation jobs.' },
];

const tutorials = [
  { title: 'Your First X3 dApp', difficulty: 'Beginner', time: '30 min', description: 'Set up a local dev node, deploy a simple EVM contract, and interact with it through the X3 SDK.' },
  { title: 'Cross-VM Token Bridge', difficulty: 'Intermediate', time: '45 min', description: 'Build a token that exists on both EVM and SVM, enabling seamless cross-VM transfers using Comits.' },
  { title: 'DeFi Liquidity Pool', difficulty: 'Intermediate', time: '60 min', description: 'Create an AMM pool with dual-VM liquidity sourcing and atomic swap execution.' },
  { title: 'GPU Swarm Job Submission', difficulty: 'Intermediate', time: '40 min', description: 'Submit an AI inference job to the GPU swarm, bid on compute, and verify results on-chain.' },
  { title: 'Custom Pallet Development', difficulty: 'Advanced', time: '90 min', description: 'Build a custom Substrate pallet that extends the X3 runtime with new functionality.' },
  { title: 'Zero-Knowledge Proofs on X3', difficulty: 'Advanced', time: '120 min', description: 'Implement and verify ZK proofs using the X3 verifier pallet and GPU swarm for proof generation.' },
];

const difficultyColors: Record<string, string> = {
  Beginner: 'bg-green-500/10 text-green-400',
  Intermediate: 'bg-yellow-500/10 text-yellow-400',
  Advanced: 'bg-red-500/10 text-red-400',
};

const LearnArchitecturePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<Tab>('architecture');

  const tabs: { key: Tab; label: string; icon: React.ReactNode }[] = [
    { key: 'architecture', label: 'Architecture', icon: <Layers size={14} /> },
    { key: 'concepts', label: 'Core Concepts', icon: <Boxes size={14} /> },
    { key: 'tokenomics', label: 'Tokenomics', icon: <Coins size={14} /> },
    { key: 'tutorials', label: 'Tutorials', icon: <GraduationCap size={14} /> },
  ];

  return (
    <div className="flex flex-col h-full bg-[#0a0a0f] text-gray-300">
      <div className="flex items-center gap-4 px-5 py-3 border-b border-[#1a1a1a]">
        <BookOpen size={18} className="text-[#ff6b35]" />
        <h1 className="text-lg font-semibold text-white">Learn</h1>
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
        {activeTab === 'architecture' && (
          <div className="max-w-3xl mx-auto">
            <h2 className="text-xl font-bold text-white mb-4">System Architecture</h2>
            <p className="text-sm text-gray-400 mb-6">
              X3 Chain unifies two virtual machines under a single consensus layer. The diagram below shows the primary data flow from user transactions through the X3 Kernel to final state commitment.
            </p>
            <pre className="bg-[#050508] border border-[#1a1a1a] rounded-lg p-5 text-sm font-mono text-[#ff6b35]/80 overflow-x-auto mb-6 leading-relaxed">
{`┌─────────────────────────────────────────────────────────┐
│                    User Transactions                     │
│              (EVM txns / SVM instructions / Comits)      │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│                     X3 Kernel                         │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │  Transaction  │  │   Comit      │  │   Block       │  │
│  │  Pool         │──│   Engine     │──│   Producer    │  │
│  └──────────────┘  └──────────────┘  └───────────────┘  │
└──────────┬──────────────────┬───────────────────────────┘
           │                  │
     ┌─────▼─────┐     ┌─────▼─────┐
     │    EVM    │     │    SVM    │
     │  Runtime  │     │  Runtime  │
     │ (Solidity)│     │  (Rust)   │
     └─────┬─────┘     └─────┬─────┘
           │                  │
           └────────┬─────────┘
                    ▼
     ┌──────────────────────────┐
     │    Canonical Ledger      │
     │  (Unified State Root)    │
     │  ┌────────┐ ┌────────┐  │
     │  │EVM Trie│ │SVM Trie│  │
     │  └────────┘ └────────┘  │
     └──────────┬───────────────┘
                ▼
     ┌──────────────────────────┐
     │     Consensus Layer      │
     │   (Block Finalization)   │
     └──────────────────────────┘`}
            </pre>
            <div className="space-y-4">
              {[
                { title: 'X3 Kernel', desc: 'The core orchestration layer that routes transactions to the appropriate VM, manages the Comit Engine for cross-VM calls, and coordinates block production.' },
                { title: 'EVM Runtime', desc: 'Full Ethereum Virtual Machine compatibility. Executes Solidity smart contracts with EIP-1559 fee handling and standard JSON-RPC interface.' },
                { title: 'SVM Runtime', desc: 'Solana Virtual Machine integration. Processes BPF programs with the account model, parallel transaction execution, and SPL token support.' },
                { title: 'Comit Engine', desc: 'Handles cross-VM atomic bundles. Validates, orders, and executes multi-VM instruction sets within single blocks.' },
                { title: 'Canonical Ledger', desc: 'Unified state commitment spanning both VM state trees in a single Merkle-Patricia trie for efficient proofs and verification.' },
              ].map((c, i) => (
                <div key={i} className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                  <h3 className="text-sm font-semibold text-white mb-1">{c.title}</h3>
                  <p className="text-xs text-gray-400">{c.desc}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'concepts' && (
          <div className="max-w-3xl mx-auto">
            <h2 className="text-xl font-bold text-white mb-4">Core Concepts</h2>
            <p className="text-sm text-gray-400 mb-6">
              Understanding these five foundational concepts will give you a complete picture of how X3 Chain works under the hood.
            </p>
            <div className="space-y-4">
              {concepts.map((c, i) => (
                <div key={i} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg flex gap-4">
                  <div className="w-10 h-10 bg-[#ff6b35]/10 rounded-lg flex items-center justify-center text-[#ff6b35] flex-shrink-0">
                    {c.icon}
                  </div>
                  <div>
                    <h3 className="text-sm font-semibold text-white mb-1">{c.title}</h3>
                    <p className="text-xs text-gray-400 leading-relaxed">{c.description}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'tokenomics' && (
          <div className="max-w-3xl mx-auto">
            <h2 className="text-xl font-bold text-white mb-4">Tokenomics</h2>
            <div className="grid grid-cols-2 gap-4 mb-6">
              <div className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                <p className="text-xs text-gray-500 mb-1">Primary Token</p>
                <p className="text-lg font-bold text-white">X3</p>
                <p className="text-xs text-gray-400 mt-1">Utility & gas token</p>
              </div>
              <div className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                <p className="text-xs text-gray-500 mb-1">Governance Token</p>
                <p className="text-lg font-bold text-white">STAR</p>
                <p className="text-xs text-gray-400 mt-1">Voting & proposals</p>
              </div>
              <div className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg col-span-2">
                <p className="text-xs text-gray-500 mb-1">Total Supply</p>
                <p className="text-lg font-bold text-white">1,000,000,000 X3</p>
                <p className="text-xs text-gray-400 mt-1">Fixed supply, no inflation</p>
              </div>
            </div>

            <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Distribution</h3>
            <div className="space-y-2 mb-6">
              {distribution.map((d, i) => (
                <div key={i} className="flex items-center gap-3">
                  <span className="text-xs text-gray-400 w-20">{d.label}</span>
                  <div className="flex-1 h-6 bg-[#111118] rounded overflow-hidden border border-[#1a1a1a]">
                    <div className="h-full rounded flex items-center pl-2" style={{ width: `${d.pct}%`, backgroundColor: d.color + '30' }}>
                      <span className="text-xs font-semibold" style={{ color: d.color }}>{d.pct}%</span>
                    </div>
                  </div>
                  <span className="text-xs text-gray-500 w-24 text-right">{(d.pct * 10_000_000).toLocaleString()} X3</span>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Token Utility</h3>
            <div className="grid grid-cols-2 gap-3">
              {utilities.map((u, i) => (
                <div key={i} className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                  <h4 className="text-sm font-semibold text-white mb-1">{u.title}</h4>
                  <p className="text-xs text-gray-400">{u.description}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'tutorials' && (
          <div className="max-w-3xl mx-auto">
            <h2 className="text-xl font-bold text-white mb-4">Tutorials</h2>
            <p className="text-sm text-gray-400 mb-6">Step-by-step guides to help you build on X3 Chain, from beginner to advanced.</p>
            <div className="space-y-3">
              {tutorials.map((t, i) => (
                <div key={i} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg hover:border-[#ff6b35]/20 transition-colors cursor-pointer">
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="text-sm font-semibold text-white">{t.title}</h3>
                    <div className="flex items-center gap-2">
                      <span className={`text-xs px-2 py-0.5 rounded-full ${difficultyColors[t.difficulty]}`}>{t.difficulty}</span>
                      <span className="flex items-center gap-1 text-xs text-gray-500"><Clock size={10} /> {t.time}</span>
                    </div>
                  </div>
                  <p className="text-xs text-gray-400">{t.description}</p>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default LearnArchitecturePanel;
