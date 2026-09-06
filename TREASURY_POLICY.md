# X3 Atomic Star — Treasury Policy

**Document:** `TREASURY_POLICY.md`
**Owner:** User (lojak)
**Effective date:** 2026-09-05
**Last audited:** 2026-09-05
**Status:** DRAFT — adapt before any external use

> This document is the operational policy for X3 funds. It does NOT contain any
> key material. Key material lives on hardware devices (Ledger / Trezor), encrypted
> 1Password backups, and steel seed plates — never in this repo.

---

## 0. Why we have two treasuries

X3 currently has **two separate treasury concepts** in play. Conflating them is the
single biggest source of operational confusion. Read this section first.

| Treasury | Purpose | Who controls it | Where it lives | Custody of funds |
|---|---|---|---|---|
| **Operational treasury (on-chain)** | Pays validator rewards, parachain fees, DAO-funded work, recurring payouts | Council / `EnsureRootOrHalfCouncil` via `pallet_treasury` spending origins | On every X3 chain, at the deterministic pallet ID `py/trsry` → `5FsRPEB95VFVZezh9xpzQ2W5qZvSLhf7C1fAtAgmNosVY9rZ` (SS58 prefix 42) | On-chain (no key) |
| **Grant treasury (this policy)** | Receives grant proceeds in DOT / ETH / USDC / SOL / stables, holds them safely, distributes to operational treasury and team per approved budgets | Multisig (2-of-3 or 3-of-5) | Off-chain (cold hardware wallets + Safe / Polkadot Vault) | Hardware devices |

The operational treasury on-chain is excellent for in-protocol spend. It is *not*
designed for receiving large lump-sum grant inflows — those go to the grant
treasury outlined here.

---

## 1. On-chain operational treasury (existing)

| Item | Value |
|---|---|
| Source | `pallet_treasury` |
| PalletId | `py/trsry` |
| Address (all X3 variants) | `5FsRPEB95VFVZezh9xpzQ2W5qZvSLhf7C1fAtAgmNosVY9rZ` |
| SS58 prefix | 42 |
| Code reference | `runtime/src/lib.rs:2181-2196` |

### Spend controls (verified 2026-09-05)

| Origin | Spend limit |
|---|---|
| `EnsureRootOrHalfCouncil` | small ≤ 1,000 X3 |
| `EnsureRootOrHalfCouncil` | medium ≤ 10,000 X3 |
| `EnsureRootOrHalfCouncil` | large ≤ 100,000 X3 |
| `EnsureRootOrHalfCouncil` | critical (unbounded) |
| `EnsureRootOrHalfCouncil` | pause / yield-config |

ProposalBond: 5%. ProposalBondMinimum: 100 X3.

### X3 Treasury Policy (overlay)

Custom pallet `pallet_x3_treasury-policy` adds per-(chain, asset, lane-class)
allocation caps, an operator funding threshold (above which requires governance
approval), and an insurance reserve. It does NOT own funds — it gates them.

### EVM-side Treasury contract — **SECURITY GAP**

`X3-contracts/evm/contracts/treasury/Treasury.sol` uses OpenZeppelin `Ownable`
(single owner) and routes fees 20% / 50% / 30% to dev / dao / lp wallets.

**Risk:** the single owner can change splits and wallet destinations, and
`routeFee` is callable by anyone. A misconfigured owner key (or any compromise
of that one key) drains the contract.

**Required fix before mainnet (EVM):**
1. Deploy EVM treasury behind a **Safe (Gnosis Safe)** multisig owner.
2. Or replace the contract with a Safe-mirror pattern (no storage of value).
3. Or restrict `routeFee` to a third-party router contract address.

---

## 2. Grant treasury (new — this policy)

A separate multisig-controlled set of accounts that receive grant funds, hold
them safely, and distribute to operational and team accounts per approved budgets.

### 2.1 Per-network structure

| Wallet | Network | Tool | Signers (recommended) | Threshold |
|---|---|---|---|---|
| **`x3-grant-safe.eth`** | Ethereum mainnet + EVMs (USDC, ETH, stables) | **Safe (Gnosis Safe)** at `safe.global` | founder + Ledger 1 + Ledger 2 (one held by co-signer) | 2-of-3 |
| **`x3-grant-polkadot`** | Polkadot / Asset Hub (DOT, parachain tokens) | Substrate-native `multisig` pallet via polkadot.js apps UI; or fellowship | founder + 2 advisors | 2-of-3 |
| **`x3-grant-base`** | Base (USDC grants) | Safe on Base | founder + Ledger 2 | 2-of-3 |
| **`x3-grant-sol`** *(only if needed)* | Solana (SOL, USDC) | Squads Protocol multisig | founder + Ledger + 1 advisor | 2-of-3 |
| **`x3-ops-hot`** | small spending money | Hardware wallet (Ledger) | founder | 1-of-1, **max $5k held** |
| **`x3-cold`** | Long-term reserves | Hardware wallet (Ledger Stax) + offline steel backup | founder | 1-of-1, never touches internet after initial sweep |

> **Signers and threshold matter.** 2-of-3 is enough for solo founder + 2 trusted
> advisors. 3-of-5 is safer if you have a wider team. Never 1-of-2. Never 1-of-1
> for any wallet holding > $1k.

### 2.2 Spending policy (how grant money moves)

```
[Grant Funder: W3F / Algorand / Starknet / Octant / Gitcoin / ...]
                              |
                              v
        [GRANT MULTISIG]    (cold, near-custody-grade)
                              |
         +-----+-----+-----+-----+-----+
         |     |     |     |     |     |
         v     v     v     v     v     v
   ops-hot  cold   infra  legal  team    reinvest
   (~5%)  (25%) (25%)  (10%)   (25%)    (10%)
                            |
                            v
              [On-chain operational treasury: 5FsRPEB95VFVZezh9xpzQ2W5qZvSLhf7C1fAtAgmNosVY9rZ]
```

| Bucket | % of inflow | Purpose | Source of truth |
|---|---|---|---|
| Infrastructure | 25% | RPC hosting, CI, observability, audit retainer | Monthly cap in `POLICY.md` |
| Legal | 10% | Legal counsel, jurisdiction filings, license tracking | Invoices + CFO sign-off |
| Team | 25% | Founder + early employees (RFC / W-2 / contractor) | Each monthly transfer approved by ledger entry |
| Operational hot wallet | 5% | Day-to-day runway, ≤ $5k sitting balance at all times | Daily sweep review |
| Cold storage | 25% | Insurance buffer; cannot exceed 90 days runway in `x3-ops-hot` | Quarterly review |
| Reinvest | 10% | Ecosystem grants outflow, validator infrastructure, hackathon entry fees | Each transfer explicitly approved |

> **No single unsigned transfer over $10k.** All transfers above $10k require
> a 2-of-3 Safe signature **and** a separate motion recorded in `treasury_transfers.csv`
> with date, amount, purpose, recipient, and budget bucket.

### 2.3 Generation procedure

See `scripts/generate-grant-multisig.sh`. The script does NOT generate any key
material — it documents the steps you take offline on your own devices. Operators
must run those steps on an **air-gapped** machine or hardware wallet, never on the
machine that holds this repo.

### 2.4 Backup & recovery

- **Hardware wallets**: Ledger Nano X / Stax or Trezor Safe 5 (Trezor for diverse supplier)
- **Seed plates**: Cryptosteel capsule or Billfodl, 24-word mnemonic recorded
- **Locations**: 1 in a home safe, 1 in a bank safety-deposit box (different jurisdiction for safety-diversity)
- **Shamir backup** (optional): 2-of-3 or 3-of-5 SSSL split for very large treasuries
- **Recovery test**: run a full restore from seed plate once per year, document in `treasury/audit-log/`

### 2.5 Custody rules

- **Cold wallet never touches the internet after initial sweep.** QR-code signing or air-gapped PC only.
- **Hot wallet holds ≤ $5k.** Auto-sweep to cold if balance exceeds.
- **Multisig signing**: ≥ 2 hardware wallets in different physical locations for any transaction > $1k.
- **No custody in browser wallets.** Use Safe UI (which keeps keys client-side behind hardware) or Polkadot Vault / Nova Wallet.

---

## 3. Security audit findings (2026-09-05)

Three issues flagged before any external use. None are catastrophic today
(probably all testnet), but they're in the repo and rotated keys are cheap.

### 3.1 Validator summaries contain clear-text secret seeds and seed phrases

**File:** `deployment/keys/validator-0{1,2,3}-summary.txt`
**Severity:** Medium (depends on chain; rotate regardless)
**Issue:** Each file contains AURA + GRANDPA secret seed (hex) and 12-word mnemonic.
**Status:** Tracked in git history (`git ls-files deployment/keys/` confirms).
**Action:** Rotate by generating new suris offline; update chain-spec if keys were used.

```bash
# Offline, on a separate machine
subkey generate --scheme sr25519 --network substrate  # AURA
subkey generate --scheme ed25519 --network substrate  # GRANDPA
```

### 3.2 Sepolia deployer wallet in repo root

**File:** `sepolia-deployer-wallet.txt`
**Severity:** Low (Sepolia is testnet; address 0x9d9...1239)
**Issue:** Plaintext private key in a comment near repo root. Easy to grep for by
mistake. Add to `.gitignore` (or move to local secret manager) and rotate the
Sepolia account before reusing the address.

### 3.3 Validator suris in plain files

**File:** `deployment/chain-specs/fresh/validator-keys/validator-{1..7}.suri`
**Severity:** Low — file header says "TESTNET-ONLY"
**Issue:** All 7 raw hex seeds in one directory. Anyone with read access becomes a
hidden validator if you launch testnet from this workspace.
**Action:** Move `*.suri` files out of repo to local secret manager before first testnet launch.

---

## 4. Where to put the generated treasury addresses (after go-time)

When you have the multisig addresses back from `scripts/generate-grant-multisig.sh`,
**do NOT** paste them into this repo. Add them to your local password manager
(1Password / Bitwarden / KeePass) under a "X3 Treasury" entry, and reference them
in grant applications as:

```
Polkadot (DOT) grant deposit address:
  x3-grant-polkadot:  <paste the multisig SS58 here — public, safe to share>
Ethereum / EVM (USDC, stables) grant deposit address:
  x3-grant-safe.eth:  <paste Safe address here>
```

These addresses ARE meant to be public — shared with grant funders. The secret
keys/seed phrases backing them NEVER leave the hardware devices.

---

## 5. Policy maintenance

- This document is reviewed quarterly, or whenever the multisig signer set changes.
- All treasurer actions are logged in `treasury/audit-log/<YYYY-MM-DD>.md`.
- Add new co-signers with a written addendum signed by the existing signer set.
- Remove co-signers only after rotating their hardware-device seeds.
