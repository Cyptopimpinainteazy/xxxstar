'use client';

import React from 'react';
import Link from 'next/link';
import {
  CreditCard,
  Globe,
  Zap,
  Shield,
  QrCode,
  Smartphone,
  Building2,
  ArrowLeftRight,
  CheckCircle2,
  Code,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const paymentFeatures = [
  {
    name: 'Instant Settlement',
    description: 'Transactions settle in seconds, not days',
    icon: <Zap className="w-6 h-6" />,
  },
  {
    name: 'Global Reach',
    description: 'Accept payments from anywhere in the world',
    icon: <Globe className="w-6 h-6" />,
  },
  {
    name: 'Low Fees',
    description: 'Fraction of traditional payment processor costs',
    icon: <CreditCard className="w-6 h-6" />,
  },
  {
    name: 'Secure',
    description: 'Cryptographic security for all transactions',
    icon: <Shield className="w-6 h-6" />,
  },
];

const integrations = [
  {
    name: 'Payment Links',
    description: 'Generate shareable payment links for any amount',
    features: ['No code required', 'Custom amounts', 'QR codes', 'Expiration'],
  },
  {
    name: 'Checkout Widget',
    description: 'Embed a payment button on any website',
    features: ['One-line integration', 'Custom styling', 'Multi-currency', 'Callbacks'],
  },
  {
    name: 'API Integration',
    description: 'Full control with our REST and WebSocket APIs',
    features: ['Programmatic payments', 'Webhooks', 'Batch payments', 'Subscriptions'],
  },
  {
    name: 'POS System',
    description: 'Accept payments at physical locations',
    features: ['NFC support', 'Receipt printing', 'Inventory sync', 'Multi-terminal'],
  },
];

const currencies = [
  { symbol: 'ATLAS', name: 'X3 STAR', type: 'Native' },
  { symbol: 'USDC', name: 'USD Coin', type: 'Stablecoin' },
  { symbol: 'USDT', name: 'Tether', type: 'Stablecoin' },
  { symbol: 'ETH', name: 'Ethereum', type: 'Bridged' },
  { symbol: 'BTC', name: 'Bitcoin', type: 'Bridged' },
  { symbol: 'DAI', name: 'Dai', type: 'Stablecoin' },
];

const codeExample = `// Accept a payment with X3 Pay
import { X3Pay } from '@x3star/payments';

const pay = new X3Pay({ apiKey: process.env.X3_PAY_KEY });

// Create a payment request
const payment = await pay.createPayment({
  amount: '49.99',
  currency: 'USDC',
  description: 'Premium Subscription',
  metadata: { userId: 'user_123' },
  
  // Optional: auto-convert to merchant's preferred currency
  settleCurrency: 'USDC',
});

// Redirect customer to payment page
// or embed checkout widget
console.log('Payment URL:', payment.checkoutUrl);

// Listen for payment completion
pay.on('payment.completed', (event) => {
  console.log('Payment received:', event.paymentId);
});`;

export default function PaymentsPage() {
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
            <div className="badge badge-success mt-4 mb-4">Payments</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Accept Crypto Payments
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Fast, secure, and affordable payments for businesses of all sizes. 
              Accept payments globally with instant settlement.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                Get Started
              </Link>
              <a href="#" className="btn-secondary">
                View Demo
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="py-12 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {paymentFeatures.map((feature, index) => (
              <div key={index} className="flex items-start">
                <div className="p-2 rounded-lg bg-emerald-500/20 text-emerald-400 mr-4">
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

      {/* Integrations */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Integration Options</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {integrations.map((integration, index) => (
              <div key={index} className="glass-card p-6">
                <h3 className="text-xl font-semibold text-white mb-2">{integration.name}</h3>
                <p className="text-gray-400 mb-4">{integration.description}</p>
                <div className="flex flex-wrap gap-2">
                  {integration.features.map((feature, i) => (
                    <span key={i} className="badge badge-default">{feature}</span>
                  ))}
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
              <h2 className="text-2xl font-bold text-white mb-4">Simple Integration</h2>
              <p className="text-gray-400 mb-6">
                Start accepting payments with just a few lines of code. Our SDK 
                handles all the complexity of blockchain payments.
              </p>
              <ul className="space-y-3">
                {[
                  'Accept any supported cryptocurrency',
                  'Auto-convert to stablecoins',
                  'Webhook notifications',
                  'Detailed analytics apps/dash-legacy-2-legacy-2board',
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
                <span className="text-sm text-gray-400">payment-integration.ts</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{codeExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Supported Currencies */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Supported Currencies</h2>
          
          <div className="grid sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            {currencies.map((currency, index) => (
              <div key={index} className="glass-card p-4 text-center">
                <p className="text-xl font-bold text-white mb-1">{currency.symbol}</p>
                <p className="text-sm text-gray-400">{currency.name}</p>
                <span className="badge badge-default text-xs mt-2">{currency.type}</span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Pricing */}
      <section className="py-16 bg-[#050505]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <h2 className="text-2xl font-bold text-white mb-8 text-center">Simple Pricing</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            <div className="glass-card p-8">
              <h3 className="text-xl font-semibold text-white mb-2">Standard</h3>
              <div className="text-4xl font-bold gradient-text mb-4">0.5%</div>
              <p className="text-gray-400 mb-6">Per transaction, no hidden fees</p>
              <ul className="space-y-3">
                {['Unlimited transactions', 'All currencies', 'API access', 'Email support'].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-4 h-4 text-emerald-500 mr-2" />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-8 border-orange-500/30">
              <h3 className="text-xl font-semibold text-white mb-2">Enterprise</h3>
              <div className="text-4xl font-bold gradient-text mb-4">Custom</div>
              <p className="text-gray-400 mb-6">Volume discounts available</p>
              <ul className="space-y-3">
                {['Custom rates', 'Dedicated support', 'SLA guarantee', 'Custom integrations'].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-4 h-4 text-emerald-500 mr-2" />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <CreditCard className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Start Accepting Payments Today
          </h2>
          <p className="text-gray-400 mb-8">
            Set up your merchant account in minutes and start accepting crypto payments.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#" className="btn-primary">
              Create Account
            </a>
            <Link href="/developers/docs" className="btn-secondary">
              Read the Docs
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
