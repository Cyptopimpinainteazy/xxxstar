'use client';

import React from 'react';
import Link from 'next/link';
import {
  Lock,
  Building2,
  Shield,
  Eye,
  Users,
  Settings,
  FileCheck,
  Zap,
  CheckCircle2,
  Server,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const features = [
  {
    name: 'Access Control',
    description: 'Whitelist addresses for transaction permissions',
    icon: <Lock className="w-6 h-6" />,
  },
  {
    name: 'Private Transactions',
    description: 'Optional privacy for sensitive operations',
    icon: <Eye className="w-6 h-6" />,
  },
  {
    name: 'Compliance Built-in',
    description: 'KYC/AML enforcement at protocol level',
    icon: <FileCheck className="w-6 h-6" />,
  },
  {
    name: 'Custom Validators',
    description: 'Run your own validator set for consensus',
    icon: <Server className="w-6 h-6" />,
  },
];

const useCases = [
  {
    title: 'Enterprise DeFi',
    description: 'Institutional-grade financial applications with compliance requirements',
    icon: <Building2 className="w-6 h-6" />,
    features: ['Whitelisted participants', 'Auditable transactions', 'Regulatory compliance'],
  },
  {
    title: 'Central Bank Digital Currencies',
    description: 'CBDC implementations with central authority controls',
    icon: <Shield className="w-6 h-6" />,
    features: ['Monetary policy controls', 'Programmable money', 'Instant settlement'],
  },
  {
    title: 'Consortium Chains',
    description: 'Multi-party networks with shared governance',
    icon: <Users className="w-6 h-6" />,
    features: ['Shared governance', 'Permissioned access', 'Cross-org interop'],
  },
  {
    title: 'Private Markets',
    description: 'Tokenized securities with restricted trading',
    icon: <Settings className="w-6 h-6" />,
    features: ['Investor restrictions', 'Transfer controls', 'Cap table management'],
  },
];

const deploymentOptions = [
  {
    name: 'Permissioned Subnet',
    description: 'Deploy as a subnet connected to X3 mainnet',
    pros: ['Mainnet security', 'Cross-chain bridging', 'Shared liquidity'],
  },
  {
    name: 'Private Chain',
    description: 'Fully isolated private blockchain',
    pros: ['Complete isolation', 'Custom parameters', 'Full control'],
  },
  {
    name: 'Hybrid Mode',
    description: 'Permissioned layer with public settlement',
    pros: ['Privacy + transparency', 'Selective disclosure', 'Public proofs'],
  },
];

export default function PermissionedPage() {
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
            <div className="badge badge-warning mt-4 mb-4">Enterprise</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Permissioned Environments
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Enterprise-grade blockchain solutions with controlled access, 
              regulatory compliance, and privacy features.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#" className="btn-primary">
                Contact Enterprise Sales
              </a>
              <Link href="/developers/docs" className="btn-secondary">
                Technical Overview
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="py-12 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {features.map((feature, index) => (
              <div key={index} className="flex items-start">
                <div className="p-2 rounded-lg bg-amber-500/20 text-amber-400 mr-4">
                  {feature.icon}
                </div>
                <div>
                  <h3 className="font-semibold text-white mb-1">{feature.name}</h3>
                  <p className="text-sm text-gray-400">{feature.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Use Cases */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Enterprise Use Cases</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {useCases.map((useCase, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-3 rounded-xl bg-orange-500/20 text-orange-400 w-fit mb-4">
                  {useCase.icon}
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

      {/* Deployment Options */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Deployment Options</h2>
          
          <div className="grid md:grid-cols-3 gap-6">
            {deploymentOptions.map((option, index) => (
              <div key={index} className="glass-card p-6">
                <h3 className="text-xl font-semibold text-white mb-2">{option.name}</h3>
                <p className="text-gray-400 mb-4">{option.description}</p>
                <ul className="space-y-2">
                  {option.pros.map((pro, i) => (
                    <li key={i} className="flex items-center text-gray-400 text-sm">
                      <CheckCircle2 className="w-4 h-4 text-emerald-500 mr-2" />
                      {pro}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Architecture */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Dual-VM Enterprise Stack</h2>
              <p className="text-gray-400 mb-6">
                Enterprise deployments leverage X3 STAR's unique dual-VM architecture, 
                enabling EVM and SVM workloads in a unified permissioned environment.
              </p>
              <ul className="space-y-3">
                {[
                  'Run Solidity and Rust smart contracts',
                  'Atomic cross-VM transactions',
                  'Unified identity and access management',
                  'Deterministic transaction ordering',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <Zap className="w-5 h-5 text-orange-400 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-8">
              <div className="space-y-4">
                <div className="p-4 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a]">
                  <p className="text-sm text-gray-400 mb-1">Permissioned Layer</p>
                  <p className="text-white font-medium">Access Control + Compliance</p>
                </div>
                <div className="flex gap-4">
                  <div className="flex-1 p-4 rounded-lg bg-purple-500/10 border border-purple-500/30">
                    <p className="text-sm text-purple-400 mb-1">EVM</p>
                    <p className="text-white text-sm">Solidity</p>
                  </div>
                  <div className="flex-1 p-4 rounded-lg bg-cyan-500/10 border border-cyan-500/30">
                    <p className="text-sm text-cyan-400 mb-1">SVM</p>
                    <p className="text-white text-sm">Rust/Anchor</p>
                  </div>
                </div>
                <div className="p-4 rounded-lg bg-orange-500/10 border border-orange-500/30">
                  <p className="text-sm text-orange-400 mb-1">Atlas Kernel</p>
                  <p className="text-white font-medium">Atomic Cross-VM Orchestration</p>
                </div>
                <div className="p-4 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a]">
                  <p className="text-sm text-gray-400 mb-1">Substrate Runtime</p>
                  <p className="text-white font-medium">Consensus + Finality</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Lock className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Enterprise Solutions
          </h2>
          <p className="text-gray-400 mb-8">
            Talk to our enterprise team about deploying a permissioned X3 STAR environment.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#" className="btn-primary">
              Contact Sales
            </a>
            <Link href="/developers/docs" className="btn-secondary">
              Read Documentation
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
