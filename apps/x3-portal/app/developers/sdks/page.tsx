'use client';

import React from 'react';
import Link from 'next/link';
import {
  Package,
  Terminal,
  Code,
  Smartphone,
  Globe,
  Server,
  CheckCircle2,
  ArrowRight,
  Download,
  Copy,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const sdks = [
  {
    name: 'JavaScript/TypeScript',
    package: '@x3star/sdk',
    version: '1.0.0',
    description: 'Full-featured SDK for Node.js and browsers',
    installCmd: 'npm install @x3star/sdk',
    features: ['TypeScript support', 'Browser & Node.js', 'Tree-shakeable'],
    docs: '/developers/docs/sdk/javascript',
  },
  {
    name: 'Python',
    package: 'x3star-sdk',
    version: '1.0.0',
    description: 'Python SDK for data science and backend',
    installCmd: 'pip install x3star-sdk',
    features: ['Async support', 'Type hints', 'Jupyter friendly'],
    docs: '/developers/docs/sdk/python',
  },
  {
    name: 'Rust',
    package: 'x3star-sdk',
    version: '1.0.0',
    description: 'Native Rust SDK for high-performance apps',
    installCmd: 'cargo add x3star-sdk',
    features: ['Zero-cost abstractions', 'Async runtime', 'WASM support'],
    docs: '/developers/docs/sdk/rust',
  },
  {
    name: 'Go',
    package: 'github.com/x3star/sdk-go',
    version: '1.0.0',
    description: 'Go SDK for backend services',
    installCmd: 'go get github.com/x3star/sdk-go',
    features: ['Goroutine safe', 'Context support', 'Minimal deps'],
    docs: '/developers/docs/sdk/go',
  },
];

const mobileSDKs = [
  {
    name: 'iOS (Swift)',
    package: 'X3StarSDK',
    installCmd: "pod 'X3StarSDK'",
    features: ['SwiftUI ready', 'Keychain integration', 'Biometric auth'],
  },
  {
    name: 'Android (Kotlin)',
    package: 'x3star-android',
    installCmd: "implementation 'io.x3star:sdk:1.0.0'",
    features: ['Jetpack Compose', 'Coroutines', 'KeyStore'],
  },
  {
    name: 'React Native',
    package: '@x3star/react-native',
    installCmd: 'npm install @x3star/react-native',
    features: ['Cross-platform', 'Expo support', 'Native modules'],
  },
  {
    name: 'Flutter',
    package: 'x3star_sdk',
    installCmd: 'flutter pub add x3star_sdk',
    features: ['Dart 3.0+', 'Platform channels', 'Null safety'],
  },
];

const cliExample = `# Install X3 CLI
npm install -g @x3star/cli

# Initialize a new project
x3 init my-project

# Generate keypair
x3 keygen --output wallet.json

# Deploy contract
x3 deploy ./target/release/my_contract.wasm

# Query account
x3 account 0x742d35Cc6634C0532925a3b844Bc...

# Submit transaction
x3 transfer --to 0x... --amount 10 --token USDC`;

const codeExample = `// TypeScript SDK Example
import { X3Client, Wallet, Transaction } from '@x3star/sdk';

// Initialize client
const client = new X3Client({
  network: 'mainnet',
  rpcEndpoint: 'https://rpc.x3star.io',
});

// Create wallet from mnemonic
const wallet = Wallet.fromMnemonic(process.env.MNEMONIC!);

// Get account balance
const balance = await client.getBalance(wallet.address, 'ATLAS');
console.log('Balance:', balance.formatted);

// Create and sign transaction
const tx = new Transaction({
  from: wallet.address,
  to: '0x742d35Cc6634C0532925a3b844Bc9e7595f...',
  amount: '100',
  token: 'USDC',
});

const signed = await wallet.signTransaction(tx);
const receipt = await client.sendTransaction(signed);

console.log('TX Hash:', receipt.hash);
console.log('Status:', receipt.status);`;

export default function SDKsPage() {
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
            <Link href="/developers" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Developers
            </Link>
            <div className="badge badge-info mt-4 mb-4">SDKs & Tools</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              SDKs & Developer Tools
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Build on X3 STAR with official SDKs for every platform. 
              JavaScript, Python, Rust, Go, and mobile frameworks.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#sdks" className="btn-primary">
                <Download className="w-4 h-4 mr-2" />
                Get SDKs
              </a>
              <Link href="/developers/docs" className="btn-secondary">
                Documentation
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Main SDKs */}
      <section id="sdks" className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Core SDKs</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {sdks.map((sdk, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-start justify-between mb-4">
                  <div>
                    <h3 className="text-xl font-semibold text-white">{sdk.name}</h3>
                    <code className="text-sm text-cyan-400">{sdk.package}</code>
                  </div>
                  <span className="badge badge-default">v{sdk.version}</span>
                </div>
                <p className="text-gray-400 mb-4">{sdk.description}</p>
                <div className="p-3 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a] mb-4 flex items-center justify-between">
                  <code className="text-sm text-orange-400">{sdk.installCmd}</code>
                  <button className="p-1 hover:bg-[#1a1a1a] rounded">
                    <Copy className="w-4 h-4 text-gray-400" />
                  </button>
                </div>
                <div className="flex flex-wrap gap-2 mb-4">
                  {sdk.features.map((feature, i) => (
                      <span key={i} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                      {feature}
                    </span>
                  ))}
                </div>
                <Link href={sdk.docs} className="text-orange-400 hover:text-orange-300 text-sm flex items-center">
                  View Documentation
                  <ArrowRight className="w-4 h-4 ml-1" />
                </Link>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Mobile SDKs */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="flex items-center gap-3 mb-8">
            <Smartphone className="w-6 h-6 text-orange-400" />
            <h2 className="text-2xl font-bold text-white">Mobile SDKs</h2>
          </div>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {mobileSDKs.map((sdk, index) => (
              <div key={index} className="glass-card p-5">
                <h3 className="font-semibold text-white mb-2">{sdk.name}</h3>
                <div className="p-2 rounded bg-[#0a0a0a] border border-[#1a1a1a] mb-3">
                  <code className="text-xs text-cyan-400 break-all">{sdk.installCmd}</code>
                </div>
                <div className="space-y-1">
                  {sdk.features.map((feature, i) => (
                    <p key={i} className="text-xs text-gray-400 flex items-center">
                      <CheckCircle2 className="w-3 h-3 text-emerald-500 mr-1" />
                      {feature}
                    </p>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CLI */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <div className="flex items-center gap-3 mb-4">
                <Terminal className="w-6 h-6 text-orange-400" />
                <h2 className="text-2xl font-bold text-white">X3 CLI</h2>
              </div>
              <p className="text-gray-400 mb-6">
                Command-line interface for managing projects, deploying contracts, 
                and interacting with the X3 STAR network.
              </p>
              <ul className="space-y-3 mb-6">
                {[
                  'Project scaffolding',
                  'Contract deployment',
                  'Account management',
                  'Transaction submission',
                  'Network queries',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-5 h-5 text-emerald-500 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
              <Link href="/developers/docs/cli" className="btn-primary">
                CLI Documentation
              </Link>
            </div>
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center">
                <div className="flex gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-red-500" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500" />
                  <div className="w-3 h-3 rounded-full bg-green-500" />
                </div>
                <span className="text-sm text-gray-400 ml-4">Terminal</span>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{cliExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Code Example */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Quick Start Example</h2>
          
          <div className="glass-card p-0 overflow-hidden">
            <div className="p-3 border-b border-[#1a1a1a] flex items-center justify-between">
              <span className="text-sm text-gray-400">quickstart.ts</span>
              <button className="text-xs text-gray-500 hover:text-white flex items-center gap-1">
                <Copy className="w-4 h-4" />
                Copy
              </button>
            </div>
            <pre className="p-4 overflow-x-auto text-sm">
              <code className="text-gray-400">{codeExample}</code>
            </pre>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Package className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Start Building Today
          </h2>
          <p className="text-gray-400 mb-8">
            Pick your favorite language and start building on X3 STAR.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Read the Docs
            </Link>
            <Link href="/developers/cookbook" className="btn-secondary">
              Browse Examples
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
