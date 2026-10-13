'use client';

import React from 'react';
import Link from 'next/link';
import {
  Wallet,
  Shield,
  Smartphone,
  Globe,
  Key,
  Fingerprint,
  QrCode,
  ArrowLeftRight,
  ExternalLink,
  CheckCircle2,
  Download,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const wallets = [
  {
    name: 'Sphere Wallet',
    type: 'Browser Extension',
    description: 'Official X3 STAR wallet with full cross-VM support',
    features: ['EVM + SVM support', 'Comit transactions', 'Hardware wallet support', 'dApp browser'],
    platforms: ['Chrome', 'Firefox', 'Brave', 'Edge'],
    downloadUrl: '#',
    docsUrl: '/developers/docs',
    featured: true,
  },
  {
    name: 'Atlas Mobile',
    type: 'Mobile Wallet',
    description: 'Mobile-first wallet for iOS and Android',
    features: ['Biometric auth', 'WalletConnect', 'NFT gallery', 'Token swaps'],
    platforms: ['iOS', 'Android'],
    downloadUrl: '#',
    docsUrl: '/developers/docs',
    featured: true,
  },
  {
    name: 'X3 CLI Wallet',
    type: 'CLI',
    description: 'Command-line wallet for developers and power users',
    features: ['Scripting support', 'Batch transactions', 'Key management', 'RPC integration'],
    platforms: ['macOS', 'Linux', 'Windows'],
    downloadUrl: '#',
    docsUrl: '/developers/docs',
    featured: false,
  },
  {
    name: 'Ledger Support',
    type: 'Hardware',
    description: 'Secure your assets with Ledger hardware wallets',
    features: ['Cold storage', 'Multi-sig support', 'Secure signing', 'Recovery'],
    platforms: ['Ledger Nano S', 'Ledger Nano X'],
    downloadUrl: '#',
    docsUrl: '/developers/docs',
    featured: false,
  },
];

const features = [
  {
    title: 'Dual VM Support',
    description: 'Manage both EVM and SVM assets from a single wallet interface',
    icon: <ArrowLeftRight className="w-6 h-6" />,
  },
  {
    title: 'Cross-VM Transactions',
    description: 'Execute Comit transactions that span both virtual machines',
    icon: <Globe className="w-6 h-6" />,
  },
  {
    title: 'Hardware Security',
    description: 'Support for Ledger and other hardware wallets',
    icon: <Shield className="w-6 h-6" />,
  },
  {
    title: 'Biometric Auth',
    description: 'Secure your wallet with fingerprint or face recognition',
    icon: <Fingerprint className="w-6 h-6" />,
  },
];

const integrationCode = `// Connect to Sphere Wallet
import { SphereWallet } from '@x3star/wallet-adapter';

const wallet = new SphereWallet();

// Connect and get accounts
await wallet.connect();
const accounts = await wallet.getAccounts();

// Execute a cross-VM Comit
const comit = await wallet.signAndSubmitComit({
  evmPayload: { /* EVM transaction */ },
  svmPayload: { /* SVM instruction */ },
});

console.log('Comit ID:', comit.id);`;

export default function WalletsPage() {
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
            <div className="badge badge-success mt-4 mb-4">Wallets</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              X3 STAR Wallets
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Secure, user-friendly wallets for managing your assets across both 
              EVM and SVM. Choose the wallet that fits your needs.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#" className="btn-primary">
                <Download className="w-4 h-4 mr-2" />
                Download Sphere Wallet
              </a>
              <Link href="/developers/docs" className="btn-secondary">
                Integration Docs
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
                <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400 mr-4">
                  {feature.icon}
                </div>
                <div>
                  <h3 className="font-semibold text-white mb-1">{feature.title}</h3>
                  <p className="text-sm text-gray-400">{feature.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Wallets */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Available Wallets</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {wallets.map((wallet, index) => (
              <div key={index} className={`glass-card p-6 ${wallet.featured ? 'border-orange-500/30' : ''}`}>
                <div className="flex items-start justify-between mb-4">
                  <div>
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="text-xl font-semibold text-white">{wallet.name}</h3>
                      {wallet.featured && <span className="badge badge-warning">Featured</span>}
                    </div>
                    <span className="badge badge-default">{wallet.type}</span>
                  </div>
                  <a href={wallet.downloadUrl} className="btn-primary text-sm py-2 px-4">
                    <Download className="w-4 h-4 mr-2" />
                    Download
                  </a>
                </div>
                <p className="text-gray-400 mb-4">{wallet.description}</p>
                <div className="flex flex-wrap gap-2 mb-4">
                  {wallet.features.map((feature, i) => (
                    <span key={i} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                      {feature}
                    </span>
                  ))}
                </div>
                <div className="flex items-center justify-between pt-4 border-t border-[#1a1a1a]">
                  <div className="flex gap-2">
                    {wallet.platforms.map((platform, i) => (
                      <span key={i} className="text-xs text-gray-500">{platform}</span>
                    ))}
                  </div>
                  <Link href={wallet.docsUrl} className="text-orange-400 hover:text-orange-300 text-sm">
                    Docs →
                  </Link>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Integration */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Easy Integration</h2>
              <p className="text-gray-400 mb-6">
                Integrate X3 STAR wallets into your dApp with just a few lines of code. 
                Our wallet adapter handles connection, signing, and cross-VM transactions.
              </p>
              <ul className="space-y-3">
                {[
                  'Unified API for all supported wallets',
                  'Automatic wallet detection',
                  'Cross-VM transaction support',
                  'TypeScript types included',
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
                <span className="text-sm text-gray-400">wallet-integration.ts</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{integrationCode}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Security */}
      <section className="py-16">
        <div className="container-wide">
          <div className="glass-card p-8 md:p-12">
            <div className="grid md:grid-cols-2 gap-8 items-center">
              <div>
                <Shield className="w-12 h-12 text-orange-400 mb-6" />
                <h2 className="text-2xl font-bold text-white mb-4">Security First</h2>
                <p className="text-gray-400 mb-6">
                  All X3 STAR wallets undergo rigorous security audits and follow 
                  industry best practices for key management and transaction signing.
                </p>
                <ul className="space-y-2 text-gray-400">
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 text-emerald-500 mr-2" />
                    Audited by leading security firms
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 text-emerald-500 mr-2" />
                    Non-custodial - you control your keys
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 text-emerald-500 mr-2" />
                    Open source and verifiable
                  </li>
                </ul>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="glass-card p-4 text-center">
                  <Key className="w-8 h-8 text-orange-400 mx-auto mb-2" />
                  <p className="text-sm text-gray-400">Self-Custody</p>
                </div>
                <div className="glass-card p-4 text-center">
                  <Fingerprint className="w-8 h-8 text-orange-400 mx-auto mb-2" />
                  <p className="text-sm text-gray-400">Biometric</p>
                </div>
                <div className="glass-card p-4 text-center">
                  <Shield className="w-8 h-8 text-orange-400 mx-auto mb-2" />
                  <p className="text-sm text-gray-400">Audited</p>
                </div>
                <div className="glass-card p-4 text-center">
                  <QrCode className="w-8 h-8 text-orange-400 mx-auto mb-2" />
                  <p className="text-sm text-gray-400">Air-gapped</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">
            Get Started with X3 STAR
          </h2>
          <p className="text-gray-400 mb-8">
            Download a wallet and start exploring the X3 STAR ecosystem today.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#" className="btn-primary">
              Download Sphere Wallet
            </a>
            <Link href="/learn/getting-started" className="btn-secondary">
              Getting Started Guide
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
