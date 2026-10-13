/**
 * React Hooks for Substrate Data
 * 
 * SWR-based hooks for fetching and caching blockchain data
 */

import useSWR, { SWRConfiguration } from 'swr';
import useSWRSubscription from 'swr/subscription';
import type { SWRSubscriptionOptions } from 'swr/subscription';
import {
  getNetworkStats,
  getRecentBlocks,
  getBlock,
  getBlockExtrinsics,
  getRecentExtrinsics,
  getAccountInfo,
  getAuthorities,
  getAuthorizedAccounts,
  getCanonicalBalance,
  isAccountAuthorized,
  subscribeNewHeads,
  type NetworkStats,
  type BlockInfo,
  type ExtrinsicInfo,
  type AccountInfo,
  type ValidatorInfo,
} from '../lib/substrate';
import type { Header } from '@polkadot/types/interfaces';

// Default SWR configuration for blockchain data
const defaultConfig: SWRConfiguration = {
  refreshInterval: 0, // Don't auto-refresh, use subscriptions instead
  revalidateOnFocus: false,
  dedupingInterval: 2000,
  errorRetryCount: 3,
  errorRetryInterval: 5000,
};

// ============================================================================
// Network Hooks
// ============================================================================

/**
 * Hook to get network statistics
 */
export function useNetworkStats(config?: SWRConfiguration) {
  return useSWR<NetworkStats, Error>(
    'network-stats',
    () => getNetworkStats(),
    {
      ...defaultConfig,
      refreshInterval: 6000, // Refresh every block time
      ...config,
    }
  );
}

/**
 * Hook to subscribe to new block headers (real-time)
 */
export function useNewHeads() {
  return useSWRSubscription<Header, Error>(
    'new-heads',
    (key: string, { next }: SWRSubscriptionOptions<Header, Error>) => {
      let unsubscribe: (() => void) | null = null;
      
      subscribeNewHeads((header) => {
        next(null, header);
      }).then((unsub) => {
        unsubscribe = unsub;
      }).catch((error) => {
        next(error);
      });

      return () => {
        if (unsubscribe) unsubscribe();
      };
    }
  );
}

// ============================================================================
// Block Hooks
// ============================================================================

/**
 * Hook to get recent blocks
 */
export function useRecentBlocks(count: number = 10, config?: SWRConfiguration) {
  return useSWR<BlockInfo[], Error>(
    ['recent-blocks', count],
    () => getRecentBlocks(count),
    {
      ...defaultConfig,
      refreshInterval: 6000,
      ...config,
    }
  );
}

/**
 * Hook to get a specific block by number or hash
 */
export function useBlock(blockId: number | string | null, config?: SWRConfiguration) {
  return useSWR<BlockInfo | null, Error>(
    blockId ? ['block', blockId] : null,
    () => blockId ? getBlock(blockId) : null,
    {
      ...defaultConfig,
      revalidateOnFocus: false,
      ...config,
    }
  );
}

/**
 * Hook to get extrinsics from a specific block
 */
export function useBlockExtrinsics(
  blockId: number | string | null,
  config?: SWRConfiguration
) {
  return useSWR<ExtrinsicInfo[], Error>(
    blockId ? ['block-extrinsics', blockId] : null,
    () => blockId ? getBlockExtrinsics(blockId) : [],
    {
      ...defaultConfig,
      ...config,
    }
  );
}

/**
 * Hook to get recent extrinsics
 */
export function useRecentExtrinsics(count: number = 20, config?: SWRConfiguration) {
  return useSWR<ExtrinsicInfo[], Error>(
    ['recent-extrinsics', count],
    () => getRecentExtrinsics(count),
    {
      ...defaultConfig,
      refreshInterval: 6000,
      ...config,
    }
  );
}

// ============================================================================
// Account Hooks
// ============================================================================

/**
 * Hook to get account information
 */
export function useAccount(address: string | null, config?: SWRConfiguration) {
  return useSWR<AccountInfo | null, Error>(
    address ? ['account', address] : null,
    () => address ? getAccountInfo(address) : null,
    {
      ...defaultConfig,
      refreshInterval: 12000, // Refresh every 2 blocks
      ...config,
    }
  );
}

/**
 * Hook to check if an account is authorized
 */
export function useIsAuthorized(address: string | null, config?: SWRConfiguration) {
  return useSWR<boolean, Error>(
    address ? ['is-authorized', address] : null,
    () => address ? isAccountAuthorized(address) : false,
    {
      ...defaultConfig,
      ...config,
    }
  );
}

/**
 * Hook to get canonical balance for an account and asset
 */
export function useCanonicalBalance(
  account: string | null,
  assetId: number,
  config?: SWRConfiguration
) {
  return useSWR<string, Error>(
    account ? ['canonical-balance', account, assetId] : null,
    () => account ? getCanonicalBalance(account, assetId) : '0',
    {
      ...defaultConfig,
      refreshInterval: 12000,
      ...config,
    }
  );
}

// ============================================================================
// Validator Hooks
// ============================================================================

/**
 * Hook to get current authorities/validators
 */
export function useAuthorities(config?: SWRConfiguration) {
  return useSWR<ValidatorInfo[], Error>(
    'authorities',
    () => getAuthorities(),
    {
      ...defaultConfig,
      refreshInterval: 60000, // Authority changes are infrequent
      ...config,
    }
  );
}

/**
 * Hook to get authorized accounts
 */
export function useAuthorizedAccounts(config?: SWRConfiguration) {
  return useSWR<string[], Error>(
    'authorized-accounts',
    () => getAuthorizedAccounts(),
    {
      ...defaultConfig,
      refreshInterval: 30000,
      ...config,
    }
  );
}

// ============================================================================
// Utility Hooks
// ============================================================================

/**
 * Hook to format balance with proper decimals
 */
export function useFormattedBalance(
  balance: string | null,
  decimals: number = 18
): string {
  if (!balance) return '0';
  
  const balanceNum = BigInt(balance);
  const divisor = BigInt(10 ** decimals);
  const integerPart = balanceNum / divisor;
  const fractionalPart = balanceNum % divisor;
  
  const fractionalStr = fractionalPart.toString().padStart(decimals, '0').slice(0, 4);
  
  return `${integerPart.toLocaleString()}.${fractionalStr}`;
}

/**
 * Hook to format address with ellipsis
 */
export function useShortAddress(address: string | null, chars: number = 6): string {
  if (!address) return '';
  if (address.length <= chars * 2 + 3) return address;
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}
