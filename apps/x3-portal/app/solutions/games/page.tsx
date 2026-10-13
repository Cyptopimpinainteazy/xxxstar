'use client';

import React from 'react';
import Link from 'next/link';
import {
  Gamepad2,
  Coins,
  Users,
  Trophy,
  Zap,
  Shield,
  Box,
  Sparkles,
  ArrowRight,
  ExternalLink,
  CheckCircle2,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const gameFeatures = [
  {
    name: 'NFT Game Assets',
    description: 'Create, trade, and use in-game items as NFTs across games',
    icon: <Box className="w-6 h-6" />,
    color: 'from-purple-500 to-indigo-500',
  },
  {
    name: 'Play-to-Earn',
    description: 'Reward players with tokens and tradeable assets',
    icon: <Coins className="w-6 h-6" />,
    color: 'from-emerald-500 to-green-500',
  },
  {
    name: 'Cross-Game Assets',
    description: 'Items that work across multiple games on X3 STAR',
    icon: <Zap className="w-6 h-6" />,
    color: 'from-orange-500 to-amber-500',
  },
  {
    name: 'Tournaments',
    description: 'On-chain tournaments with automated prize distribution',
    icon: <Trophy className="w-6 h-6" />,
    color: 'from-yellow-500 to-amber-500',
  },
  {
    name: 'Guilds & DAOs',
    description: 'Player-owned organizations with governance',
    icon: <Users className="w-6 h-6" />,
    color: 'from-blue-500 to-cyan-500',
  },
  {
    name: 'Anti-Cheat',
    description: 'Verifiable game state and fair randomness',
    icon: <Shield className="w-6 h-6" />,
    color: 'from-red-500 to-rose-500',
  },
];

const sdkFeatures = [
  'Unity SDK for seamless blockchain integration',
  'Unreal Engine plugin support',
  'JavaScript/TypeScript game SDK',
  'Gasless transactions for better UX',
  'Session keys for frictionless gameplay',
  'Real-time event subscriptions',
];

const games = [
  {
    name: 'Atlas Quest',
    genre: 'RPG',
    players: '35K+',
    description: 'Epic fantasy RPG with on-chain progression',
    image: '⚔️',
  },
  {
    name: 'Sphere Racers',
    genre: 'Racing',
    players: '22K+',
    description: 'High-speed racing with NFT vehicles',
    image: '🏎️',
  },
  {
    name: 'Crypto Kingdoms',
    genre: 'Strategy',
    players: '18K+',
    description: 'Build and conquer in this strategy game',
    image: '🏰',
  },
  {
    name: 'X3 Cards',
    genre: 'TCG',
    players: '28K+',
    description: 'Trading card game with rare collectibles',
    image: '🃏',
  },
];

const codeExample = `// Initialize game SDK
import { X3GameSDK, SessionKey } from '@x3star/game-sdk';

const game = new X3GameSDK({
  gameId: 'atlas-quest',
  network: 'mainnet',
});

// Create session key for gasless gameplay
const session = await game.createSession({
  duration: '24h',
  permissions: ['mint-item', 'transfer-item'],
});

// Mint an in-game item (gasless)
const item = await game.mintItem({
  type: 'weapon',
  attributes: { damage: 50, rarity: 'legendary' },
  session: session.key,
});

console.log('Item minted:', item.tokenId);`;

export default function GamesPage() {
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
            <div className="badge badge-pink mt-4 mb-4">Gaming</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Build Web3 Games
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Everything you need to build blockchain games. NFT assets, play-to-earn 
              mechanics, tournaments, and seamless player experiences.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                <Gamepad2 className="w-4 h-4 mr-2" />
                Start Building
              </Link>
              <Link href="/community/grants" className="btn-secondary">
                Gaming Grants
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Gaming Features</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6">
            {gameFeatures.map((feature, index) => (
              <div key={index} className="glass-card p-6">
                <div className={`p-3 rounded-xl bg-gradient-to-br ${feature.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{feature.icon}</span>
                </div>
                <h3 className="font-semibold text-white mb-2">{feature.name}</h3>
                <p className="text-sm text-gray-400">{feature.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* SDK */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Game SDK</h2>
              <p className="text-gray-400 mb-6">
                Our game SDK makes blockchain integration seamless. Focus on building 
                great games while we handle the blockchain complexity.
              </p>
              <ul className="space-y-3">
                {sdkFeatures.map((feature, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-5 h-5 text-emerald-500 mr-3" />
                    {feature}
                  </li>
                ))}
              </ul>
              <div className="mt-6">
                <Link href="/developers/docs" className="btn-primary">
                  View SDK Docs
                </Link>
              </div>
            </div>
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center justify-between">
                <span className="text-sm text-gray-400">game-integration.ts</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{codeExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Games */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Games on X3 STAR</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {games.map((game, index) => (
              <div key={index} className="glass-card p-6 hover:border-orange-500/30 transition-colors cursor-pointer group">
                <div className="text-5xl mb-4">{game.image}</div>
                <div className="flex items-center gap-2 mb-2">
                  <h3 className="font-semibold text-white group-hover:text-orange-400 transition-colors">
                    {game.name}
                  </h3>
                  <span className="badge badge-default text-xs">{game.genre}</span>
                </div>
                <p className="text-sm text-gray-400 mb-4">{game.description}</p>
                <p className="text-sm font-medium gradient-text">{game.players} players</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Benefits */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid md:grid-cols-3 gap-8">
            <div className="text-center">
              <div className="text-4xl font-bold gradient-text mb-2">10x</div>
              <p className="text-gray-400">Faster integration than other chains</p>
            </div>
            <div className="text-center">
              <div className="text-4xl font-bold gradient-text mb-2">$0.001</div>
              <p className="text-gray-400">Average transaction cost</p>
            </div>
            <div className="text-center">
              <div className="text-4xl font-bold gradient-text mb-2">100K+</div>
              <p className="text-gray-400">Active gamers on X3 STAR</p>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Sparkles className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Ready to Build Your Game?
          </h2>
          <p className="text-gray-400 mb-8">
            Get started with our game SDK and join the growing ecosystem of X3 STAR games.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Game Dev Guide
            </Link>
            <Link href="/community/grants" className="btn-secondary">
              Apply for Grant
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
