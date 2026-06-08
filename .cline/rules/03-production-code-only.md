# Rule: Production Code Only

## Purpose
All code merged into main must be production-quality. No demo paths, mock backends, hardcoded test addresses, or in-memory-only persistence should be shipped to production.

## Required Behavior
- Prefer production-quality implementations over demo/mock versions.
- When adding a new adapter, it must connect to real infrastructure, not a mock.
- Persistence must use the production storage layer (RocksDB, DB, etc.), not in-memory hashmaps.
- Addresses, keys, and endpoints must be configurable, not hardcoded.
- Feature flags must gate incomplete features — incomplete features must be behind flags.

## Forbidden Behavior
- Do NOT ship `localhost` or `127.0.0.1` as the default production endpoint.
- Do NOT hardcode test private keys in production code.
- Do NOT use `HashMap` / `Vec` as permanent storage where a DB is required.
- Do NOT ship `mockall`, `mock_impl`, or test-only dependencies in production binaries.
- Do NOT leave `#[cfg(test)]` workarounds that hide missing production paths.
- Do NOT claim mainnet-readiness with mock RPC endpoints or fake bridge relayers.

## Proof Required
- Search for hardcoded test addresses in production source paths.
- Verify production binaries don't link test-only crates.
- Feature flags audit: check that incomplete features are gated.