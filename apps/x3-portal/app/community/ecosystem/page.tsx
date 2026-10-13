'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  Search,
  Filter,
  ExternalLink,
  Users,
  TrendingUp,
  Star,
  ChevronDown,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const categories = [
  'All',
  'DeFi',
  'NFT',
  'Gaming',
  'Infrastructure',
  'Wallets',
  'Social',
  'Developer Tools',
  'Analytics',
];

const projects = [
  {
    name: 'X3 Swap',
    category: 'DeFi',
    description: 'Cross-VM decentralized exchange with unified liquidity pools',
    logo: '🔄',
    tvl: '$45M',
    users: '25K+',
    featured: true,
    url: '#',
  },
  {
    name: 'Atlas Lend',
    category: 'DeFi',
    description: 'Lending protocol supporting assets from both EVM and SVM',
    logo: '💰',
    tvl: '$32M',
    users: '12K+',
    featured: true,
    url: '#',
  },
  {
    name: 'Sphere NFT',
    category: 'NFT',
    description: 'NFT marketplace for digital collectibles on X3 STAR',
    logo: '🖼️',
    tvl: '10K+ Collections',
    users: '50K+',
    featured: true,
    url: '#',
  },
  {
    name: 'Sphere Wallet',
    category: 'Wallets',
    description: 'Multi-VM wallet supporting both EVM and SVM addresses',
    logo: '👛',
    tvl: null,
    users: '75K+',
    featured: true,
    url: '#',
  },
  {
    name: 'X3 Bridge',
    category: 'Infrastructure',
    description: 'Cross-chain bridge connecting X3 STAR to other networks',
    logo: '🌉',
    tvl: '$20M Volume',
    users: '15K+',
    featured: false,
    url: '#',
  },
  {
    name: 'DualPool',
    category: 'DeFi',
    description: 'Concentrated liquidity AMM for cross-VM trading',
    logo: '🌊',
    tvl: '$18M',
    users: '8K+',
    featured: false,
    url: '#',
  },
  {
    name: 'Atlas Quest',
    category: 'Gaming',
    description: 'Play-to-earn RPG with on-chain assets',
    logo: '⚔️',
    tvl: null,
    users: '35K+',
    featured: false,
    url: '#',
  },
  {
    name: 'X3 Analytics',
    category: 'Analytics',
    description: 'Comprehensive analytics apps/dash-legacy-2-legacy-2board for X3 STAR',
    logo: '📊',
    tvl: null,
    users: '20K+',
    featured: false,
    url: '#',
  },
  {
    name: 'Comit SDK',
    category: 'Developer Tools',
    description: 'TypeScript SDK for building cross-VM applications',
    logo: '🛠️',
    tvl: null,
    users: '2K+ devs',
    featured: false,
    url: '#',
  },
  {
    name: 'X3 Social',
    category: 'Social',
    description: 'Decentralized social platform with on-chain identity',
    logo: '💬',
    tvl: null,
    users: '15K+',
    featured: false,
    url: '#',
  },
  {
    name: 'Yield Sphere',
    category: 'DeFi',
    description: 'Automated yield optimizer across both VMs',
    logo: '🌱',
    tvl: '$12M',
    users: '5K+',
    featured: false,
    url: '#',
  },
  {
    name: 'NFT Launcher',
    category: 'NFT',
    description: 'No-code NFT collection deployment tool',
    logo: '🚀',
    tvl: null,
    users: '3K+',
    featured: false,
    url: '#',
  },
];

const stats = [
  { label: 'Total Projects', value: '120+' },
  { label: 'Total TVL', value: '$150M+' },
  { label: 'Active Users', value: '250K+' },
  { label: 'Daily Transactions', value: '1.2M' },
];

export default function EcosystemPage() {
  const [selectedCategory, setSelectedCategory] = useState('All');
  const [searchQuery, setSearchQuery] = useState('');

  const filteredProjects = projects.filter((project) => {
    const matchesCategory = selectedCategory === 'All' || project.category === selectedCategory;
    const matchesSearch = project.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      project.description.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const featuredProjects = filteredProjects.filter(p => p.featured);
  const otherProjects = filteredProjects.filter(p => !p.featured);

  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-16 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-30">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <Link href="/community" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Community
            </Link>
            <div className="badge badge-info mt-4 mb-4">Ecosystem</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Ecosystem Directory
            </h1>
            <p className="text-xl text-gray-400">
              Discover projects, dApps, and services built on X3 STAR
            </p>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {stats.map((stat, index) => (
              <div key={index} className="glass-card p-6 text-center">
                <p className="text-3xl font-bold gradient-text">{stat.value}</p>
                <p className="text-sm text-gray-400 mt-2">{stat.label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Search and Filter */}
      <section className="py-8">
        <div className="container-wide">
          <div className="flex flex-col md:flex-row gap-4">
            <div className="relative flex-1">
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
              <input
                type="text"
                placeholder="Search projects..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-12 pr-4 py-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-500 focus:border-orange-500/50 focus:outline-none"
              />
            </div>
            <div className="flex gap-2 flex-wrap">
              {categories.map((category) => (
                <button
                  key={category}
                  onClick={() => setSelectedCategory(category)}
                  className={`px-4 py-2 rounded-xl text-sm transition-colors ${
                    selectedCategory === category
                      ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30'
                      : 'bg-[#0a0a0a] text-gray-400 border border-[#1a1a1a] hover:bg-[#111111]'
                  }`}
                >
                  {category}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* Featured Projects */}
      {featuredProjects.length > 0 && (
        <section className="py-8">
          <div className="container-wide">
            <h2 className="text-xl font-bold text-white mb-6 flex items-center">
              <Star className="w-5 h-5 text-orange-400 mr-2" />
              Featured Projects
            </h2>
            
            <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
              {featuredProjects.map((project, index) => (
                <a
                  key={index}
                  href={project.url}
                  className="glass-card p-6 hover:border-orange-500/30 transition-colors group"
                >
                  <div className="flex items-center justify-between mb-4">
                    <span className="text-4xl">{project.logo}</span>
                    <ExternalLink className="w-4 h-4 text-gray-500 group-hover:text-orange-400 transition-colors" />
                  </div>
                  <div className="flex items-center gap-2 mb-2">
                    <h3 className="font-semibold text-white group-hover:text-orange-400 transition-colors">
                      {project.name}
                    </h3>
                    <span className="badge badge-default text-xs">{project.category}</span>
                  </div>
                  <p className="text-sm text-gray-400 mb-4">{project.description}</p>
                  <div className="flex items-center justify-between text-sm">
                    {project.tvl && (
                      <span className="text-emerald-400 flex items-center">
                        <TrendingUp className="w-4 h-4 mr-1" />
                        {project.tvl}
                      </span>
                    )}
                    <span className="text-gray-400 flex items-center">
                      <Users className="w-4 h-4 mr-1" />
                      {project.users}
                    </span>
                  </div>
                </a>
              ))}
            </div>
          </div>
        </section>
      )}

      {/* All Projects */}
      <section className="py-8 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-xl font-bold text-white mb-6">All Projects</h2>
          
          {otherProjects.length > 0 ? (
            <div className="grid sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
              {otherProjects.map((project, index) => (
                <a
                  key={index}
                  href={project.url}
                  className="glass-card p-6 hover:border-orange-500/30 transition-colors group"
                >
                  <div className="flex items-center justify-between mb-4">
                    <span className="text-3xl">{project.logo}</span>
                    <ExternalLink className="w-4 h-4 text-gray-500 group-hover:text-orange-400 transition-colors" />
                  </div>
                  <div className="flex items-center gap-2 mb-2">
                    <h3 className="font-semibold text-white group-hover:text-orange-400 transition-colors">
                      {project.name}
                    </h3>
                  </div>
                  <span className="badge badge-default text-xs mb-3">{project.category}</span>
                  <p className="text-sm text-gray-400 mb-4">{project.description}</p>
                  <div className="flex items-center justify-between text-sm">
                    {project.tvl && (
                      <span className="text-emerald-400">{project.tvl}</span>
                    )}
                    <span className="text-gray-400 flex items-center">
                      <Users className="w-3 h-3 mr-1" />
                      {project.users}
                    </span>
                  </div>
                </a>
              ))}
            </div>
          ) : (
            <div className="glass-card p-12 text-center">
              <p className="text-gray-400">No projects found matching your criteria</p>
            </div>
          )}
        </div>
      </section>

      {/* Submit Project */}
      <section className="py-16">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-2xl font-bold text-white mb-4">
            List Your Project
          </h2>
          <p className="text-gray-400 mb-8">
            Building on X3 STAR? Get your project listed in the ecosystem directory.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#" className="btn-primary">
              Submit Project
            </a>
            <Link href="/community/grants" className="btn-secondary">
              Apply for Grant
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
