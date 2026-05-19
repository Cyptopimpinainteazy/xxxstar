# X3 Mobile SDK Setup Guide

## Quick Start

The X3 Mobile SDK provides seamless integration of blockchain functionality into React Native and native mobile applications.

### Prerequisites

- Node.js 18+
- React Native CLI v0.71+
- iOS: Xcode 14.0+ (Apple Silicon compatible)
- Android: Android Studio with API level 24+
- Rust toolchain installed

### Installation

```bash
# Add to your React Native project
npm install @x3/mobile-sdk

# Or with yarn
yarn add @x3/mobile-sdk

# Link native modules
react-native link @x3/mobile-sdk
```

### Basic Setup

```typescript
import { MobileWallet, BiometricAuth, MobileTransactionSigner } from '@x3/mobile-sdk';

// Initialize SDK
const wallet = await MobileWallet.initializeAsync({
    platform: 'ios', // or 'android'
    networkId: 'mainnet',
    rpcEndpoint: 'https://api.x3.chain/rpc'
});

// Enable biometric authentication
const biometrics = new BiometricAuth(wallet);
await biometrics.enrollBiometric('face');

// Create transaction signer
const signer = new MobileTransactionSigner(wallet);
```

## Core Components

### 1. Mobile Wallet

The core wallet engine providing account management and balance tracking.

```typescript
// Import seed phrase
const wallet = await MobileWallet.importFromSeed(
    seedPhrase,
    "m/44'/60'/0'/0/0" // EVM derivation path
);

// Get account
const account = wallet.getAccount(0);
console.log('Address:', account.address);
console.log('Balance:', account.balance);

// Fetch latest balance from chain
const freshBalance = await wallet.fetchBalance(account.address);
```

**Key Methods:**
- `importFromSeed(seedPhrase, derivationPath)` — Import wallet from BIP-39 mnemonic
- `getAccount(index)` — Get account at index
- `fetchBalance(address)` — Get current balance from RPC
- `trackTransaction()` — Subscribe to transaction updates

### 2. Biometric Authentication

Secure device authentication with Face ID, Fingerprint, Iris, or PIN fallback.

```typescript
const auth = new BiometricAuth(wallet);

// Enroll new biometric
await auth.enrollBiometric('face');

// Verify biometric (e.g., for transaction approval)
const verified = await auth.verify(biometricData);
if (verified) {
    // Proceed with sensitive operation
}

// PIN fallback if biometric fails
const pinVerified = await auth.verifyPIN('1234');

// Session management (auto-logout after timeout)
const sessionValid = await auth.getSession();
```

**Features:**
- Face ID with liveness detection
- Fingerprint matching with anti-spoofing
- Iris recognition support
- PIN fallback with rate limiting
- Session timeouts (default: 300 seconds)
- Automatic lockout after 5 failed attempts

### 3. Transaction Signing

On-device transaction signing without exposing private keys.

```typescript
const signer = new MobileTransactionSigner(wallet);

// Add account for signing
await signer.addAccount(wallet.getAccount(0), 'ED25519');

// Create signing request (user review required)
const signingRequest = signer.createSigningRequest(
    transactionData,
    120 // 120-second expiry
);

// Display to user for approval/rejection
// After user approval:
const signature = await signer.approveAndSign(signingRequest);

// Or batch sign multiple transactions
const signatures = await signer.batchSign([
    tx1, tx2, tx3
]);
```

**Signature Algorithms:**
- `ED25519` — SHA-512 based signing
- `ECDSA` — SECP256K1 curve with SHA-256

## QR Code & Deep Linking

### QR Code Scanning

```typescript
import { QRScanner } from '@x3/mobile-sdk';

const scanner = new QRScanner();

// Parse QR code data
const qrData = scanner.parseQRString(qrCodeContent);
console.log('Recipient:', qrData.address);
console.log('Amount:', qrData.amount);
console.log('Memo:', qrData.memo);

// Validate address format
const isValid = scanner.validateAddress(qrData.address);

// Detect phishing attempts
const isPhishing = scanner.detectPhishing(qrData.address);
```

### QR Generation

```typescript
// Generate receive QR
const receiveQR = scanner.generateReceiveQR('0x123...');

// Generate payment QR with amount
const paymentQR = scanner.generatePaymentQR({
    address: '0x123...',
    amount: '1000',
    memo: 'Payment for service'
});
```

### Deep Linking

```typescript
import { DeeplinkHandler } from '@x3/mobile-sdk';

const deeplink = new DeeplinkHandler();

// Parse incoming deeplink
const request = deeplink.fromURL('x3://sendTransaction?to=0x123&amount=100');

// Handle different request types
await deeplink.handle(request);

// Generate deeplinks for sharing
const sendLink = deeplink.generateSendDeeplink({
    recipient: '0x123...',
    amount: '100'
});
```

## Wallet Management

### HD Wallet Support

Hierarchical deterministic wallet with BIP-44 compliance:

```typescript
// Standard paths
// m/44'/60'/0'/0/0 — EVM (Ethereum, Polygon)
// m/44'/501'/0'/0' — Solana
// m/44'/118'/0'/0/0 — Cosmos

const account0 = await wallet.deriveAccount("m/44'/60'/0'/0/0");
const account1 = await wallet.deriveAccount("m/44'/60'/0'/0/1");
const cosmosAccount = await wallet.deriveAccount("m/44'/118'/0'/0/0");
```

### Address Management

```typescript
// List all addresses
const addresses = wallet.getAllAddresses();

// Create new address
const newAddress = await wallet.createAddress();

// Rename address
await wallet.renameAddress(address, 'My Spending Account');

// Export private key (for backup only)
const privateKey = await wallet.exportPrivateKey(address, password);
```

## Error Handling

```typescript
import { MobileSDKError } from '@x3/mobile-sdk';

try {
    const wallet = await MobileWallet.importFromSeed(seed, path);
} catch (error) {
    if (error instanceof MobileSDKError) {
        switch (error.code) {
            case 'INVALID_SEED':
                console.error('Invalid BIP-39 seed phrase');
                break;
            case 'BIOMETRIC_UNAVAILABLE':
                console.error('Biometric not available on device');
                break;
            case 'SIGNING_REJECTED':
                console.error('User rejected transaction signature');
                break;
        }
    }
}
```

## Best Practices

### Security

1. **Never expose private keys** — Always use signing requests for transactions
2. **Enable biometrics** — Require authentication for sensitive operations
3. **Validate addresses** — Check QR codes for phishing patterns
4. **Use HTTPS only** — All RPC endpoints must use TLS/SSL
5. **Backup recovery phrase** — Store seed in secure location (offline preferred)

### Performance

1. **Cache balances** — Reduce RPC calls by caching and refreshing periodically
2. **Batch transactions** — Use `batchSign()` for multiple operations
3. **Lazy load accounts** — Don't derive all accounts on startup
4. **Debounce updates** — Prevent excessive balance refresh calls

### UX

1. **Show timeout warnings** — Alert users before session expires
2. **Provide pin fallback** — Support PIN if biometric fails
3. **Clear error messages** — Explain what went wrong and how to fix it
4. **Handle app backgrounding** — Lock session when app goes to background

## Testing

```typescript
// Mock setup for testing
import { MockWallet } from '@x3/mobile-sdk/testing';

const mockWallet = new MockWallet({
    balance: 10000,
    address: '0xtest...'
});

// Unit test example
describe('Mobile Wallet', () => {
    it('should import seed phrase', async () => {
        const wallet = await MobileWallet.importFromSeed(testSeed, derivationPath);
        expect(wallet.getAccount(0).address).toBeDefined();
    });
});
```

## Migration Guide

### From Previous SDK Version

```typescript
// Old API
const wallet = initWallet(seed);

// New API
const wallet = await MobileWallet.importFromSeed(seed, derivationPath);
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Biometric not available" | Check device has Face ID/Fingerprint enabled |
| "Invalid seed phrase" | Verify seed has 12 or 24 words |
| "Connection timeout" | Check RPC endpoint is reachable |
| "Signature rejected" | User explicitly rejected — retry with valid transaction |

---

**Version**: 1.0.0  
**Last Updated**: 2024  
**Documentation**: https://docs.x3.chain/mobile-sdk
