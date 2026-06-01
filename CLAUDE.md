# Production Agent Rules

This repository is production-gated.

- Only real production code: no stubs/placeholders/mock production paths.
- No skipped/weakened/deleted tests to force green builds.
- Critical path changes (consensus/VM/bridge/wallet/RPC/staking/DEX/governance) require tests and invariant coverage.
- No secrets in repo (private keys/mnemonics/RPC tokens/.env secrets).
- Mainnet readiness cannot be claimed without passing all release gates.

## Required local gates

```bash
make guard
make test
make audit
make mainnet-check
make fresh-machine-check
```

## Completion definition

A task is complete only when implementation is real, runtime-wired, tested, documented, and all applicable gates pass.