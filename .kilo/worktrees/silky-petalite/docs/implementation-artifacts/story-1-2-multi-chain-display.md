# Story 1.2: Multi-Chain Display

**Epic:** 1 - Multi-Chain Wallet Foundation  
**Priority:** P0  
**Status:** ready-for-dev

---

## Story Definition

As a user,
I want to see my balances across all supported chains in a unified view,
So that I can monitor my total portfolio value at a glance.

---

## Acceptance Criteria

### AC 1.2.1: Unified Balance Display

| Given | When | Then |
|-------|------|------|
| User has wallets on multiple chains | User opens wallet dashboard | All chain balances displayed with total value |

**Implementation Notes:**
- Aggregate balances from EVM, SVM, and Substrate
- Calculate total portfolio value in USD
- Show individual chain breakdowns

### AC 1.2.2: Network Switching

| Given | When | Then |
|-------|------|------|
| User is on one network | User switches network | UI updates to show selected network's data |

**Implementation Notes:**
- Network selector dropdown
- Persist last selected network
- Show network indicator in header

### AC 1.2.3: Token List View

| Given | When | Then |
|-------|------|------|
| User has multiple tokens | User views token list | Each token shows symbol, balance, value, 24h change |

**Implementation Notes:**
- Sort by value (descending)
- Color code positive/negative changes
- Show icons for each token

---

## Technical Implementation Notes

### File Location
- `apps/x3-desktop/src/components/panels/wallet/` (React components)
- `apps/x3-desktop/src/stores/walletStore.ts` (State management)

### Dependencies
- Existing wallet store already has token data
- Use existing UI components from wallet panel

### Architecture Notes
- Reuse existing walletStore for balance data
- Create new WalletBalancePanel component
- Integrate with existing DexPanel for price data

---

## Definition of Done

- [ ] Unified balance display shows all chain balances
- [ ] Total portfolio value calculated and displayed
- [ ] Network switching works correctly
- [ ] Token list shows all required fields
- [ ] 24h change displays with color coding
- [ ] Responsive design works on mobile

---

**Story Key:** 1-2-multi-chain-display  
**Created:** 2026-02-13  
**Sprint:** 1
