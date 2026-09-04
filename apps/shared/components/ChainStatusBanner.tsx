'use client';

import React from 'react';
import clsx from 'clsx';
import { Sparkles, Server, Zap } from 'lucide-react';

interface ChainStatusBannerProps {
  status?: string;
  isConnected?: boolean;
}

export function ChainStatusBanner({ status = 'Unknown', isConnected = false }: ChainStatusBannerProps) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-lg p-3 bg-gradient-to-r from-indigo-600 via-pink-500 to-yellow-400 text-white shadow-lg">
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-full flex items-center justify-center bg-white/10">
          <Sparkles className="w-6 h-6 text-white animate-pulse" />
        </div>
        <div>
          <div className="text-xs uppercase tracking-wide text-white/90">Network</div>
          <div className="font-bold text-lg">
            <span className="bg-clip-text text-transparent bg-gradient-to-r from-white to-yellow-200">{status}</span>
            {isConnected ? ' ⚡️' : ' 🔴'}
          </div>
        </div>
      </div>

      <div className="hidden md:flex items-center gap-3">
        <span className={clsx('px-3 py-1 rounded-full text-sm', isConnected ? 'bg-white/10' : 'bg-red-600/30')}>
          {isConnected ? 'Connected' : 'Disconnected'}
        </span>
        <button className="px-3 py-1 bg-white/10 rounded-md hover:bg-white/20">Details</button>
      </div>
    </div>
  );
}

export default ChainStatusBanner;
