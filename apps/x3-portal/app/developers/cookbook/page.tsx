'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  Copy,
  Check,
  ChevronRight,
  Search,
  Code,
  Terminal,
  Layers,
  Zap,
  Database,
  Wallet,
  ArrowRight,
} from 'lucide-react';

interface CodeSnippet {
  id: string;
  title: string;
  description: string;
  category: string;
  language: string;
  code: string;
  tags: string[];
}

const snippets: CodeSnippet[] = [
  {
    id: '1',
    title: 'Connect to X3 STAR Network',
    description: 'Initialize a connection to the X3 Atlas Sphere RPC endpoint.',
    category: 'Getting Started',
    language: 'typescript',
    code: `import { ApiPromise, WsProvider } from '@polkadot/api';

async function connectToX3() {
  // Connect to X3 STAR testnet
  const provider = new WsProvider('wss://rpc.testnet.atlas-sphere.io');
  const api = await ApiPromise.create({ provider });
  
  // Get chain info
  const chain = await api.rpc.system.chain();
  const nodeName = await api.rpc.system.name();
  
  console.log(\`Connected to \${chain} via \${nodeName}\`);
  return api;
}`,
    tags: ['connection', 'polkadot.js', 'setup'],
  },
  {
    id: '2',
    title: 'Query Canonical Ledger Balance',
    description: 'Fetch the balance of an account from the unified canonical ledger.',
    category: 'Ledger',
    language: 'typescript',
    code: `async function getCanonicalBalance(
  api: ApiPromise,
  account: string,
  assetId: number = 0
) {
  const response = await api.rpc.atlasKernel.getCanonicalBalance(
    account,
    assetId
  );
  
  console.log(\`Balance: \${response.toString()}\`);
  return response;
}

// Usage
const balance = await getCanonicalBalance(
  api,
  '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY'
);`,
    tags: ['balance', 'ledger', 'query'],
  },
  {
    id: '3',
    title: 'Check Account Authorization',
    description: 'Verify if an account is authorized to submit Comit transactions.',
    category: 'Authorization',
    language: 'typescript',
    code: `async function checkAuthorization(api: ApiPromise, account: string) {
  const isAuthorized = await api.rpc.atlasKernel.isAuthorized(account);
  
  if (isAuthorized.isTrue) {
    console.log(\`Account \${account} is authorized\`);
  } else {
    console.log(\`Account \${account} is NOT authorized\`);
    console.log('Request authorization from governance');
  }
  
  return isAuthorized.isTrue;
}`,
    tags: ['authorization', 'security', 'comit'],
  },
  {
    id: '4',
    title: 'Submit a Basic Comit Transaction',
    description: 'Create and submit an atomic cross-VM Comit transaction.',
    category: 'Cross-VM',
    language: 'typescript',
    code: `import { Keyring } from '@polkadot/api';

async function submitComit(
  api: ApiPromise,
  evmPayload: Uint8Array,
  svmPayload: Uint8Array
) {
  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');
  
  // Create Comit ID (32 bytes)
  const comitId = api.createType('H256', randomBytes(32));
  
  // Get current nonce for account
  const nonce = await api.query.atlasKernel.nonces(alice.address);
  
  // Calculate prepare root
  const prepareRoot = calculatePrepareRoot(evmPayload, svmPayload);
  
  // Submit Comit
  const tx = api.tx.atlasKernel.submitComit(
    comitId,
    evmPayload,
    svmPayload,
    nonce,
    1000000000000n, // fee
    prepareRoot
  );
  
  const hash = await tx.signAndSend(alice);
  console.log(\`Comit submitted: \${hash.toHex()}\`);
  
  return hash;
}`,
    tags: ['comit', 'cross-vm', 'atomic', 'transaction'],
  },
  {
    id: '5',
    title: 'Deploy EVM Smart Contract',
    description: 'Deploy a Solidity contract to the X3 EVM execution environment.',
    category: 'EVM',
    language: 'typescript',
    code: `import { ethers } from 'ethers';

async function deployEVMContract(
  abi: any[],
  bytecode: string,
  constructorArgs: any[] = []
) {
  // Connect to X3 EVM endpoint
  const provider = new ethers.JsonRpcProvider(
    'https://evm.testnet.atlas-sphere.io'
  );
  
  // Create wallet
  const wallet = new ethers.Wallet(process.env.PRIVATE_KEY!, provider);
  
  // Create contract factory
  const factory = new ethers.ContractFactory(abi, bytecode, wallet);
  
  // Deploy
  console.log('Deploying contract...');
  const contract = await factory.deploy(...constructorArgs);
  await contract.waitForDeployment();
  
  const address = await contract.getAddress();
  console.log(\`Contract deployed at: \${address}\`);
  
  return contract;
}`,
    tags: ['evm', 'deploy', 'solidity', 'ethers'],
  },
  {
    id: '6',
    title: 'Listen for Comit Events',
    description: 'Subscribe to Comit lifecycle events from the Atlas Kernel.',
    category: 'Events',
    language: 'typescript',
    code: `async function subscribeToComitEvents(api: ApiPromise) {
  // Subscribe to all system events
  const unsubscribe = await api.query.system.events((events) => {
    events.forEach((record) => {
      const { event } = record;
      
      if (event.section === 'atlasKernel') {
        switch (event.method) {
          case 'ComitSubmitted':
            console.log('Comit submitted:', event.data.toJSON());
            break;
          case 'ComitExecutionStarted':
            console.log('Execution started:', event.data.toJSON());
            break;
          case 'ComitExecutionCompleted':
            console.log('Execution completed:', event.data.toJSON());
            break;
          case 'ComitFinalized':
            console.log('Comit finalized:', event.data.toJSON());
            break;
          case 'ComitFailed':
            console.error('Comit failed:', event.data.toJSON());
            break;
        }
      }
    });
  });
  
  return unsubscribe;
}`,
    tags: ['events', 'subscription', 'comit', 'realtime'],
  },
  {
    id: '7',
    title: 'Get Asset Metadata',
    description: 'Retrieve metadata for registered assets including symbol and decimals.',
    category: 'Assets',
    language: 'typescript',
    code: `async function getAssetMetadata(api: ApiPromise, assetId: number) {
  const metadata = await api.rpc.atlasKernel.getAssetMetadata(assetId);
  
  if (metadata.isSome) {
    const { symbol, decimals } = metadata.unwrap();
    console.log(\`Asset \${assetId}:\`);
    console.log(\`  Symbol: \${symbol.toString()}\`);
    console.log(\`  Decimals: \${decimals.toNumber()}\`);
    return { symbol: symbol.toString(), decimals: decimals.toNumber() };
  } else {
    console.log(\`Asset \${assetId} not found\`);
    return null;
  }
}

// List common assets
const assets = [
  { id: 0, name: 'ATLAS (Native)' },
  { id: 1, name: 'Wrapped ETH' },
  { id: 2, name: 'Wrapped SOL' },
  { id: 3, name: 'USDC' },
];`,
    tags: ['assets', 'metadata', 'tokens'],
  },
  {
    id: '8',
    title: 'Cross-VM Token Transfer',
    description: 'Transfer tokens between EVM and SVM representations atomically.',
    category: 'Cross-VM',
    language: 'typescript',
    code: `async function crossVMTransfer(
  api: ApiPromise,
  fromVM: 'evm' | 'svm',
  toVM: 'evm' | 'svm',
  assetId: number,
  amount: bigint,
  recipient: string
) {
  // This creates a Comit that:
  // 1. Burns tokens on source VM
  // 2. Mints tokens on destination VM
  // Both happen atomically
  
  const evmPayload = fromVM === 'evm' 
    ? encodeBurnPayload(assetId, amount)
    : encodeMintPayload(assetId, amount, recipient);
    
  const svmPayload = fromVM === 'svm'
    ? encodeSVMBurnPayload(assetId, amount)
    : encodeSVMMintPayload(assetId, amount, recipient);
  
  // Submit as Comit for atomic execution
  const hash = await submitComit(api, evmPayload, svmPayload);
  
  console.log(\`Cross-VM transfer initiated: \${hash}\`);
  return hash;
}`,
    tags: ['cross-vm', 'transfer', 'atomic', 'tokens'],
  },
];

const categories = [
  { id: 'all', label: 'All', icon: <Code className="w-4 h-4" /> },
  { id: 'Getting Started', label: 'Getting Started', icon: <Zap className="w-4 h-4" /> },
  { id: 'Ledger', label: 'Ledger', icon: <Database className="w-4 h-4" /> },
  { id: 'Authorization', label: 'Auth', icon: <Wallet className="w-4 h-4" /> },
  { id: 'Cross-VM', label: 'Cross-VM', icon: <Layers className="w-4 h-4" /> },
  { id: 'EVM', label: 'EVM', icon: <Terminal className="w-4 h-4" /> },
  { id: 'Events', label: 'Events', icon: <Zap className="w-4 h-4" /> },
  { id: 'Assets', label: 'Assets', icon: <Database className="w-4 h-4" /> },
];

export default function CookbookPage() {
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const filteredSnippets = snippets.filter((snippet) => {
    const categoryMatch = selectedCategory === 'all' || snippet.category === selectedCategory;
    const searchMatch = snippet.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
                       snippet.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
                       snippet.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    return categoryMatch && searchMatch;
  });

  const copyToClipboard = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-16 relative overflow-hidden border-b border-[#1a1a1a]">
        <div className="absolute inset-0 mesh-gradient opacity-20" />
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="badge badge-success mb-4">Cookbook</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              X3 STAR Cookbook
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Ready-to-use code snippets and examples for building on X3 Atlas Sphere. 
              Copy, paste, and customize for your application.
            </p>

            {/* Search */}
            <div className="relative max-w-xl">
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-500" />
              <input
                type="text"
                placeholder="Search snippets..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="input-search pl-12"
              />
            </div>
          </div>
        </div>
      </section>

      <div className="container-wide py-12">
        {/* Category Filter */}
        <div className="flex flex-wrap gap-2 mb-8">
          {categories.map((category) => (
            <button
              key={category.id}
              onClick={() => setSelectedCategory(category.id)}
              className={`flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                selectedCategory === category.id
                  ? 'bg-emerald-500 text-white'
                : 'bg-[#0a0a0a] text-gray-400 hover:text-white hover:bg-[#1a1a1a]'
              }`}
            >
              <span className="mr-2">{category.icon}</span>
              {category.label}
            </button>
          ))}
        </div>

        {/* Snippets */}
        <div className="space-y-6">
          {filteredSnippets.map((snippet) => (
            <div key={snippet.id} className="glass-card overflow-hidden">
              <div className="p-6 border-b border-[#1a1a1a]">
                <div className="flex items-start justify-between mb-2">
                  <h3 className="text-lg font-semibold text-white">{snippet.title}</h3>
                  <span className="badge badge-info">{snippet.category}</span>
                </div>
                <p className="text-gray-400 mb-3">{snippet.description}</p>
                <div className="flex flex-wrap gap-2">
                  {snippet.tags.map((tag) => (
                    <span key={tag} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-500">
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
              
              <div className="relative">
                <div className="absolute top-4 right-4 flex items-center space-x-2 z-10">
                  <span className="text-xs text-gray-600 uppercase">{snippet.language}</span>
                  <button
                    onClick={() => copyToClipboard(snippet.code, snippet.id)}
                    className="flex items-center px-3 py-1 rounded-lg bg-[#111111] hover:bg-[#1a1a1a] text-sm text-gray-400 transition-colors"
                  >
                    {copiedId === snippet.id ? (
                      <>
                        <Check className="w-4 h-4 mr-1 text-emerald-500" />
                        Copied!
                      </>
                    ) : (
                      <>
                        <Copy className="w-4 h-4 mr-1" />
                        Copy
                      </>
                    )}
                  </button>
                </div>
                <div className="code-block rounded-none border-0">
                  <pre className="text-sm text-gray-400 overflow-x-auto">{snippet.code}</pre>
                </div>
              </div>
            </div>
          ))}
        </div>

        {filteredSnippets.length === 0 && (
          <div className="text-center py-16">
            <div className="text-gray-400 mb-4">No snippets found matching your search.</div>
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

        {/* Contribute CTA */}
        <div className="mt-12 p-8 rounded-2xl bg-gradient-to-r from-emerald-500/10 to-cyan-500/10 border border-emerald-500/20">
          <h3 className="text-xl font-bold text-white mb-2">Contribute to the Cookbook</h3>
          <p className="text-gray-400 mb-4">
            Have a useful code snippet? Share it with the community by contributing to our GitHub repository.
          </p>
          <Link
            href="https://github.com/atlas-sphere/atlas-sphere/tree/main/docs/cookbook"
            className="btn-secondary inline-flex items-center"
          >
            Contribute on GitHub
            <ArrowRight className="ml-2 w-4 h-4" />
          </Link>
        </div>
      </div>
    </div>
  );
}
