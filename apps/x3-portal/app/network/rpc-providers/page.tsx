'use client';

import React from 'react';
import Link from 'next/link';
import {
  Server,
  Zap,
  Globe,
  Shield,
  CheckCircle2,
  Copy,
  ExternalLink,
  Code,
  Clock,
  Users,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const rpcProviders = [
  {
    name: 'X3 Atlas Testnet (Official)',
    description: 'Official X3 Atlas Sphere testnet RPC maintained by the core team. Full archive node with EVM and SVM support.',
    tier: 'Free',
    features: ['Archive Node', 'EVM + SVM', 'Faucet Available'],
    endpoints: {
      http: 'http://rpc.testnet.atlas-sphere.io:9944',
      ws: 'ws://rpc.testnet.atlas-sphere.io:9944',
    },
    limits: '1000 req/min',
    latency: '~50ms',
    chainId: '9944',
    evmChainId: '2151',
  },
  {
    name: 'Local Development Node',
    description: 'Run your own local development node for testing. Includes Alice/Bob dev accounts with pre-funded balances.',
    tier: 'Dev',
    features: ['Instant Blocks', 'Pre-funded Accounts', 'Full Control'],
    endpoints: {
      http: 'http://127.0.0.1:9944',
      ws: 'ws://127.0.0.1:9944',
    },
    limits: 'Unlimited',
    latency: '<1ms',
    chainId: '9944',
    evmChainId: '2151',
  },
  {
    name: 'AtlasNode Pro (Coming Soon)',
    description: 'High-performance RPC with guaranteed uptime and priority support. Enterprise-ready infrastructure.',
    tier: 'Pro',
    features: ['99.9% SLA', 'Unlimited Requests', 'Priority Support'],
    endpoints: {
      http: 'https://api.atlasnode.io/v1/{API_KEY}',
      ws: 'wss://ws.atlasnode.io/v1/{API_KEY}',
    },
    limits: 'Unlimited',
    latency: '~15ms',
    chainId: '9944',
    evmChainId: '2151',
  },
];

const features = [
  {
    title: 'Low Latency',
    description: 'Global edge deployment ensures fast response times worldwide',
    icon: <Zap className="w-5 h-5" />,
  },
  {
    title: 'High Availability',
    description: 'Redundant infrastructure with automatic failover',
    icon: <Shield className="w-5 h-5" />,
  },
  {
    title: 'Full Archive',
    description: 'Access to full historical blockchain data',
    icon: <Server className="w-5 h-5" />,
  },
  {
    title: 'WebSocket Support',
    description: 'Real-time subscriptions and event streaming',
    icon: <Globe className="w-5 h-5" />,
  },
];

const methods = [
  { method: 'eth_blockNumber', description: 'Get current block number (EVM)' },
  { method: 'eth_call', description: 'Execute contract call (EVM)' },
  { method: 'eth_sendRawTransaction', description: 'Submit transaction (EVM)' },
  { method: 'eth_getBalance', description: 'Get account balance (EVM)' },
  { method: 'atlasKernel_submitComit', description: 'Submit cross-VM Comit transaction' },
  { method: 'atlasKernel_getCanonicalBalance', description: 'Query canonical ledger balance' },
  { method: 'atlasKernel_isAuthorized', description: 'Check account authorization status' },
  { method: 'atlasKernel_getAssetMetadata', description: 'Get asset info (symbol, decimals)' },
  { method: 'state_getStorage', description: 'Query Substrate state storage' },
  { method: 'chain_getBlock', description: 'Get block by hash (Substrate)' },
  { method: 'chain_getBlockHash', description: 'Get block hash by number' },
  { method: 'system_health', description: 'Get node health status' },
];

const TierBadge = ({ tier }: { tier: string }) => {
  const colors: Record<string, string> = {
    Free: 'badge-default',
    Pro: 'badge-success',
    Enterprise: 'badge-purple',
  };
  return <span className={`badge ${colors[tier] || 'badge-default'}`}>{tier}</span>;
};

export default function RPCProvidersPage() {
  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

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
            <div className="badge badge-info mt-4 mb-4">Infrastructure</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              RPC Providers
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Connect to the X3 Atlas Sphere network through reliable RPC providers. 
              Both Substrate native (Polkadot API) and EVM-compatible (ethers.js) connections supported.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="#providers" className="btn-primary">
                View Providers
              </Link>
              <Link href="/developers/api" className="btn-secondary">
                API Reference
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

      {/* Providers */}
      <section id="providers" className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Available Providers</h2>
          
          <div className="space-y-6">
            {rpcProviders.map((provider, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-6">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="text-xl font-semibold text-white">{provider.name}</h3>
                      <TierBadge tier={provider.tier} />
                    </div>
                    <p className="text-gray-400 mb-4">{provider.description}</p>
                    
                    <div className="flex flex-wrap gap-2 mb-4">
                      {provider.features.map((feature, i) => (
                        <span key={i} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                          {feature}
                        </span>
                      ))}
                    </div>
                    
                    <div className="flex items-center gap-6 text-sm text-gray-400">
                      <span className="flex items-center">
                        <Clock className="w-4 h-4 mr-1" />
                        {provider.latency} latency
                      </span>
                      <span className="flex items-center">
                        <Users className="w-4 h-4 mr-1" />
                        {provider.limits}
                      </span>
                    </div>
                  </div>
                  
                  <div className="lg:w-96 space-y-3">
                    <div className="p-3 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a]">
                      <div className="flex items-center justify-between mb-1">
                        <span className="text-xs text-gray-400">HTTP</span>
                        <button
                          onClick={() => copyToClipboard(provider.endpoints.http)}
                          className="text-gray-400 hover:text-white"
                        >
                          <Copy className="w-4 h-4" />
                        </button>
                      </div>
                      <code className="text-sm text-cyan-400 break-all">{provider.endpoints.http}</code>
                    </div>
                    <div className="p-3 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a]">
                      <div className="flex items-center justify-between mb-1">
                        <span className="text-xs text-gray-400">WebSocket</span>
                        <button
                          onClick={() => copyToClipboard(provider.endpoints.ws)}
                          className="text-gray-400 hover:text-white"
                        >
                          <Copy className="w-4 h-4" />
                        </button>
                      </div>
                      <code className="text-sm text-cyan-400 break-all">{provider.endpoints.ws}</code>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Quick Start */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Quick Start</h2>
          
          <div className="grid lg:grid-cols-2 gap-8">
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center">
                <Code className="w-4 h-4 text-gray-400 mr-2" />
                <span className="text-sm text-gray-400">JavaScript / ethers.js (EVM)</span>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{`import { ethers } from 'ethers';

// Connect to X3 Atlas Sphere Testnet
const provider = new ethers.JsonRpcProvider(
  'http://rpc.testnet.atlas-sphere.io:9944'
);

// Get block number
const blockNumber = await provider.getBlockNumber();
console.log('Current block:', blockNumber);

// Or connect locally
// const localProvider = new ethers.JsonRpcProvider(
//   'http://127.0.0.1:9944'
// );`}</code>
              </pre>
            </div>
            
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center">
                <Code className="w-4 h-4 text-gray-400 mr-2" />
                <span className="text-sm text-gray-400">JavaScript / @polkadot/api (Substrate)</span>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{`import { ApiPromise, WsProvider } from '@polkadot/api';

// Connect to X3 Atlas Sphere
const wsProvider = new WsProvider(
  'ws://rpc.testnet.atlas-sphere.io:9944'
);
const api = await ApiPromise.create({ provider: wsProvider });

// Query canonical balance
const balance = await api.rpc.atlasKernel
  .getCanonicalBalance(accountId, 0); // Asset ID 0 = X3
console.log('X3 Balance:', balance.toHuman());`}</code>
              </pre>
            </div>
          </div>
          
          {/* MetaMask Config */}
          <div className="mt-8 glass-card p-6">
            <h3 className="text-lg font-semibold text-white mb-4">MetaMask Network Configuration</h3>
            <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
              <div className="p-3 rounded-lg bg-[#0a0a0a]">
                <span className="text-xs text-gray-500">Network Name</span>
                <div className="text-white font-mono">X3 Atlas Testnet</div>
              </div>
              <div className="p-3 rounded-lg bg-[#0a0a0a]">
                <span className="text-xs text-gray-500">RPC URL</span>
                <div className="text-cyan-400 font-mono text-sm break-all">http://rpc.testnet.atlas-sphere.io:9944</div>
              </div>
              <div className="p-3 rounded-lg bg-[#0a0a0a]">
                <span className="text-xs text-gray-500">Chain ID</span>
                <div className="text-white font-mono">2151</div>
              </div>
              <div className="p-3 rounded-lg bg-[#0a0a0a]">
                <span className="text-xs text-gray-500">Currency Symbol</span>
                <div className="text-orange-400 font-mono">X3</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Supported Methods */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Supported Methods</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {methods.map((method, index) => (
              <div key={index} className="glass-card p-4">
                <code className="text-orange-400 text-sm">{method.method}</code>
                <p className="text-sm text-gray-400 mt-2">{method.description}</p>
              </div>
            ))}
          </div>
          
          <div className="mt-8 text-center">
            <Link href="/developers/api" className="btn-secondary">
              View Full API Reference
            </Link>
          </div>
        </div>
      </section>

      {/* Become Provider */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">
            Become an RPC Provider
          </h2>
          <p className="text-gray-400 mb-8">
            Run your own X3 STAR node and join the network of RPC providers. 
            Earn rewards while supporting network decentralization.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Node Setup Guide
            </Link>
            <Link href="/community/forum" className="btn-secondary">
              Provider Community
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
