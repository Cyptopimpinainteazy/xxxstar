'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  BookOpen,
  Code,
  Terminal,
  Layers,
  Database,
  Zap,
  Shield,
  ChevronRight,
  Search,
  Menu,
  X,
  FileText,
  Cpu,
  Globe,
  Settings,
  Box,
  ArrowRight,
} from 'lucide-react';

const docSections = [
  {
    id: 'getting-started',
    title: 'Getting Started',
    icon: <Zap className="w-5 h-5" />,
    items: [
      { title: 'Introduction', href: '/developers/docs/intro' },
      { title: 'Quick Start', href: '/developers/docs/quickstart' },
      { title: 'Installation', href: '/developers/docs/installation' },
      { title: 'Configuration', href: '/developers/docs/configuration' },
    ],
  },
  {
    id: 'core-concepts',
    title: 'Core Concepts',
    icon: <Cpu className="w-5 h-5" />,
    items: [
      { title: 'Dual VM Architecture', href: '/developers/docs/dual-vm' },
      { title: 'Atlas Kernel', href: '/developers/docs/atlas-kernel' },
      { title: 'Comit Transactions', href: '/developers/docs/comits' },
      { title: 'Canonical Ledger', href: '/developers/docs/canonical-ledger' },
      { title: 'Account Authorization', href: '/developers/docs/authorization' },
    ],
  },
  {
    id: 'evm-development',
    title: 'EVM Development',
    icon: <Code className="w-5 h-5" />,
    items: [
      { title: 'Deploying Contracts', href: '/developers/docs/evm-deploy' },
      { title: 'Interacting with Contracts', href: '/developers/docs/evm-interact' },
      { title: 'Hardhat Integration', href: '/developers/docs/hardhat' },
      { title: 'Foundry Integration', href: '/developers/docs/foundry' },
      { title: 'ERC Standards', href: '/developers/docs/erc-standards' },
    ],
  },
  {
    id: 'svm-development',
    title: 'SVM Development',
    icon: <Terminal className="w-5 h-5" />,
    items: [
      { title: 'Writing Programs', href: '/developers/docs/svm-programs' },
      { title: 'Anchor Framework', href: '/developers/docs/anchor' },
      { title: 'Program Deployment', href: '/developers/docs/svm-deploy' },
      { title: 'Account Model', href: '/developers/docs/svm-accounts' },
      { title: 'SPL Tokens', href: '/developers/docs/spl-tokens' },
    ],
  },
  {
    id: 'cross-vm',
    title: 'Cross-VM Operations',
    icon: <Layers className="w-5 h-5" />,
    items: [
      { title: 'Creating Comits', href: '/developers/docs/creating-comits' },
      { title: 'Atomic Execution', href: '/developers/docs/atomic-execution' },
      { title: 'Cross-VM Assets', href: '/developers/docs/cross-vm-assets' },
      { title: 'Error Handling', href: '/developers/docs/error-handling' },
      { title: 'Best Practices', href: '/developers/docs/best-practices' },
    ],
  },
  {
    id: 'ai-swarm',
    title: 'AI Swarm & Compute',
    icon: <Cpu className="w-5 h-5" />,
    items: [
      { title: 'AI Swarm Overview', href: '/x3/swarm' },
      { title: 'Prediction Markets', href: '/x3/swarm/predictions' },
      { title: 'Blockspace Auctions', href: '/x3/swarm/auctions' },
      { title: 'GPU Marketplace', href: '/x3/swarm/gpu' },
      { title: 'Agent Development', href: '/developers/docs/ai-agents' },
    ],
  },
  {
    id: 'node-operations',
    title: 'Node Operations',
    icon: <Database className="w-5 h-5" />,
    items: [
      { title: 'Running a Node', href: '/developers/docs/run-node' },
      { title: 'Becoming a Validator', href: '/developers/docs/validator' },
      { title: 'Chain Specifications', href: '/developers/docs/chain-spec' },
      { title: 'Key Management', href: '/developers/docs/keys' },
      { title: 'Monitoring', href: '/developers/docs/monitoring' },
    ],
  },
];

const quickLinks = [
  { title: 'RPC API Reference', href: '/developers/api', icon: <Globe className="w-4 h-4" /> },
  { title: 'Cookbook Examples', href: '/developers/cookbook', icon: <BookOpen className="w-4 h-4" /> },
  { title: 'AI Swarm Hub', href: '/x3/swarm', icon: <Cpu className="w-4 h-4" /> },
  { title: 'GitHub Repository', href: 'https://github.com/atlas-sphere', icon: <Code className="w-4 h-4" /> },
  { title: 'SDKs & Tools', href: '/developers/sdks', icon: <Box className="w-4 h-4" /> },
];

export default function DocsPage() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero with unique Developers section header */}
      <section className="py-16 relative overflow-hidden border-b border-[#1a1a1a] page-header-developers">
        <div className="absolute inset-0 bg-gradient-to-br from-violet-950/20 via-black to-purple-950/10" />
        <div className="absolute inset-0 mesh-gradient opacity-10" />
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="inline-flex items-center px-3 py-1 rounded-full bg-violet-500/10 border border-violet-500/30 mb-4">
              <Code className="w-4 h-4 mr-2 text-violet-400" />
              <span className="text-sm text-violet-300">Documentation</span>
            </div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              X3 STAR <span className="text-violet-400">Documentation</span>
            </h1>
            <p className="text-xl text-gray-500 mb-8">
              Official documentation for the X3 Atlas Sphere blockchain. 
              Everything you need to build on the first dual VM Layer-1.
            </p>
            
            {/* Search */}
            <div className="relative max-w-xl">
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-600" />
              <input
                type="text"
                placeholder="Search documentation..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-12 pr-4 py-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-600 focus:outline-none focus:border-violet-500"
              />
            </div>
          </div>
        </div>
      </section>

      <div className="container-wide py-12">
        <div className="grid lg:grid-cols-4 gap-8">
          {/* Sidebar */}
          <aside className="lg:col-span-1">
            {/* Mobile toggle */}
            <button
              className="lg:hidden flex items-center w-full p-4 mb-4 glass-card"
              onClick={() => setSidebarOpen(!sidebarOpen)}
            >
              {sidebarOpen ? <X className="w-5 h-5 mr-2" /> : <Menu className="w-5 h-5 mr-2" />}
              Documentation Menu
            </button>

            <nav className={`space-y-6 ${sidebarOpen ? 'block' : 'hidden lg:block'}`}>
              {/* Quick Links */}
              <div className="glass-card p-4">
                <h3 className="text-sm font-semibold text-gray-500 uppercase tracking-wider mb-3">
                  Quick Links
                </h3>
                <div className="space-y-2">
                  {quickLinks.map((link) => (
                    <Link
                      key={link.title}
                      href={link.href}
                      className="flex items-center text-sm text-gray-400 hover:text-white hover:bg-[#1a1a1a] rounded-lg px-3 py-2 transition-colors"
                    >
                      <span className="text-violet-400 mr-2">{link.icon}</span>
                      {link.title}
                    </Link>
                  ))}
                </div>
              </div>

              {/* Doc Sections */}
              {docSections.map((section) => (
                <div key={section.id}>
                  <h3 className="flex items-center text-sm font-semibold text-white mb-3">
                    <span className="text-orange-400 mr-2">{section.icon}</span>
                    {section.title}
                  </h3>
                  <ul className="space-y-1 ml-7">
                    {section.items.map((item) => (
                      <li key={item.title}>
                        <Link
                          href={item.href}
                          className="block text-sm text-gray-500 hover:text-white hover:bg-[#1a1a1a] rounded-lg px-3 py-2 transition-colors"
                        >
                          {item.title}
                        </Link>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </nav>
          </aside>

          {/* Main Content */}
          <main className="lg:col-span-3">
            {/* Welcome Card */}
            <div className="glass-card p-8 mb-8">
              <h2 className="text-2xl font-bold text-white mb-4">Welcome to X3 STAR Docs</h2>
              <p className="text-gray-500 mb-6">
                X3 STAR (Atlas Sphere) is a next-generation Layer-1 blockchain that enables native 
                interoperability between EVM and SVM. This documentation will help you understand 
                the architecture and start building cross-VM applications.
              </p>
              <div className="flex flex-wrap gap-4">
                <Link href="/developers/docs/quickstart" className="btn-primary">
                  Quick Start Guide
                </Link>
                <Link href="/learn/tutorials" className="btn-secondary">
                  View Tutorials
                </Link>
              </div>
            </div>

            {/* Section Cards */}
            <div className="grid md:grid-cols-2 gap-6">
              {docSections.map((section) => (
                <div key={section.id} className="glass-card-hover p-6 card-lift">
                  <div className="flex items-center mb-4">
                    <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400 mr-3">
                      {section.icon}
                    </div>
                    <h3 className="font-semibold text-white">{section.title}</h3>
                  </div>
                  <ul className="space-y-2 mb-4">
                    {section.items.slice(0, 3).map((item) => (
                      <li key={item.title}>
                        <Link
                          href={item.href}
                          className="text-sm text-gray-500 hover:text-violet-400 flex items-center"
                        >
                          <ChevronRight className="w-4 h-4 mr-1" />
                          {item.title}
                        </Link>
                      </li>
                    ))}
                  </ul>
                  {section.items.length > 3 && (
                    <Link
                      href={section.items[0].href}
                      className="text-sm text-orange-400 hover:text-orange-300 flex items-center"
                    >
                      View all {section.items.length} articles
                      <ArrowRight className="w-4 h-4 ml-1" />
                    </Link>
                  )}
                </div>
              ))}
            </div>

            {/* Code Example */}
            <div className="mt-8 glass-card p-6">
              <h3 className="text-lg font-semibold text-white mb-4">Quick Example: Query the Network</h3>
              <div className="code-block">
                <pre className="text-sm text-gray-400">
{`// Query Atlas Kernel RPC
const response = await fetch('https://rpc.testnet.atlas-sphere.io', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    id: 1,
    jsonrpc: '2.0',
    method: 'atlasKernel_getAuthorizedAccounts',
    params: [null]
  })
});

const data = await response.json();
console.log('Authorized accounts:', data.result);`}
                </pre>
              </div>
            </div>

            {/* Help Section */}
            <div className="mt-8 p-6 rounded-2xl bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/20">
              <h3 className="text-lg font-semibold text-white mb-2">Need Help?</h3>
              <p className="text-gray-500 mb-4">
                Can't find what you're looking for? Join our community for support.
              </p>
              <div className="flex flex-wrap gap-4">
                <Link href="https://discord.gg/x3atlas" className="btn-ghost text-orange-400">
                  Discord Community
                </Link>
                <Link href="/community/forum" className="btn-ghost text-orange-400">
                  Developer Forum
                </Link>
                <Link href="https://github.com/atlas-sphere/atlas-sphere/issues" className="btn-ghost text-orange-400">
                  GitHub Issues
                </Link>
              </div>
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
