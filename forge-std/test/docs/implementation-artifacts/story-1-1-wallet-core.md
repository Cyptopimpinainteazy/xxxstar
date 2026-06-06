# Story 1.1: Wallet Core Implementation

**Epic:** 1 - Multi-Chain Wallet Foundation  
**Priority:** P0  
**Status:** ready-for-dev

---

## Story Definition

As a user,
I want to create or import a wallet,
So that I can manage my digital assets across multiple chains.

---

## Acceptance Criteria

### AC 1.1.1: Import Existing Wallet

| Given | When | Then |
|-------|------|------|
| User has seed phrase | User imports seed phrase | Wallet displays correct address and balances |

**Implementation Notes:**
- Support BIP-39 mnemonic phrases
- Derive addresses for supported chains (ETH, DOT, SOL, etc.)
- Secure storage of encrypted seed

### AC 1.1.2: Create New Wallet

| Given | When | Then |
|-------|------|------|
| User wants new wallet | User selects "Create New" | New wallet with seed phrase is generated |

**Implementation Notes:**
- Generate cryptographically secure random seed
- Display seed phrase for user backup (with warnings)
- Never store plaintext seed

### AC 1.1.3: Hardware Wallet Support

| Given | When | Then |
|-------|------|------|
| User has hardware wallet | User connects device | Wallet imports addresses from device |

**Implementation Notes:**
- Support Ledger and Trezor devices
- Implement WebUSB/HID communication
- Derive addresses on-device

---

## Technical Implementation Notes

### File Location
- `apps/x3-desktop/src-tauri/src/wallet/` (Rust backend)
- `apps/x3-desktop/src/stores/walletStore.ts` (Frontend state)

### Dependencies
- `substrate-bip39` - Mnemonic handling
- `ledger-device-rust-sdk` - Hardware wallet (optional)
- `wallet-core` - Multi-chain address derivation

### Architecture Notes
- Use Tauri commands for secure operations
- Store encrypted seed in OS keychain
- Derive addresses on-demand, never store private keys

---

## Definition of Done

- [x] Wallet can import BIP-39 seed phrase
- [x] Wallet can generate new seed phrase
- [x] Wallet derives correct addresses for ETH, DOT, SOL
- [x] Seed is stored encrypted (never plaintext)
- [ ] Hardware wallet connection works (if available) - Future enhancement
- [x] Unit tests pass for address derivation
- [x] Integration tests pass for wallet operations

---

**Story Key:** 1-1-wallet-core  
**Created:** 2026-02-13  
**Sprint:** 1
