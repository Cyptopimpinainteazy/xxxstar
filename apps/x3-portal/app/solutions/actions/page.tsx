'use client';

import React from 'react';
import Link from 'next/link';
import {
  Link2,
  Share2,
  Zap,
  QrCode,
  MessageSquare,
  ShoppingCart,
  Vote,
  Gift,
  ArrowRight,
  ExternalLink,
  Code,
  Smartphone,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const actionTypes = [
  {
    name: 'Transfer',
    description: 'Send tokens with a simple link',
    icon: <Share2 className="w-6 h-6" />,
    example: 'x3star://send?to=0x...&amount=10&token=USDC',
  },
  {
    name: 'Swap',
    description: 'Initiate token swaps directly',
    icon: <Zap className="w-6 h-6" />,
    example: 'x3star://swap?from=ATLAS&to=USDC&amount=100',
  },
  {
    name: 'Vote',
    description: 'Cast governance votes',
    icon: <Vote className="w-6 h-6" />,
    example: 'x3star://vote?proposal=123&choice=yes',
  },
  {
    name: 'Mint',
    description: 'Mint NFTs or tokens',
    icon: <Gift className="w-6 h-6" />,
    example: 'x3star://mint?collection=0x...&id=42',
  },
];

const useCases = [
  {
    title: 'Social Payments',
    description: 'Share payment links on Twitter, Discord, or anywhere',
    icon: <MessageSquare className="w-6 h-6" />,
    color: 'from-blue-500 to-cyan-500',
  },
  {
    title: 'E-Commerce',
    description: 'Add "Pay with X3" buttons to your store',
    icon: <ShoppingCart className="w-6 h-6" />,
    color: 'from-emerald-500 to-green-500',
  },
  {
    title: 'QR Payments',
    description: 'Generate QR codes for in-person payments',
    icon: <QrCode className="w-6 h-6" />,
    color: 'from-purple-500 to-indigo-500',
  },
  {
    title: 'Deep Linking',
    description: 'Open dApps directly to specific actions',
    icon: <Link2 className="w-6 h-6" />,
    color: 'from-orange-500 to-amber-500',
  },
];

const codeExample = `// Create an Action link
import { ActionBuilder } from '@x3star/actions';

// Create a payment action
const paymentAction = ActionBuilder.payment({
  recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f...',
  amount: '25.00',
  currency: 'USDC',
  memo: 'Coffee payment',
});

console.log(paymentAction.url);
// x3star://pay?to=0x742d...&amount=25&token=USDC&memo=Coffee

// Generate QR code
const qrCode = await paymentAction.toQRCode();

// Create a Blink (shareable card)
const blink = await paymentAction.toBlink({
  title: 'Pay for Coffee',
  description: 'Scan to pay $25 USDC',
  image: 'https://example.com/coffee.png',
});

// Share on social media
console.log(blink.shareUrl);`;

const blinkExample = {
  title: 'Donate to X3 Foundation',
  description: 'Support open source development',
  action: 'Donate',
  amounts: ['$5', '$10', '$25', 'Custom'],
};

export default function ActionsPage() {
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
            <div className="badge badge-info mt-4 mb-4">Actions & Blinks</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Blockchain Links & Actions
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Create shareable links that execute blockchain transactions. 
              Blinks turn any URL into a transaction-ready experience.
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

      {/* Action Types */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Action Types</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {actionTypes.map((action, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400 w-fit mb-4">
                  {action.icon}
                </div>
                <h3 className="font-semibold text-white mb-2">{action.name}</h3>
                <p className="text-sm text-gray-400 mb-4">{action.description}</p>
                <code className="text-xs text-cyan-400 break-all">{action.example}</code>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Blink Preview */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">What are Blinks?</h2>
              <p className="text-gray-400 mb-6">
                Blinks are rich, shareable cards that preview blockchain actions. 
                When shared on social media, they unfurl into interactive transaction cards.
              </p>
              <ul className="space-y-3">
                {[
                  'Rich previews on Twitter, Discord, Telegram',
                  'One-click transactions from any platform',
                  'Customizable appearance and branding',
                  'Built-in analytics and tracking',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <Zap className="w-5 h-5 text-orange-400 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-6 max-w-sm mx-auto">
              <div className="aspect-video bg-gradient-to-br from-orange-500/20 to-purple-500/20 rounded-lg mb-4 flex items-center justify-center">
                <span className="text-6xl">🎁</span>
              </div>
              <h3 className="font-semibold text-white mb-2">{blinkExample.title}</h3>
              <p className="text-sm text-gray-400 mb-4">{blinkExample.description}</p>
              <div className="grid grid-cols-4 gap-2 mb-4">
                {blinkExample.amounts.map((amount, i) => (
                  <button
                    key={i}
                    className={`py-2 rounded-lg text-sm ${i === 1 ? 'bg-orange-500 text-white' : 'bg-[#111111] text-gray-400 hover:bg-[#1a1a1a]'}`}
                  >
                    {amount}
                  </button>
                ))}
              </div>
              <button className="w-full btn-primary">{blinkExample.action}</button>
            </div>
          </div>
        </div>
      </section>

      {/* Use Cases */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Use Cases</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {useCases.map((useCase, index) => (
              <div key={index} className="glass-card p-6">
                <div className={`p-3 rounded-xl bg-gradient-to-br ${useCase.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{useCase.icon}</span>
                </div>
                <h3 className="font-semibold text-white mb-2">{useCase.title}</h3>
                <p className="text-sm text-gray-400">{useCase.description}</p>
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
              <h2 className="text-2xl font-bold text-white mb-4">Easy to Create</h2>
              <p className="text-gray-400 mb-6">
                Build Actions and Blinks with our SDK. Generate shareable links, 
                QR codes, and rich previews in just a few lines of code.
              </p>
              <Link href="/developers/docs" className="btn-primary">
                View Documentation
              </Link>
            </div>
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center justify-between">
                <span className="text-sm text-gray-400">create-action.ts</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{codeExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Link2 className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Create Your First Action
          </h2>
          <p className="text-gray-400 mb-8">
            Start building shareable blockchain experiences today.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Get Started
            </Link>
            <Link href="/developers/cookbook" className="btn-secondary">
              View Examples
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
