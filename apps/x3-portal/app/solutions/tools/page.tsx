'use client';

import React from 'react';
import Link from 'next/link';
import {
  Wrench,
  Terminal,
  Code2,
  TestTube,
  Bug,
  Gauge,
  GitBranch,
  Package,
  Blocks,
  Cpu,
  ArrowRight,
  ExternalLink,
  CheckCircle2,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const devTools = [
  {
    name: 'X3 CLI',
    description: 'Command-line interface for building, testing, and deploying X3 STAR applications',
    icon: <Terminal className="w-6 h-6" />,
    features: ['Project scaffolding', 'Local devnet', 'Contract deployment'],
    installCmd: 'npm install -g @x3star/cli',
    docsUrl: '/developers/docs',
  },
  {
    name: 'Comit SDK',
    description: 'TypeScript SDK for building cross-VM applications with type safety',
    icon: <Code2 className="w-6 h-6" />,
    features: ['Full TypeScript support', 'Cross-VM transactions', 'Event subscriptions'],
    installCmd: 'npm install @x3star/sdk',
    docsUrl: '/developers/docs',
  },
  {
    name: 'X3 Test Framework',
    description: 'Testing framework for unit and integration tests on X3 STAR',
    icon: <TestTube className="w-6 h-6" />,
    features: ['Mock VM execution', 'Snapshot testing', 'Gas profiling'],
    installCmd: 'npm install -D @x3star/test',
    docsUrl: '/developers/docs',
  },
  {
    name: 'Debugger',
    description: 'Step-through debugger for EVM and SVM smart contracts',
    icon: <Bug className="w-6 h-6" />,
    features: ['Breakpoints', 'Variable inspection', 'Call stack traces'],
    installCmd: 'VS Code Extension',
    docsUrl: '/developers/docs',
  },
];

const infraTools = [
  {
    name: 'X3 Indexer',
    description: 'High-performance blockchain data indexing service',
    icon: <Blocks className="w-6 h-6" />,
    status: 'Live',
  },
  {
    name: 'Gas Station',
    description: 'Gasless transaction relay for better UX',
    icon: <Gauge className="w-6 h-6" />,
    status: 'Beta',
  },
  {
    name: 'Event Webhooks',
    description: 'Real-time notifications for on-chain events',
    icon: <GitBranch className="w-6 h-6" />,
    status: 'Live',
  },
  {
    name: 'IPFS Pinning',
    description: 'Decentralized storage integration for dApps',
    icon: <Package className="w-6 h-6" />,
    status: 'Live',
  },
];

const templates = [
  { name: 'Cross-VM DEX', description: 'Decentralized exchange template', lang: 'Solidity + Rust' },
  { name: 'NFT Marketplace', description: 'Full-featured NFT platform', lang: 'TypeScript' },
  { name: 'DAO Framework', description: 'Governance and voting system', lang: 'Solidity' },
  { name: 'Token Launchpad', description: 'Token creation and distribution', lang: 'TypeScript' },
];

export default function ToolsPage() {
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
            <div className="badge badge-info mt-4 mb-4">Developer Tools</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Build Faster with X3 Tools
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Everything you need to build, test, and deploy applications on X3 STAR. 
              From CLI tools to SDKs, we've got you covered.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                Get Started
              </Link>
              <a href="https://github.com/x3star" className="btn-secondary">
                <GitBranch className="w-4 h-4 mr-2" />
                GitHub
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Dev Tools */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Development Tools</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {devTools.map((tool, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-start justify-between mb-4">
                  <div className="p-3 rounded-xl bg-orange-500/20 text-orange-400">
                    {tool.icon}
                  </div>
                  <Link href={tool.docsUrl} className="text-gray-400 hover:text-orange-400">
                    <ExternalLink className="w-5 h-5" />
                  </Link>
                </div>
                <h3 className="text-xl font-semibold text-white mb-2">{tool.name}</h3>
                <p className="text-gray-400 mb-4">{tool.description}</p>
                <div className="flex flex-wrap gap-2 mb-4">
                  {tool.features.map((feature, i) => (
                    <span key={i} className="badge badge-default">{feature}</span>
                  ))}
                </div>
                <div className="p-3 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a]">
                  <code className="text-sm text-cyan-400">{tool.installCmd}</code>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Infrastructure */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Infrastructure Services</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {infraTools.map((tool, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <div className="p-2 rounded-lg bg-blue-500/20 text-blue-400">
                    {tool.icon}
                  </div>
                  <span className={`badge ${tool.status === 'Live' ? 'badge-success' : 'badge-warning'}`}>
                    {tool.status}
                  </span>
                </div>
                <h3 className="font-semibold text-white mb-2">{tool.name}</h3>
                <p className="text-sm text-gray-400">{tool.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Templates */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Starter Templates</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {templates.map((template, index) => (
              <div key={index} className="glass-card p-6 hover:border-orange-500/30 transition-colors cursor-pointer group">
                <h3 className="font-semibold text-white mb-2 group-hover:text-orange-400 transition-colors">
                  {template.name}
                </h3>
                <p className="text-sm text-gray-400 mb-4">{template.description}</p>
                <span className="text-xs text-gray-500">{template.lang}</span>
              </div>
            ))}
          </div>
          
          <div className="mt-8 text-center">
            <Link href="/developers/cookbook" className="btn-secondary">
              View All Templates
            </Link>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Wrench className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Start Building Today
          </h2>
          <p className="text-gray-400 mb-8">
            Get up and running in minutes with our comprehensive tooling.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Read the Docs
            </Link>
            <Link href="/community/forum" className="btn-secondary">
              Get Help
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
