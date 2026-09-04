/**
 * Graceful Wallet Connection Hook
 * 
 * Handles MetaMask and other wallet connections without throwing errors.
 * Wallets are only connected when explicitly requested by the user.
 */

'use client';

import { useState, useCallback, useEffect } from 'react';

export type WalletType = 'metamask' | 'phantom' | 'none';

export interface WalletState {
  // Connection state
  isConnected: boolean;
  isConnecting: boolean;
  error: string | null;
  
  // Wallet info
  walletType: WalletType;
  address: string | null;
  chainId: number | null;
  
  // Availability
  hasMetaMask: boolean;
  hasPhantom: boolean;
  
  // Actions
  connectMetaMask: () => Promise<void>;
  connectPhantom: () => Promise<void>;
  disconnect: () => void;
  clearError: () => void;
}

// Check if MetaMask is available (without triggering connection)
function checkMetaMaskAvailable(): boolean {
  if (typeof window === 'undefined') return false;
  return !!(window as any).ethereum?.isMetaMask;
}

// Check if Phantom is available
function checkPhantomAvailable(): boolean {
  if (typeof window === 'undefined') return false;
  return !!(window as any).solana?.isPhantom;
}

export function useWalletConnection(): WalletState {
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [walletType, setWalletType] = useState<WalletType>('none');
  const [address, setAddress] = useState<string | null>(null);
  const [chainId, setChainId] = useState<number | null>(null);
  const [hasMetaMask, setHasMetaMask] = useState(false);
  const [hasPhantom, setHasPhantom] = useState(false);

  // Check wallet availability on mount (without connecting)
  useEffect(() => {
    // Small delay to ensure extensions are injected
    const timer = setTimeout(() => {
      setHasMetaMask(checkMetaMaskAvailable());
      setHasPhantom(checkPhantomAvailable());
    }, 100);
    
    return () => clearTimeout(timer);
  }, []);

  // Listen for account changes (only if already connected)
  useEffect(() => {
    if (typeof window === 'undefined') return;
    
    const ethereum = (window as any).ethereum;
    if (!ethereum || !isConnected || walletType !== 'metamask') return;

    const handleAccountsChanged = (accounts: string[]) => {
      if (accounts.length === 0) {
        // User disconnected
        setIsConnected(false);
        setAddress(null);
        setWalletType('none');
      } else {
        setAddress(accounts[0]);
      }
    };

    const handleChainChanged = (chainIdHex: string) => {
      setChainId(parseInt(chainIdHex, 16));
    };

    ethereum.on('accountsChanged', handleAccountsChanged);
    ethereum.on('chainChanged', handleChainChanged);

    return () => {
      ethereum.removeListener('accountsChanged', handleAccountsChanged);
      ethereum.removeListener('chainChanged', handleChainChanged);
    };
  }, [isConnected, walletType]);

  // Connect to MetaMask (only when user explicitly requests)
  const connectMetaMask = useCallback(async () => {
    if (typeof window === 'undefined') {
      setError('Window not available');
      return;
    }

    const ethereum = (window as any).ethereum;
    if (!ethereum?.isMetaMask) {
      setError('MetaMask not installed. Please install MetaMask extension.');
      return;
    }

    setIsConnecting(true);
    setError(null);

    try {
      // Request account access
      const accounts = await ethereum.request({ 
        method: 'eth_requestAccounts' 
      });

      if (accounts.length > 0) {
        setAddress(accounts[0]);
        setIsConnected(true);
        setWalletType('metamask');

        // Get chain ID
        const chainIdHex = await ethereum.request({ method: 'eth_chainId' });
        setChainId(parseInt(chainIdHex, 16));
      }
    } catch (err: any) {
      // Handle user rejection gracefully
      if (err.code === 4001) {
        setError('Connection rejected. Please approve the connection in MetaMask.');
      } else if (err.code === -32002) {
        setError('Connection request pending. Please check MetaMask.');
      } else {
        setError(err.message || 'Failed to connect to MetaMask');
      }
      console.warn('[Wallet] MetaMask connection failed:', err.message);
    } finally {
      setIsConnecting(false);
    }
  }, []);

  // Connect to Phantom (only when user explicitly requests)
  const connectPhantom = useCallback(async () => {
    if (typeof window === 'undefined') {
      setError('Window not available');
      return;
    }

    const phantom = (window as any).solana;
    if (!phantom?.isPhantom) {
      setError('Phantom not installed. Please install Phantom wallet.');
      return;
    }

    setIsConnecting(true);
    setError(null);

    try {
      const response = await phantom.connect();
      setAddress(response.publicKey.toString());
      setIsConnected(true);
      setWalletType('phantom');
    } catch (err: any) {
      if (err.code === 4001) {
        setError('Connection rejected. Please approve the connection in Phantom.');
      } else {
        setError(err.message || 'Failed to connect to Phantom');
      }
      console.warn('[Wallet] Phantom connection failed:', err.message);
    } finally {
      setIsConnecting(false);
    }
  }, []);

  // Disconnect wallet
  const disconnect = useCallback(() => {
    if (walletType === 'phantom') {
      const phantom = (window as any).solana;
      if (phantom) {
        phantom.disconnect();
      }
    }
    // MetaMask doesn't have a disconnect method, just clear state
    
    setIsConnected(false);
    setAddress(null);
    setChainId(null);
    setWalletType('none');
    setError(null);
  }, [walletType]);

  // Clear error
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
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
  };
}

/**
 * Format wallet address for display
 */
export function formatAddress(address: string | null, chars = 4): string {
  if (!address) return '';
  return `${address.slice(0, chars + 2)}...${address.slice(-chars)}`;
}

/**
 * Get chain name from chain ID
 */
export function getChainName(chainId: number | null): string {
  if (!chainId) return 'Unknown';
  
  const chains: Record<number, string> = {
    1: 'Ethereum Mainnet',
    5: 'Goerli Testnet',
    11155111: 'Sepolia Testnet',
    137: 'Polygon',
    80001: 'Mumbai Testnet',
    42161: 'Arbitrum One',
    10: 'Optimism',
    8453: 'Base',
    // X3 X3 Chain chain IDs
    1337: 'X3 Local Dev',
    3333: 'X3 Testnet',
    3000: 'X3 Mainnet',
  };
  
  return chains[chainId] || `Chain ${chainId}`;
}
