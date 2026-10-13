'use client';

import React from 'react';
import Link from 'next/link';
import {
  Bot,
  Brain,
  Cpu,
  Database,
  Network,
  Shield,
  Zap,
  Code,
  ArrowRight,
  CheckCircle2,
  Sparkles,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const aiUseCases = [
  {
    title: 'AI Agents',
    description: 'Deploy autonomous AI agents that can execute blockchain transactions and manage assets',
    icon: <Bot className="w-6 h-6" />,
    features: ['Wallet Control', 'DeFi Automation', 'Cross-VM Operations'],
    color: 'from-purple-500 to-indigo-500',
  },
  {
    title: 'On-Chain ML',
    description: 'Run machine learning models on-chain with verifiable inference',
    icon: <Brain className="w-6 h-6" />,
    features: ['Verifiable Inference', 'Model Registry', 'Trustless ML'],
    color: 'from-cyan-500 to-blue-500',
  },
  {
    title: 'AI Data Markets',
    description: 'Create decentralized marketplaces for AI training data and model weights',
    icon: <Database className="w-6 h-6" />,
    features: ['Data Tokenization', 'Access Control', 'Revenue Sharing'],
    color: 'from-emerald-500 to-green-500',
  },
  {
    title: 'Compute Networks',
    description: 'Build decentralized GPU and compute networks for AI workloads',
    icon: <Cpu className="w-6 h-6" />,
    features: ['GPU Marketplace', 'Task Distribution', 'Proof of Compute'],
    color: 'from-orange-500 to-amber-500',
  },
];

const advantages = [
  {
    title: 'Dual VM Flexibility',
    description: 'Leverage EVM for complex smart contracts and SVM for high-throughput operations',
    icon: <Network className="w-5 h-5" />,
  },
  {
    title: 'Verifiable Computation',
    description: 'Ensure AI outputs are verifiable and tamper-proof through blockchain attestation',
    icon: <Shield className="w-5 h-5" />,
  },
  {
    title: 'High Performance',
    description: 'Execute AI-related transactions with low latency using the SVM adapter',
    icon: <Zap className="w-5 h-5" />,
  },
  {
    title: 'Composability',
    description: 'Compose AI modules with DeFi, NFTs, and other blockchain primitives',
    icon: <Code className="w-5 h-5" />,
  },
];

const aiProjects = [
  { name: 'AtlasGPT', category: 'AI Agents', status: 'Live', description: 'Autonomous trading agent' },
  { name: 'VerifyML', category: 'On-Chain ML', status: 'Beta', description: 'Verifiable inference' },
  { name: 'DataDAO', category: 'Data Markets', status: 'Live', description: 'Training data marketplace' },
  { name: 'ComputeX3', category: 'Compute', status: 'Coming Soon', description: 'GPU network' },
];

const codeExample = `// Deploy an AI agent that executes cross-VM operations
import { AtlasAgent, AgentConfig } from '@x3/agent-sdk';

const agent = new AtlasAgent({
  name: 'TradingBot',
  capabilities: ['swap', 'lend', 'stake'],
  crossVM: true,
  
  // Agent decision logic
  async onTrigger(context) {
    const { evmState, svmState } = context;
    
    // AI model inference
    const decision = await this.model.predict(evmState, svmState);
    
    if (decision.shouldSwap) {
      return this.createComit({
        evmPayload: decision.evmAction,
        svmPayload: decision.svmAction,
      });
    }
  }
});

// Register and fund the agent
await agent.deploy({ initialFunds: '100 ATLAS' });`;

export default function AISolutionsPage() {
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
            <div className="badge badge-purple mt-4 mb-4">AI & ML</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              AI-Powered Blockchain
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Build intelligent applications with AI agents, on-chain ML, and decentralized 
              compute. X3 STAR provides the infrastructure for the next generation of AI x Crypto.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                <Sparkles className="w-4 h-4 mr-2" />
                Start Building
              </Link>
              <Link href="/developers/cookbook" className="btn-secondary">
                View Examples
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Use Cases */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">AI Use Cases</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {aiUseCases.map((useCase, index) => (
              <div key={index} className="glass-card p-6">
                <div className={`p-3 rounded-xl bg-gradient-to-br ${useCase.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{useCase.icon}</span>
                </div>
                <h3 className="text-xl font-semibold text-white mb-2">{useCase.title}</h3>
                <p className="text-gray-400 mb-4">{useCase.description}</p>
                <div className="flex flex-wrap gap-2">
                  {useCase.features.map((feature, i) => (
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
          <h2 className="text-2xl font-bold text-white mb-8">Why X3 STAR for AI?</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {advantages.map((advantage, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-2 rounded-lg bg-purple-500/20 text-purple-400 w-fit mb-4">
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
              <h2 className="text-2xl font-bold text-white mb-4">Deploy AI Agents</h2>
              <p className="text-gray-400 mb-6">
                Create autonomous AI agents that can execute complex cross-VM operations, 
                manage assets, and interact with DeFi protocols—all with verifiable execution.
              </p>
              <ul className="space-y-3">
                {[
                  'Autonomous wallet and asset management',
                  'Cross-VM operation execution',
                  'Verifiable on-chain decisions',
                  'Integration with AI/ML models',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-5 h-5 text-purple-500 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center justify-between">
                <span className="text-sm text-gray-400">ai-agent.ts</span>
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
          <h2 className="text-2xl font-bold text-white mb-8">AI Ecosystem</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {aiProjects.map((project, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="font-semibold text-white">{project.name}</h3>
                  <span className={`badge ${project.status === 'Live' ? 'badge-success' : project.status === 'Beta' ? 'badge-warning' : 'badge-default'}`}>
                    {project.status}
                  </span>
                </div>
                <p className="text-sm text-gray-400 mb-2">{project.description}</p>
                <p className="text-xs text-gray-500">{project.category}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">
            Build the Future of AI x Crypto
          </h2>
          <p className="text-gray-400 mb-8">
            Get started with X3 STAR's AI infrastructure and join the next wave of intelligent applications.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Documentation
            </Link>
            <Link href="/community/grants" className="btn-secondary">
              AI Grants Program
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
