/// <reference types="vitest/globals" />
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import WalletPanel from '../WalletPanel';
import { vi } from 'vitest';

// Mock stores and hooks used by WalletPanel (use Vitest `vi` globals)
vi.mock('@/stores/walletStore', () => ({
  useWalletStore: vi.fn()
}));
vi.mock('@/stores/socialStore', () => ({
  useSocialStore: vi.fn(() => ({ inbox: [], pendingRequests: [], respondFriendRequest: vi.fn() }))
}));
vi.mock('@/stores/applicationStore', () => ({
  useApplicationStore: vi.fn(() => ({ applications: [] }))
}));
vi.mock('@/hooks/useWindowManager', () => ({
  useWindowManager: vi.fn(() => ({ launch: vi.fn() }))
}));

import * as walletStore from '@/stores/walletStore';

describe('History export UI', () => {
  const universalWallet = {
    evm_address: '0xDEADBEEF',
    evm_chain_count: 1,
    solana_address: 'So1anaAddr',
    substrate_address: '1abc',
    mnemonic: 'seed words',
    warning: 'none'
  };

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  test('Export disabled when no transactions selected', () => {
    (walletStore.useWalletStore as any).mockReturnValue({
      activeView: 'history',
      setActiveView: vi.fn(),
      disconnect: vi.fn(),
      universalWallet,
      evmChainCount: 1,
      setEvmChainCount: vi.fn(),
      generateWallet: vi.fn(),
      transactions: []
    });

    render(<WalletPanel />);

    const btn = screen.getByText('Export CSV');
    expect(btn).toBeDisabled();
  });

  test('Export creates CSV when transactions present', () => {
    const txs = [
      { id: 'tx1', type: 'receive', time: '2026-02-27T12:00:00Z', amount: '1.00', symbol: 'ETH', status: 'confirmed' }
    ];

    (walletStore.useWalletStore as any).mockReturnValue({
      activeView: 'history',
      setActiveView: vi.fn(),
      disconnect: vi.fn(),
      universalWallet,
      evmChainCount: 1,
      setEvmChainCount: vi.fn(),
      generateWallet: vi.fn(),
      transactions: txs
    });

    if (!(URL as any).createObjectURL) {
      (URL as any).createObjectURL = vi.fn(() => 'blob:fake');
    }
    if (!(URL as any).revokeObjectURL) {
      (URL as any).revokeObjectURL = vi.fn();
    }
    const createSpy = vi.spyOn(URL as any, 'createObjectURL').mockReturnValue('blob:fake');
    const revokeSpy = vi.spyOn(URL as any, 'revokeObjectURL').mockImplementation(() => {});
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    render(<WalletPanel />);

    const btn = screen.getByText('Export CSV');
    expect(btn).toBeEnabled();

    fireEvent.click(btn);

    expect(createSpy).toHaveBeenCalled();
    expect(clickSpy).toHaveBeenCalled();
  });
});
