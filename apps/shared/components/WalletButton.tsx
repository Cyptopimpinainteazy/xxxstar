/**
 * WalletButton Component
 * 
 * A button that allows users to connect their wallet (MetaMask or Phantom).
 * Only initiates connection when user explicitly clicks.
 */

'use client';

import React, { useState, useEffect } from 'react';
import { 
  Wallet, 
  ChevronDown, 
  LogOut, 
  Copy, 
  Check,
  AlertCircle,
  X
} from 'lucide-react';
import { useWalletConnection, formatAddress, getChainName } from '../hooks/useWalletConnection';
import clsx from 'clsx';

interface WalletButtonProps {
  className?: string;
  showChainInfo?: boolean;
}

export function WalletButton({ className, showChainInfo = true }: WalletButtonProps) {
  const [mounted, setMounted] = useState(false);
  
  // Prevent hydration mismatch by only rendering after mount
  useEffect(() => {
    setMounted(true);
  }, []);

  // Show placeholder during SSR and initial render
  if (!mounted) {
    return (
      <button
        className={clsx(
          'flex items-center gap-2 px-4 py-2 rounded-xl',
          'bg-gradient-to-r from-orange-500 to-red-500',
          'text-white font-medium opacity-80',
          className
        )}
        disabled
      >
        <Wallet className="w-4 h-4" />
        <span className="text-sm">Connect Wallet</span>
      </button>
    );
  }

  return <WalletButtonInner className={className} showChainInfo={showChainInfo} />;
}

// Inner component that uses hooks - only renders on client
function WalletButtonInner({ className, showChainInfo = true }: WalletButtonProps) {
  const {
    isConnected,
    isConnecting,
    error,
    walletType,
    address,
    chainId,
    hasMetaMask,
    hasPhantom,
    connectMetaMask,
    connectPhantom,
    disconnect,
    clearError,
  } = useWalletConnection();

  const [showDropdown, setShowDropdown] = useState(false);
  const [showWalletOptions, setShowWalletOptions] = useState(false);
  const [copied, setCopied] = useState(false);

  const copyAddress = () => {
    if (address) {
      navigator.clipboard.writeText(address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  // Connected state - show address and dropdown
  if (isConnected && address) {
    return (
      <div className="relative">
        <button
          onClick={() => setShowDropdown(!showDropdown)}
          className={clsx(
            'flex items-center gap-2 px-4 py-2 rounded-xl',
            'bg-[#0a0a0a] border border-[#1a1a1a]',
            'hover:border-orange-500/30 transition-all',
            className
          )}
        >
          <div className={clsx(
            'w-2 h-2 rounded-full',
            walletType === 'metamask' ? 'bg-orange-500' : 'bg-purple-500'
          )} />
          <span className="text-white font-mono text-sm">
            {formatAddress(address)}
          </span>
          <ChevronDown className={clsx(
            'w-4 h-4 text-gray-400 transition-transform',
            showDropdown && 'rotate-180'
          )} />
        </button>

        {showDropdown && (
          <>
            {/* Backdrop */}
            <div 
              className="fixed inset-0 z-40" 
              onClick={() => setShowDropdown(false)} 
            />
            
            {/* Dropdown */}
            <div className="absolute right-0 mt-2 w-64 z-50 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] shadow-2xl overflow-hidden">
              {/* Wallet info */}
              <div className="p-4 border-b border-[#1a1a1a]">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs text-gray-500">Connected with</span>
                  <span className={clsx(
                    'px-2 py-0.5 rounded text-xs font-medium',
                    walletType === 'metamask' 
                      ? 'bg-orange-500/20 text-orange-400' 
                      : 'bg-purple-500/20 text-purple-400'
                  )}>
                    {walletType === 'metamask' ? 'MetaMask' : 'Phantom'}
                  </span>
                </div>
                
                <button 
                  onClick={copyAddress}
                  className="flex items-center gap-2 w-full p-2 rounded-lg bg-[#111] hover:bg-[#1a1a1a] transition-colors"
                >
                  <span className="text-sm text-white font-mono flex-1 text-left truncate">
                    {address}
                  </span>
                  {copied ? (
                    <Check className="w-4 h-4 text-green-400" />
                  ) : (
                    <Copy className="w-4 h-4 text-gray-500" />
                  )}
                </button>

                {showChainInfo && chainId && walletType === 'metamask' && (
                  <div className="mt-2 text-xs text-gray-500">
                    Network: {getChainName(chainId)}
                  </div>
                )}
              </div>

              {/* Actions */}
              <div className="p-2">
                <button
                  onClick={() => {
                    disconnect();
                    setShowDropdown(false);
                  }}
                  className="flex items-center gap-2 w-full px-3 py-2 rounded-lg text-red-400 hover:bg-red-500/10 transition-colors"
                >
                  <LogOut className="w-4 h-4" />
                  <span className="text-sm">Disconnect</span>
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    );
  }

  // Connecting state
  if (isConnecting) {
    return (
      <button
        disabled
        className={clsx(
          'flex items-center gap-2 px-4 py-2 rounded-xl',
          'bg-orange-500/20 border border-orange-500/30',
          'text-orange-400 cursor-wait',
          className
        )}
      >
        <div className="w-4 h-4 border-2 border-orange-400 border-t-transparent rounded-full animate-spin" />
        <span className="text-sm">Connecting...</span>
      </button>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="relative">
        <button
          onClick={() => setShowWalletOptions(true)}
          className={clsx(
            'flex items-center gap-2 px-4 py-2 rounded-xl',
            'bg-red-500/10 border border-red-500/30',
            'text-red-400 hover:bg-red-500/20 transition-all',
            className
          )}
        >
          <AlertCircle className="w-4 h-4" />
          <span className="text-sm">Connection Error</span>
          <button 
            onClick={(e) => { e.stopPropagation(); clearError(); }}
            className="ml-1 hover:text-red-300"
          >
            <X className="w-3 h-3" />
          </button>
        </button>
        
        {/* Error tooltip */}
        <div className="absolute right-0 mt-2 w-64 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-xs text-red-300">
          {error}
        </div>
      </div>
    );
  }

  // Disconnected state - show connect button
  return (
    <div className="relative">
      <button
        onClick={() => setShowWalletOptions(!showWalletOptions)}
        className={clsx(
          'flex items-center gap-2 px-4 py-2 rounded-xl',
          'bg-gradient-to-r from-orange-500 to-red-500',
          'text-white font-medium hover:opacity-90 transition-opacity',
          className
        )}
      >
        <Wallet className="w-4 h-4" />
        <span className="text-sm">Connect Wallet</span>
      </button>

      {showWalletOptions && (
        <>
          {/* Backdrop */}
          <div 
            className="fixed inset-0 z-40" 
            onClick={() => setShowWalletOptions(false)} 
          />
          
          {/* Wallet options dropdown */}
          <div className="absolute right-0 mt-2 w-64 z-50 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] shadow-2xl overflow-hidden">
            <div className="p-3 border-b border-[#1a1a1a]">
              <span className="text-xs text-gray-500">Select Wallet</span>
            </div>
            
            <div className="p-2 space-y-1">
              {/* MetaMask */}
              <button
                onClick={() => {
                  connectMetaMask();
                  setShowWalletOptions(false);
                }}
                disabled={!hasMetaMask}
                className={clsx(
                  'flex items-center gap-3 w-full px-3 py-3 rounded-lg transition-colors',
                  hasMetaMask 
                    ? 'hover:bg-[#111] cursor-pointer' 
                    : 'opacity-50 cursor-not-allowed'
                )}
              >
                <div className="w-8 h-8 rounded-lg bg-orange-500/20 flex items-center justify-center">
                  <span className="text-orange-400 text-xs font-bold">MM</span>
                </div>
                <div className="flex-1 text-left">
                  <div className="text-sm text-white font-medium">MetaMask</div>
                  <div className="text-xs text-gray-500">
                    {hasMetaMask ? 'EVM Wallet' : 'Not installed'}
                  </div>
                </div>
              </button>

              {/* Phantom */}
              <button
                onClick={() => {
                  connectPhantom();
                  setShowWalletOptions(false);
                }}
                disabled={!hasPhantom}
                className={clsx(
                  'flex items-center gap-3 w-full px-3 py-3 rounded-lg transition-colors',
                  hasPhantom 
                    ? 'hover:bg-[#111] cursor-pointer' 
                    : 'opacity-50 cursor-not-allowed'
                )}
              >
                <div className="w-8 h-8 rounded-lg bg-purple-500/20 flex items-center justify-center">
                  <span className="text-purple-400 text-xs font-bold">👻</span>
                </div>
                <div className="flex-1 text-left">
                  <div className="text-sm text-white font-medium">Phantom</div>
                  <div className="text-xs text-gray-500">
                    {hasPhantom ? 'SVM Wallet' : 'Not installed'}
                  </div>
                </div>
              </button>
            </div>

            {!hasMetaMask && !hasPhantom && (
              <div className="p-3 border-t border-[#1a1a1a]">
                <p className="text-xs text-gray-500 text-center">
                  No wallets detected. Install MetaMask or Phantom.
                </p>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
