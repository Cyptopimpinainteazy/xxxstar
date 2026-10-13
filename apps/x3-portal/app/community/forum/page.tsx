'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  MessageSquare,
  Search,
  Filter,
  TrendingUp,
  Clock,
  User,
  MessageCircle,
  Eye,
  Pin,
  ChevronRight,
  Plus,
  ArrowUp,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const categories = [
  { name: 'All', count: 1247, color: 'bg-gray-500' },
  { name: 'General', count: 423, color: 'bg-blue-500' },
  { name: 'Development', count: 312, color: 'bg-emerald-500' },
  { name: 'Governance', count: 156, color: 'bg-purple-500' },
  { name: 'Support', count: 198, color: 'bg-orange-500' },
  { name: 'Showcase', count: 89, color: 'bg-pink-500' },
  { name: 'Trading', count: 69, color: 'bg-yellow-500' },
];

const pinnedTopics = [
  {
    title: 'Welcome to the X3 STAR Community Forum',
    category: 'General',
    author: 'X3 Team',
    replies: 234,
    views: 12453,
    lastActivity: '2 hours ago',
  },
  {
    title: 'X3 STAR Roadmap 2024 - Your Feedback Wanted',
    category: 'Governance',
    author: 'X3 Team',
    replies: 89,
    views: 5632,
    lastActivity: '5 hours ago',
  },
];

const recentTopics = [
  {
    id: 1,
    title: 'How to set up a validator node on testnet?',
    category: 'Development',
    author: 'validator_mike',
    authorAvatar: 'VM',
    replies: 12,
    views: 456,
    lastActivity: '15 min ago',
    isHot: true,
  },
  {
    id: 2,
    title: 'Cross-VM swap failing with error 0x11',
    category: 'Support',
    author: 'defi_builder',
    authorAvatar: 'DB',
    replies: 8,
    views: 234,
    lastActivity: '32 min ago',
    isHot: false,
  },
  {
    id: 3,
    title: 'Proposal: Reduce minimum validator stake',
    category: 'Governance',
    author: 'community_voice',
    authorAvatar: 'CV',
    replies: 67,
    views: 1823,
    lastActivity: '1 hour ago',
    isHot: true,
  },
  {
    id: 4,
    title: 'Showcase: My first dApp on X3 STAR',
    category: 'Showcase',
    author: 'newdev_sarah',
    authorAvatar: 'NS',
    replies: 23,
    views: 567,
    lastActivity: '2 hours ago',
    isHot: false,
  },
  {
    id: 5,
    title: 'Best practices for Comit transactions',
    category: 'Development',
    author: 'atlas_expert',
    authorAvatar: 'AE',
    replies: 34,
    views: 892,
    lastActivity: '3 hours ago',
    isHot: false,
  },
  {
    id: 6,
    title: 'ATLAS price discussion - Q1 2024',
    category: 'Trading',
    author: 'crypto_analyst',
    authorAvatar: 'CA',
    replies: 156,
    views: 4532,
    lastActivity: '4 hours ago',
    isHot: true,
  },
  {
    id: 7,
    title: 'Tutorial: Building a cross-VM NFT marketplace',
    category: 'Development',
    author: 'nft_wizard',
    authorAvatar: 'NW',
    replies: 45,
    views: 1234,
    lastActivity: '5 hours ago',
    isHot: false,
  },
  {
    id: 8,
    title: 'Network congestion - Jan 20th incident report',
    category: 'General',
    author: 'X3 Team',
    authorAvatar: 'X3',
    replies: 28,
    views: 2156,
    lastActivity: '6 hours ago',
    isHot: false,
  },
];

const topContributors = [
  { name: 'atlas_expert', posts: 234, reputation: 4523 },
  { name: 'defi_builder', posts: 189, reputation: 3812 },
  { name: 'validator_mike', posts: 156, reputation: 3245 },
  { name: 'nft_wizard', posts: 134, reputation: 2987 },
  { name: 'community_voice', posts: 112, reputation: 2654 },
];

export default function ForumPage() {
  const [selectedCategory, setSelectedCategory] = useState('All');
  const [sortBy, setSortBy] = useState<'recent' | 'popular'>('recent');

  const filteredTopics = selectedCategory === 'All' 
    ? recentTopics 
    : recentTopics.filter(t => t.category === selectedCategory);

  return (
    <div className="min-h-screen pt-20">
      {/* Header */}
      <section className="py-12 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-30">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6">
            <div>
              <Link href="/community" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
                ← Back to Community
              </Link>
              <h1 className="text-3xl font-bold text-white mt-4">Community Forum</h1>
              <p className="text-gray-400 mt-2">Discuss, ask questions, and share with the X3 STAR community</p>
            </div>
            <button className="btn-primary flex items-center">
              <Plus className="w-4 h-4 mr-2" />
              New Topic
            </button>
          </div>
        </div>
      </section>

      {/* Search and Filter */}
      <section className="py-6 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="flex flex-col md:flex-row gap-4">
            <div className="relative flex-1">
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
              <input
                type="text"
                placeholder="Search topics..."
                className="w-full pl-12 pr-4 py-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-500 focus:border-orange-500/50 focus:outline-none"
              />
            </div>
            <div className="flex gap-3">
              <button
                onClick={() => setSortBy('recent')}
                className={`px-4 py-3 rounded-xl flex items-center ${sortBy === 'recent' ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30' : 'bg-[#0a0a0a] text-gray-400 border border-[#1a1a1a] hover:bg-[#111111]'}`}
              >
                <Clock className="w-4 h-4 mr-2" />
                Recent
              </button>
              <button
                onClick={() => setSortBy('popular')}
                className={`px-4 py-3 rounded-xl flex items-center ${sortBy === 'popular' ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30' : 'bg-[#0a0a0a] text-gray-400 border border-[#1a1a1a] hover:bg-[#111111]'}`}
              >
                <TrendingUp className="w-4 h-4 mr-2" />
                Popular
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* Main Content */}
      <section className="py-8">
        <div className="container-wide">
          <div className="grid lg:grid-cols-4 gap-8">
            {/* Sidebar */}
            <div className="lg:col-span-1 space-y-6">
              {/* Categories */}
              <div className="glass-card p-4">
                <h3 className="font-semibold text-white mb-4">Categories</h3>
                <div className="space-y-2">
                  {categories.map((cat) => (
                    <button
                      key={cat.name}
                      onClick={() => setSelectedCategory(cat.name)}
                      className={`w-full flex items-center justify-between p-2 rounded-lg transition-colors ${selectedCategory === cat.name ? 'bg-orange-500/20 text-orange-400' : 'text-gray-400 hover:bg-[#0a0a0a]'}`}
                    >
                      <div className="flex items-center">
                        <span className={`w-2 h-2 rounded-full ${cat.color} mr-3`} />
                        {cat.name}
                      </div>
                      <span className="text-sm">{cat.count}</span>
                    </button>
                  ))}
                </div>
              </div>

              {/* Top Contributors */}
              <div className="glass-card p-4">
                <h3 className="font-semibold text-white mb-4">Top Contributors</h3>
                <div className="space-y-3">
                  {topContributors.map((contributor, index) => (
                    <div key={contributor.name} className="flex items-center justify-between">
                      <div className="flex items-center">
                        <span className="text-sm text-gray-500 w-5">{index + 1}</span>
                        <div className="w-8 h-8 rounded-full bg-gradient-to-br from-orange-500 to-amber-500 flex items-center justify-center ml-2 mr-3">
                          <span className="text-white text-xs font-bold">{contributor.name.charAt(0).toUpperCase()}</span>
                        </div>
                        <span className="text-sm text-white">{contributor.name}</span>
                      </div>
                      <span className="text-xs text-gray-500">{contributor.reputation}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Topics List */}
            <div className="lg:col-span-3 space-y-4">
              {/* Pinned Topics */}
              {pinnedTopics.map((topic, index) => (
                <div key={index} className="glass-card p-4 border-l-4 border-orange-500">
                  <div className="flex items-start justify-between">
                    <div className="flex items-start">
                      <Pin className="w-4 h-4 text-orange-400 mr-3 mt-1 flex-shrink-0" />
                      <div>
                        <h3 className="font-medium text-white hover:text-orange-400 cursor-pointer">
                          {topic.title}
                        </h3>
                        <div className="flex items-center gap-4 mt-2 text-sm text-gray-400">
                          <span className="badge badge-default">{topic.category}</span>
                          <span>by {topic.author}</span>
                        </div>
                      </div>
                    </div>
                    <div className="text-right text-sm text-gray-400">
                      <div className="flex items-center gap-4">
                        <span className="flex items-center">
                          <MessageCircle className="w-4 h-4 mr-1" />
                          {topic.replies}
                        </span>
                        <span className="flex items-center">
                          <Eye className="w-4 h-4 mr-1" />
                          {topic.views}
                        </span>
                      </div>
                      <p className="mt-1">{topic.lastActivity}</p>
                    </div>
                  </div>
                </div>
              ))}

              {/* Regular Topics */}
              {filteredTopics.map((topic) => (
                <div key={topic.id} className="glass-card p-4 hover:border-orange-500/30 transition-colors cursor-pointer">
                  <div className="flex items-start justify-between">
                    <div className="flex items-start">
                      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-slate-600 to-slate-700 flex items-center justify-center mr-4 flex-shrink-0">
                        <span className="text-white text-sm font-bold">{topic.authorAvatar}</span>
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <h3 className="font-medium text-white hover:text-orange-400">
                            {topic.title}
                          </h3>
                          {topic.isHot && (
                            <span className="badge badge-warning text-xs">Hot</span>
                          )}
                        </div>
                        <div className="flex items-center gap-4 mt-2 text-sm text-gray-400">
                          <span className="badge badge-default">{topic.category}</span>
                          <span>by {topic.author}</span>
                        </div>
                      </div>
                    </div>
                    <div className="text-right text-sm text-gray-400">
                      <div className="flex items-center gap-4">
                        <span className="flex items-center">
                          <MessageCircle className="w-4 h-4 mr-1" />
                          {topic.replies}
                        </span>
                        <span className="flex items-center">
                          <Eye className="w-4 h-4 mr-1" />
                          {topic.views}
                        </span>
                      </div>
                      <p className="mt-1">{topic.lastActivity}</p>
                    </div>
                  </div>
                </div>
              ))}

              {/* Load More */}
              <div className="text-center pt-4">
                <button className="btn-secondary">
                  Load More Topics
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
