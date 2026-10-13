'use client';

import React from 'react';
import Link from 'next/link';
import {
  Coins,
  Shield,
  Lock,
  Clock,
  Users,
  FileCheck,
  Zap,
  Settings,
  ArrowRight,
  CheckCircle2,
  Code,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const extensions = [
  {
    name: 'Transfer Hooks',
    description: 'Execute custom logic on every token transfer',
    icon: <Zap className="w-6 h-6" />,
    useCases: ['Royalty enforcement', 'Transfer fees', 'Compliance checks'],
    color: 'from-orange-500 to-amber-500',
  },
  {
    name: 'Confidential Transfers',
    description: 'Privacy-preserving token transfers with zero-knowledge proofs',
    icon: <Shield className="w-6 h-6" />,
    useCases: ['Private payments', 'Salary disbursement', 'Anonymous voting'],
    color: 'from-purple-500 to-indigo-500',
  },
  {
    name: 'Transfer Restrictions',
    description: 'Programmable rules for who can send and receive tokens',
    icon: <Lock className="w-6 h-6" />,
    useCases: ['KYC/AML compliance', 'Investor accreditation', 'Geo-restrictions'],
    color: 'from-red-500 to-rose-500',
  },
  {
    name: 'Interest-Bearing',
    description: 'Tokens that automatically accrue interest over time',
    icon: <Clock className="w-6 h-6" />,
    useCases: ['Savings accounts', 'Yield tokens', 'Rebasing tokens'],
    color: 'from-emerald-500 to-green-500',
  },
  {
    name: 'Non-Transferable',
    description: 'Soulbound tokens that cannot be transferred',
    icon: <Users className="w-6 h-6" />,
    useCases: ['Credentials', 'Reputation', 'Membership'],
    color: 'from-blue-500 to-cyan-500',
  },
  {
    name: 'Metadata Pointer',
    description: 'Rich on-chain metadata for tokens',
    icon: <FileCheck className="w-6 h-6" />,
    useCases: ['NFT attributes', 'Token descriptions', 'Dynamic metadata'],
    color: 'from-pink-500 to-fuchsia-500',
  },
];

const codeExample = `// Create a token with transfer hooks and restrictions
import { TokenBuilder, TransferHook, TransferRestriction } from '@x3star/token-extensions';

const token = await TokenBuilder.create({
  name: 'Compliant Token',
  symbol: 'CMPL',
  decimals: 18,
  
  extensions: [
    // Add transfer hook for royalty enforcement
    new TransferHook({
      program: ROYALTY_PROGRAM,
      extraAccounts: [creatorAccount, feeAccount],
    }),
    
    // Add transfer restriction for KYC compliance
    new TransferRestriction({
      type: 'allowlist',
      authority: complianceAuthority,
    }),
  ],
});

console.log('Token created:', token.mint);`;

const benefits = [
  {
    title: 'Programmable Compliance',
    description: 'Build regulatory requirements directly into tokens',
  },
  {
    title: 'Cross-VM Compatible',
    description: 'Extensions work seamlessly across EVM and SVM',
  },
  {
    title: 'Composable',
    description: 'Combine multiple extensions for complex functionality',
  },
  {
    title: 'Gas Efficient',
    description: 'Optimized for minimal transaction costs',
  },
];

export default function TokenExtensionsPage() {
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
            <div className="badge badge-purple mt-4 mb-4">Token Extensions</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Programmable Tokens
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Create tokens with built-in functionality. Transfer hooks, restrictions, 
              confidential transfers, and more—all native to X3 STAR.
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

      {/* Extensions Grid */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Available Extensions</h2>
          
          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {extensions.map((ext, index) => (
              <div key={index} className="glass-card p-6">
                <div className={`p-3 rounded-xl bg-gradient-to-br ${ext.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{ext.icon}</span>
                </div>
                <h3 className="text-lg font-semibold text-white mb-2">{ext.name}</h3>
                <p className="text-gray-400 mb-4">{ext.description}</p>
                <div className="space-y-2">
                  <p className="text-xs text-gray-500 uppercase tracking-wide">Use Cases</p>
                  <div className="flex flex-wrap gap-2">
                    {ext.useCases.map((useCase, i) => (
                      <span key={i} className="badge badge-default">{useCase}</span>
                    ))}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Code Example */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Simple to Implement</h2>
              <p className="text-gray-400 mb-6">
                Create tokens with advanced features using our intuitive SDK. 
                Combine extensions to build exactly what you need.
              </p>
              <ul className="space-y-3">
                {benefits.map((benefit, i) => (
                  <li key={i} className="flex items-start">
                    <CheckCircle2 className="w-5 h-5 text-emerald-500 mr-3 mt-0.5" />
                    <div>
                      <span className="text-white font-medium">{benefit.title}</span>
                      <p className="text-sm text-gray-400">{benefit.description}</p>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center justify-between">
                <span className="text-sm text-gray-400">create-token.ts</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{codeExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Use Cases */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Real-World Applications</h2>
          
          <div className="grid md:grid-cols-3 gap-6">
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-4">Security Tokens</h3>
              <p className="text-gray-400 mb-4">
                Issue compliant security tokens with built-in transfer restrictions 
                for investor accreditation and holding periods.
              </p>
              <div className="flex flex-wrap gap-2">
                <span className="badge badge-default">Transfer Restrictions</span>
                <span className="badge badge-default">Metadata</span>
              </div>
            </div>
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-4">NFT Royalties</h3>
              <p className="text-gray-400 mb-4">
                Enforce creator royalties on every secondary sale with transfer 
                hooks that cannot be bypassed.
              </p>
              <div className="flex flex-wrap gap-2">
                <span className="badge badge-default">Transfer Hooks</span>
                <span className="badge badge-default">Metadata Pointer</span>
              </div>
            </div>
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-4">Corporate Credits</h3>
              <p className="text-gray-400 mb-4">
                Issue non-transferable credentials and reputation tokens for 
                employees, partners, or community members.
              </p>
              <div className="flex flex-wrap gap-2">
                <span className="badge badge-default">Non-Transferable</span>
                <span className="badge badge-default">Metadata</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Coins className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Create Your First Extended Token
          </h2>
          <p className="text-gray-400 mb-8">
            Follow our guide to create tokens with advanced features in minutes.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Token Extension Guide
            </Link>
            <Link href="/developers/cookbook" className="btn-secondary">
              Code Examples
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
