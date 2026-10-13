'use client';

import React from 'react';
import Link from 'next/link';
import {
  Layers,
  Cpu,
  Database,
  Zap,
  Shield,
  Globe,
  ArrowRight,
  Code,
  Box,
  GitBranch,
  Lock,
  Coins,
} from 'lucide-react';

const concepts = [
  {
    icon: <Layers className="w-8 h-8" />,
    title: 'Dual VM Architecture',
    description: 'X3 Atlas Sphere runs two virtual machines in parallel: the Ethereum Virtual Machine (EVM) and Solana Virtual Machine (SVM). Both VMs execute within the same block, sharing state through the canonical ledger.',
    details: [
      'EVM for Solidity/Vyper smart contracts',
      'SVM for Rust/Anchor programs',
      'Deterministic execution ordering',
      'Shared account abstraction',
    ],
  },
  {
    icon: <Cpu className="w-8 h-8" />,
    title: 'Atlas Kernel',
    description: 'The Atlas Kernel is the core pallet that orchestrates cross-VM operations. It manages Comit transactions, the canonical ledger, account authorization, and asset registry.',
    details: [
      'Comit transaction processing',
      'Canonical ledger management',
      'Account authorization system',
      'Asset registry and metadata',
    ],
  },
  {
    icon: <Zap className="w-8 h-8" />,
    title: 'Comit Transactions',
    description: 'Comits are atomic cross-VM transactions that execute payloads on both EVM and SVM simultaneously. Either both succeed or both fail, ensuring consistency.',
    details: [
      'Atomic cross-VM execution',
      'EVM and SVM payloads in one tx',
      'Prepare root verification',
      'Deterministic ordering',
    ],
  },
  {
    icon: <Database className="w-8 h-8" />,
    title: 'Canonical Ledger',
    description: 'The canonical ledger is a unified balance sheet that tracks all assets across both VMs. Assets exist once and can be accessed from either execution environment.',
    details: [
      'Single source of truth for balances',
      'No wrapped tokens needed',
      'Cross-VM asset transfers',
      'Unified liquidity pools',
    ],
  },
  {
    icon: <Shield className="w-8 h-8" />,
    title: 'Account Authorization',
    description: 'The authorization system controls which accounts can submit Comit transactions. Accounts must be explicitly authorized before they can interact with cross-VM features.',
    details: [
      'Explicit account authorization',
      'Root privilege management',
      'Authorization events and tracking',
      'Dev bypass for testing',
    ],
  },
  {
    icon: <Globe className="w-8 h-8" />,
    title: 'Consensus & Finality',
    description: 'X3 uses Aura for block authoring with 6-second slots and GRANDPA for Byzantine fault-tolerant finality, providing fast and secure transaction confirmation.',
    details: [
      '6-second block time',
      'Aura block authoring',
      'GRANDPA finality',
      'Future Tendermint-style BFT',
    ],
  },
];

const terminology = [
  { term: 'Comit', definition: 'An atomic cross-VM transaction containing payloads for both EVM and SVM execution.' },
  { term: 'Canonical Ledger', definition: 'The unified balance sheet tracking all assets across both virtual machines.' },
  { term: 'Atlas Kernel', definition: 'The core Substrate pallet orchestrating cross-VM operations and state management.' },
  { term: 'Prepare Root', definition: 'A cryptographic commitment to Comit inputs used for verification.' },
  { term: 'EVM Adapter', definition: 'The Frontier-based component that executes Ethereum-compatible smart contracts.' },
  { term: 'SVM Adapter', definition: 'The component that executes Solana-compatible programs using Sealevel.' },
  { term: 'Nonce', definition: 'A sequential counter for each account to prevent replay attacks on Comits.' },
  { term: 'Asset Registry', definition: 'The on-chain registry of all supported assets with their metadata.' },
];

export default function CoreConceptsPage() {
  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="badge badge-purple mb-4">Core Concepts</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Understanding X3 Atlas Sphere
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Master the fundamental concepts that make X3 Atlas Sphere the first truly 
              interoperable dual VM blockchain.
            </p>
          </div>
        </div>
      </section>

      {/* Main Concepts */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid gap-8">
            {concepts.map((concept, index) => (
              <div key={index} className="glass-card p-8">
                <div className="flex flex-col lg:flex-row gap-8">
                  <div className="flex-shrink-0">
                    <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center text-indigo-400">
                      {concept.icon}
                    </div>
                  </div>
                  <div className="flex-1">
                    <h2 className="text-2xl font-bold text-white mb-4">{concept.title}</h2>
                    <p className="text-gray-400 mb-6">{concept.description}</p>
                    <div className="grid sm:grid-cols-2 gap-4">
                      {concept.details.map((detail, i) => (
                        <div key={i} className="flex items-center text-sm text-gray-400">
                          <div className="w-2 h-2 rounded-full bg-indigo-500 mr-3" />
                          {detail}
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Architecture Diagram */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">System Architecture</h2>
          
          <div className="glass-card p-8">
            <div className="grid md:grid-cols-5 gap-4 items-center">
              {/* User */}
              <div className="text-center">
                <div className="w-16 h-16 rounded-xl bg-[#0a0a0a] mx-auto mb-2 flex items-center justify-center">
                  <Box className="w-8 h-8 text-gray-400" />
                </div>
                <div className="text-sm text-gray-400">User/DApp</div>
              </div>
              
              <div className="hidden md:flex justify-center">
                <ArrowRight className="w-6 h-6 text-gray-600" />
              </div>
              
              {/* Runtime */}
              <div className="text-center">
                <div className="w-16 h-16 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 mx-auto mb-2 flex items-center justify-center">
                  <Cpu className="w-8 h-8 text-white" />
                </div>
                <div className="text-sm text-gray-400">Atlas Kernel</div>
              </div>
              
              <div className="hidden md:flex justify-center">
                <GitBranch className="w-6 h-6 text-gray-600" />
              </div>
              
              {/* VMs */}
              <div className="flex flex-col gap-4">
                <div className="p-3 rounded-xl bg-blue-500/20 text-center">
                  <div className="text-sm font-semibold text-blue-400">EVM</div>
                  <div className="text-xs text-gray-500">Solidity</div>
                </div>
                <div className="p-3 rounded-xl bg-emerald-500/20 text-center">
                  <div className="text-sm font-semibold text-emerald-400">SVM</div>
                  <div className="text-xs text-gray-500">Rust</div>
                </div>
              </div>
            </div>
            
            <div className="mt-8 pt-8 border-t border-[#1a1a1a]">
              <div className="grid md:grid-cols-3 gap-6 text-center">
                <div>
                  <div className="flex items-center justify-center mb-2">
                    <Database className="w-5 h-5 text-purple-400 mr-2" />
                    <span className="font-semibold text-white">Canonical Ledger</span>
                  </div>
                  <p className="text-xs text-gray-600">Unified asset balances across VMs</p>
                </div>
                <div>
                  <div className="flex items-center justify-center mb-2">
                    <Lock className="w-5 h-5 text-purple-400 mr-2" />
                    <span className="font-semibold text-white">Authorization</span>
                  </div>
                  <p className="text-xs text-gray-600">Account permission management</p>
                </div>
                <div>
                  <div className="flex items-center justify-center mb-2">
                    <Coins className="w-5 h-5 text-purple-400 mr-2" />
                    <span className="font-semibold text-white">Asset Registry</span>
                  </div>
                  <p className="text-xs text-gray-600">Token metadata and tracking</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Terminology */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Terminology</h2>
          
          <div className="grid md:grid-cols-2 gap-4">
            {terminology.map((item, index) => (
              <div key={index} className="glass-card p-6">
                <h3 className="font-semibold text-indigo-400 mb-2">{item.term}</h3>
                <p className="text-sm text-gray-400">{item.definition}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Comit Flow */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Comit Transaction Lifecycle</h2>
          
          <div className="glass-card p-8">
            <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
              {[
                { step: 1, name: 'Submission', desc: 'Comit validated and queued' },
                { step: 2, name: 'Execution', desc: 'EVM + SVM payloads processed' },
                { step: 3, name: 'Verification', desc: 'Prepare root validated' },
                { step: 4, name: 'Finalization', desc: 'Ledger updated, events emitted' },
              ].map((item, index) => (
                <React.Fragment key={item.step}>
                  <div className="flex-1 text-center">
                    <div className="w-12 h-12 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 mx-auto mb-3 flex items-center justify-center text-white font-bold">
                      {item.step}
                    </div>
                    <h4 className="font-semibold text-white mb-1">{item.name}</h4>
                    <p className="text-xs text-gray-600">{item.desc}</p>
                  </div>
                  {index < 3 && (
                    <ArrowRight className="hidden md:block w-6 h-6 text-gray-600 flex-shrink-0" />
                  )}
                </React.Fragment>
              ))}n            </div>
            
            <div className="mt-8 pt-8 border-t border-[#1a1a1a]">
              <h4 className="font-semibold text-white mb-4">Events Emitted:</h4>
              <div className="flex flex-wrap gap-2">
                {['ComitSubmitted', 'ComitExecutionStarted', 'ComitExecutionCompleted', 'ComitFinalized'].map((event) => (
                  <span key={event} className="badge badge-info font-mono text-xs">{event}</span>
                ))}
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Next Steps */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Continue Learning</h2>
          
          <div className="grid md:grid-cols-3 gap-6">
            <Link href="/learn/architecture" className="glass-card-hover p-6 card-lift">
              <Layers className="w-8 h-8 text-indigo-400 mb-4" />
              <h3 className="font-semibold text-white mb-2">Architecture Deep Dive</h3>
              <p className="text-sm text-gray-400">Explore the technical architecture in detail.</p>
            </Link>
            <Link href="/developers/api" className="glass-card-hover p-6 card-lift">
              <Code className="w-8 h-8 text-indigo-400 mb-4" />
              <h3 className="font-semibold text-white mb-2">RPC API Reference</h3>
              <p className="text-sm text-gray-400">Learn about available RPC methods.</p>
            </Link>
            <Link href="/learn/tutorials" className="glass-card-hover p-6 card-lift">
              <Zap className="w-8 h-8 text-indigo-400 mb-4" />
              <h3 className="font-semibold text-white mb-2">Hands-on Tutorials</h3>
              <p className="text-sm text-gray-400">Start building with step-by-step guides.</p>
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
