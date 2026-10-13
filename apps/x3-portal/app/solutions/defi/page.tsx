'use client';

import React from 'react';
import Link from 'next/link';
import {
  BarChart3,
  ArrowLeftRight,
  Droplets,
  Lock,
  TrendingUp,
  Shield,
  Zap,
  Globe,
  ArrowRight,
  ExternalLink,
  CheckCircle2,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const defiProducts = [
  {
    title: 'Decentralized Exchanges',
    description: 'Build AMMs and order books that span both EVM and SVM with unified liquidity',
    icon: <ArrowLeftRight className="w-6 h-6" />,
    features: ['Cross-VM Swaps', 'Unified Liquidity', 'MEV Protection'],
    color: 'from-orange-500 to-amber-500',
  },
  {
    title: 'Lending Protocols',
    description: 'Create lending markets with assets from both virtual machines as collateral',
    icon: <TrendingUp className="w-6 h-6" />,
    features: ['Cross-VM Collateral', 'Flash Loans', 'Variable Rates'],
    color: 'from-blue-500 to-cyan-500',
  },
  {
    title: 'Liquidity Provision',
    description: 'Provide liquidity across both VMs and earn rewards from unified pools',
    icon: <Droplets className="w-6 h-6" />,
    features: ['Single-sided LP', 'Concentrated Liquidity', 'Auto-compounding'],
    color: 'from-emerald-500 to-green-500',
  },
  {
    title: 'Yield Aggregators',
    description: 'Optimize yield strategies spanning EVM and SVM protocols',
    icon: <BarChart3 className="w-6 h-6" />,
    features: ['Cross-VM Vaults', 'Strategy Automation', 'Risk Management'],
    color: 'from-purple-500 to-indigo-500',
  },
];

const advantages = [
  {
    title: 'Atomic Cross-VM Swaps',
    description: 'Execute swaps between EVM and SVM assets in a single atomic transaction',
    icon: <Zap className="w-5 h-5" />,
  },
  {
    title: 'Unified Liquidity',
    description: 'Access liquidity from both virtual machines without bridging delays',
    icon: <Droplets className="w-5 h-5" />,
  },
  {
    title: 'MEV Protection',
    description: 'Built-in ordering guarantees prevent front-running and sandwich attacks',
    icon: <Shield className="w-5 h-5" />,
  },
  {
    title: 'Composability',
    description: 'Compose protocols across VMs—use SVM assets in EVM protocols and vice versa',
    icon: <Globe className="w-5 h-5" />,
  },
];

const protocols = [
  { name: 'X3 Swap', type: 'DEX', tvl: '$45M', description: 'Cross-VM AMM' },
  { name: 'Atlas Lend', type: 'Lending', tvl: '$32M', description: 'Unified lending' },
  { name: 'Sphere Yield', type: 'Yield', tvl: '$28M', description: 'Yield optimizer' },
  { name: 'Dual Pool', type: 'Liquidity', tvl: '$21M', description: 'Cross-VM pools' },
];

const codeExample = `// Execute a cross-VM swap
const swap = await atlasClient.comit({
  evmPayload: {
    contract: WETH_ADDRESS,
    method: 'approve',
    args: [ROUTER_ADDRESS, amount],
  },
  svmPayload: {
    program: DEX_PROGRAM,
    instruction: 'swap',
    accounts: [user, poolA, poolB],
    data: { amountIn: amount, minOut: minReceive },
  },
});

// Both execute atomically or both fail
console.log('Swap completed:', swap.comitId);`;

export default function DeFiSolutionsPage() {
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
            <Link href="/solutions" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Solutions
            </Link>
            <div className="badge badge-success mt-4 mb-4">DeFi</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Cross-VM DeFi
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Build DeFi protocols that leverage both EVM and SVM. Unified liquidity, 
              atomic swaps, and composability across virtual machines.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                Start Building
              </Link>
              <Link href="/developers/cookbook" className="btn-secondary">
                View Examples
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Products */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">DeFi Building Blocks</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {defiProducts.map((product, index) => (
              <div key={index} className="glass-card p-6">
                <div className={`p-3 rounded-xl bg-gradient-to-br ${product.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{product.icon}</span>
                </div>
                <h3 className="text-xl font-semibold text-white mb-2">{product.title}</h3>
                <p className="text-gray-400 mb-4">{product.description}</p>
                <div className="flex flex-wrap gap-2">
                  {product.features.map((feature, i) => (
                    <span key={i} className="badge badge-default">{feature}</span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Advantages */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">X3 STAR DeFi Advantages</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {advantages.map((advantage, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400 w-fit mb-4">
                  {advantage.icon}
                </div>
                <h3 className="font-semibold text-white mb-2">{advantage.title}</h3>
                <p className="text-sm text-gray-400">{advantage.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Code Example */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Simple Cross-VM DeFi</h2>
              <p className="text-gray-400 mb-6">
                Build complex DeFi operations with simple, atomic transactions. 
                The Atlas Kernel handles cross-VM coordination automatically.
              </p>
              <ul className="space-y-3">
                {[
                  'Atomic execution across both VMs',
                  'Single transaction, single fee',
                  'Automatic rollback on failure',
                  'Built-in MEV protection',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-5 h-5 text-emerald-500 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center justify-between">
                <span className="text-sm text-gray-400">cross-vm-swap.ts</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{codeExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Ecosystem */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">DeFi Ecosystem</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {protocols.map((protocol, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="font-semibold text-white">{protocol.name}</h3>
                  <span className="badge badge-default">{protocol.type}</span>
                </div>
                <p className="text-sm text-gray-400 mb-2">{protocol.description}</p>
                <p className="text-lg font-bold gradient-text">{protocol.tvl} TVL</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">
            Build the Future of DeFi
          </h2>
          <p className="text-gray-400 mb-8">
            Get started with X3 STAR's DeFi primitives and build protocols that span both VMs.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Documentation
            </Link>
            <Link href="/community/grants" className="btn-secondary">
              Apply for Grant
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
