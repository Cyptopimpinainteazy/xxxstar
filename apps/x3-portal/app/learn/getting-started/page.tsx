'use client';

import React from 'react';
import Link from 'next/link';
import {
  Rocket,
  BookOpen,
  Code,
  Terminal,
  Wallet,
  ArrowRight,
  CheckCircle,
  Clock,
  Layers,
  Zap,
  Play,
  Download,
  ExternalLink,
} from 'lucide-react';

const steps = [
  {
    number: '01',
    title: 'Set Up Your Environment',
    description: 'Install the required tools and configure your development environment for X3 Atlas Sphere.',
    icon: <Terminal className="w-6 h-6" />,
    time: '10 min',
    items: [
      'Install Rust and Cargo',
      'Add WASM target',
      'Clone the repository',
      'Build the node',
    ],
  },
  {
    number: '02',
    title: 'Run a Local Node',
    description: 'Start a development node and connect to the X3 Atlas Sphere network.',
    icon: <Rocket className="w-6 h-6" />,
    time: '5 min',
    items: [
      'Start dev node with --dev flag',
      'Connect via RPC endpoint',
      'Explore Atlas Kernel RPCs',
      'Query the canonical ledger',
    ],
  },
  {
    number: '03',
    title: 'Connect a Wallet',
    description: 'Set up a wallet and get testnet tokens from the faucet.',
    icon: <Wallet className="w-6 h-6" />,
    time: '5 min',
    items: [
      'Install compatible wallet',
      'Add X3 network',
      'Get testnet tokens',
      'Check your balance',
    ],
  },
  {
    number: '04',
    title: 'Deploy Your First Contract',
    description: 'Deploy a smart contract to either the EVM or SVM execution environment.',
    icon: <Code className="w-6 h-6" />,
    time: '15 min',
    items: [
      'Choose your VM (EVM or SVM)',
      'Write your contract',
      'Compile and deploy',
      'Interact via RPC',
    ],
  },
];

const quickLinks = [
  {
    title: 'Documentation',
    description: 'Complete reference for X3 Atlas Sphere',
    href: '/developers/docs',
    icon: <BookOpen className="w-5 h-5" />,
  },
  {
    title: 'Tokenomics',
    description: 'How X3Coin powers the ecosystem',
    href: '/learn/tokenomics',
    icon: <Zap className="w-5 h-5" />,
  },
  {
    title: 'Cookbook',
    description: 'Copy-paste code examples',
    href: '/developers/cookbook',
    icon: <Code className="w-5 h-5" />,
  },
  {
    title: 'Tutorials',
    description: 'Step-by-step learning guides',
    href: '/learn/tutorials',
    icon: <Layers className="w-5 h-5" />,
  },
];

export default function GettingStartedPage() {
  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero with unique Learn section header */}
      <section className="py-20 relative overflow-hidden page-header-learn">
        <div className="absolute inset-0 bg-gradient-to-br from-emerald-950/20 via-black to-teal-950/10" />
        <div className="absolute inset-0 mesh-gradient opacity-10" />
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="inline-flex items-center px-3 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 mb-4">
              <BookOpen className="w-4 h-4 mr-2 text-emerald-400" />
              <span className="text-sm text-emerald-300">Getting Started</span>
            </div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Start building on <span className="text-emerald-400">X3 Atlas Sphere</span>
            </h1>
            <p className="text-xl text-gray-500 mb-8">
              Get up and running with the first dual VM blockchain in under 30 minutes. 
              This guide will walk you through everything you need to know.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="#setup" className="btn-primary flex items-center">
                <Play className="mr-2 w-5 h-5" />
                Start Tutorial
              </Link>
              <Link href="/developers/docs" className="btn-secondary flex items-center">
                <BookOpen className="mr-2 w-5 h-5" />
                Read Docs
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Prerequisites */}
      <section className="py-16 border-t border-[#1a1a1a] bg-black">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Prerequisites</h2>
          <div className="grid md:grid-cols-3 gap-6">
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-3">Operating System</h3>
              <p className="text-sm text-gray-500 mb-4">
                Linux or macOS recommended. Windows via WSL2.
              </p>
              <div className="flex flex-wrap gap-2">
                <span className="badge badge-info">Ubuntu 22.04+</span>
                <span className="badge badge-info">macOS 12+</span>
                <span className="badge badge-info">WSL2</span>
              </div>
            </div>
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-3">Rust Toolchain</h3>
              <p className="text-sm text-gray-500 mb-4">
                Rust stable with WASM target for building the runtime.
              </p>
              <div className="code-block text-xs">
                <code>rustup target add wasm32-unknown-unknown</code>
              </div>
            </div>
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-3">Build Dependencies</h3>
              <p className="text-sm text-gray-500 mb-4">
                cmake, pkg-config, OpenSSL, libclang required.
              </p>
              <div className="code-block text-xs">
                <code>sudo apt install build-essential cmake pkg-config libssl-dev clang</code>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Steps */}
      <section id="setup" className="py-16 bg-black">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-12">Getting Started Steps</h2>
          
          <div className="space-y-8">
            {steps.map((step, index) => (
              <div key={index} className="glass-card p-8">
                <div className="flex flex-col lg:flex-row lg:items-start gap-6">
                  {/* Step number */}
                  <div className="flex-shrink-0">
                    <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-orange-500 to-red-600 flex items-center justify-center text-2xl font-bold text-white">
                      {step.number}
                    </div>
                  </div>
                  
                  {/* Content */}
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="text-xl font-bold text-white">{step.title}</h3>
                      <span className="badge badge-fire flex items-center">
                        <Clock className="w-3 h-3 mr-1" />
                        {step.time}
                      </span>
                    </div>
                    <p className="text-gray-500 mb-4">{step.description}</p>
                    
                    <div className="grid sm:grid-cols-2 gap-3">
                      {step.items.map((item, i) => (
                        <div key={i} className="flex items-center text-sm text-gray-400">
                          <CheckCircle className="w-4 h-4 text-emerald-500 mr-2 flex-shrink-0" />
                          {item}
                        </div>
                      ))}
                    </div>
                  </div>
                  
                  {/* Icon */}
                  <div className="hidden lg:flex flex-shrink-0 p-4 rounded-xl bg-[#1a1a1a] text-orange-400">
                    {step.icon}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Quick Start Code */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Quick Start Commands</h2>
          
          <div className="grid lg:grid-cols-2 gap-8">
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-4">1. Clone and Build</h3>
              <div className="code-block">
                <pre className="text-sm text-gray-400">
{`# Clone the repository
git clone https://github.com/atlas-sphere/atlas-sphere.git
cd atlas-sphere

# Install Rust dependencies
rustup target add wasm32-unknown-unknown

# Build the node
cargo build --release`}
                </pre>
              </div>
            </div>
            
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-4">2. Run Development Node</h3>
              <div className="code-block">
                <pre className="text-sm text-gray-400">
{`# Start a development node
./target/release/atlas-sphere-node --dev --tmp

# Or use the convenience script
./run-dev-node.sh

# RPC available at http://127.0.0.1:9944`}
                </pre>
              </div>
            </div>
            
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-4">3. Query the Network</h3>
              <div className="code-block">
                <pre className="text-sm text-gray-400">
{`# Check authorized accounts
curl http://127.0.0.1:9944 -H "Content-Type: application/json" \\
  -d '{"id":1,"jsonrpc":"2.0",
       "method":"atlasKernel_getAuthorizedAccounts",
       "params":[null]}'`}
                </pre>
              </div>
            </div>
            
            <div className="glass-card p-6">
              <h3 className="font-semibold text-white mb-4">4. Connect to Testnet</h3>
              <div className="code-block">
                <pre className="text-sm text-gray-400">
{`# Testnet RPC endpoint
https://rpc.testnet.atlas-sphere.io

# Get testnet tokens
https://faucet.testnet.atlas-sphere.io

# Block explorer
https://explorer.testnet.atlas-sphere.io`}
                </pre>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Quick Links */}
      <section className="py-16 bg-black">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Continue Learning</h2>
          
          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
            {quickLinks.map((link, index) => (
              <Link
                key={index}
                href={link.href}
                className="glass-card-hover p-6 card-lift group"
              >
                <div className="p-3 rounded-xl bg-[#1a1a1a] w-fit mb-4 group-hover:bg-orange-500/20 transition-colors">
                  <span className="text-gray-500 group-hover:text-orange-400 transition-colors">
                    {link.icon}
                  </span>
                </div>
                <h3 className="font-semibold text-white mb-2">{link.title}</h3>
                <p className="text-sm text-gray-500">{link.description}</p>
              </Link>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a] bg-black">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">Need help?</h2>
          <p className="text-gray-500 mb-8">
            Join our community on Discord or check out the developer forum for support.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="https://discord.gg/x3atlas" className="btn-primary">
              Join Discord
            </Link>
            <Link href="/community/forum" className="btn-secondary">
              Visit Forum
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
