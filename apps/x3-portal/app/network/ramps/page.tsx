'use client';

import React from 'react';
import Link from 'next/link';
import {
  ArrowUpDown,
  CreditCard,
  Building2,
  Globe,
  Shield,
  Zap,
  CheckCircle2,
  ExternalLink,
  AlertCircle,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const onramps = [
  {
    name: 'MoonPay',
    type: 'Fiat On-Ramp',
    description: 'Buy crypto with credit card, bank transfer, or Apple Pay',
    features: ['170+ countries', '80+ currencies', 'Instant delivery'],
    methods: ['Credit Card', 'Debit Card', 'Bank Transfer', 'Apple Pay'],
    status: 'Live',
  },
  {
    name: 'Transak',
    type: 'Fiat On-Ramp',
    description: 'Global fiat to crypto gateway',
    features: ['100+ countries', 'Low fees', 'Fast KYC'],
    methods: ['Credit Card', 'Bank Transfer', 'Google Pay'],
    status: 'Live',
  },
  {
    name: 'Ramp Network',
    type: 'Fiat On-Ramp',
    description: 'Non-custodial fiat gateway',
    features: ['EU & UK focus', 'Open Banking', 'Self-custody'],
    methods: ['Bank Transfer', 'Card', 'Open Banking'],
    status: 'Live',
  },
  {
    name: 'Wyre',
    type: 'Fiat On-Ramp',
    description: 'Enterprise-grade payment rails',
    features: ['US focus', 'ACH support', 'API access'],
    methods: ['ACH', 'Wire Transfer', 'Debit Card'],
    status: 'Coming Soon',
  },
];

const offramps = [
  {
    name: 'Circle',
    type: 'USDC Off-Ramp',
    description: 'Convert USDC to fiat bank transfer',
    features: ['Same-day settlement', 'Business accounts', 'No minimum'],
    status: 'Live',
  },
  {
    name: 'Coinbase',
    type: 'Exchange Off-Ramp',
    description: 'Sell to fiat via Coinbase account',
    features: ['Multiple currencies', 'Instant sell', 'PayPal withdrawal'],
    status: 'Live',
  },
  {
    name: 'Kraken',
    type: 'Exchange Off-Ramp',
    description: 'Professional-grade off-ramp',
    features: ['Wire transfer', 'SEPA', 'Low fees'],
    status: 'Live',
  },
];

const fees = [
  { method: 'Credit Card', percentage: '2.5-4.5%', time: 'Instant' },
  { method: 'Debit Card', percentage: '1.5-3%', time: 'Instant' },
  { method: 'Bank Transfer', percentage: '0.5-1.5%', time: '1-3 days' },
  { method: 'Wire Transfer', percentage: '0.1-0.5%', time: '1-2 days' },
];

export default function RampsPage() {
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
            <Link href="/network" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Network
            </Link>
            <div className="badge badge-info mt-4 mb-4">Fiat Gateway</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              On & Off Ramps
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Convert between fiat currency and crypto. Multiple providers for 
              global coverage with competitive rates.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#onramps" className="btn-primary">
                Buy Crypto
              </a>
              <a href="#offramps" className="btn-secondary">
                Sell Crypto
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Quick Stats */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-4 gap-6">
            {[
              { icon: <Globe className="w-5 h-5" />, value: '170+', label: 'Countries' },
              { icon: <CreditCard className="w-5 h-5" />, value: '80+', label: 'Currencies' },
              { icon: <Zap className="w-5 h-5" />, value: '<5min', label: 'Avg. Time' },
              { icon: <Shield className="w-5 h-5" />, value: '100%', label: 'Compliant' },
            ].map((stat, index) => (
              <div key={index} className="flex items-center gap-4">
                <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400">
                  {stat.icon}
                </div>
                <div>
                  <p className="text-2xl font-bold text-white">{stat.value}</p>
                  <p className="text-sm text-gray-400">{stat.label}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* On-Ramps */}
      <section id="onramps" className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">On-Ramps (Buy Crypto)</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {onramps.map((ramp, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-start justify-between mb-4">
                  <div>
                    <h3 className="text-xl font-semibold text-white">{ramp.name}</h3>
                    <span className="text-sm text-gray-400">{ramp.type}</span>
                  </div>
                  <span className={`badge ${ramp.status === 'Live' ? 'badge-success' : 'badge-warning'}`}>
                    {ramp.status}
                  </span>
                </div>
                <p className="text-gray-400 mb-4">{ramp.description}</p>
                <div className="flex flex-wrap gap-2 mb-4">
                  {ramp.features.map((feature, i) => (
                    <span key={i} className="badge badge-default">{feature}</span>
                  ))}
                </div>
                <div className="border-t border-[#1a1a1a] pt-4">
                  <p className="text-sm text-gray-400 mb-2">Payment Methods</p>
                  <div className="flex flex-wrap gap-2">
                    {ramp.methods.map((method, i) => (
                      <span key={i} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                        {method}
                      </span>
                    ))}
                  </div>
                </div>
                {ramp.status === 'Live' && (
                  <button className="btn-primary w-full mt-4">
                    Buy with {ramp.name}
                    <ExternalLink className="w-4 h-4 ml-2" />
                  </button>
                )}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Off-Ramps */}
      <section id="offramps" className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Off-Ramps (Sell Crypto)</h2>
          
          <div className="grid md:grid-cols-3 gap-6">
            {offramps.map((ramp, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-start justify-between mb-4">
                  <h3 className="text-lg font-semibold text-white">{ramp.name}</h3>
                  <span className={`badge ${ramp.status === 'Live' ? 'badge-success' : 'badge-warning'}`}>
                    {ramp.status}
                  </span>
                </div>
                <p className="text-sm text-gray-400 mb-4">{ramp.description}</p>
                <ul className="space-y-2">
                  {ramp.features.map((feature, i) => (
                    <li key={i} className="flex items-center text-gray-400 text-sm">
                      <CheckCircle2 className="w-4 h-4 text-emerald-500 mr-2" />
                      {feature}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Fee Comparison */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Fee Comparison</h2>
          
          <div className="glass-card overflow-hidden">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left p-4 text-gray-400 font-medium">Method</th>
                  <th className="text-left p-4 text-gray-400 font-medium">Typical Fees</th>
                  <th className="text-left p-4 text-gray-400 font-medium">Processing Time</th>
                </tr>
              </thead>
              <tbody>
                {fees.map((fee, index) => (
                  <tr key={index} className="border-b border-[#0a0a0a]">
                    <td className="p-4 text-white">{fee.method}</td>
                    <td className="p-4 text-gray-400">{fee.percentage}</td>
                    <td className="p-4 text-gray-400">{fee.time}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          
          <div className="mt-4 p-4 rounded-lg bg-amber-500/10 border border-amber-500/30 flex items-start">
            <AlertCircle className="w-5 h-5 text-amber-400 mr-3 mt-0.5" />
            <p className="text-sm text-amber-200">
              Fees vary by provider, region, and payment method. Always check the final fee 
              before completing a transaction. Network gas fees may apply additionally.
            </p>
          </div>
        </div>
      </section>

      {/* Integration */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Integrate Ramps in Your dApp</h2>
              <p className="text-gray-400 mb-6">
                Add fiat on/off ramps directly in your application. Our partners provide 
                embeddable widgets and APIs for seamless integration.
              </p>
              <ul className="space-y-3 mb-6">
                {[
                  'White-label widget integration',
                  'Direct API access',
                  'Webhook notifications',
                  'Custom branding options',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-5 h-5 text-emerald-500 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
              <Link href="/developers/docs" className="btn-primary">
                Integration Guide
              </Link>
            </div>
            <div className="glass-card p-8 text-center">
              <Building2 className="w-16 h-16 text-orange-400 mx-auto mb-4" />
              <h3 className="text-xl font-semibold text-white mb-2">Partner Program</h3>
              <p className="text-gray-400 mb-4">
                Are you a payment provider? Join our partner program.
              </p>
              <a href="#" className="btn-secondary">
                Become a Partner
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <ArrowUpDown className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Ready to Get Started?
          </h2>
          <p className="text-gray-400 mb-8">
            Buy your first ATLAS tokens in minutes with your preferred payment method.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#onramps" className="btn-primary">
              Buy ATLAS
            </a>
            <Link href="/solutions/wallets" className="btn-secondary">
              Get a Wallet
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
