# Mainnet Readiness

This document is an index of launch evidence, not a declaration of launch
approval. X3 is not approved for mainnet until every required release gate
passes with current, reproducible evidence.

## Canonical evidence

- [Launch scope](LAUNCH_SCOPE.md) defines the production boundary and
  explicitly excluded capabilities.
- [Release gates](RELEASE_GATES.md) defines the mandatory validation gates.
- [Testnet gap ledger](TESTNET_GAP_LEDGER.md) records unresolved
  implementation and operational gaps.
- [Testnet verification](TESTNET_VERIFICATION.md) records completed testnet
  evidence and its limits.
- [Feature registry](FEATURE_REGISTRY.toml) is the source of feature-level
  readiness scores; validate it with `scripts/check-readiness-consistency.sh`.
- [Security policy](docs/security/SECURITY.md) defines vulnerability reporting
  and response procedures.

## Required release evidence

Before a mainnet release candidate can be approved, maintainers must produce
and retain:

1. A clean, reproducible runtime build using `srtool`.
2. Validated node and runtime artifacts plus reviewed chain specifications.
3. Passing critical runtime, supply-ledger, packet, bridge, fee, and slashing
   test suites.
4. A current security review and a clean forbidden-secret scan.
5. Explicit closure or accepted risk approval for every blocker in
   `TESTNET_GAP_LEDGER.md`.

The automated `make audit` gate validates the locally verifiable subset of
this evidence. Passing it does not by itself authorize a launch.
