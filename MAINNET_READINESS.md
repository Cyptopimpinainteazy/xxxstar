# Mainnet Readiness

Mainnet is blocked unless all required release gates pass.

## Required gates

- `make guard`
- `make test`
- `make audit`
- `make mainnet-check`
- `make fresh-machine-check`

## Mandatory controls

- No critical/high unresolved security findings
- Replay protection and nonce uniqueness verified
- Cross-VM atomic commit/rollback verified
- Canonical supply invariants verified
- Secrets externalized
- Rollback and migration procedures documented