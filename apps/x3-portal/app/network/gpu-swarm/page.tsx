'use client';

import React from 'react';
import SwarmMap from '@/components/gpu-swarm/SwarmMap';

export default function GpuSwarmPage() {
  return (
    <div className="min-h-screen bg-black">
      {/* Cyberpunk Background Effects */}
      <div className="fixed inset-0 pointer-events-none">
        {/* Gradient Overlay */}
        <div className="absolute inset-0 bg-gradient-to-br from-cyan-900/10 via-black to-purple-900/10" />
        
        {/* Animated Grid */}
        <div 
          className="absolute inset-0 opacity-[0.03]"
          style={{
            backgroundImage: `
              linear-gradient(rgba(0, 255, 255, 0.1) 1px, transparent 1px),
              linear-gradient(90deg, rgba(0, 255, 255, 0.1) 1px, transparent 1px)
            `,
            backgroundSize: '50px 50px',
          }}
        />

        {/* Scan Line */}
        <div 
          className="absolute inset-0 opacity-[0.02]"
          style={{
            backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0, 255, 255, 0.03) 2px, rgba(0, 255, 255, 0.03) 4px)',
          }}
        />
      </div>

      {/* Content */}
      <div className="relative z-10 container-wide py-8 pt-24">
        {/* Page Header */}
        <div className="mb-8">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-3 h-3 bg-cyan-500 rounded-full animate-pulse shadow-[0_0_20px_rgba(0,255,255,0.8)]" />
            <span className="text-cyan-400 font-mono text-sm uppercase tracking-[0.3em]">Network Status: Online</span>
          </div>
          <h1 className="text-4xl md:text-5xl font-bold text-white mb-2">
            <span className="bg-gradient-to-r from-cyan-400 via-purple-400 to-pink-400 bg-clip-text text-transparent">
              GPU Swarm Network
            </span>
          </h1>
          <p className="text-gray-500 max-w-2xl font-mono">
            Real-time visualization of the distributed GPU compute network. Monitor node status, 
            task distribution, and global hashrate across the Atlas Sphere infrastructure.
          </p>
        </div>

        {/* Decorative Cyber Lines */}
        <div className="relative mb-8">
          <div className="absolute left-0 top-1/2 w-32 h-px bg-gradient-to-r from-cyan-500/50 to-transparent" />
          <div className="absolute right-0 top-1/2 w-32 h-px bg-gradient-to-l from-purple-500/50 to-transparent" />
        </div>

        {/* Main Map Component */}
        <SwarmMap className="mb-8" />

        {/* Additional Info Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mt-8">
          {/* Network Health */}
          <div className="p-6 bg-black/60 backdrop-blur-xl border border-cyan-500/20 rounded-xl relative overflow-hidden group hover:border-cyan-500/40 transition-all">
            <div className="absolute top-0 right-0 w-32 h-32 bg-cyan-500/5 rounded-full blur-3xl group-hover:bg-cyan-500/10 transition-all" />
            <div className="relative">
              <div className="w-12 h-12 rounded-lg bg-cyan-500/20 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-cyan-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <h3 className="text-lg font-bold text-white mb-2">Network Health</h3>
              <p className="text-gray-500 text-sm font-mono mb-4">Real-time network diagnostics and uptime monitoring</p>
              <div className="flex items-center gap-2">
                <span className="text-2xl font-bold text-cyan-400 font-mono">99.7%</span>
                <span className="text-xs text-gray-500 uppercase">Uptime</span>
              </div>
            </div>
          </div>

          {/* Task Distribution */}
          <div className="p-6 bg-black/60 backdrop-blur-xl border border-purple-500/20 rounded-xl relative overflow-hidden group hover:border-purple-500/40 transition-all">
            <div className="absolute top-0 right-0 w-32 h-32 bg-purple-500/5 rounded-full blur-3xl group-hover:bg-purple-500/10 transition-all" />
            <div className="relative">
              <div className="w-12 h-12 rounded-lg bg-purple-500/20 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
                </svg>
              </div>
              <h3 className="text-lg font-bold text-white mb-2">Task Distribution</h3>
              <p className="text-gray-500 text-sm font-mono mb-4">Load balancing across geographic regions</p>
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-gray-400">Americas</span>
                  <span className="text-purple-400 font-mono">32%</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-400">Europe</span>
                  <span className="text-purple-400 font-mono">28%</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-400">Asia Pacific</span>
                  <span className="text-purple-400 font-mono">40%</span>
                </div>
              </div>
            </div>
          </div>

          {/* Rewards Pool */}
          <div className="p-6 bg-black/60 backdrop-blur-xl border border-pink-500/20 rounded-xl relative overflow-hidden group hover:border-pink-500/40 transition-all">
            <div className="absolute top-0 right-0 w-32 h-32 bg-pink-500/5 rounded-full blur-3xl group-hover:bg-pink-500/10 transition-all" />
            <div className="relative">
              <div className="w-12 h-12 rounded-lg bg-pink-500/20 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-pink-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <h3 className="text-lg font-bold text-white mb-2">Rewards Pool</h3>
              <p className="text-gray-500 text-sm font-mono mb-4">Current epoch distribution pending</p>
              <div className="flex items-baseline gap-2">
                <span className="text-2xl font-bold text-pink-400 font-mono">1,247,832</span>
                <span className="text-xs text-gray-500 uppercase">X3 Tokens</span>
              </div>
            </div>
          </div>
        </div>

        {/* Join Network CTA */}
        <div className="mt-12 p-8 bg-gradient-to-r from-cyan-500/10 via-purple-500/10 to-pink-500/10 rounded-2xl border border-cyan-500/20 relative overflow-hidden">
          <div className="absolute inset-0 bg-[url('data:image/svg+xml,%3Csvg%20viewBox%3D%220%200%2040%2040%22%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%3E%3Cpath%20d%3D%22M0%2040L40%200M-10%2010L10%20-10M30%2050L50%2030%22%20stroke%3D%22rgba(0%2C255%2C255%2C0.05)%22%20stroke-width%3D%221%22%2F%3E%3C%2Fsvg%3E')] opacity-50" />
          <div className="relative flex flex-col md:flex-row items-center justify-between gap-6">
            <div>
              <h3 className="text-2xl font-bold text-white mb-2">Join the GPU Swarm</h3>
              <p className="text-gray-400 font-mono max-w-xl">
                Contribute your GPU compute power to the network and earn X3 tokens. 
                Support decentralized AI, MEV optimization, and cross-chain computations.
              </p>
            </div>
            <div className="flex gap-4">
              <a 
                href="/developers/gpu-node-setup" 
                className="px-6 py-3 bg-cyan-500 text-black font-bold rounded-lg hover:bg-cyan-400 transition-all shadow-[0_0_30px_rgba(0,255,255,0.3)] hover:shadow-[0_0_40px_rgba(0,255,255,0.5)]"
              >
                Setup Node
              </a>
              <a 
                href="/learn/gpu-swarm" 
                className="px-6 py-3 border border-cyan-500/50 text-cyan-400 font-bold rounded-lg hover:bg-cyan-500/10 transition-all"
              >
                Learn More
              </a>
            </div>
          </div>
        </div>

        {/* Footer Decoration */}
        <div className="mt-8 flex items-center justify-center gap-4 opacity-30">
          <div className="w-1 h-1 bg-cyan-500 rounded-full" />
          <div className="w-24 h-px bg-gradient-to-r from-transparent via-cyan-500 to-transparent" />
          <div className="text-xs font-mono text-gray-600 uppercase tracking-widest">Atlas Sphere GPU Network</div>
          <div className="w-24 h-px bg-gradient-to-r from-transparent via-cyan-500 to-transparent" />
          <div className="w-1 h-1 bg-cyan-500 rounded-full" />
        </div>
      </div>
    </div>
  );
}
