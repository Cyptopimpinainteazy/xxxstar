'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  BookOpen,
  Code,
  Layers,
  Wallet,
  Rocket,
  Shield,
  Zap,
  Clock,
  Users,
  ChevronRight,
  Play,
  CheckCircle,
  Filter,
} from 'lucide-react';

type Difficulty = 'beginner' | 'intermediate' | 'advanced';
type Category = 'all' | 'getting-started' | 'smart-contracts' | 'cross-vm' | 'defi' | 'nft';

interface Tutorial {
  id: string;
  title: string;
  description: string;
  difficulty: Difficulty;
  category: Category;
  duration: string;
  author: string;
  image: string;
  tags: string[];
  href: string;
}

const tutorials: Tutorial[] = [
  {
    id: '1',
    title: 'Introduction to X3 Atlas Sphere',
    description: 'Learn the fundamentals of the first dual VM blockchain and understand its unique architecture.',
    difficulty: 'beginner',
    category: 'getting-started',
    duration: '20 min',
    author: 'Atlas Team',
    image: '/tutorials/intro.jpg',
    tags: ['Basics', 'Architecture', 'Overview'],
    href: '/learn/tutorials/introduction',
  },
  {
    id: '2',
    title: 'Setting Up Your Development Environment',
    description: 'Configure Rust, build tools, and everything needed to develop on X3 Atlas Sphere.',
    difficulty: 'beginner',
    category: 'getting-started',
    duration: '30 min',
    author: 'Atlas Team',
    image: '/tutorials/setup.jpg',
    tags: ['Setup', 'Rust', 'Tools'],
    href: '/learn/tutorials/setup',
  },
  {
    id: '3',
    title: 'Deploying Your First EVM Contract',
    description: 'Deploy a Solidity smart contract to the X3 EVM execution environment.',
    difficulty: 'beginner',
    category: 'smart-contracts',
    duration: '25 min',
    author: 'Atlas Team',
    image: '/tutorials/evm.jpg',
    tags: ['EVM', 'Solidity', 'Deploy'],
    href: '/learn/tutorials/evm-contract',
  },
  {
    id: '4',
    title: 'Building Solana Programs for X3',
    description: 'Write and deploy Rust programs to the X3 SVM execution environment using Anchor.',
    difficulty: 'intermediate',
    category: 'smart-contracts',
    duration: '45 min',
    author: 'Atlas Team',
    image: '/tutorials/svm.jpg',
    tags: ['SVM', 'Rust', 'Anchor'],
    href: '/learn/tutorials/svm-program',
  },
  {
    id: '5',
    title: 'Cross-VM Comit Transactions',
    description: 'Create atomic transactions that execute on both EVM and SVM simultaneously.',
    difficulty: 'intermediate',
    category: 'cross-vm',
    duration: '40 min',
    author: 'Atlas Team',
    image: '/tutorials/cross-vm.jpg',
    tags: ['Cross-VM', 'Comits', 'Atomic'],
    href: '/learn/tutorials/cross-vm-comits',
  },
  {
    id: '6',
    title: 'Building a Cross-VM DEX',
    description: 'Build a decentralized exchange that aggregates liquidity from both EVM and SVM protocols.',
    difficulty: 'advanced',
    category: 'defi',
    duration: '90 min',
    author: 'Atlas Team',
    image: '/tutorials/dex.jpg',
    tags: ['DeFi', 'DEX', 'Cross-VM'],
    href: '/learn/tutorials/cross-vm-dex',
  },
  {
    id: '7',
    title: 'NFT Marketplace on X3',
    description: 'Create an NFT marketplace that supports both ERC-721 and Metaplex standards.',
    difficulty: 'advanced',
    category: 'nft',
    duration: '75 min',
    author: 'Atlas Team',
    image: '/tutorials/nft.jpg',
    tags: ['NFT', 'ERC-721', 'Metaplex'],
    href: '/learn/tutorials/nft-marketplace',
  },
  {
    id: '8',
    title: 'Atlas Kernel Deep Dive',
    description: 'Understand the core pallet that powers cross-VM orchestration and the canonical ledger.',
    difficulty: 'advanced',
    category: 'cross-vm',
    duration: '60 min',
    author: 'Atlas Team',
    image: '/tutorials/kernel.jpg',
    tags: ['Atlas Kernel', 'Substrate', 'Pallet'],
    href: '/learn/tutorials/atlas-kernel',
  },
];

const difficultyColors = {
  beginner: 'badge-success',
  intermediate: 'badge-warning',
  advanced: 'badge-purple',
};

const categories = [
  { id: 'all', label: 'All Tutorials', icon: <BookOpen className="w-4 h-4" /> },
  { id: 'getting-started', label: 'Getting Started', icon: <Rocket className="w-4 h-4" /> },
  { id: 'smart-contracts', label: 'Smart Contracts', icon: <Code className="w-4 h-4" /> },
  { id: 'cross-vm', label: 'Cross-VM', icon: <Layers className="w-4 h-4" /> },
  { id: 'defi', label: 'DeFi', icon: <Zap className="w-4 h-4" /> },
  { id: 'nft', label: 'NFT', icon: <Shield className="w-4 h-4" /> },
];

export default function TutorialsPage() {
  const [selectedCategory, setSelectedCategory] = useState<Category>('all');
  const [selectedDifficulty, setSelectedDifficulty] = useState<Difficulty | 'all'>('all');

  const filteredTutorials = tutorials.filter((tutorial) => {
    const categoryMatch = selectedCategory === 'all' || tutorial.category === selectedCategory;
    const difficultyMatch = selectedDifficulty === 'all' || tutorial.difficulty === selectedDifficulty;
    return categoryMatch && difficultyMatch;
  });

  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="badge badge-info mb-4">Learn</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Tutorials & Guides
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Step-by-step tutorials to help you master X3 Atlas Sphere development. 
              From basics to advanced cross-VM applications.
            </p>
          </div>
        </div>
      </section>

      {/* Filters */}
      <section className="py-8 border-y border-[#1a1a1a]">
        <div className="container-wide">
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-6">
            {/* Category filters */}
            <div className="flex flex-wrap gap-2">
              {categories.map((category) => (
                <button
                  key={category.id}
                  onClick={() => setSelectedCategory(category.id as Category)}
                  className={`flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                    selectedCategory === category.id
                      ? 'bg-indigo-500 text-white'
                      : 'bg-[#0a0a0a] text-gray-400 hover:text-white hover:bg-[#1a1a1a]'
                  }`}
                >
                  <span className="mr-2">{category.icon}</span>
                  {category.label}
                </button>
              ))}
            </div>

            {/* Difficulty filter */}
            <div className="flex items-center gap-2">
              <Filter className="w-4 h-4 text-gray-400" />
              <select
                value={selectedDifficulty}
                onChange={(e) => setSelectedDifficulty(e.target.value as Difficulty | 'all')}
                className="bg-[#0a0a0a] border border-[#1a1a1a] rounded-lg px-4 py-2 text-sm text-white focus:outline-none focus:border-indigo-500"
              >
                <option value="all">All Levels</option>
                <option value="beginner">Beginner</option>
                <option value="intermediate">Intermediate</option>
                <option value="advanced">Advanced</option>
              </select>
            </div>
          </div>
        </div>
      </section>

      {/* Tutorials Grid */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
            {filteredTutorials.map((tutorial) => (
              <Link
                key={tutorial.id}
                href={tutorial.href}
                className="glass-card-hover overflow-hidden card-lift group"
              >
                {/* Image placeholder */}
                <div className="h-48 bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center">
                  <div className="w-16 h-16 rounded-full bg-[#111111] flex items-center justify-center group-hover:bg-indigo-500/20 transition-colors">
                    <Play className="w-8 h-8 text-white" />
                  </div>
                </div>
                
                <div className="p-6">
                  <div className="flex items-center gap-2 mb-3">
                    <span className={`badge ${difficultyColors[tutorial.difficulty]}`}>
                      {tutorial.difficulty}
                    </span>
                    <span className="text-xs text-gray-600 flex items-center">
                      <Clock className="w-3 h-3 mr-1" />
                      {tutorial.duration}
                    </span>
                  </div>
                  
                  <h3 className="text-lg font-semibold text-white mb-2 group-hover:text-indigo-400 transition-colors">
                    {tutorial.title}
                  </h3>
                  <p className="text-sm text-gray-400 mb-4 line-clamp-2">
                    {tutorial.description}
                  </p>
                  
                  <div className="flex flex-wrap gap-2 mb-4">
                    {tutorial.tags.map((tag) => (
                      <span key={tag} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                        {tag}
                      </span>
                    ))}
                  </div>
                  
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600">By {tutorial.author}</span>
                    <span className="text-indigo-400 flex items-center group-hover:text-indigo-300">
                      Start <ChevronRight className="w-4 h-4 ml-1" />
                    </span>
                  </div>
                </div>
              </Link>
            ))}
          </div>

          {filteredTutorials.length === 0 && (
            <div className="text-center py-16">
              <div className="text-gray-400 mb-4">No tutorials found matching your filters.</div>
              <button
                onClick={() => {
                  setSelectedCategory('all');
                  setSelectedDifficulty('all');
                }}
                className="btn-secondary"
              >
                Clear Filters
              </button>
            </div>
          )}
        </div>
      </section>

      {/* Learning Path */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Recommended Learning Path</h2>
          
          <div className="relative">
            {/* Connection line */}
            <div className="absolute left-8 top-0 bottom-0 w-0.5 bg-gradient-to-b from-indigo-500 via-purple-500 to-cyan-500 hidden md:block" />
            
            <div className="space-y-8">
              {[
                { step: 1, title: 'Learn the Basics', desc: 'Understand X3 architecture and dual VM concept', tutorials: ['Introduction to X3 Atlas Sphere'] },
                { step: 2, title: 'Set Up Environment', desc: 'Install tools and run a local node', tutorials: ['Setting Up Your Development Environment'] },
                { step: 3, title: 'Deploy Contracts', desc: 'Learn to deploy on EVM and SVM', tutorials: ['Deploying Your First EVM Contract', 'Building Solana Programs for X3'] },
                { step: 4, title: 'Master Cross-VM', desc: 'Build atomic cross-VM applications', tutorials: ['Cross-VM Comit Transactions', 'Atlas Kernel Deep Dive'] },
                { step: 5, title: 'Build Real Apps', desc: 'Create production-ready dApps', tutorials: ['Building a Cross-VM DEX', 'NFT Marketplace on X3'] },
              ].map((item) => (
                <div key={item.step} className="flex items-start gap-8">
                  <div className="flex-shrink-0 w-16 h-16 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-xl font-bold text-white relative z-10">
                    {item.step}
                  </div>
                  <div className="flex-1 glass-card p-6">
                    <h3 className="text-lg font-semibold text-white mb-2">{item.title}</h3>
                    <p className="text-gray-400 mb-4">{item.desc}</p>
                    <div className="flex flex-wrap gap-2">
                      {item.tutorials.map((t) => (
                        <span key={t} className="text-xs px-3 py-1 rounded-full bg-indigo-500/20 text-indigo-300">
                          {t}
                        </span>
                      ))}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">Ready to start building?</h2>
          <p className="text-gray-400 mb-8">
            Jump into the documentation or get hands-on with the cookbook examples.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              View Documentation
            </Link>
            <Link href="/developers/cookbook" className="btn-secondary">
              Browse Cookbook
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
