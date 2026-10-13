'use client';

import { useState } from 'react';
import Link from 'next/link';

interface X3Script {
  id: string;
  name: string;
  description: string;
  category: 'arbitrage' | 'liquidation' | 'mev' | 'liquidity' | 'utility';
  author: string;
  executions: number;
  success_rate: number;
  avg_profit: string;
  code: string;
}

const SAMPLE_SCRIPTS: X3Script[] = [
  {
    id: '1',
    name: 'arb.x3',
    description: 'Cross-DEX arbitrage between Uniswap V3 and SushiSwap',
    category: 'arbitrage',
    author: '5Grwva...utQY',
    executions: 12456,
    success_rate: 94.2,
    avg_profit: '0.05 ETH',
    code: `// arb.x3 - Cross-DEX Arbitrage
@swarm @atomic
fn execute(amount: u256, min_profit: u256) -> u256 {
    let borrowed = flash_borrow(AAVE, WETH, amount);
    let uni_price = get_price(UNISWAP, WETH, USDC);
    let sushi_price = get_price(SUSHI, WETH, USDC);
    
    if uni_price > sushi_price {
        // Buy Sushi, sell Uni
        let usdc = swap(SUSHI, WETH, USDC, amount);
        let weth = swap(UNI, USDC, WETH, usdc);
        profit = weth - amount;
    }
    
    flash_repay(AAVE, WETH, amount + fee);
    return profit;
}`,
  },
  {
    id: '2',
    name: 'flash.x3',
    description: 'Flash loan liquidation with multi-protocol support',
    category: 'liquidation',
    author: '5FHne...94ty',
    executions: 3421,
    success_rate: 87.5,
    avg_profit: '0.12 ETH',
    code: `// flash.x3 - Flash Liquidation
@swarm @atomic
fn liquidate(borrower: address, debt: u256) -> u256 {
    let borrowed = flash_borrow(AAVE, debt_asset, debt);
    
    call(AAVE, "liquidationCall", 
        (collateral, debt_asset, borrower, debt));
    
    let seized = balance_of(collateral, self);
    let swapped = swap_to(collateral, debt_asset, seized);
    
    flash_repay(AAVE, debt_asset, debt + fee);
    return swapped - debt - fee;
}`,
  },
  {
    id: '3',
    name: 'mev_smooth.x3',
    description: 'MEV smoothing and fair distribution to validators',
    category: 'mev',
    author: '5DAAnr...TXFy',
    executions: 8934,
    success_rate: 99.1,
    avg_profit: 'N/A',
    code: `// mev_smooth.x3 - MEV Distribution
@swarm
fn distribute_epoch(epoch: u256) -> u256 {
    let total_mev = epochs[epoch].total_mev;
    let validator_pool = total_mev * 9000 / 10000;
    
    for (validator, contrib) in contributions {
        let avg = validator_pool / block_count;
        let individual = contrib * validator_pool / total;
        let smoothed = (individual * 70 + avg * 30) / 100;
        balances[validator] += smoothed;
    }
    
    return total_distributed;
}`,
  },
  {
    id: '4',
    name: 'jit_lp.x3',
    description: 'Just-in-time liquidity provision for large swaps',
    category: 'liquidity',
    author: '5HGjWA...UMaw',
    executions: 2156,
    success_rate: 91.3,
    avg_profit: '0.02 ETH',
    code: `// jit_lp.x3 - JIT Liquidity
@swarm @atomic @mempool
fn provide_jit(pool: address, pending: SwapData) -> u256 {
    let (price, tick) = call(pool, "slot0");
    
    // Concentrate liquidity around pending swap
    let tick_lower = tick - TICK_RANGE;
    let tick_upper = tick + TICK_RANGE;
    
    let token_id = call(NFT_MANAGER, "mint", 
        (token0, token1, fee, tick_lower, tick_upper));
    
    // Wait for swap, then remove
    return token_id;
}`,
  },
];

export default function ScriptsPage() {
  const [selectedScript, setSelectedScript] = useState<X3Script | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [categoryFilter, setCategoryFilter] = useState<string>('all');

  const filteredScripts = SAMPLE_SCRIPTS.filter((script) => {
    const matchesSearch = script.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      script.description.toLowerCase().includes(searchTerm.toLowerCase());
    const matchesCategory = categoryFilter === 'all' || script.category === categoryFilter;
    return matchesSearch && matchesCategory;
  });

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-indigo-900 to-slate-900">
      {/* Header */}
      <header className="border-b border-indigo-800/30 backdrop-blur-xl bg-black/20">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <Link href="/" className="text-2xl font-bold bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
                X3 Sphere
              </Link>
              <span className="text-indigo-400/60">/</span>
              <span className="text-white font-medium">Script Library</span>
            </div>
            <nav className="flex items-center space-x-6">
              <Link href="/x3/evolution" className="text-gray-400 hover:text-white transition">Evolution</Link>
              <Link href="/x3/swarm" className="text-gray-400 hover:text-white transition">Swarm</Link>
              <Link href="/x3/verifier" className="text-gray-400 hover:text-white transition">Verifier</Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-6 py-8">
        {/* Search and Filters */}
        <div className="flex flex-col md:flex-row gap-4 mb-8">
          <div className="flex-1">
            <input
              type="text"
              placeholder="Search scripts..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full px-4 py-3 bg-black/40 border border-indigo-800/30 rounded-xl text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500"
            />
          </div>
          <div className="flex gap-2">
            {['all', 'arbitrage', 'liquidation', 'mev', 'liquidity'].map((cat) => (
              <button
                key={cat}
                onClick={() => setCategoryFilter(cat)}
                className={`px-4 py-2 rounded-lg text-sm transition ${
                  categoryFilter === cat
                    ? 'bg-indigo-600 text-white'
                    : 'bg-black/40 text-gray-400 hover:bg-indigo-600/20'
                }`}
              >
                {cat.charAt(0).toUpperCase() + cat.slice(1)}
              </button>
            ))}
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Script List */}
          <div className="lg:col-span-1 space-y-4">
            {filteredScripts.map((script) => (
              <ScriptCard
                key={script.id}
                script={script}
                selected={selectedScript?.id === script.id}
                onClick={() => setSelectedScript(script)}
              />
            ))}
          </div>

          {/* Script Detail / Editor */}
          <div className="lg:col-span-2">
            {selectedScript ? (
              <ScriptDetail script={selectedScript} />
            ) : (
              <div className="bg-black/40 rounded-2xl border border-indigo-800/30 p-8 h-full flex items-center justify-center">
                <div className="text-center text-gray-500">
                  <p className="text-4xl mb-4">📜</p>
                  <p className="text-xl">Select a script to view details</p>
                  <p className="text-sm mt-2">Or create a new X3 script</p>
                  <button className="mt-4 px-6 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg transition">
                    Create New Script
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}

function ScriptCard({ script, selected, onClick }: {
  script: X3Script;
  selected: boolean;
  onClick: () => void;
}) {
  const categoryColors = {
    arbitrage: 'text-green-400 bg-green-400/10',
    liquidation: 'text-red-400 bg-red-400/10',
    mev: 'text-yellow-400 bg-yellow-400/10',
    liquidity: 'text-blue-400 bg-blue-400/10',
    utility: 'text-purple-400 bg-purple-400/10',
  };

  return (
    <div
      onClick={onClick}
      className={`bg-black/40 rounded-xl border p-4 cursor-pointer transition ${
        selected
          ? 'border-indigo-500 bg-indigo-900/20'
          : 'border-indigo-800/30 hover:border-indigo-600/50'
      }`}
    >
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-white font-medium">{script.name}</h3>
        <span className={`px-2 py-1 rounded text-xs ${categoryColors[script.category]}`}>
          {script.category}
        </span>
      </div>
      <p className="text-gray-400 text-sm mb-3">{script.description}</p>
      <div className="flex items-center justify-between text-xs text-gray-500">
        <span>{script.executions.toLocaleString()} runs</span>
        <span className="text-green-400">{script.success_rate}% success</span>
      </div>
    </div>
  );
}

function ScriptDetail({ script }: { script: X3Script }) {
  return (
    <div className="bg-black/40 rounded-2xl border border-indigo-800/30 overflow-hidden">
      {/* Header */}
      <div className="p-6 border-b border-indigo-800/30">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-2xl font-bold text-white">{script.name}</h2>
          <div className="flex gap-2">
            <button className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg transition">
              Deploy
            </button>
            <button className="px-4 py-2 bg-black/40 hover:bg-indigo-600/20 text-white rounded-lg transition">
              Fork
            </button>
          </div>
        </div>
        <p className="text-gray-400">{script.description}</p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 border-b border-indigo-800/30">
        <div className="p-4 text-center border-r border-indigo-800/30">
          <div className="text-2xl font-bold text-white">{script.executions.toLocaleString()}</div>
          <div className="text-xs text-gray-500">Executions</div>
        </div>
        <div className="p-4 text-center border-r border-indigo-800/30">
          <div className="text-2xl font-bold text-green-400">{script.success_rate}%</div>
          <div className="text-xs text-gray-500">Success Rate</div>
        </div>
        <div className="p-4 text-center border-r border-indigo-800/30">
          <div className="text-2xl font-bold text-cyan-400">{script.avg_profit}</div>
          <div className="text-xs text-gray-500">Avg Profit</div>
        </div>
        <div className="p-4 text-center">
          <div className="text-lg font-mono text-gray-400">{script.author}</div>
          <div className="text-xs text-gray-500">Author</div>
        </div>
      </div>

      {/* Code */}
      <div className="p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-white font-medium">Source Code</h3>
          <button className="text-indigo-400 hover:text-indigo-300 text-sm">
            Copy to Clipboard
          </button>
        </div>
        <pre className="bg-black/60 rounded-xl p-4 overflow-x-auto">
          <code className="text-sm text-gray-300 font-mono whitespace-pre">
            {script.code}
          </code>
        </pre>
      </div>

      {/* Actions */}
      <div className="p-6 bg-black/20 border-t border-indigo-800/30">
        <div className="flex items-center justify-between">
          <div className="text-sm text-gray-500">
            Gas estimate: ~250,000 | Recommended min profit: 0.01 ETH
          </div>
          <div className="flex gap-2">
            <button className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white rounded-lg transition">
              Execute on Swarm
            </button>
            <button className="px-4 py-2 bg-black/40 hover:bg-indigo-600/20 text-white rounded-lg transition">
              Simulate
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
