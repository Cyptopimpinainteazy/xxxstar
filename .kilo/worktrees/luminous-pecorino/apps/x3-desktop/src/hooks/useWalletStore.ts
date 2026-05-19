/**
 * walletStore - Tauri Desktop Core Wallet Store Integration
 *
 * This module provides secure wallet storage functionality with:
 * - Local storage persistence using Tauri's store plugin
 * - Encryption for sensitive data (mnemonics, private keys)
 * - Wallet recovery functionality
 * - Multi-chain wallet support
 */

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface WalletAccount {
  chain: string;
  address: string;
  publicKey: string;
  derivationPath: string;
  isDefault: boolean;
}

export interface WalletMetadata {
  id: string;
  name: string;
  createdAt: number;
  lastUsed: number;
  chainCount: number;
  isEncrypted: boolean;
}

export interface EncryptedWalletData {
  encryptedMnemonic: string;
  encryptedSeed: string;
  iv: string;
  salt: string;
  derivationPath: string;
  createdAt: number;
}

// ── Hook ──────────────────────────────────────────────────────────────────────

export function useWalletStore() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [walletCount, setWalletCount] = useState(0);
  const [activeWalletId, setActiveWalletId] = useState<string | null>(null);
  const [wallets, setWallets] = useState<Record<string, EncryptedWalletData>>({});
  const [accounts, setAccounts] = useState<Record<string, WalletAccount[]>>({});

  // Initialize the wallet store
  useEffect(() => {
    const initStore = async () => {
      try {
        // In production, this would initialize the wallet store
        setIsInitialized(true);
        console.log('[useWalletStore] Wallet store initialized');
      } catch (error) {
        console.error('[useWalletStore] Initialization failed:', error);
      }
    };

    initStore();
  }, []);

  // Store a wallet
  const storeWallet = useCallback(
    async (
      walletId: string,
      mnemonic: string,
      seed: string,
      derivationPath: string,
    ): Promise<void> => {
      try {
        await invoke('store_wallet_encrypted', {
          walletId,
          mnemonic,
          seed,
          derivationPath,
        });

        setWalletCount((prev) => prev + 1);
        console.log(`[useWalletStore] Wallet ${walletId} stored`);
      } catch (error) {
        console.error(`[useWalletStore] Failed to store wallet ${walletId}:`, error);
        throw error;
      }
    },
    [],
  );

  // Retrieve a wallet
  const retrieveWallet = useCallback(
    async (walletId: string): Promise<{ mnemonic: string; seed: string }> => {
      try {
        const result = await invoke<string>('retrieve_wallet_encrypted', {
          walletId,
        });

        const parsed = JSON.parse(result);
        return {
          mnemonic: parsed.mnemonic,
          seed: parsed.seed,
        };
      } catch (error) {
        console.error(`[useWalletStore] Failed to retrieve wallet ${walletId}:`, error);
        throw error;
      }
    },
    [],
  );

  // Delete a wallet
  const deleteWallet = useCallback(
    async (walletId: string): Promise<void> => {
      try {
        await invoke('delete_wallet', { walletId });

        setWalletCount((prev) => Math.max(0, prev - 1));
        if (activeWalletId === walletId) {
          setActiveWalletId(null);
        }

        console.log(`[useWalletStore] Wallet ${walletId} deleted`);
      } catch (error) {
        console.error(`[useWalletStore] Failed to delete wallet ${walletId}:`, error);
        throw error;
      }
    },
    [activeWalletId],
  );

  // Export wallet backup
  const exportBackup = useCallback(
    async (walletId: string): Promise<string> => {
      try {
        const result = await invoke<string>('export_wallet_backup', { walletId });
        return result;
      } catch (error) {
        console.error(`[useWalletStore] Failed to export backup for ${walletId}:`, error);
        throw error;
      }
    },
    [],
  );

  // Import wallet from backup
  const importBackup = useCallback(
    async (backup: string): Promise<string> => {
      try {
        const result = await invoke<string>('import_wallet_backup', { backup });
        setWalletCount((prev) => prev + 1);
        return result;
      } catch (error) {
        console.error('[useWalletStore] Failed to import backup:', error);
        throw error;
      }
    },
    [],
  );

  // Set active wallet
  const setActiveWallet = useCallback(
    async (walletId: string): Promise<void> => {
      try {
        setActiveWalletId(walletId);
        console.log(`[useWalletStore] Active wallet set to ${walletId}`);
      } catch (error) {
        console.error(`[useWalletStore] Failed to set active wallet ${walletId}:`, error);
        throw error;
      }
    },
    [],
  );

  // Get wallet metadata
  const getWalletMetadata = useCallback(
    async (walletId: string): Promise<WalletMetadata> => {
      try {
        // In production, this would fetch metadata from the backend
        return {
          id: walletId,
          name: `Wallet ${walletId.substring(0, 8)}`,
          createdAt: Date.now(),
          lastUsed: Date.now(),
          chainCount: 1,
          isEncrypted: true,
        };
      } catch (error) {
        console.error(`[useWalletStore] Failed to get metadata for ${walletId}:`, error);
        throw error;
      }
    },
    [],
  );

  // Add account to wallet
  const addAccount = useCallback(
    async (walletId: string, account: WalletAccount): Promise<void> => {
      try {
        setAccounts((prev) => ({
          ...prev,
          [walletId]: [...(prev[walletId] || []), account],
        }));
        console.log(`[useWalletStore] Account added to wallet ${walletId}`);
      } catch (error) {
        console.error(`[useWalletStore] Failed to add account to ${walletId}:`, error);
        throw error;
      }
    },
    [],
  );

  // Get accounts for wallet
  const getAccounts = useCallback(
    async (walletId: string): Promise<WalletAccount[]> => {
      try {
        return accounts[walletId] || [];
      } catch (error) {
        console.error(`[useWalletStore] Failed to get accounts for ${walletId}:`, error);
        throw error;
      }
    },
    [accounts],
  );

  return {
    isInitialized,
    walletCount,
    activeWalletId,
    wallets,
    accounts,
    storeWallet,
    retrieveWallet,
    deleteWallet,
    exportBackup,
    importBackup,
    setActiveWallet,
    getWalletMetadata,
    addAccount,
    getAccounts,
  };
}

// ── Hook for active wallet operations ─────────────────────────────────────────

export function useActiveWallet() {
  const {
    activeWalletId,
    setActiveWallet,
    retrieveWallet,
    exportBackup,
    getWalletMetadata,
  } = useWalletStore();

  const [activeWalletData, setActiveWalletData] = useState<{
    mnemonic: string;
    seed: string;
  } | null>(null);

  // Load active wallet data
  useEffect(() => {
    const loadActiveWallet = async () => {
      if (activeWalletId) {
        try {
          const data = await retrieveWallet(activeWalletId);
          setActiveWalletData(data);
        } catch (error) {
          console.error('[useActiveWallet] Failed to load active wallet:', error);
        }
      }
    };

    loadActiveWallet();
  }, [activeWalletId, retrieveWallet]);

  return {
    activeWalletId,
    activeWalletData,
    setActiveWallet,
    exportBackup,
    getWalletMetadata,
  };
}
