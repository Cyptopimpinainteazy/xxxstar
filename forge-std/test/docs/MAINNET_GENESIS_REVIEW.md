# MAINNET GENESIS REVIEW

Target: v0.4 Internal-Only Mainnet RC

## Review Checklist

- no Alice, Bob, Charlie, Dave, Eve, Ferdie development keys in authorities or endowed accounts
- no dev sudo authority unless explicitly timeboxed and governance-documented
- ExternalBridgesEnabled=false by default
- mint authority restricted to governance origin only
- route limits configured for internal-only domains
- validator authorities set from explicit production key material
- bootnodes set and reachable
- token symbol and decimals match canonical asset metadata
- treasury and council addresses match signed governance records

## Evidence Sources

- node/src/chain_spec.rs
- runtime/src/lib.rs
- chain-specs/x3-mainnet-plain.json
- chain-specs/x3-mainnet-raw.json

## Verification Commands

- ./scripts/mainnet/generate_mainnet_chain_spec.sh
- rg "Alice|Bob|Charlie|Ferdie" chain-specs/x3-mainnet-plain.json chain-specs/x3-mainnet-raw.json
- rg "ExternalBridgesEnabled|external_bridges" chain-specs/x3-mainnet-plain.json chain-specs/x3-mainnet-raw.json

## Result

Status: PENDING_FRESH_GENERATION
