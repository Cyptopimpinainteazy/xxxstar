# RC6 Validator Onboarding Dry Run

## Goal

Prove that the onboarding guide is actionable for a fresh validator host.

## Dry Run Status

- Documentation path: PASS
- Toolchain path: PASS after setting stable as the default rustup toolchain
- Build path: FAIL due repository build blocker in `sp-wasm-interface`
- Bootnode/bootstrap path: PENDING until real bootnode IPs are assigned

## Dry Run Steps

1. Install OS dependencies listed in [VALIDATOR_ONBOARDING.md](../../docs/testnet/VALIDATOR_ONBOARDING.md).
2. Confirm `rustup default stable` is set on the machine.
3. Confirm `cargo --version` works before building.
4. Build the release node binary.
5. Start the validator with the published raw chain spec.
6. Confirm peer discovery through the published bootnode address.
7. Confirm block import and RPC behavior are healthy.

## Observation

The doc itself is sufficient to follow, but the repo currently blocks the build step, so a real fresh-machine join is still gated on the RC6 build fix and live bootnodes.