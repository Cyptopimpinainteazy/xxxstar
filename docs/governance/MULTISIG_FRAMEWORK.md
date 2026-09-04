# X3 Multisig Governance Framework

## Overview

X3 mainnet launches with an authority-set (Aura + GRANDPA) consensus model. Governance is managed via **multisig accounts** until a full on-chain governance pallet (Council + Technical Committee + Referenda) is activated post-launch.

## Governance Roles

| Role | Authority | Method |
|---|---|---|
| **Genesis Council** | 5 signers, 3-of-5 multisig | Appoints validators, approves runtime upgrades |
| **Technical Committee** | 3 signers, 2-of-3 multisig | Emergency fixes, fast-track urgent upgrades |
| **Treasury** | 5 signers, 3-of-5 multisig | Protocol fee management, grants, operational spending |

## Multisig Configuration

### Genesis Council
```
Threshold: 3 of 5
Chain: x3-mainnet (SS58 prefix: 42)
Multisig address: <DERIVED_AT_GENESIS>
```

### Technical Committee
```
Threshold: 2 of 3
Chain: x3-mainnet (SS58 prefix: 42)
Multisig address: <DERIVED_AT_GENESIS>
```

### Treasury Multisig
```
Threshold: 3 of 5
Chain: x3-mainnet (SS58 prefix: 42)
Multisig address: <DERIVED_AT_GENESIS>
```

## Key Management

1. **Each signer** generates their own sr25519 key pair via `subkey generate`
2. **Public keys** are shared through an out-of-band secure channel
3. **Multisig address** is derived from ordered public keys + threshold using `subkey inspect`
4. **No single person** holds two seats on the same multisig

```bash
# Generate a key
subkey generate --scheme sr25519

# Derive multisig address (example: 3-of-5)
subkey inspect "multisig://0x<SIGNER1>?threshold=3&signatory=0x<SIGNER2>&signatory=0x<SIGNER3>&signatory=0x<SIGNER4>&signatory=0x<SIGNER5>"
```

## Upgrade Process

### Standard Upgrade

1. Runtime WASM is built from a tagged release
2. WASM hash is verified by all Technical Committee members
3. Council multisig approves the upgrade via `sudo()` or `pallet-scheduler`
4. `try-runtime` dry-run is executed before the upgrade call
5. Upgrade is enacted
6. Block production continues — no chain halt

### Emergency Hotfix

1. Technical Committee identifies critical issue
2. 2-of-3 multisig approves hotfix
3. Hotfix runtime is deployed immediately
4. Full Council ratifies within 7 days or hotfix is reverted

## Timeline

| Phase | Governance Body | Action |
|---|---|---|
| Genesis | Genesis Council | Appoint initial validator set, set fees, enable governance |
| Month 1-3 | Genesis Council + TC | Runtime upgrades, validator set changes, treasury operations |
| Month 4+ | Full on-chain governance | Council elections, public referendum, delegated voting |