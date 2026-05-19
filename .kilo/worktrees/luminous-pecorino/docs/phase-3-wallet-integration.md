# Phase 3: Wallet Integration - Implementation Guide

## Overview

Phase 3 implements comprehensive wallet management functionality across the X3 Chain ecosystem, including:

1. **Wallet Panels** - Enhanced UI components for wallet management
2. **Wallet Service API** - Backend RPC endpoints for wallet operations
3. **Wallet App** - Standalone web wallet application

## Implementation Summary

### 1. Wallet Panels (`apps/x3-desktop/src/components/panels/wallet/`)

#### New Components Created

**WalletPanels.tsx** - Core wallet panel components:
- `NetworkSelectorPanel` - Network selection (mainnet/testnet/local)
- `WalletDashboardPanel` - Main dashboard with balance and quick actions
- `WalletTransactionsPanel` - Transaction history and management
- `WalletSettingsPanel` - Wallet configuration and security settings

#### Features

- **Multi-network Support**: Switch between mainnet, testnet, and local devnet
- **Balance Display**: Real-time token balances with USD values
- **Quick Actions**: Send, Receive, Swap, and History access
- **Transaction History**: Filterable transaction list with status badges
- **Security Settings**: PIN protection, biometric auth, hardware wallet support
- **Backup & Recovery**: Mnemonic phrase display and export options

### 2. Wallet Service API (`crates/x3-rpc/src/wallet_service_rpc.rs`)

#### New RPC Methods

| Method | Description |
|--------|-------------|
| `wallet_createWallet` | Create a new wallet with optional mnemonic |
| `wallet_importWallet` | Import existing wallet from mnemonic |
| `wallet_backupWallet` | Backup wallet data with encryption |
| `wallet_getBalance` | Get wallet balance for all tokens |
| `wallet_signTransaction` | Sign transaction with wallet |
| `wallet_submitTransaction` | Submit signed transaction to network |
| `wallet_getTransactions` | Get transaction history |
| `wallet_getWalletStatus` | Get wallet connection status |
| `wallet_listWallets` | List all wallets |
| `wallet_setNetwork` | Set network for wallet |
| `wallet_getNetworks` | Get available networks |

#### Request/Response Types

```rust
// Example: CreateWalletRequest
pub struct CreateWalletRequest {
    pub wallet_name: String,
    pub password_hash: String,
    pub mnemonic: Option<String>,
    pub network: String,
}

// Example: CreateWalletResponse
pub struct CreateWalletResponse {
    pub wallet_id: String,
    pub address: String,
    pub mnemonic: Option<String>,
    pub created_at: u64,
}
```

#### Integration

The wallet service API is integrated into the node RPC module in `node/src/rpc.rs`:

```rust
// Initialize Wallet Service RPC
let wallet_service = Arc::new(WalletServiceRpc::<Block, FullClient>::new(client.clone()));

// Register wallet service RPC methods
module.register_method("wallet_createWallet", {
    let wallet_service = wallet_service.clone();
    move |params: serde_json::Value, _| {
        // Implementation
    }
})?;
```

### 3. Wallet App (`apps/wallet/src/app/page.tsx`)

#### Standalone Web Wallet

The wallet app provides a complete web-based interface for wallet management:

**Features:**
- Dashboard with total balance and token breakdown
- Send assets to any address
- Receive assets with QR code
- Swap tokens with real-time rates
- Transaction history with filtering
- Settings with network and security options

**Navigation:**
- Dashboard
- Send
- Receive
- Swap
- History
- Settings

#### Integration with Wallet Store

**walletAppStore.ts** - Connects wallet app with existing Zustand store:

```typescript
export function useWalletAppStore() {
  // State from existing wallet store
  const { isConnected, tokens, transactions, universalWallet, ... } = useWalletStore();
  
  // Actions
  const { generateWallet, importWallet, disconnect, ... } = useWalletStore();
  
  // App-specific actions
  const connectWallet = useCallback(async () => { ... });
  const disconnectWallet = useCallback(async () => { ... });
  const signTransaction = useCallback(async (txData: string) => { ... });
  const submitTransaction = useCallback(async (signedTx: string) => { ... });
  const switchNetwork = useCallback(async (network: string) => { ... });
}
```

### 4. Wallet Service Client (`apps/x3-desktop/src/services/walletService.ts`)

#### TypeScript Client

```typescript
export class WalletServiceClient {
  async createWallet(request: CreateWalletRequest): Promise<CreateWalletResponse>;
  async importWallet(request: ImportWalletRequest): Promise<CreateWalletResponse>;
  async backupWallet(request: BackupWalletRequest): Promise<BackupWalletResponse>;
  async getBalance(request: GetBalanceRequest): Promise<GetBalanceResponse>;
  async signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse>;
  async submitTransaction(request: SubmitTransactionRequest): Promise<SubmitTransactionResponse>;
  async getTransactions(request: GetTransactionsRequest): Promise<GetTransactionsResponse>;
  async getWalletStatus(request: GetWalletStatusRequest): Promise<GetWalletStatusResponse>;
  async listWallets(request: ListWalletsRequest): Promise<ListWalletsResponse>;
  async setNetwork(request: SetNetworkRequest): Promise<SetNetworkResponse>;
  async getNetworks(): Promise<NetworkConfig[]>;
}
```

## Architecture

### Frontend Architecture

```
apps/wallet/
├── src/
│   ├── app/
│   │   ├── page.tsx              # Main wallet app (Next.js)
│   │   └── layout.tsx
│   └── lib/
│       └── walletAppStore.ts     # Wallet store integration
└── package.json

apps/x3-desktop/
└── src/
    ├── components/panels/wallet/
    │   └── WalletPanels.tsx      # Wallet panel components
    └── services/
        └── walletService.ts      # Wallet service client
```

### Backend Architecture

```
crates/x3-rpc/
└── src/
    ├── lib.rs                    # RPC module exports
    ├── wallet_dex_rpc.rs         # Existing DEX RPC
    └── wallet_service_rpc.rs     # New wallet service RPC

node/
└── src/
    └── rpc.rs                    # Node RPC wiring
```

## Usage

### Frontend Usage

#### Wallet Panels

```tsx
import { WalletDashboardPanel, WalletTransactionsPanel, WalletSettingsPanel } from '@/components/panels/wallet/WalletPanels';

// In your component
<WalletDashboardPanel className="p-6" />
<WalletTransactionsPanel className="p-6" />
<WalletSettingsPanel className="p-6" />
```

#### Wallet Service Client

```typescript
import { walletService } from '@/services/walletService';

// Create wallet
const response = await walletService.createWallet({
  walletName: 'My Wallet',
  passwordHash: 'hashed_password',
  network: 'mainnet',
});

// Get balance
const balance = await walletService.getBalance({
  walletId: 'wallet_123',
  network: 'mainnet',
});
```

### Backend Usage

The wallet service API is automatically available through the node's JSON-RPC interface.

#### Example RPC Call

```bash
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "wallet_createWallet",
    "params": [{
      "walletName": "My Wallet",
      "passwordHash": "hashed_password",
      "network": "mainnet"
    }],
    "id": 1
  }'
```

## Security Considerations

1. **Password Hashing**: Passwords should be hashed before sending to the backend
2. **Mnemonic Security**: Mnemonics should never be stored in plain text
3. **Transaction Signing**: Transactions should be signed locally before submission
4. **Network Selection**: Users should verify network before signing transactions
5. **Backup Security**: Wallet backups should be encrypted with user password

## Testing

### Manual Testing

1. **Wallet Panels**:
   - Open X3 Desktop
   - Navigate to Wallet panel
   - Test network switching
   - Test balance display
   - Test transaction history

2. **Wallet App**:
   - Navigate to `/wallet` route
   - Test send/receive/swap flows
   - Test transaction history
   - Test settings

3. **RPC Endpoints**:
   - Test `wallet_createWallet`
   - Test `wallet_getBalance`
   - Test `wallet_signTransaction`
   - Test `wallet_submitTransaction`

### Automated Testing

```typescript
// Example test for wallet service
describe('WalletService', () => {
  it('should create wallet', async () => {
    const response = await walletService.createWallet({
      walletName: 'Test Wallet',
      passwordHash: 'test_hash',
      network: 'testnet',
    });
    expect(response.walletId).toBeDefined();
    expect(response.address).toBeDefined();
  });
});
```

## Future Enhancements

1. **Hardware Wallet Support**: Add Ledger and Trezor integration
2. **Biometric Auth**: Implement fingerprint/face ID support
3. **Multi-signature Wallets**: Add support for multi-sig wallets
4. **DApp Integration**: Connect with decentralized applications
5. **Staking UI**: Add staking interface for token staking
6. **Governance UI**: Add voting interface for governance proposals

## Migration Guide

### From Phase 2 to Phase 3

1. **Update Dependencies**:
   ```bash
   npm install @x3-chain/wallet-service
   ```

2. **Update Wallet Store**:
   - Import new wallet service client
   - Add sync logic for backend state

3. **Update UI Components**:
   - Replace old wallet components with new `WalletPanels`
   - Update navigation to use new wallet app routes

4. **Update RPC Calls**:
   - Replace old RPC calls with new wallet service API
   - Update request/response types

## Support

For issues or questions:
- Check the X3 Chain documentation
- Join the X3 Chain Discord
- File an issue on GitHub
