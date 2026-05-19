import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useWalletStore } from './walletStore';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('walletStore', () => {
  beforeEach(() => {
    // Reset store state
    const { disconnect } = useWalletStore.getState();
    disconnect();
    vi.clearAllMocks();
  });

  it('should initialize with default state', () => {
    const state = useWalletStore.getState();
    expect(state.isConnected).toBe(false);
    expect(state.activeAccountIndex).toBe(0);
  });

  it('should set connected state', () => {
    const { setConnected } = useWalletStore.getState();
    setConnected(true);
    expect(useWalletStore.getState().isConnected).toBe(true);
  });

  it('should add a transaction', () => {
    const { addTransaction } = useWalletStore.getState();
    const mockTx = {
      id: 'tx1',
      type: 'send' as const,
      status: 'confirmed' as const,
      amount: 10,
      symbol: 'ETH',
      from: '0x1',
      to: '0x2',
      timestamp: Date.now(),
      hash: '0xhash',
      network: 'evm' as const,
    };

    addTransaction(mockTx);
    expect(useWalletStore.getState().transactions[0]).toEqual(mockTx);
  });

  it('should generate a wallet using Tauri invoke', async () => {
    const mockWallet = {
      mnemonic: 'test mnemonic',
      seed_hex: 'seed',
      evm_address: '0x123',
      evm_private_key: '0xabc',
      solana_address: 'sol123',
      solana_private_key: 'solabc',
      substrate_address: 'sub123',
      evm_chain_count: 60000,
      warning: 'test warning',
    };

    (invoke as any).mockResolvedValue(mockWallet);

    const { generateWallet } = useWalletStore.getState();
    await generateWallet();

    const state = useWalletStore.getState();
    expect(state.universalWallet).toEqual(mockWallet);
    expect(state.isConnected).toBe(true);
    expect(state.isLoading).toBe(false);
    expect(invoke).toHaveBeenCalledWith('generate_universal_wallet');
  });

  it('should import a wallet using Tauri invoke', async () => {
    const mockWallet = {
      mnemonic: 'test mnemonic',
      seed_hex: 'seed',
      evm_address: '0x123',
      evm_private_key: '0xabc',
      solana_address: 'sol123',
      solana_private_key: 'solabc',
      substrate_address: 'sub123',
      evm_chain_count: 60000,
      warning: 'test warning',
    };

    (invoke as any).mockResolvedValue(mockWallet);

    const { importWallet } = useWalletStore.getState();
    const mnemonic = 'test mnemonic';
    await importWallet(mnemonic);

    const state = useWalletStore.getState();
    expect(state.universalWallet).toEqual(mockWallet);
    expect(state.isConnected).toBe(true);
    expect(invoke).toHaveBeenCalledWith('import_universal_wallet', { mnemonic });
  });

  it('should handle disconnect', () => {
    const { setConnected, setUniversalWallet, disconnect } = useWalletStore.getState();
    setConnected(true);
    setUniversalWallet({ mnemonic: 'test' } as any);
    
    disconnect();
    
    const state = useWalletStore.getState();
    expect(state.isConnected).toBe(false);
    expect(state.universalWallet).toBeNull();
    expect(state.tokens).toEqual([]);
    expect(state.activeView).toBe('dashboard');
  });

  it('should update earning states', () => {
    const { setGpuEarning, setCpuEarning } = useWalletStore.getState();
    
    setGpuEarning(true);
    expect(useWalletStore.getState().gpuEarningEnabled).toBe(true);
    
    setCpuEarning(true);
    expect(useWalletStore.getState().cpuEarningEnabled).toBe(true);
  });

  it('should switch active view', () => {
    const { setActiveView } = useWalletStore.getState();
    setActiveView('security');
    expect(useWalletStore.getState().activeView).toBe('security');
  });
});
