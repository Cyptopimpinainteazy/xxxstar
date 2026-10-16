'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  Code,
  Copy,
  Check,
  ChevronRight,
  Search,
  Database,
  Zap,
  Shield,
  Coins,
  FileText,
  Terminal,
} from 'lucide-react';

interface RpcMethod {
  name: string;
  description: string;
  params: { name: string; type: string; description: string; required: boolean }[];
  returns: string;
  example: {
    request: string;
    response: string;
  };
  category: string;
}

const rpcMethods: RpcMethod[] = [
  {
    name: 'atlasKernel_getCanonicalBalance',
    description: 'Query the canonical ledger balance for a specific account and asset.',
    params: [
      { name: 'account', type: 'AccountId', description: 'The account address to query', required: true },
      { name: 'asset_id', type: 'u32', description: 'The asset ID (0 for native token)', required: true },
      { name: 'at', type: 'BlockHash', description: 'Optional block hash to query at', required: false },
    ],
    returns: 'Balance (u128)',
    example: {
      request: `curl -X POST http://127.0.0.1:9944 \\
  -H "Content-Type: application/json" \\
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getCanonicalBalance",
    "params": ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 0, null]
  }'`,
      response: `{
  "jsonrpc": "2.0",
  "result": "1000000000000000000",
  "id": 1
}`,
    },
    category: 'Ledger',
  },
  {
    name: 'atlasKernel_getAssetMetadata',
    description: 'Get metadata for a registered asset including symbol and decimals.',
    params: [
      { name: 'asset_id', type: 'u32', description: 'The asset ID to query', required: true },
      { name: 'at', type: 'BlockHash', description: 'Optional block hash to query at', required: false },
    ],
    returns: 'AssetMetadata { symbol: String, decimals: u8 }',
    example: {
      request: `curl -X POST http://127.0.0.1:9944 \\
  -H "Content-Type: application/json" \\
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getAssetMetadata",
    "params": [0, null]
  }'`,
      response: `{
  "jsonrpc": "2.0",
  "result": {
    "symbol": "ATLAS",
    "decimals": 18
  },
  "id": 1
}`,
    },
    category: 'Assets',
  },
  {
    name: 'atlasKernel_isAuthorized',
    description: 'Check if an account is authorized to submit Comit transactions.',
    params: [
      { name: 'account', type: 'AccountId', description: 'The account address to check', required: true },
      { name: 'at', type: 'BlockHash', description: 'Optional block hash to query at', required: false },
    ],
    returns: 'bool',
    example: {
      request: `curl -X POST http://127.0.0.1:9944 \\
  -H "Content-Type: application/json" \\
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_isAuthorized",
    "params": ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", null]
  }'`,
      response: `{
  "jsonrpc": "2.0",
  "result": true,
  "id": 1
}`,
    },
    category: 'Authorization',
  },
  {
    name: 'atlasKernel_getAuthorizedAccounts',
    description: 'List all accounts that are authorized to submit Comit transactions.',
    params: [
      { name: 'at', type: 'BlockHash', description: 'Optional block hash to query at', required: false },
    ],
    returns: 'Vec<AccountId>',
    example: {
      request: `curl -X POST http://127.0.0.1:9944 \\
  -H "Content-Type: application/json" \\
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getAuthorizedAccounts",
    "params": [null]
  }'`,
      response: `{
  "jsonrpc": "2.0",
  "result": [
    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"
  ],
  "id": 1
}`,
    },
    category: 'Authorization',
  },
  {
    name: 'atlasKernel_getAuthorities',
    description: 'Get the current set of network authorities (validators).',
    params: [
      { name: 'at', type: 'BlockHash', description: 'Optional block hash to query at', required: false },
    ],
    returns: 'Vec<AuthorityId>',
    example: {
      request: `curl -X POST http://127.0.0.1:9944 \\
  -H "Content-Type: application/json" \\
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "atlasKernel_getAuthorities",
    "params": [null]
  }'`,
      response: `{
  "jsonrpc": "2.0",
  "result": [
    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
  ],
  "id": 1
}`,
    },
    category: 'Network',
  },
];

const categories = [
  { id: 'all', label: 'All Methods', icon: <Code className="w-4 h-4" /> },
  { id: 'Ledger', label: 'Ledger', icon: <Database className="w-4 h-4" /> },
  { id: 'Assets', label: 'Assets', icon: <Coins className="w-4 h-4" /> },
  { id: 'Authorization', label: 'Authorization', icon: <Shield className="w-4 h-4" /> },
  { id: 'Network', label: 'Network', icon: <Zap className="w-4 h-4" /> },
];

export default function ApiPage() {
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [copiedMethod, setCopiedMethod] = useState<string | null>(null);

  const filteredMethods = rpcMethods.filter((method) => {
    const categoryMatch = selectedCategory === 'all' || method.category === selectedCategory;
    const searchMatch = method.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                       method.description.toLowerCase().includes(searchQuery.toLowerCase());
    return categoryMatch && searchMatch;
  });

  const copyToClipboard = (text: string, methodName: string) => {
    navigator.clipboard.writeText(text);
    setCopiedMethod(methodName);
    setTimeout(() => setCopiedMethod(null), 2000);
  };

  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-16 relative overflow-hidden border-b border-[#1a1a1a]">
        <div className="absolute inset-0 mesh-gradient opacity-20" />
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="badge badge-purple mb-4">API Reference</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              X3 STAR RPC API
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Complete reference documentation for the X3 Atlas Sphere RPC API. 
              Interact with the Atlas Kernel and query network state.
            </p>

            {/* Endpoint info */}
            <div className="glass-card p-4 inline-block">
              <div className="text-sm text-gray-400 mb-1">Testnet RPC Endpoint</div>
              <div className="flex items-center space-x-2">
                <code className="text-orange-400 font-mono">https://rpc.testnet.atlas-sphere.io</code>
                <button
                  onClick={() => copyToClipboard('https://rpc.testnet.atlas-sphere.io', 'endpoint')}
                  className="p-1 hover:bg-[#1a1a1a] rounded"
                >
                  {copiedMethod === 'endpoint' ? (
                    <Check className="w-4 h-4 text-emerald-500" />
                  ) : (
                    <Copy className="w-4 h-4 text-gray-400" />
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <div className="container-wide py-12">
        {/* Filters */}
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-8">
          <div className="flex flex-wrap gap-2">
            {categories.map((category) => (
              <button
                key={category.id}
                onClick={() => setSelectedCategory(category.id)}
                className={`flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  selectedCategory === category.id
                    ? 'bg-orange-500 text-white'
                    : 'bg-[#0a0a0a] text-gray-400 hover:text-white hover:bg-[#1a1a1a]'
                }`}
              >
                <span className="mr-2">{category.icon}</span>
                {category.label}
              </button>
            ))}
          </div>

          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
            <input
              type="text"
              placeholder="Search methods..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10 pr-4 py-2 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-500 focus:outline-none focus:border-orange-500 w-full md:w-64"
            />
          </div>
        </div>

        {/* Methods */}
        <div className="space-y-6">
          {filteredMethods.map((method) => (
            <div key={method.name} className="glass-card overflow-hidden">
              {/* Header */}
              <div className="p-6 border-b border-[#1a1a1a]">
                <div className="flex items-start justify-between">
                  <div>
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="text-lg font-mono font-semibold text-orange-400">
                        {method.name}
                      </h3>
                      <span className="badge badge-info">{method.category}</span>
                    </div>
                    <p className="text-gray-400">{method.description}</p>
                  </div>
                </div>
              </div>

              {/* Parameters */}
              <div className="p-6 border-b border-[#1a1a1a] bg-white/2">
                <h4 className="text-sm font-semibold text-white mb-3">Parameters</h4>
                {method.params.length > 0 ? (
                  <div className="space-y-2">
                    {method.params.map((param) => (
                      <div key={param.name} className="flex items-start text-sm">
                        <code className="text-cyan-400 font-mono mr-2">{param.name}</code>
                        <span className="text-gray-600 mr-2">({param.type})</span>
                        {param.required && (
                          <span className="badge badge-warning text-xs mr-2">required</span>
                        )}
                        <span className="text-gray-400">- {param.description}</span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-gray-600 text-sm">No parameters required</p>
                )}

                <h4 className="text-sm font-semibold text-white mt-4 mb-2">Returns</h4>
                <code className="text-emerald-400 font-mono text-sm">{method.returns}</code>
              </div>

              {/* Example */}
              <div className="p-6">
                <h4 className="text-sm font-semibold text-white mb-3">Example</h4>
                
                <div className="space-y-4">
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-xs text-gray-600 uppercase tracking-wider">Request</span>
                      <button
                        onClick={() => copyToClipboard(method.example.request, `${method.name}-req`)}
                        className="flex items-center text-xs text-gray-400 hover:text-white"
                      >
                        {copiedMethod === `${method.name}-req` ? (
                          <>
                            <Check className="w-3 h-3 mr-1 text-emerald-500" />
                            Copied
                          </>
                        ) : (
                          <>
                            <Copy className="w-3 h-3 mr-1" />
                            Copy
                          </>
                        )}
                      </button>
                    </div>
                    <div className="code-block">
                      <pre className="text-xs text-gray-400 whitespace-pre-wrap">{method.example.request}</pre>
                    </div>
                  </div>

                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-xs text-gray-600 uppercase tracking-wider">Response</span>
                      <button
                        onClick={() => copyToClipboard(method.example.response, `${method.name}-res`)}
                        className="flex items-center text-xs text-gray-400 hover:text-white"
                      >
                        {copiedMethod === `${method.name}-res` ? (
                          <>
                            <Check className="w-3 h-3 mr-1 text-emerald-500" />
                            Copied
                          </>
                        ) : (
                          <>
                            <Copy className="w-3 h-3 mr-1" />
                            Copy
                          </>
                        )}
                      </button>
                    </div>
                    <div className="code-block">
                      <pre className="text-xs text-gray-400 whitespace-pre-wrap">{method.example.response}</pre>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>

        {filteredMethods.length === 0 && (
          <div className="text-center py-16">
            <div className="text-gray-400 mb-4">No methods found matching your search.</div>
            <button
              onClick={() => {
                setSelectedCategory('all');
                setSearchQuery('');
              }}
              className="btn-secondary"
            >
              Clear Filters
            </button>
          </div>
        )}

        {/* Additional Info */}
        <div className="mt-12 glass-card p-8">
          <h3 className="text-xl font-bold text-white mb-4">Standard Substrate RPC Methods</h3>
          <p className="text-gray-400 mb-6">
            In addition to Atlas Kernel-specific methods, X3 STAR also exposes standard Substrate RPC methods:
          </p>
          <div className="grid md:grid-cols-3 gap-4">
            {[
              { prefix: 'system_', desc: 'System information and health' },
              { prefix: 'chain_', desc: 'Chain state and blocks' },
              { prefix: 'state_', desc: 'Runtime state queries' },
              { prefix: 'author_', desc: 'Transaction submission' },
              { prefix: 'rpc_', desc: 'RPC method discovery' },
              { prefix: 'grandpa_', desc: 'GRANDPA finality' },
            ].map((item) => (
              <div key={item.prefix} className="p-4 rounded-xl bg-[#0a0a0a]">
                <code className="text-orange-400 font-mono">{item.prefix}*</code>
                <p className="text-sm text-gray-500 mt-1">{item.desc}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
