'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  ChevronRight,
  ChevronLeft,
  Search,
  Menu,
  X,
  BookOpen,
  Code,
  Terminal,
  Layers,
  Database,
  Zap,
  Cpu,
  Globe,
  Box,
  ExternalLink,
  Copy,
  Check,
} from 'lucide-react';

export interface DocSection {
  id: string;
  title: string;
  icon: React.ReactNode;
  items: { title: string; href: string; description?: string }[];
}

const docSections: DocSection[] = [
  {
    id: 'getting-started',
    title: 'Getting Started',
    icon: <Zap className="w-5 h-5" />,
    items: [
      { title: 'Introduction', href: '/developers/docs/intro', description: 'Overview of X3 Atlas Sphere' },
      { title: 'Quick Start', href: '/developers/docs/quickstart', description: 'Get up and running in minutes' },
      { title: 'Installation', href: '/developers/docs/installation', description: 'Install dependencies and tools' },
      { title: 'Configuration', href: '/developers/docs/configuration', description: 'Configure your environment' },
    ],
  },
  {
    id: 'core-concepts',
    title: 'Core Concepts',
    icon: <Cpu className="w-5 h-5" />,
    items: [
      { title: 'Dual VM Architecture', href: '/developers/docs/dual-vm', description: 'How EVM and SVM coexist' },
      { title: 'Atlas Kernel', href: '/developers/docs/atlas-kernel', description: 'The orchestration layer' },
      { title: 'Comit Transactions', href: '/developers/docs/comits', description: 'Cross-VM atomic operations' },
      { title: 'Canonical Ledger', href: '/developers/docs/canonical-ledger', description: 'Unified asset storage' },
      { title: 'Account Authorization', href: '/developers/docs/authorization', description: 'Permission system' },
    ],
  },
  {
    id: 'evm-development',
    title: 'EVM Development',
    icon: <Code className="w-5 h-5" />,
    items: [
      { title: 'Deploying Contracts', href: '/developers/docs/evm-deploy', description: 'Deploy Solidity contracts' },
      { title: 'Interacting with Contracts', href: '/developers/docs/evm-interact', description: 'Call contract methods' },
      { title: 'Hardhat Integration', href: '/developers/docs/hardhat', description: 'Use Hardhat with X3' },
      { title: 'Foundry Integration', href: '/developers/docs/foundry', description: 'Use Foundry with X3' },
      { title: 'ERC Standards', href: '/developers/docs/erc-standards', description: 'Token standards support' },
    ],
  },
  {
    id: 'svm-development',
    title: 'SVM Development',
    icon: <Terminal className="w-5 h-5" />,
    items: [
      { title: 'Writing Programs', href: '/developers/docs/svm-programs', description: 'Create Solana programs' },
      { title: 'Anchor Framework', href: '/developers/docs/anchor', description: 'Use Anchor on X3' },
      { title: 'Program Deployment', href: '/developers/docs/svm-deploy', description: 'Deploy to SVM' },
      { title: 'Account Model', href: '/developers/docs/svm-accounts', description: 'SVM account structure' },
      { title: 'SPL Tokens', href: '/developers/docs/spl-tokens', description: 'Solana token standard' },
    ],
  },
  {
    id: 'cross-vm',
    title: 'Cross-VM Operations',
    icon: <Layers className="w-5 h-5" />,
    items: [
      { title: 'Creating Comits', href: '/developers/docs/creating-comits', description: 'Build cross-VM transactions' },
      { title: 'Atomic Execution', href: '/developers/docs/atomic-execution', description: 'All-or-nothing execution' },
      { title: 'Cross-VM Assets', href: '/developers/docs/cross-vm-assets', description: 'Unified asset management' },
      { title: 'Error Handling', href: '/developers/docs/error-handling', description: 'Handle failures gracefully' },
      { title: 'Best Practices', href: '/developers/docs/best-practices', description: 'Recommended patterns' },
    ],
  },
  {
    id: 'node-operations',
    title: 'Node Operations',
    icon: <Database className="w-5 h-5" />,
    items: [
      { title: 'Running a Node', href: '/developers/docs/run-node', description: 'Start your own node' },
      { title: 'Becoming a Validator', href: '/developers/docs/validator', description: 'Join the validator set' },
      { title: 'Chain Specifications', href: '/developers/docs/chain-spec', description: 'Network parameters' },
      { title: 'Key Management', href: '/developers/docs/keys', description: 'Secure key handling' },
      { title: 'Monitoring', href: '/developers/docs/monitoring', description: 'Track node health' },
    ],
  },
];

const quickLinks = [
  { title: 'RPC API Reference', href: '/developers/api', icon: <Globe className="w-4 h-4" /> },
  { title: 'Cookbook Examples', href: '/developers/cookbook', icon: <BookOpen className="w-4 h-4" /> },
  { title: 'GitHub Repository', href: 'https://github.com/atlas-sphere', icon: <Code className="w-4 h-4" /> },
  { title: 'SDKs & Tools', href: '/developers/sdks', icon: <Box className="w-4 h-4" /> },
];

// Flatten all items for navigation
const allDocItems = docSections.flatMap(section => section.items);

interface DocLayoutProps {
  children: React.ReactNode;
  title: string;
  description?: string;
  lastUpdated?: string;
}

export default function DocLayout({ children, title, description, lastUpdated }: DocLayoutProps) {
  const pathname = usePathname();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // Find current, previous, and next pages
  const currentIndex = allDocItems.findIndex(item => item.href === pathname);
  const prevPage = currentIndex > 0 ? allDocItems[currentIndex - 1] : null;
  const nextPage = currentIndex < allDocItems.length - 1 ? allDocItems[currentIndex + 1] : null;

  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Header */}
      <section className="py-8 border-b border-[#1a1a1a] bg-gradient-to-br from-violet-950/10 via-black to-purple-950/5">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          {/* Breadcrumb */}
          <nav className="flex items-center space-x-2 text-sm text-gray-500 mb-4">
            <Link href="/developers/docs" className="hover:text-white">Docs</Link>
            <ChevronRight className="w-4 h-4" />
            <span className="text-white">{title}</span>
          </nav>
          
          <h1 className="text-3xl md:text-4xl font-bold text-white mb-2">{title}</h1>
          {description && <p className="text-lg text-gray-500">{description}</p>}
          {lastUpdated && (
            <p className="text-sm text-gray-600 mt-2">Last updated: {lastUpdated}</p>
          )}
        </div>
      </section>

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="grid lg:grid-cols-4 gap-8">
          {/* Sidebar */}
          <aside className="lg:col-span-1">
            <button
              className="lg:hidden flex items-center w-full p-4 mb-4 glass-card"
              onClick={() => setSidebarOpen(!sidebarOpen)}
            >
              {sidebarOpen ? <X className="w-5 h-5 mr-2" /> : <Menu className="w-5 h-5 mr-2" />}
              Navigation
            </button>

            <nav className={`space-y-6 sticky top-24 ${sidebarOpen ? 'block' : 'hidden lg:block'}`}>
              {/* Search */}
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-600" />
                <input
                  type="text"
                  placeholder="Search docs..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full pl-10 pr-4 py-2 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-600 text-sm focus:outline-none focus:border-violet-500"
                />
              </div>

              {/* Quick Links */}
              <div className="glass-card p-4">
                <h3 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3">
                  Quick Links
                </h3>
                <div className="space-y-1">
                  {quickLinks.map((link) => (
                    <Link
                      key={link.title}
                      href={link.href}
                      className="flex items-center text-sm text-gray-400 hover:text-white hover:bg-[#1a1a1a] rounded-lg px-3 py-2 transition-colors"
                    >
                      <span className="text-violet-400 mr-2">{link.icon}</span>
                      {link.title}
                      {link.href.startsWith('http') && <ExternalLink className="w-3 h-3 ml-auto text-gray-600" />}
                    </Link>
                  ))}
                </div>
              </div>

              {/* Doc Sections */}
              {docSections.map((section) => (
                <div key={section.id}>
                  <h3 className="flex items-center text-sm font-semibold text-white mb-2">
                    <span className="text-orange-400 mr-2">{section.icon}</span>
                    {section.title}
                  </h3>
                  <ul className="space-y-1 ml-7 border-l border-[#1a1a1a]">
                    {section.items.map((item) => {
                      const isActive = pathname === item.href;
                      return (
                        <li key={item.title}>
                          <Link
                            href={item.href}
                            className={`block text-sm rounded-r-lg px-3 py-1.5 -ml-px border-l-2 transition-colors ${
                              isActive
                                ? 'border-l-orange-500 text-orange-400 bg-orange-500/10'
                                : 'border-l-transparent text-gray-500 hover:text-white hover:border-l-[#333333]'
                            }`}
                          >
                            {item.title}
                          </Link>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              ))}
            </nav>
          </aside>

          {/* Main Content */}
          <main className="lg:col-span-3">
            <div className="prose prose-invert max-w-none">
              {children}
            </div>

            {/* Pagination */}
            <div className="mt-12 pt-8 border-t border-[#1a1a1a] flex justify-between">
              {prevPage ? (
                <Link
                  href={prevPage.href}
                  className="flex items-center text-gray-400 hover:text-white transition-colors"
                >
                  <ChevronLeft className="w-5 h-5 mr-2" />
                  <div>
                    <div className="text-xs text-gray-600">Previous</div>
                    <div className="font-medium">{prevPage.title}</div>
                  </div>
                </Link>
              ) : <div />}
              
              {nextPage ? (
                <Link
                  href={nextPage.href}
                  className="flex items-center text-gray-400 hover:text-white transition-colors text-right"
                >
                  <div>
                    <div className="text-xs text-gray-600">Next</div>
                    <div className="font-medium">{nextPage.title}</div>
                  </div>
                  <ChevronRight className="w-5 h-5 ml-2" />
                </Link>
              ) : <div />}
            </div>

            {/* Feedback */}
            <div className="mt-8 p-6 glass-card">
              <h3 className="font-semibold text-white mb-2">Was this helpful?</h3>
              <div className="flex items-center space-x-4">
                <button className="px-4 py-2 rounded-lg bg-[#1a1a1a] hover:bg-emerald-500/20 text-gray-400 hover:text-emerald-400 transition-colors">
                  Yes 👍
                </button>
                <button className="px-4 py-2 rounded-lg bg-[#1a1a1a] hover:bg-red-500/20 text-gray-400 hover:text-red-400 transition-colors">
                  No 👎
                </button>
                <Link
                  href="https://github.com/atlas-sphere/docs/issues/new"
                  className="ml-auto text-sm text-violet-400 hover:text-violet-300"
                >
                  Edit this page on GitHub
                </Link>
              </div>
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}

// Code block component with copy functionality
export function CodeBlock({ children, language = 'typescript', title }: { children: string; language?: string; title?: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(children);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative my-6">
      {title && (
        <div className="flex items-center justify-between px-4 py-2 bg-[#0a0a0a] border-b border-[#1a1a1a] rounded-t-xl">
          <span className="text-sm text-gray-500">{title}</span>
          <span className="text-xs text-gray-600">{language}</span>
        </div>
      )}
      <div className={`code-block ${!title ? 'rounded-xl' : 'rounded-b-xl rounded-t-none'}`}>
        <button
          onClick={handleCopy}
          className="absolute top-3 right-3 p-2 rounded-lg bg-[#1a1a1a] hover:bg-[#222222] text-gray-500 hover:text-white transition-colors"
        >
          {copied ? <Check className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
        </button>
        <pre className="text-sm overflow-x-auto">
          <code className={`language-${language}`}>{children}</code>
        </pre>
      </div>
    </div>
  );
}

// Callout component
export function Callout({ type = 'info', title, children }: { type?: 'info' | 'warning' | 'danger' | 'success'; title?: string; children: React.ReactNode }) {
  const styles = {
    info: 'bg-blue-500/10 border-blue-500/30 text-blue-400',
    warning: 'bg-amber-500/10 border-amber-500/30 text-amber-400',
    danger: 'bg-red-500/10 border-red-500/30 text-red-400',
    success: 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400',
  };

  return (
    <div className={`my-6 p-4 rounded-xl border ${styles[type]}`}>
      {title && <div className="font-semibold mb-2">{title}</div>}
      <div className="text-gray-300">{children}</div>
    </div>
  );
}
