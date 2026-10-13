'use client';

import React from 'react';
import Link from 'next/link';
import {
  Map,
  Building2,
  FileCheck,
  Shield,
  Globe,
  TrendingUp,
  Banknote,
  Landmark,
  CheckCircle2,
  ArrowRight,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const assetClasses = [
  {
    name: 'Real Estate',
    description: 'Tokenize commercial and residential properties',
    icon: <Building2 className="w-6 h-6" />,
    examples: ['Commercial buildings', 'Residential properties', 'REITs', 'Fractional ownership'],
    marketSize: '$326T',
  },
  {
    name: 'Commodities',
    description: 'Digital representation of physical commodities',
    icon: <Map className="w-6 h-6" />,
    examples: ['Precious metals', 'Energy', 'Agriculture', 'Carbon credits'],
    marketSize: '$120T',
  },
  {
    name: 'Securities',
    description: 'Compliant tokenized securities and equity',
    icon: <TrendingUp className="w-6 h-6" />,
    examples: ['Private equity', 'Bonds', 'Funds', 'Stock'],
    marketSize: '$115T',
  },
  {
    name: 'Art & Collectibles',
    description: 'Fractional ownership of fine art and collectibles',
    icon: <Landmark className="w-6 h-6" />,
    examples: ['Fine art', 'Wine', 'Classic cars', 'Memorabilia'],
    marketSize: '$2T',
  },
];

const features = [
  {
    title: 'Programmable Compliance',
    description: 'Build regulatory requirements directly into tokens with transfer restrictions and hooks',
    icon: <FileCheck className="w-5 h-5" />,
  },
  {
    title: 'Cross-VM Liquidity',
    description: 'Access liquidity from both EVM and SVM ecosystems without bridging',
    icon: <Globe className="w-5 h-5" />,
  },
  {
    title: 'Institutional Grade',
    description: 'Enterprise security, custody solutions, and regulatory frameworks',
    icon: <Shield className="w-5 h-5" />,
  },
  {
    title: 'Fractional Ownership',
    description: 'Divide high-value assets into accessible investment units',
    icon: <Banknote className="w-5 h-5" />,
  },
];

const complianceFeatures = [
  'KYC/AML verification integration',
  'Investor accreditation checks',
  'Jurisdictional transfer restrictions',
  'Holding period enforcement',
  'Cap table management',
  'Dividend distribution automation',
];

const partners = [
  { name: 'Institutional Custody', description: 'Secure custody solutions' },
  { name: 'Legal Framework', description: 'Regulatory compliance' },
  { name: 'Audit Partners', description: 'Third-party verification' },
  { name: 'Transfer Agents', description: 'Securities servicing' },
];

export default function RWAPage() {
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
            <div className="badge badge-success mt-4 mb-4">Real World Assets</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Tokenize Real World Assets
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Bridge traditional finance with blockchain. Tokenize real estate, 
              commodities, securities, and more with built-in compliance.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                Start Tokenizing
              </Link>
              <a href="#" className="btn-secondary">
                Talk to Sales
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-3 gap-6 text-center">
            <div className="glass-card p-6">
              <p className="text-3xl font-bold gradient-text">$500T+</p>
              <p className="text-sm text-gray-400 mt-2">Addressable Market</p>
            </div>
            <div className="glass-card p-6">
              <p className="text-3xl font-bold gradient-text">24/7</p>
              <p className="text-sm text-gray-400 mt-2">Trading & Settlement</p>
            </div>
            <div className="glass-card p-6">
              <p className="text-3xl font-bold gradient-text">T+0</p>
              <p className="text-sm text-gray-400 mt-2">Instant Settlement</p>
            </div>
          </div>
        </div>
      </section>

      {/* Asset Classes */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Supported Asset Classes</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {assetClasses.map((asset, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-start justify-between mb-4">
                  <div className="p-3 rounded-xl bg-orange-500/20 text-orange-400">
                    {asset.icon}
                  </div>
                  <span className="text-emerald-400 font-medium">{asset.marketSize}</span>
                </div>
                <h3 className="text-xl font-semibold text-white mb-2">{asset.name}</h3>
                <p className="text-gray-400 mb-4">{asset.description}</p>
                <div className="flex flex-wrap gap-2">
                  {asset.examples.map((example, i) => (
                    <span key={i} className="badge badge-default">{example}</span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Platform Features</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {features.map((feature, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-2 rounded-lg bg-emerald-500/20 text-emerald-400 w-fit mb-4">
                  {feature.icon}
                </div>
                <h3 className="font-semibold text-white mb-2">{feature.title}</h3>
                <p className="text-sm text-gray-400">{feature.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Compliance */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Built-in Compliance</h2>
              <p className="text-gray-400 mb-6">
                X3 STAR's token extensions enable programmable compliance that travels 
                with the token. Build regulatory requirements directly into assets.
              </p>
              <ul className="space-y-3">
                {complianceFeatures.map((feature, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-5 h-5 text-emerald-500 mr-3" />
                    {feature}
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-8">
              <h3 className="font-semibold text-white mb-6">Compliance Flow</h3>
              <div className="space-y-4">
                {[
                  { step: '1', title: 'Investor Onboarding', desc: 'KYC/AML verification' },
                  { step: '2', title: 'Accreditation Check', desc: 'Verify investor status' },
                  { step: '3', title: 'Token Issuance', desc: 'Mint compliant tokens' },
                  { step: '4', title: 'Ongoing Compliance', desc: 'Transfer restrictions enforced' },
                ].map((item, i) => (
                  <div key={i} className="flex items-center">
                    <div className="w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center mr-4">
                      <span className="text-orange-400 font-bold">{item.step}</span>
                    </div>
                    <div>
                      <p className="text-white font-medium">{item.title}</p>
                      <p className="text-sm text-gray-400">{item.desc}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Partners */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Ecosystem Partners</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {partners.map((partner, index) => (
              <div key={index} className="glass-card p-6 text-center">
                <h3 className="font-semibold text-white mb-2">{partner.name}</h3>
                <p className="text-sm text-gray-400">{partner.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Map className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Ready to Tokenize Your Assets?
          </h2>
          <p className="text-gray-400 mb-8">
            Connect with our team to discuss your tokenization project.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#" className="btn-primary">
              Contact Sales
            </a>
            <Link href="/developers/docs" className="btn-secondary">
              Technical Docs
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
