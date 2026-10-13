'use client';

import React from 'react';
import Link from 'next/link';
import {
  Layers,
  Cpu,
  Database,
  Network,
  Shield,
  Zap,
  ArrowRight,
  GitBranch,
  Box,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const layers = [
  {
    name: 'Application Layer',
    description: 'dApps, wallets, and user interfaces',
    components: ['Web3 dApps', 'Mobile Wallets', 'Block Explorer', 'DEX Interfaces'],
    color: 'from-purple-500 to-pink-500',
  },
  {
    name: 'Smart Contract Layer',
    description: 'Dual VM execution environment',
    components: ['EVM Contracts (Solidity)', 'SVM Programs (Rust)', 'Cross-VM Bridge', 'Token Standards'],
    color: 'from-cyan-500 to-blue-500',
  },
  {
    name: 'Orchestration Layer',
    description: 'Atlas Kernel for cross-VM coordination',
    components: ['Comit Transactions', 'State Synchronization', 'Nonce Management', 'Fee Handling'],
    color: 'from-orange-500 to-amber-500',
  },
  {
    name: 'Consensus Layer',
    description: 'Block production and finality',
    components: ['Aura Block Authoring', 'GRANDPA Finality', 'Validator Selection', 'Slot Management'],
    color: 'from-emerald-500 to-green-500',
  },
  {
    name: 'Network Layer',
    description: 'P2P communication and data availability',
    components: ['libp2p Networking', 'Gossip Protocol', 'Peer Discovery', 'Block Propagation'],
    color: 'from-blue-500 to-indigo-500',
  },
];

const coreComponents = [
  {
    name: 'Atlas Kernel',
    description: 'The heart of X3 STAR - orchestrates atomic cross-VM transactions',
    icon: <Cpu className="w-6 h-6" />,
    features: [
      'Comit transaction processing',
      'Canonical ledger management',
      'Cross-VM state coordination',
      'Deterministic ordering',
    ],
  },
  {
    name: 'EVM Integration',
    description: 'Full Ethereum Virtual Machine compatibility via Frontier',
    icon: <Box className="w-6 h-6" />,
    features: [
      'Solidity smart contracts',
      'Web3 RPC compatibility',
      'ERC-20/721/1155 support',
      'Ethers.js/Web3.js support',
    ],
  },
  {
    name: 'SVM Integration',
    description: 'Solana Virtual Machine for high-performance programs',
    icon: <Zap className="w-6 h-6" />,
    features: [
      'Rust/Anchor programs',
      'Parallel execution',
      'Token extensions',
      'Cross-program invocation',
    ],
  },
  {
    name: 'Substrate Runtime',
    description: 'FRAME-based runtime for modularity and upgradability',
    icon: <Layers className="w-6 h-6" />,
    features: [
      'Forkless upgrades',
      'Modular pallets',
      'On-chain governance',
      'WASM execution',
    ],
  },
];

const comitFlow = [
  { step: 1, title: 'Submission', desc: 'User submits Comit with EVM + SVM payloads' },
  { step: 2, title: 'Validation', desc: 'Verify nonce, authorization, payload sizes' },
  { step: 3, title: 'Execution', desc: 'Execute both VM payloads atomically' },
  { step: 4, title: 'Verification', desc: 'Recompute prepare_root from inputs' },
  { step: 5, title: 'Finalization', desc: 'Update canonical ledger, emit events' },
];

export default function ArchitecturePage() {
  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-30">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <Link href="/learn" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Learn
            </Link>
            <div className="badge badge-info mt-4 mb-4">Architecture</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              System Architecture
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Deep dive into X3 STAR's innovative dual-VM architecture and how 
              it enables native EVM-SVM interoperability.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                Technical Docs
              </Link>
              <a href="https://github.com/x3star" className="btn-secondary">
                View Source
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Layer Stack */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Architecture Stack</h2>
          
          <div className="space-y-4">
            {layers.map((layer, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex flex-col md:flex-row md:items-center gap-4">
                  <div className={`w-16 h-16 rounded-xl bg-gradient-to-br ${layer.color} flex items-center justify-center shrink-0`}>
                    <span className="text-2xl font-bold text-white">{layers.length - index}</span>
                  </div>
                  <div className="flex-1">
                    <h3 className="text-xl font-semibold text-white mb-1">{layer.name}</h3>
                    <p className="text-gray-400 mb-3">{layer.description}</p>
                    <div className="flex flex-wrap gap-2">
                      {layer.components.map((component, i) => (
                        <span key={i} className="badge badge-default">{component}</span>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Core Components */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Core Components</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {coreComponents.map((component, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-3 rounded-xl bg-orange-500/20 text-orange-400 w-fit mb-4">
                  {component.icon}
                </div>
                <h3 className="text-xl font-semibold text-white mb-2">{component.name}</h3>
                <p className="text-gray-400 mb-4">{component.description}</p>
                <ul className="space-y-2">
                  {component.features.map((feature, i) => (
                    <li key={i} className="flex items-center text-gray-400 text-sm">
                      <div className="w-1.5 h-1.5 rounded-full bg-orange-400 mr-2" />
                      {feature}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Comit Flow */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Comit Transaction Flow</h2>
          
          <div className="glass-card p-8">
            <div className="flex flex-col md:flex-row items-start justify-between gap-6">
              {comitFlow.map((step, index) => (
                <React.Fragment key={index}>
                  <div className="flex-1 text-center">
                    <div className="w-12 h-12 rounded-full bg-orange-500/20 flex items-center justify-center mx-auto mb-3">
                      <span className="text-orange-400 font-bold">{step.step}</span>
                    </div>
                    <h4 className="font-semibold text-white mb-1">{step.title}</h4>
                    <p className="text-sm text-gray-400">{step.desc}</p>
                  </div>
                  {index < comitFlow.length - 1 && (
                    <ArrowRight className="w-6 h-6 text-gray-600 hidden md:block mt-4" />
                  )}
                </React.Fragment>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* Dual VM Diagram */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Dual-VM Architecture</h2>
          
          <div className="glass-card p-8">
            <div className="grid md:grid-cols-3 gap-8">
              {/* EVM */}
              <div className="p-6 rounded-xl bg-purple-500/10 border border-purple-500/30">
                <h3 className="text-lg font-semibold text-purple-400 mb-4">EVM Domain</h3>
                <ul className="space-y-2 text-sm text-gray-400">
                  <li>• Solidity Contracts</li>
                  <li>• ERC Token Standards</li>
                  <li>• Web3 Compatibility</li>
                  <li>• Gas Metering</li>
                </ul>
              </div>
              
              {/* Atlas Kernel */}
              <div className="p-6 rounded-xl bg-orange-500/10 border border-orange-500/30">
                <h3 className="text-lg font-semibold text-orange-400 mb-4">Atlas Kernel</h3>
                <ul className="space-y-2 text-sm text-gray-400">
                  <li>• Atomic Orchestration</li>
                  <li>• State Synchronization</li>
                  <li>• Canonical Ledger</li>
                  <li>• Cross-VM Bridge</li>
                </ul>
              </div>
              
              {/* SVM */}
              <div className="p-6 rounded-xl bg-cyan-500/10 border border-cyan-500/30">
                <h3 className="text-lg font-semibold text-cyan-400 mb-4">SVM Domain</h3>
                <ul className="space-y-2 text-sm text-gray-400">
                  <li>• Rust Programs</li>
                  <li>• Token Extensions</li>
                  <li>• Parallel Execution</li>
                  <li>• Compute Units</li>
                </ul>
              </div>
            </div>
            
            {/* Unified Layer */}
            <div className="mt-8 p-6 rounded-xl bg-emerald-500/10 border border-emerald-500/30">
              <h3 className="text-lg font-semibold text-emerald-400 mb-4 text-center">
                Substrate Runtime (FRAME)
              </h3>
              <div className="flex flex-wrap justify-center gap-4 text-sm text-gray-400">
                <span>Consensus (Aura + GRANDPA)</span>
                <span>•</span>
                <span>Forkless Upgrades</span>
                <span>•</span>
                <span>On-Chain Governance</span>
                <span>•</span>
                <span>WASM Runtime</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <GitBranch className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Explore the Codebase
          </h2>
          <p className="text-gray-400 mb-8">
            Dive into the implementation details in our open source repository.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="https://github.com/x3star" className="btn-primary">
              View on GitHub
            </a>
            <Link href="/developers/docs" className="btn-secondary">
              Technical Docs
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
