> Production-only enforcement baseline for all agents.

# Production Gate Rules

- Do not ship stubs/placeholders/mock production paths.
- Do not skip/weaken/delete tests to force green.
- Critical path changes (consensus/VM/bridge/RPC/wallet/staking/DEX/governance) require test + invariant coverage.
- Mainnet readiness cannot be claimed unless all release gates pass.

Required commands:

```bash
make guard
make test
make audit
make mainnet-check
make fresh-machine-check
```
