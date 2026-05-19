/**
 * React Hooks for Substrate Data (ported from explorer)
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
  fetchRpcStats,
  type RealRpcStats,
} from '@/lib/substrate';
import type { Header } from '@polkadot/types/interfaces';

const defaultConfig: SWRConfiguration = {
  refreshInterval: 0,
  revalidateOnFocus: false,
  dedupingInterval: 2000,
  errorRetryCount: 3,
  errorRetryInterval: 5000,
};

export function useNetworkStats(config?: SWRConfiguration) {
  return useSWR<NetworkStats, Error>('network-stats', () => getNetworkStats(), {
    ...defaultConfig,
    refreshInterval: 6000,
    ...config,
  });
}

export function useNewHeads() {
  return useSWRSubscription<Header, Error>('new-heads', (_key: string, { next }: SWRSubscriptionOptions<Header, Error>) => {
    let unsubscribe: (() => void) | null = null;
    subscribeNewHeads((header) => next(null, header))
      .then((unsub) => { unsubscribe = unsub; })
      .catch((error) => next(error));
    return () => { if (unsubscribe) unsubscribe(); };
  });
}

export function useRecentBlocks(count = 10, config?: SWRConfiguration) {
  return useSWR<BlockInfo[], Error>(['recent-blocks', count], () => getRecentBlocks(count), {
    ...defaultConfig,
    refreshInterval: 6000,
    ...config,
  });
}

export function useBlock(blockId: number | string | null, config?: SWRConfiguration) {
  return useSWR<BlockInfo | null, Error>(
    blockId ? ['block', blockId] : null,
    () => (blockId ? getBlock(blockId) : null),
    { ...defaultConfig, revalidateOnFocus: false, ...config },
  );
}

export function useBlockExtrinsics(blockId: number | string | null, config?: SWRConfiguration) {
  return useSWR<ExtrinsicInfo[], Error>(
    blockId ? ['block-extrinsics', blockId] : null,
    () => (blockId ? getBlockExtrinsics(blockId) : []),
    { ...defaultConfig, ...config },
  );
}

export function useRecentExtrinsics(count = 20, config?: SWRConfiguration) {
  return useSWR<ExtrinsicInfo[], Error>(['recent-extrinsics', count], () => getRecentExtrinsics(count), {
    ...defaultConfig,
    refreshInterval: 6000,
    ...config,
  });
}

export function useAccount(address: string | null, config?: SWRConfiguration) {
  return useSWR<AccountInfo | null, Error>(
    address ? ['account', address] : null,
    () => (address ? getAccountInfo(address) : null),
    { ...defaultConfig, refreshInterval: 12000, ...config },
  );
}

export function useIsAuthorized(address: string | null, config?: SWRConfiguration) {
  return useSWR<boolean, Error>(
    address ? ['is-authorized', address] : null,
    () => (address ? isAccountAuthorized(address) : false),
    { ...defaultConfig, ...config },
  );
}

export function useCanonicalBalance(account: string | null, assetId: number, config?: SWRConfiguration) {
  return useSWR<string, Error>(
    account ? ['canonical-balance', account, assetId] : null,
    () => (account ? getCanonicalBalance(account, assetId) : '0'),
    { ...defaultConfig, refreshInterval: 12000, ...config },
  );
}

export function useAuthorities(config?: SWRConfiguration) {
  return useSWR<ValidatorInfo[], Error>('authorities', () => getAuthorities(), {
    ...defaultConfig,
    refreshInterval: 60000,
    ...config,
  });
}

export function useAuthorizedAccounts(config?: SWRConfiguration) {
  return useSWR<string[], Error>('authorized-accounts', () => getAuthorizedAccounts(), {
    ...defaultConfig,
    refreshInterval: 30000,
    ...config,
  });
}

export function useFormattedBalance(balance: string | null, decimals = 18): string {
  if (!balance) return '0';
  const balanceNum = BigInt(balance);
  const divisor = BigInt(10 ** decimals);
  const integerPart = balanceNum / divisor;
  const fractionalPart = balanceNum % divisor;
  return `${integerPart.toLocaleString()}.${fractionalPart.toString().padStart(decimals, '0').slice(0, 4)}`;
}

export function useShortAddress(address: string | null, chars = 6): string {
  if (!address) return '';
  if (address.length <= chars * 2 + 3) return address;
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}

export function useRpcStats(config?: SWRConfiguration) {
  return useSWR<RealRpcStats | null, Error>('rpc-stats', () => fetchRpcStats(), {
    ...defaultConfig,
    refreshInterval: 5000,
    ...config,
  });
}

// ── Governance Hooks ───────────────────────────────────────────────────────────

import {
  getGovernanceSnapshot,
  getProposalList,
  getProposalTally,
  getDelegation,
  getAIProposals,
  getTopDelegates,
  type GovernanceSnapshot,
  type GovernanceProposal,
  type Delegation,
} from '@/lib/substrate';

export function useGovernanceSnapshot(config?: SWRConfiguration) {
  return useSWR<GovernanceSnapshot | null, Error>('governance-snapshot', () => getGovernanceSnapshot(), {
    ...defaultConfig,
    refreshInterval: 12000,
    ...config,
  });
}

export function useProposalList(config?: SWRConfiguration) {
  return useSWR<GovernanceProposal[], Error>('proposal-list', () => getProposalList(), {
    ...defaultConfig,
    refreshInterval: 6000,
    ...config,
  });
}

export function useProposalTally(proposalId: number | null, config?: SWRConfiguration) {
  return useSWR<{ ayes: number; nays: number; total: number } | null, Error>(
    proposalId ? ['proposal-tally', proposalId] : null,
    () => (proposalId ? getProposalTally(proposalId) : null),
    { ...defaultConfig, refreshInterval: 6000, ...config },
  );
}

export function useDelegation(address: string | null, config?: SWRConfiguration) {
  return useSWR<Delegation | null, Error>(
    address ? ['delegation', address] : null,
    () => (address ? getDelegation(address) : null),
    { ...defaultConfig, refreshInterval: 30000, ...config },
  );
}

export function useAIProposals(config?: SWRConfiguration) {
  return useSWR<GovernanceProposal[], Error>('ai-proposals', () => getAIProposals(), {
    ...defaultConfig,
    refreshInterval: 12000,
    ...config,
  });
}

export function useTopDelegates(count = 10, config?: SWRConfiguration) {
  return useSWR<{ address: string; power: string }[], Error>(
    ['top-delegates', count],
    () => getTopDelegates(count),
    { ...defaultConfig, refreshInterval: 60000, ...config },
  );
}

// ── Treasury Hooks ─────────────────────────────────────────────────────────────

import {
  getTreasurySnapshot,
  getTreasuryBalance,
  getTreasuryWallets,
  type TreasurySnapshot,
  type TreasuryWallet,
} from '@/lib/substrate';

export function useTreasurySnapshot(config?: SWRConfiguration) {
  return useSWR<TreasurySnapshot | null, Error>('treasury-snapshot', () => getTreasurySnapshot(), {
    ...defaultConfig,
    refreshInterval: 12000,
    ...config,
  });
}

export function useTreasuryBalance(config?: SWRConfiguration) {
  return useSWR<string, Error>('treasury-balance', () => getTreasuryBalance(), {
    ...defaultConfig,
    refreshInterval: 12000,
    ...config,
  });
}

export function useTreasuryWallets(config?: SWRConfiguration) {
  return useSWR<TreasuryWallet[], Error>('treasury-wallets', () => getTreasuryWallets(), {
    ...defaultConfig,
    refreshInterval: 12000,
    ...config,
  });
}

// ── My Votes Hook ──────────────────────────────────────────────────────────────

export function useMyVotes(address: string | null, config?: SWRConfiguration) {
  return useSWR<any[], Error>(
    address ? ['my-votes', address] : null,
    async () => {
      if (!address) return [];
      const api = await (await import('@/lib/substrate')).getApi();
      try {
        const entries = await api.query.governance.votes.entries(address);
        return entries.map(([key, value]: any) => ({
          proposalId: key.args[0].toNumber(),
          vote: value,
        }));
      } catch (e) {
        console.error('Error fetching my votes:', e);
        return [];
      }
    },
    { ...defaultConfig, refreshInterval: 12000, ...config },
  );
}
