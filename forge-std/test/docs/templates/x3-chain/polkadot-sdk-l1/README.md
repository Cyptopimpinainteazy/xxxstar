# X3 Chain Starter: Polkadot SDK L1

## Upstream

- <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/templates/index.html>

## Goal

Bootstrap a standalone X3 Chain-compatible Layer 1 baseline in Rust.

## Checklist

1. Select an upstream L1/node template.
2. Pin upstream commit/tag and record it.
3. Configure X3 chain spec defaults (name, id, bootnodes, telemetry).
4. Wire runtime pallets required by X3 Chain.
5. Add CI compile/test gates.
