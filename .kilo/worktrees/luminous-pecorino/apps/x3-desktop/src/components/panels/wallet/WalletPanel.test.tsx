import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import WalletPanel from './WalletPanel';
import { useWalletStore } from '@/stores/walletStore';
import { useApplicationStore } from '@/stores/applicationStore';
import { useWindowManager } from '@/hooks/useWindowManager';
import { useSocialStore } from '@/stores/socialStore';

// Mock the stores and hooks
vi.mock('@/stores/walletStore', () => ({
  useWalletStore: vi.fn(),
}));

vi.mock('@/stores/applicationStore', () => ({
  useApplicationStore: vi.fn(),
}));

vi.mock('@/hooks/useWindowManager', () => ({
  useWindowManager: vi.fn(),
}));

vi.mock('@/stores/socialStore', () => ({
  useSocialStore: vi.fn(),
}));

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock qrcode
vi.mock('qrcode', () => ({
  default: {
    toDataURL: vi.fn().mockResolvedValue('mock-qr-code'),
  },
}));

// Mock lucide-react icons to avoid heavy rendering
vi.mock('lucide-react', async () => {
  const actual = await vi.importActual('lucide-react');
  return {
    ...actual as any,
  };
});

describe('WalletPanel', () => {
  const mockSetActiveView = vi.fn();
  const mockDisconnect = vi.fn();
  const mockGenerateWallet = vi.fn();

  const defaultWalletState = {
    activeView: 'dashboard',
    setActiveView: mockSetActiveView,
    disconnect: mockDisconnect,
    universalWallet: null,
    isConnected: false,
    tokens: [
      { symbol: 'ETH', name: 'Ethereum', balance: 1, value: 2000, color: 'blue', network: 'evm' }
    ],
    transactions: [],
    addressBook: [],
    portfolioTokens: [],
    comits: [],
    gpuEarningEnabled: false,
    cpuEarningEnabled: false,
    phoneEarningEnabled: false,
    storageContributionEnabled: false,
    setGpuEarning: vi.fn(),
    setCpuEarning: vi.fn(),
    setPhoneEarning: vi.fn(),
    setStorageContribution: vi.fn(),
    generateWallet: mockGenerateWallet,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Improved mocking for Zustand stores to handle selectors
    const createMockStore = (state: any) => {
      return vi.fn((selector) => (selector ? selector(state) : state));
    };

    (useWalletStore as any).mockImplementation(createMockStore(defaultWalletState));
    (useApplicationStore as any).mockImplementation(createMockStore({ applications: [] }));
    (useWindowManager as any).mockReturnValue({ launch: vi.fn() });
    (useSocialStore as any).mockImplementation(createMockStore({ inbox: [], pendingRequests: [] }));
  });

  it('renders the setup view when disconnected', () => {
    render(<WalletPanel />);
    expect(screen.getByText(/DIGITAL COMMAND CENTER/i)).toBeDefined();
    expect(screen.getByText(/Initialize Swarm Wallet/i)).toBeDefined();
  });

  it('renders the dashboard when connected', () => {
    const connectedState = {
      ...defaultWalletState,
      isConnected: true,
      universalWallet: { evm_address: '0x123', evm_chain_count: 60000 },
    };
    (useWalletStore as any).mockImplementation((selector: any) => selector ? selector(connectedState) : connectedState);

    render(<WalletPanel />);
    expect(screen.getByText(/Net Worth/i)).toBeDefined();
    expect(screen.getByText(/Network Insights/i)).toBeDefined();
  });

  it('switches views when sidebar items are clicked', () => {
    const connectedState = {
      ...defaultWalletState,
      isConnected: true,
      universalWallet: { evm_address: '0x123', evm_chain_count: 60000 },
    };
    (useWalletStore as any).mockImplementation((selector: any) => selector ? selector(connectedState) : connectedState);

    render(<WalletPanel />);
    const securityBtn = screen.getByText('Security');
    fireEvent.click(securityBtn);
    expect(mockSetActiveView).toHaveBeenCalledWith('security');
  });

  it('shows the DApps view when selected', () => {
    const dappsState = {
      ...defaultWalletState,
      isConnected: true,
      activeView: 'dapps',
      universalWallet: { evm_address: '0x123', evm_chain_count: 60000 },
    };
    (useWalletStore as any).mockImplementation((selector: any) => selector ? selector(dappsState) : dappsState);
    (useApplicationStore as any).mockImplementation((selector: any) => selector ? selector({ applications: [] }) : { applications: [] });

    render(<WalletPanel />);
    expect(screen.getByText(/X3 App Ecosystem/i)).toBeDefined();
  });

  it('shows the Security view when selected', () => {
    const securityViewState = {
      ...defaultWalletState,
      isConnected: true,
      activeView: 'security',
      universalWallet: { evm_address: '0x123', evm_chain_count: 60000 },
    };
    (useWalletStore as any).mockImplementation((selector: any) => selector ? selector(securityViewState) : securityViewState);

    render(<WalletPanel />);
    expect(screen.getByText(/ENCLAVE FIREWALL/i)).toBeDefined();
  });

  it('handles disconnect action', () => {
    const connectedState = {
      ...defaultWalletState,
      isConnected: true,
      universalWallet: { evm_address: '0x123', evm_chain_count: 60000 },
    };
    (useWalletStore as any).mockImplementation((selector: any) => selector ? selector(connectedState) : connectedState);

    render(<WalletPanel />);
    const logoutBtn = screen.getByTestId('logout-btn');
    fireEvent.click(logoutBtn);
    expect(mockDisconnect).toHaveBeenCalled();
  });

  it('can trigger wallet generation', async () => {
    render(<WalletPanel />);
    const generateBtn = screen.getByText(/Initialize Swarm Wallet/i);
    fireEvent.click(generateBtn);
    await waitFor(() => {
      expect(mockGenerateWallet).toHaveBeenCalled();
    });
  });
});
