import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

// ── Interfaces ──────────────────────────────────────────────────────────────

export interface Token {
  symbol: string;
  name: string;
  balance: number;
  value: number;
  change24h: number;
  icon: string;
  network: 'evm' | 'svm' | 'substrate';
  color?: string;
}

export interface Transaction {
  id: string;
  type: 'send' | 'receive' | 'swap' | 'comit' | 'mint';
  status: 'confirmed' | 'pending' | 'failed';
  amount: number;
  symbol: string;
  from: string;
  to: string;
  timestamp: number;
  time?: string;
  hash: string;
  network: 'evm' | 'svm' | 'substrate';
  comitId?: string;
  blockNumber?: number;
}

export interface WalletAccount {
  address: string;
  name: string;
  network: 'evm' | 'svm' | 'substrate' | 'universal';
  balance: string;
  isAuthorized?: boolean;
}

export type ActiveView =
  | 'dashboard'
  | 'send'
  | 'receive'
  | 'swap'
  | 'history'
  | 'settings'
  | 'comit'
  | 'mint'
  | 'portfolio'
  | 'addressBook'
  | 'security'
  | 'social'
  | 'earn'
  | 'dapps';

export interface UniversalWallet {
  mnemonic: string;
  seed_hex: string;
  evm_address: string;
  evm_private_key: string;
  solana_address: string;
  solana_private_key: string;
  substrate_address: string;
  evm_chain_count: number;
  warning: string;
}

export interface AddressBookContact {
  name: string;
  address: string;
  ens: string;
  color: string;
}

export interface PortfolioToken {
  name: string;
  symbol: string;
  supply: string;
  holders: number;
  network: string;
  tvl: number;
  color: string;
  apy: number;
}

export interface ComitTask {
  id: string;
  title: string;
  description: string;
  totalValue: string;
  runs: string;
  status: 'ACTIVE' | 'PAUSED';
  color: string;
  icon: string;
}

export interface WalletState {
  isConnected: boolean;
  isLoading: boolean;
  accounts: WalletAccount[];
  activeAccountIndex: number;
  totalBalance: number;
  tokens: Token[];
  transactions: Transaction[];
  pendingComits: string[];
  activeView: ActiveView;
  evmChainCount: number;
  universalWallet: UniversalWallet | null;
  addressBook: AddressBookContact[];
  portfolioTokens: PortfolioToken[];
  comits: ComitTask[];
  // contribution states
  gpuEarningEnabled: boolean;
  cpuEarningEnabled: boolean;
  phoneEarningEnabled: boolean;
  storageContributionEnabled: boolean;
}

export interface WalletActions {
  setConnected: (connected: boolean) => void;
  setLoading: (loading: boolean) => void;
  addAccount: (account: WalletAccount) => void;
  setAccounts: (accounts: WalletAccount[]) => void;
  setActiveAccountIndex: (index: number) => void;
  setActiveView: (view: ActiveView) => void;
  setTokens: (tokens: Token[]) => void;
  addTransaction: (tx: Transaction) => void;
  disconnect: () => void;
  setUniversalWallet: (wallet: UniversalWallet | null) => void;
  setEvmChainCount: (count: number) => void;
  addContact: (contact: AddressBookContact) => void;
  addPortfolioToken: (token: PortfolioToken) => void;
  addComit: (comit: ComitTask) => void;
  
  // contribution actions
  setGpuEarning: (enabled: boolean) => void;
  setCpuEarning: (enabled: boolean) => void;
  setPhoneEarning: (enabled: boolean) => void;
  setStorageContribution: (enabled: boolean) => void;
  
  // Async actions
  generateWallet: () => Promise<void>;
  importWallet: (mnemonic: string) => Promise<void>;
}

// ── Initial state ───────────────────────────────────────────────────────────

const initialState: WalletState = {
  isConnected: false,
  isLoading: false,
  accounts: [],
  activeAccountIndex: 0,
  totalBalance: 0,
  pendingComits: [],
  activeView: 'dashboard',
  evmChainCount: 0,
  universalWallet: null,
  tokens: [
    { symbol: 'X3', name: 'X3 Sphere', balance: 1250.0, value: 3750.0, change24h: 5.2, icon: '⭐', network: 'substrate', color: 'from-orange-500 to-yellow-500' },
    { symbol: 'ETH', name: 'Ethereum', balance: 2.45, value: 8304.50, change24h: -1.3, icon: '◆', network: 'evm', color: 'from-blue-500 to-indigo-500' },
    { symbol: 'SOL', name: 'Solana', balance: 15.8, value: 1580.0, change24h: 3.8, icon: '◎', network: 'svm', color: 'from-purple-500 to-pink-500' },
    { symbol: 'USDC', name: 'USD Coin', balance: 500.0, value: 500.0, change24h: 0.01, icon: '$', network: 'evm', color: 'from-green-500 to-emerald-500' },
  ],
  transactions: [
    { id: '1', type: 'receive', amount: 500, symbol: 'USDC', time: '10m ago', status: 'confirmed', from: '0x...', to: '0x...', timestamp: Date.now(), hash: '0x...', network: 'evm' },
    { id: '2', type: 'swap', amount: 2.5, symbol: 'ETH', time: '2h ago', status: 'confirmed', from: '0x...', to: '0x...', timestamp: Date.now(), hash: '0x...', network: 'evm' },
    { id: '3', type: 'send', amount: 125, symbol: 'X3', time: '1d ago', status: 'pending', from: '0x...', to: '0x...', timestamp: Date.now(), hash: '0x...', network: 'substrate' },
  ] as Transaction[],
  addressBook: [
    { name: 'Alice (Cold Storage)', address: '0x1F98...334A', ens: 'alice.eth', color: 'bg-blue-500/20 text-blue-400' },
    { name: 'Bob', address: '0x742D...2ABC', ens: 'bob.eth', color: 'bg-green-500/20 text-green-400' },
    { name: 'DAO Treasury', address: '0x882B...119A', ens: 'treasury.dao.eth', color: 'bg-orange-500/20 text-orange-400' }
  ],
  portfolioTokens: [
    { name: 'X3 Omni-Token', symbol: 'X3OMNI', supply: '100M', holders: 14205, network: 'all-evm', tvl: 4500000, color: 'from-blue-500 to-indigo-600', apy: 12.5 },
    { name: 'Pepe Sonic', symbol: 'PEPSO', supply: '1B', holders: 8200, network: 'svm', tvl: 850000, color: 'from-green-500 to-emerald-600', apy: 4.2 },
    { name: 'X3 Yield', symbol: 'ayATLAS', supply: '5M', holders: 2501, network: 'substrate', tvl: 12500000, color: 'from-orange-500 to-red-600', apy: 18.0 }
  ],
  comits: [
    { id: '1', title: 'DCA to USDC on ETH', description: 'Execute swap when ETH > $3,000', totalValue: '$5,000', runs: '5/10', status: 'ACTIVE', color: 'orange', icon: 'Zap' },
    { id: '2', title: 'Yield Harvest', description: 'Compound Curve rewards daily', totalValue: '2.5 ETH', runs: '125/∞', status: 'PAUSED', color: 'blue', icon: 'Shield' }
  ],
  gpuEarningEnabled: false,
  cpuEarningEnabled: false,
  phoneEarningEnabled: false,
  storageContributionEnabled: false,
};

// ── Store ───────────────────────────────────────────────────────────────────

export const useWalletStore = create<WalletState & WalletActions>()(
  persist(
    (set) => ({
      ...initialState,

      setConnected: (connected) => set({ isConnected: connected }),
      setLoading: (loading) => set({ isLoading: loading }),

      addAccount: (account) =>
        set((state) => ({ accounts: [...state.accounts, account] })),

      setAccounts: (accounts) => set({ accounts }),

      setActiveAccountIndex: (index) => set({ activeAccountIndex: index }),

      setActiveView: (view) => set({ activeView: view }),

      setTokens: (tokens) =>
        set({ tokens, totalBalance: tokens.reduce((s, t) => s + t.value, 0) }),

      addTransaction: (tx) =>
        set((state) => ({ transactions: [tx, ...state.transactions] })),

      disconnect: () =>
        set({
          isConnected: false,
          accounts: [],
          activeAccountIndex: 0,
          totalBalance: 0,
          tokens: [],
          transactions: [],
          pendingComits: [],
          activeView: 'dashboard',
          universalWallet: null,
        }),

      setUniversalWallet: (wallet) => set({ universalWallet: wallet }),

      setEvmChainCount: (count) => set({ evmChainCount: count }),

      addContact: (contact) => set((state) => ({ addressBook: [...state.addressBook, contact] })),
      addPortfolioToken: (token) => set((state) => ({ portfolioTokens: [...state.portfolioTokens, token] })),
      addComit: (comit) => set((state) => ({ comits: [...state.comits, comit] })),

      setGpuEarning: (enabled) => set({ gpuEarningEnabled: enabled }),
      setCpuEarning: (enabled) => set({ cpuEarningEnabled: enabled }),
      setPhoneEarning: (enabled) => set({ phoneEarningEnabled: enabled }),
      setStorageContribution: (enabled) => set({ storageContributionEnabled: enabled }),

      generateWallet: async () => {
        set({ isLoading: true });
        try {
          const wallet = await invoke<UniversalWallet>("generate_universal_wallet");
          set({ universalWallet: wallet, isConnected: true, evmChainCount: wallet.evm_chain_count });
        } catch (error) {
          console.error("Failed to generate wallet:", error);
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      importWallet: async (mnemonic: string) => {
        set({ isLoading: true });
        try {
          const wallet = await invoke<UniversalWallet>("import_universal_wallet", { mnemonic });
          set({ universalWallet: wallet, isConnected: true, evmChainCount: wallet.evm_chain_count });
        } catch (error) {
          console.error("Failed to import wallet:", error);
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },
    }),
    { name: 'x3-universal-wallet' },
  ),
);
