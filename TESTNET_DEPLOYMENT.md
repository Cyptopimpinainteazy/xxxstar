# TESTNET_DEPLOYMENT.md

Deployment support will be filled during Phase 8. Baseline notes:

- Canonical node binary: `x3-chain-node` from crate `node/` ([[bin]] x3-chain-node -> src/main.rs).
- Prebuilt at `target/release/x3-chain-node` during this audit (verify freshness).
- Chain-spec loader ids: dev, local, local-testnet, staging, production (+ three-validator aliases) via node/src/chain_spec.rs.
- Existing scaffolds in repo: `quickstart-testnet.sh`, `deployment/key-gen-testnet.sh` (referenced), Dockerfiles (validator/indexer/mainnet-check), infra/infra-structure folders. All to be RE-VERIFIED before acceptance.
