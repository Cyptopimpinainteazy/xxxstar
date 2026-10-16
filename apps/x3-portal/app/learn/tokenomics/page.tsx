'use client';

import React from 'react';
import Link from 'next/link';
import {
  Coins,
  ArrowRight,
  ArrowLeftRight,
  Database,
  Layers,
  Zap,
  Shield,
  Globe,
  CheckCircle,
  Lock,
  GitMerge,
  RefreshCw,
  Wallet,
  TrendingUp,
} from 'lucide-react';

const tokenDetails = {
  name: 'X3Coin',
  symbol: 'X3',
  decimals: 18,
  assetId: 0,
  uses: [
    'Gas fees for EVM transactions',
    'Compute unit fees for SVM transactions',
    'Comit execution fees (dual-VM atomic transactions)',
    'Validator staking and rewards',
    'Governance voting power',
  ],
};

const assetTable = [
  { id: 0, symbol: 'X3', name: 'X3Coin (Native)', decimals: 18, vm: 'native', origin: 'Native' },
  { id: 1, symbol: 'USDC', name: 'USD Coin', decimals: 6, vm: 'evm', origin: 'Bridged from Ethereum' },
  { id: 2, symbol: 'USDT', name: 'Tether USD', decimals: 6, vm: 'evm', origin: 'Bridged from Ethereum' },
  { id: 3, symbol: 'WETH', name: 'Wrapped Ether', decimals: 18, vm: 'evm', origin: 'Bridged from Ethereum' },
  { id: 100, symbol: 'SOL', name: 'Solana', decimals: 9, vm: 'svm', origin: 'Bridged from Solana' },
  { id: 101, symbol: 'sUSDC', name: 'Solana USDC', decimals: 6, vm: 'svm', origin: 'Bridged from Solana' },
];

const feeStructure = [
  { operation: 'Simple EVM Transfer', gas: '21,000', x3Cost: '~0.00021 X3' },
  { operation: 'EVM Contract Call', gas: '50,000-150,000', x3Cost: '~0.0005-0.0015 X3' },
  { operation: 'SVM Transfer', gas: '5,000 CU', x3Cost: '~0.00005 X3' },
  { operation: 'SVM Program Call', gas: '200,000 CU', x3Cost: '~0.002 X3' },
  { operation: 'Cross-VM Comit', gas: '150,000 + 250,000 CU', x3Cost: '~0.004 X3' },
  { operation: 'Token Mint (Dual VM)', gas: '500,000 combined', x3Cost: '~0.005 X3' },
];

export default function TokenomicsPage() {
  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <Link href="/learn" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Learn
            </Link>
            <div className="badge badge-warning mt-4 mb-4">Tokenomics</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              How <span className="gradient-text">X3Coin</span> Powers the Ecosystem
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              X3 is the native token that fuels all operations on Atlas Sphere - from gas fees 
              to cross-VM Comit execution. Learn how it enables seamless interoperability.
            </p>
          </div>
        </div>
      </section>

      {/* X3 Token Card */}
      <section className="py-16">
        <div className="container-wide">
          <div className="glass-card p-8 relative overflow-hidden">
            <div className="absolute top-0 right-0 w-64 h-64 bg-gradient-to-br from-orange-500/20 to-yellow-500/20 rounded-full blur-3xl" />
            
            <div className="relative z-10 grid lg:grid-cols-2 gap-8">
              <div>
                <div className="flex items-center gap-4 mb-6">
                  <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-orange-500 to-yellow-600 flex items-center justify-center">
                    <Coins className="w-8 h-8 text-white" />
                  </div>
                  <div>
                    <h2 className="text-3xl font-bold text-white">{tokenDetails.name}</h2>
                    <span className="text-xl text-orange-400 font-mono">${tokenDetails.symbol}</span>
                  </div>
                </div>
                
                <div className="grid grid-cols-2 gap-4 mb-6">
                  <div className="p-4 rounded-xl bg-[#0a0a0a]">
                    <div className="text-sm text-gray-500 mb-1">Asset ID</div>
                    <div className="text-xl font-mono text-white">{tokenDetails.assetId}</div>
                  </div>
                  <div className="p-4 rounded-xl bg-[#0a0a0a]">
                    <div className="text-sm text-gray-500 mb-1">Decimals</div>
                    <div className="text-xl font-mono text-white">{tokenDetails.decimals}</div>
                  </div>
                </div>
                
                <p className="text-gray-400">
                  X3 is accessible from <strong className="text-white">both EVM and SVM</strong> simultaneously. 
                  It exists once in the Canonical Ledger - no wrapped versions needed.
                </p>
              </div>
              
              <div>
                <h3 className="text-lg font-semibold text-white mb-4">X3 is used for:</h3>
                <ul className="space-y-3">
                  {tokenDetails.uses.map((use, i) => (
                    <li key={i} className="flex items-start gap-3">
                      <CheckCircle className="w-5 h-5 text-green-500 flex-shrink-0 mt-0.5" />
                      <span className="text-gray-300">{use}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Canonical Ledger Architecture */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">The Canonical Ledger Architecture</h2>
          
          <div className="glass-card p-8">
            <div className="mb-8">
              <p className="text-gray-400 mb-4">
                The key innovation in Atlas Sphere is the <strong className="text-orange-400">Canonical Ledger</strong> - 
                a single source of truth for all balances that <strong className="text-white">both VMs can read from and write to</strong>:
              </p>
            </div>
            
            {/* Architecture Diagram */}
            <div className="bg-[#0a0a0a] rounded-xl p-6 mb-8 font-mono text-sm overflow-x-auto">
              <pre className="text-gray-300">
{`┌─────────────────────────────────────────────────────────────────┐
│                    CANONICAL LEDGER                             │
│    CanonicalLedger: Map<(AccountId, AssetId), Balance>         │
│                                                                 │
│    Account A, X3      →  1,000,000                             │
│    Account A, USDC    →  50,000                                │
│    Account B, wSOL    →  100                                   │
│    Account B, X3      →  25,000                                │
└─────────────────────────────────────────────────────────────────┘
         ↑                              ↑
         │                              │
    ┌────┴────┐                    ┌────┴────┐
    │   EVM   │                    │   SVM   │
    │ Adapter │                    │ Adapter │
    └─────────┘                    └─────────┘`}
              </pre>
            </div>
            
            <div className="grid md:grid-cols-3 gap-6">
              <div className="p-4 rounded-xl bg-[#0a0a0a]">
                <Database className="w-6 h-6 text-purple-400 mb-3" />
                <h4 className="font-semibold text-white mb-2">Single Source of Truth</h4>
                <p className="text-sm text-gray-500">All balances stored once, accessible from both VMs</p>
              </div>
              <div className="p-4 rounded-xl bg-[#0a0a0a]">
                <RefreshCw className="w-6 h-6 text-blue-400 mb-3" />
                <h4 className="font-semibold text-white mb-2">No Wrapped Tokens</h4>
                <p className="text-sm text-gray-500">Assets exist once - no fragmented liquidity</p>
              </div>
              <div className="p-4 rounded-xl bg-[#0a0a0a]">
                <Zap className="w-6 h-6 text-yellow-400 mb-3" />
                <h4 className="font-semibold text-white mb-2">Atomic Updates</h4>
                <p className="text-sm text-gray-500">Cross-VM operations update ledger atomically</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* How External Tokens Enter */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">How External Tokens Enter the System</h2>
          
          <div className="grid lg:grid-cols-2 gap-8">
            {/* EVM Bridge */}
            <div className="glass-card p-6">
              <div className="flex items-center gap-3 mb-4">
                <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
                  <span className="text-blue-400 font-bold">ETH</span>
                </div>
                <ArrowRight className="w-5 h-5 text-gray-600" />
                <div className="w-10 h-10 rounded-lg bg-orange-500/20 flex items-center justify-center">
                  <span className="text-orange-400 font-bold">X3</span>
                </div>
              </div>
              <h3 className="text-lg font-semibold text-white mb-3">EVM Tokens (ERC-20)</h3>
              <div className="bg-[#0a0a0a] rounded-lg p-4 font-mono text-xs mb-4 overflow-x-auto">
                <pre className="text-gray-300">
{`// On Ethereum mainnet
function bridgeToX3(uint256 amount, bytes32 x3Recipient) {
    // 1. Lock tokens on Ethereum
    token.transferFrom(msg.sender, lockContract, amount);
    
    // 2. Emit bridge event (relayer picks up)
    emit BridgeToX3(address(token), amount, x3Recipient);
}

// On X3 Atlas - registered in Canonical Ledger
AssetRegistry: wETH → AssetId(3), decimals: 18`}
                </pre>
              </div>
              <p className="text-sm text-gray-500">
                ERC-20 tokens are locked on Ethereum and minted as registered assets on X3.
              </p>
            </div>
            
            {/* SVM Bridge */}
            <div className="glass-card p-6">
              <div className="flex items-center gap-3 mb-4">
                <div className="w-10 h-10 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                  <span className="text-emerald-400 font-bold">SOL</span>
                </div>
                <ArrowRight className="w-5 h-5 text-gray-600" />
                <div className="w-10 h-10 rounded-lg bg-orange-500/20 flex items-center justify-center">
                  <span className="text-orange-400 font-bold">X3</span>
                </div>
              </div>
              <h3 className="text-lg font-semibold text-white mb-3">SVM Tokens (SPL)</h3>
              <div className="bg-[#0a0a0a] rounded-lg p-4 font-mono text-xs mb-4 overflow-x-auto">
                <pre className="text-gray-300">
{`// On Solana mainnet
pub fn bridge_to_x3(
    ctx: Context<Bridge>, 
    amount: u64, 
    x3_recipient: [u8; 32]
) -> Result<()> {
    // 1. Lock SPL tokens
    token::transfer(ctx.accounts.transfer_ctx(), amount)?;
    
    // 2. Emit bridge event
    emit!(BridgeToX3Event { amount, recipient });
    Ok(())
}

// On X3 Atlas - registered in Canonical Ledger
AssetRegistry: wSOL → AssetId(100), decimals: 9`}
                </pre>
              </div>
              <p className="text-sm text-gray-500">
                SPL tokens are locked on Solana and minted as registered assets on X3.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Asset Registry Table */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Registered Assets</h2>
          
          <div className="glass-card overflow-hidden">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-[#1a1a1a]">
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Asset ID</th>
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Symbol</th>
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Name</th>
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Decimals</th>
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Native VM</th>
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Origin</th>
                  </tr>
                </thead>
                <tbody>
                  {assetTable.map((asset) => (
                    <tr key={asset.id} className="border-b border-[#1a1a1a] hover:bg-[#0a0a0a]">
                      <td className="py-4 px-6 font-mono text-white">{asset.id}</td>
                      <td className="py-4 px-6">
                        <span className={`px-2 py-1 rounded text-xs font-semibold ${
                          asset.vm === 'native' ? 'bg-orange-500/20 text-orange-400' :
                          asset.vm === 'evm' ? 'bg-blue-500/20 text-blue-400' :
                          'bg-emerald-500/20 text-emerald-400'
                        }`}>
                          {asset.symbol}
                        </span>
                      </td>
                      <td className="py-4 px-6 text-gray-300">{asset.name}</td>
                      <td className="py-4 px-6 text-gray-400 font-mono">{asset.decimals}</td>
                      <td className="py-4 px-6">
                        <span className={`text-xs ${
                          asset.vm === 'native' ? 'text-orange-400' :
                          asset.vm === 'evm' ? 'text-blue-400' : 'text-emerald-400'
                        }`}>
                          {asset.vm.toUpperCase()}
                        </span>
                      </td>
                      <td className="py-4 px-6 text-sm text-gray-500">{asset.origin}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </section>

      {/* Cross-VM Token Flow */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Cross-VM Token Flow (Comit)</h2>
          
          <div className="glass-card p-8">
            <div className="mb-6">
              <p className="text-gray-400">
                The magic happens with <strong className="text-orange-400">Comit transactions</strong> - 
                atomic operations that span both VMs. Here&apos;s an example of a cross-VM swap:
              </p>
            </div>
            
            <div className="bg-[#0a0a0a] rounded-xl p-6 mb-8 font-mono text-sm overflow-x-auto">
              <div className="text-gray-500 mb-4">{`// User wants to: Swap 100 wETH (EVM) → 2000 USDC (SVM)`}</div>
              <pre className="text-gray-300">
{`1. Build Dual Comit:
   ┌────────────────────────────────────────┐
   │ Comit Transaction                      │
   │ ├─ EVM Payload: swap(wETH, 100)       │
   │ ├─ SVM Payload: receive(USDC, 2000)   │
   │ └─ prepare_root: hash(inputs)         │
   └────────────────────────────────────────┘

2. Atlas Kernel Executes:
   ├─ EVM Adapter: Execute swap contract ✓
   ├─ SVM Adapter: Execute receive CPI ✓
   └─ BOTH succeed? → Update Canonical Ledger

3. Result (ATOMIC):
   CanonicalLedger[user, wETH]  -= 100
   CanonicalLedger[user, USDC]  += 2000`}
              </pre>
            </div>
            
            <div className="flex items-center justify-center gap-4 p-4 rounded-xl bg-green-500/10 border border-green-500/20">
              <CheckCircle className="w-6 h-6 text-green-500" />
              <span className="text-green-400">
                Both operations succeed or both fail - no partial state, no rug risk
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* Fee Structure */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Fee Structure (Paid in X3)</h2>
          
          <div className="glass-card overflow-hidden">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-[#1a1a1a]">
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Operation</th>
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Gas / Compute Units</th>
                    <th className="text-left py-4 px-6 text-sm font-semibold text-gray-400">Estimated X3 Cost</th>
                  </tr>
                </thead>
                <tbody>
                  {feeStructure.map((fee, i) => (
                    <tr key={i} className="border-b border-[#1a1a1a] hover:bg-[#0a0a0a]">
                      <td className="py-4 px-6 text-gray-300">{fee.operation}</td>
                      <td className="py-4 px-6 text-gray-400 font-mono text-sm">{fee.gas}</td>
                      <td className="py-4 px-6 text-orange-400 font-mono">{fee.x3Cost}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="p-4 border-t border-[#1a1a1a] bg-[#0a0a0a]">
              <p className="text-xs text-gray-500">
                * Estimates based on current gas prices. Actual costs may vary based on network congestion.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Full System Diagram */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Complete System Architecture</h2>
          
          <div className="glass-card p-8">
            <div className="bg-[#0a0a0a] rounded-xl p-6 font-mono text-xs overflow-x-auto">
              <pre className="text-gray-300">
{`┌─────────────────────────────────────────────────────────────────────────┐
│                        ATLAS SPHERE (X3 Chain)                         │
│                                                                         │
│  ┌─────────────┐     ┌─────────────────────┐     ┌─────────────┐       │
│  │  Ethereum   │────▶│   CANONICAL LEDGER  │◀────│   Solana    │       │
│  │   Bridge    │     │                     │     │   Bridge    │       │
│  └─────────────┘     │  X3 (native gas)    │     └─────────────┘       │
│        │             │  wETH (from ETH)    │            │              │
│        │             │  USDC (from ETH)    │            │              │
│        │             │  wSOL (from SOL)    │            │              │
│        │             │  sUSDC (from SOL)   │            │              │
│        │             └─────────────────────┘            │              │
│        │                   ↑       ↑                    │              │
│        │             ┌─────┴───┐ ┌─┴─────┐              │              │
│        └────────────▶│   EVM   │ │  SVM  │◀─────────────┘              │
│                      │ Adapter │ │Adapter│                             │
│                      └─────────┘ └───────┘                             │
│                            ↑       ↑                                   │
│                            └───┬───┘                                   │
│                         ┌──────┴──────┐                                │
│                         │ ATLAS KERNEL│                                │
│                         │   (Comits)  │                                │
│                         └─────────────┘                                │
└─────────────────────────────────────────────────────────────────────────┘`}
              </pre>
            </div>
            
            <div className="mt-8 grid md:grid-cols-2 lg:grid-cols-4 gap-4">
              <div className="p-4 rounded-xl bg-orange-500/10 border border-orange-500/20">
                <Coins className="w-5 h-5 text-orange-400 mb-2" />
                <h4 className="font-semibold text-white text-sm mb-1">X3 = Native Gas</h4>
                <p className="text-xs text-gray-500">Used for ALL transaction fees</p>
              </div>
              <div className="p-4 rounded-xl bg-purple-500/10 border border-purple-500/20">
                <Database className="w-5 h-5 text-purple-400 mb-2" />
                <h4 className="font-semibold text-white text-sm mb-1">No Wrapped Tokens</h4>
                <p className="text-xs text-gray-500">Assets exist once in ledger</p>
              </div>
              <div className="p-4 rounded-xl bg-blue-500/10 border border-blue-500/20">
                <GitMerge className="w-5 h-5 text-blue-400 mb-2" />
                <h4 className="font-semibold text-white text-sm mb-1">Shared State</h4>
                <p className="text-xs text-gray-500">Both VMs read/write same balances</p>
              </div>
              <div className="p-4 rounded-xl bg-green-500/10 border border-green-500/20">
                <Zap className="w-5 h-5 text-green-400 mb-2" />
                <h4 className="font-semibold text-white text-sm mb-1">Atomic Comits</h4>
                <p className="text-xs text-gray-500">Cross-VM swaps in one tx</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Key Points Summary */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Key Takeaways</h2>
          
          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[
              {
                icon: <Coins className="w-6 h-6 text-orange-400" />,
                title: 'X3 Powers Everything',
                desc: 'Native gas token for all EVM, SVM, and Comit operations',
              },
              {
                icon: <Database className="w-6 h-6 text-purple-400" />,
                title: 'Unified Liquidity',
                desc: 'No wrapped tokens - assets exist once, accessible from both VMs',
              },
              {
                icon: <ArrowLeftRight className="w-6 h-6 text-blue-400" />,
                title: 'Seamless Bridging',
                desc: 'Lock on source chain, mint on X3 - relayers handle the rest',
              },
              {
                icon: <Zap className="w-6 h-6 text-yellow-400" />,
                title: 'Atomic Cross-VM Swaps',
                desc: 'Trade EVM tokens for SVM tokens in a single transaction',
              },
              {
                icon: <Shield className="w-6 h-6 text-green-400" />,
                title: 'No Partial State',
                desc: 'Comits are all-or-nothing - both VMs succeed or both revert',
              },
              {
                icon: <Globe className="w-6 h-6 text-cyan-400" />,
                title: 'True Interoperability',
                desc: 'Build dApps that leverage best of Ethereum AND Solana ecosystems',
              },
            ].map((item, i) => (
              <div key={i} className="glass-card p-6">
                {item.icon}
                <h3 className="font-semibold text-white mt-4 mb-2">{item.title}</h3>
                <p className="text-sm text-gray-500">{item.desc}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Get X3 Section */}
      <section className="py-16">
        <div className="container-wide">
          <div className="glass-card p-8">
            <div className="grid md:grid-cols-2 gap-8 items-center">
              <div>
                <h2 className="text-2xl font-bold text-white mb-4">Get X3 Tokens</h2>
                <p className="text-gray-400 mb-6">
                  Ready to start building or transacting on Atlas Sphere? Get testnet X3 tokens 
                  from our faucet to experiment with the dual-VM architecture.
                </p>
                <div className="flex flex-wrap gap-4">
                  <Link 
                    href="https://faucet.testnet.atlas-sphere.io" 
                    className="btn-primary px-6 py-3 inline-flex items-center"
                  >
                    <Wallet className="w-4 h-4 mr-2" />
                    Testnet Faucet
                  </Link>
                  <Link 
                    href="/developers/docs/quickstart" 
                    className="btn-secondary px-6 py-3 inline-flex items-center"
                  >
                    Quick Start Guide
                    <ArrowRight className="w-4 h-4 ml-2" />
                  </Link>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="p-4 rounded-xl bg-[#0a0a0a] text-center">
                  <TrendingUp className="w-6 h-6 text-orange-400 mx-auto mb-2" />
                  <div className="text-2xl font-bold text-white">6s</div>
                  <div className="text-xs text-gray-500">Block Time</div>
                </div>
                <div className="p-4 rounded-xl bg-[#0a0a0a] text-center">
                  <Zap className="w-6 h-6 text-yellow-400 mx-auto mb-2" />
                  <div className="text-2xl font-bold text-white">~0.001</div>
                  <div className="text-xs text-gray-500">Avg Fee (X3)</div>
                </div>
                <div className="p-4 rounded-xl bg-[#0a0a0a] text-center">
                  <Layers className="w-6 h-6 text-blue-400 mx-auto mb-2" />
                  <div className="text-2xl font-bold text-white">2</div>
                  <div className="text-xs text-gray-500">Virtual Machines</div>
                </div>
                <div className="p-4 rounded-xl bg-[#0a0a0a] text-center">
                  <Shield className="w-6 h-6 text-green-400 mx-auto mb-2" />
                  <div className="text-2xl font-bold text-white">100%</div>
                  <div className="text-xs text-gray-500">Atomic Execution</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
