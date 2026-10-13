'use client';

import React from 'react';
import Link from 'next/link';
import {
  ShoppingCart,
  Store,
  CreditCard,
  Package,
  BarChart3,
  Globe,
  Zap,
  Shield,
  CheckCircle2,
  Code,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const features = [
  {
    name: 'Easy Integration',
    description: 'Drop-in plugins for popular e-commerce platforms',
    icon: <ShoppingCart className="w-6 h-6" />,
  },
  {
    name: 'Multi-Currency',
    description: 'Accept ATLAS, stablecoins, and more',
    icon: <CreditCard className="w-6 h-6" />,
  },
  {
    name: 'Instant Settlement',
    description: 'Funds available immediately after confirmation',
    icon: <Zap className="w-6 h-6" />,
  },
  {
    name: 'Global Reach',
    description: 'Accept payments from anywhere in the world',
    icon: <Globe className="w-6 h-6" />,
  },
];

const platforms = [
  {
    name: 'Shopify',
    description: 'One-click install from App Store',
    status: 'Available',
    features: ['Checkout integration', 'Order sync', 'Refunds'],
  },
  {
    name: 'WooCommerce',
    description: 'WordPress plugin for WooCommerce',
    status: 'Available',
    features: ['Gateway plugin', 'Webhooks', 'Multi-currency'],
  },
  {
    name: 'Magento',
    description: 'Extension for Magento 2',
    status: 'Available',
    features: ['Payment module', 'Admin panel', 'B2B support'],
  },
  {
    name: 'Custom',
    description: 'REST API for custom integrations',
    status: 'Available',
    features: ['Full API access', 'Webhooks', 'SDK support'],
  },
];

const benefits = [
  { metric: '0.5%', label: 'Transaction Fee', sublabel: 'vs 2.9% credit cards' },
  { metric: '0', label: 'Chargebacks', sublabel: 'Irreversible payments' },
  { metric: '<2s', label: 'Settlement', sublabel: 'vs days with banks' },
  { metric: '200+', label: 'Countries', sublabel: 'Global acceptance' },
];

const codeExample = `// Commerce API Example
import { X3Commerce } from '@x3star/commerce';

const commerce = new X3Commerce({ 
  apiKey: process.env.X3_COMMERCE_KEY,
  webhookSecret: process.env.WEBHOOK_SECRET,
});

// Create checkout session
app.post('/checkout', async (req, res) => {
  const { items, customerId } = req.body;
  
  const session = await commerce.createCheckout({
    items: items.map(item => ({
      name: item.name,
      price: item.price,
      quantity: item.quantity,
    })),
    currency: 'USDC',
    customerId,
    successUrl: 'https://yourstore.com/success',
    cancelUrl: 'https://yourstore.com/cart',
  });
  
  res.json({ checkoutUrl: session.url });
});

// Handle webhook
app.post('/frontend/webhook', (req, res) => {
  const event = commerce.verifyWebhook(req);
  
  if (event.type === 'payment.completed') {
    // Fulfill order
    fulfillOrder(event.data.orderId);
  }
  
  res.sendStatus(200);
});`;

export default function CommercePage() {
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
            <div className="badge badge-success mt-4 mb-4">Commerce</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Commerce Tooling
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Accept crypto payments on your store. Plugins for Shopify, WooCommerce, 
              Magento, and custom integrations via our API.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                Get Started
              </Link>
              <a href="#" className="btn-secondary">
                View Demo Store
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Benefits */}
      <section className="py-12 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {benefits.map((benefit, index) => (
              <div key={index} className="glass-card p-6 text-center">
                <p className="text-3xl font-bold gradient-text mb-1">{benefit.metric}</p>
                <p className="text-white font-medium">{benefit.label}</p>
                <p className="text-sm text-gray-400">{benefit.sublabel}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Features</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {features.map((feature, index) => (
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

      {/* Platforms */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Supported Platforms</h2>
          
          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
            {platforms.map((platform, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-xl font-semibold text-white">{platform.name}</h3>
                  <span className="badge badge-success text-xs">{platform.status}</span>
                </div>
                <p className="text-sm text-gray-400 mb-4">{platform.description}</p>
                <div className="flex flex-wrap gap-2">
                  {platform.features.map((feature, i) => (
                    <span key={i} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                      {feature}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Code Example */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Custom Integration</h2>
              <p className="text-gray-400 mb-6">
                Building a custom store? Use our Commerce API to create checkout 
                sessions, handle webhooks, and manage orders programmatically.
              </p>
              <ul className="space-y-3">
                {[
                  'Hosted checkout pages',
                  'Webhook notifications',
                  'Order management API',
                  'Detailed analytics',
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
                <span className="text-sm text-gray-400">server.ts</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{codeExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Dashboard Preview */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Merchant Dashboard</h2>
          
          <div className="glass-card p-6">
            <div className="grid md:grid-cols-4 gap-6 mb-6">
              {[
                { label: 'Today', value: '$1,234', change: '+12%' },
                { label: 'This Week', value: '$8,567', change: '+8%' },
                { label: 'This Month', value: '$32,456', change: '+23%' },
                { label: 'Orders', value: '156', change: '+5%' },
              ].map((stat, i) => (
                <div key={i} className="p-4 rounded-lg bg-[#0a0a0a]">
                  <p className="text-sm text-gray-400 mb-1">{stat.label}</p>
                  <p className="text-2xl font-bold text-white">{stat.value}</p>
                  <p className="text-sm text-emerald-400">{stat.change}</p>
                </div>
              ))}
            </div>
            <div className="h-48 rounded-lg bg-[#0a0a0a] flex items-center justify-center">
              <BarChart3 className="w-12 h-12 text-gray-600" />
              <span className="ml-4 text-gray-500">Revenue Chart Preview</span>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Store className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Start Accepting Crypto Payments
          </h2>
          <p className="text-gray-400 mb-8">
            Set up your merchant account in minutes and start accepting payments.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#" className="btn-primary">
              Create Merchant Account
            </a>
            <Link href="/developers/docs" className="btn-secondary">
              View Documentation
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
