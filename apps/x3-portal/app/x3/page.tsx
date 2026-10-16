'use client';

import Link from 'next/link';
import { useEffect, useState } from 'react';

export default function X3LandingPage() {
  const [stats, setStats] = useState({
    mutations: 0,
    swarmNodes: 0,
    receipts: 0,
    scripts: 0,
  });

  useEffect(() => {
    // Animate stats
    const interval = setInterval(() => {
      setStats((prev) => ({
        mutations: Math.min(prev.mutations + 3, 89),
        swarmNodes: Math.min(prev.swarmNodes + 8, 247),
        receipts: Math.min(prev.receipts + 523, 15234),
        scripts: Math.min(prev.scripts + 1, 42),
      }));
    }, 50);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-violet-900 to-slate-900">
      {/* Hero */}
      <header className="relative overflow-hidden">
        {/* Animated background */}
        <div className="absolute inset-0 overflow-hidden">
          <div className="absolute -top-1/2 -left-1/2 w-full h-full bg-gradient-to-br from-purple-500/20 to-transparent rounded-full blur-3xl animate-pulse" />
          <div className="absolute -bottom-1/2 -right-1/2 w-full h-full bg-gradient-to-tl from-blue-500/20 to-transparent rounded-full blur-3xl animate-pulse delay-1000" />
        </div>

        <nav className="relative container mx-auto px-6 py-6 flex items-center justify-between">
          <Link href="/" className="text-2xl font-bold bg-gradient-to-r from-purple-400 via-pink-400 to-cyan-400 bg-clip-text text-transparent">
            X3 Sphere
          </Link>
          <div className="flex items-center gap-6">
            <Link href="/x3/evolution" className="text-gray-400 hover:text-white transition">Evolution</Link>
            <Link href="/x3/swarm" className="text-gray-400 hover:text-white transition">Swarm</Link>
            <Link href="/x3/verifier" className="text-gray-400 hover:text-white transition">Verifier</Link>
            <Link href="/x3/scripts" className="text-gray-400 hover:text-white transition">Scripts</Link>
            <button className="px-4 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 text-white rounded-lg transition">
              Connect Wallet
            </button>
          </div>
        </nav>

        <div className="relative container mx-auto px-6 py-24 text-center">
          <div className="inline-block px-4 py-2 bg-purple-500/20 border border-purple-500/30 rounded-full text-purple-300 text-sm mb-6">
            🧬 The Self-Evolving Blockchain
          </div>
          <h1 className="text-6xl md:text-7xl font-bold text-white mb-6">
            X3 <span className="bg-gradient-to-r from-purple-400 via-pink-400 to-cyan-400 bg-clip-text text-transparent">Adaptive</span>
            <br />Intelligence Chain
          </h1>
          <p className="text-xl text-gray-400 max-w-3xl mx-auto mb-10">
            The world&apos;s first L1 blockchain that modifies itself based on network conditions, 
            usage patterns, and AI-generated optimizations. Runtime mutations, swarm execution, 
            and atomic MEV protection - all in one chain.
          </p>
          <div className="flex items-center justify-center gap-4">
            <Link 
              href="/x3/evolution"
              className="px-8 py-4 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 text-white rounded-xl font-medium transition transform hover:scale-105"
            >
              Explore Evolution
            </Link>
            <Link 
              href="/x3/scripts"
              className="px-8 py-4 bg-white/10 hover:bg-white/20 text-white rounded-xl font-medium transition border border-white/20"
            >
              Browse Scripts
            </Link>
          </div>
        </div>
      </header>

      {/* Stats */}
      <section className="container mx-auto px-6 py-16">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
          <StatCard icon="🧬" value={stats.mutations.toString()} label="Runtime Mutations" />
          <StatCard icon="🖥️" value={stats.swarmNodes.toString()} label="Swarm Nodes" />
          <StatCard icon="📜" value={stats.receipts.toLocaleString()} label="Verified Receipts" />
          <StatCard icon="⚡" value={stats.scripts.toString()} label="X3 Scripts" />
        </div>
      </section>

      {/* Features */}
      <section className="container mx-auto px-6 py-16">
        <h2 className="text-4xl font-bold text-white text-center mb-16">
          Revolutionary <span className="text-purple-400">Architecture</span>
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          <FeatureCard
            icon="🧬"
            title="Evolution Core"
            description="AI-driven runtime mutations that optimize gas prices, block weights, and VM allocation based on real-time network conditions."
            link="/x3/evolution"
            color="purple"
          />
          <FeatureCard
            icon="🌐"
            title="Swarm Network"
            description="Decentralized execution layer with 247+ nodes providing parallel computation, MEV extraction, and off-chain processing."
            link="/x3/swarm"
            color="blue"
          />
          <FeatureCard
            icon="✅"
            title="Receipt Verifier"
            description="On-chain verification of swarm execution with Merkle proofs, dispute resolution, and fair reward distribution."
            link="/x3/verifier"
            color="green"
          />
        </div>
      </section>

      {/* X3 Language */}
      <section className="container mx-auto px-6 py-16">
        <div className="bg-black/40 rounded-3xl border border-purple-800/30 p-8 md:p-12">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
            <div>
              <div className="inline-block px-3 py-1 bg-purple-500/20 rounded-full text-purple-300 text-sm mb-4">
                Domain-Specific Language
              </div>
              <h2 className="text-4xl font-bold text-white mb-6">
                X3: The Language of <span className="text-purple-400">MEV</span>
              </h2>
              <p className="text-gray-400 mb-6">
                Purpose-built for trading, MEV extraction, and DeFi automation. 
                30+ built-in functions, atomic execution guarantees, and swarm-native annotations.
              </p>
              <ul className="space-y-3 text-gray-300">
                <li className="flex items-center gap-3">
                  <span className="text-green-400">✓</span>
                  Flash loan primitives built-in
                </li>
                <li className="flex items-center gap-3">
                  <span className="text-green-400">✓</span>
                  Atomic cross-DEX execution
                </li>
                <li className="flex items-center gap-3">
                  <span className="text-green-400">✓</span>
                  Mempool-aware annotations
                </li>
                <li className="flex items-center gap-3">
                  <span className="text-green-400">✓</span>
                  Gas-optimized bytecode
                </li>
              </ul>
              <Link 
                href="/x3/scripts"
                className="inline-block mt-8 px-6 py-3 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition"
              >
                Browse Script Library →
              </Link>
            </div>
            <div className="bg-black/60 rounded-2xl p-6 border border-purple-800/30">
              <pre className="text-sm font-mono overflow-x-auto">
                <code className="text-gray-300">
{`// arb.x3 - Atomic Arbitrage
@swarm @atomic
fn execute(amount: u256) -> u256 {
    // Flash borrow from Aave
    let borrowed = flash_borrow(AAVE, WETH, amount);
    
    // Get prices
    let uni = get_price(UNI, WETH, USDC);
    let sushi = get_price(SUSHI, WETH, USDC);
    
    // Execute profitable direction
    if uni > sushi {
        let usdc = swap(SUSHI, WETH, USDC, amount);
        let weth = swap(UNI, USDC, WETH, usdc);
        profit = weth - amount;
    }
    
    // Repay and return profit
    flash_repay(AAVE, WETH, amount + fee);
    return profit;
}`}
                </code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* How It Works */}
      <section className="container mx-auto px-6 py-16">
        <h2 className="text-4xl font-bold text-white text-center mb-16">
          How It <span className="text-cyan-400">Works</span>
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-8">
          <StepCard
            step={1}
            title="Write X3 Script"
            description="Create your MEV strategy using the X3 domain language"
          />
          <StepCard
            step={2}
            title="Deploy to Swarm"
            description="Your script is distributed across 247+ execution nodes"
          />
          <StepCard
            step={3}
            title="Execute & Prove"
            description="Nodes execute and generate cryptographic receipts"
          />
          <StepCard
            step={4}
            title="Verify & Reward"
            description="On-chain verification distributes rewards fairly"
          />
        </div>
      </section>

      {/* CTA */}
      <section className="container mx-auto px-6 py-24">
        <div className="bg-gradient-to-r from-purple-600/20 to-pink-600/20 rounded-3xl border border-purple-500/30 p-12 text-center">
          <h2 className="text-4xl font-bold text-white mb-6">
            Ready to Build on the Future?
          </h2>
          <p className="text-xl text-gray-400 max-w-2xl mx-auto mb-10">
            Join the next generation of blockchain development. Write adaptive smart contracts,
            participate in the swarm network, and shape the evolution of the chain.
          </p>
          <div className="flex items-center justify-center gap-4">
            <Link 
              href="/developers"
              className="px-8 py-4 bg-gradient-to-r from-purple-600 to-pink-600 text-white rounded-xl font-medium transition transform hover:scale-105"
            >
              Start Building
            </Link>
            <Link 
              href="https://docs.atlas-sphere.io/x3"
              className="px-8 py-4 bg-white/10 hover:bg-white/20 text-white rounded-xl font-medium transition border border-white/20"
            >
              Read Docs
            </Link>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-purple-800/30 py-12">
        <div className="container mx-auto px-6 text-center text-gray-500">
          <p>© 2024 X3 Sphere • The Self-Evolving Blockchain</p>
        </div>
      </footer>
    </div>
  );
}

function StatCard({ icon, value, label }: { icon: string; value: string; label: string }) {
  return (
    <div className="bg-black/40 rounded-2xl border border-purple-800/30 p-6 text-center">
      <span className="text-3xl">{icon}</span>
      <div className="text-4xl font-bold text-white mt-2">{value}</div>
      <div className="text-gray-500 text-sm mt-1">{label}</div>
    </div>
  );
}

function FeatureCard({ icon, title, description, link, color }: {
  icon: string;
  title: string;
  description: string;
  link: string;
  color: 'purple' | 'blue' | 'green';
}) {
  const colors = {
    purple: 'from-purple-500/20 to-purple-600/10 border-purple-500/30 hover:border-purple-400/50',
    blue: 'from-blue-500/20 to-blue-600/10 border-blue-500/30 hover:border-blue-400/50',
    green: 'from-green-500/20 to-green-600/10 border-green-500/30 hover:border-green-400/50',
  };

  return (
    <Link href={link}>
      <div className={`bg-gradient-to-br ${colors[color]} rounded-2xl border p-8 transition transform hover:scale-105 cursor-pointer h-full`}>
        <span className="text-4xl">{icon}</span>
        <h3 className="text-xl font-semibold text-white mt-4 mb-3">{title}</h3>
        <p className="text-gray-400">{description}</p>
      </div>
    </Link>
  );
}

function StepCard({ step, title, description }: { step: number; title: string; description: string }) {
  return (
    <div className="text-center">
      <div className="w-16 h-16 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center mx-auto mb-4">
        <span className="text-2xl font-bold text-white">{step}</span>
      </div>
      <h3 className="text-lg font-semibold text-white mb-2">{title}</h3>
      <p className="text-gray-500 text-sm">{description}</p>
    </div>
  );
}
