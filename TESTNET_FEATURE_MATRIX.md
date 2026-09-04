# TESTNET_FEATURE_MATRIX.md

Live feature matrix. Populated from direct inventory of code + entry points. Column meaning:
- Reachability = is this feature wired into a real production path a user/operator can trigger?
- Status: complete | partial | disconnected | placeholder | broken | unsafe | unverified.

| Feature | Advertised purpose | Real entry point | Implementation | Integration | Test coverage | Reachability | Security/reliability | Evidence | Status |
|---------|-------------------|------------------|----------------|-------------|---------------|--------------|----------------------|----------|--------|

## Wired runtime/consensus surface (canonical node `x3-chain-node`)
Chain spec loader: node/src/chain_spec.rs `load_spec` ids: dev, local, local-testnet, three-validator, staging, production. WASM embedded via `require_embedded_wasm`. (verify: which WASM actually embedded per spec)
