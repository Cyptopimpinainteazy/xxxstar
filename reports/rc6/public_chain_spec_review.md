# RC6 Public Chain Spec Review

## Goal

Validate that RC6 package generation creates public testnet chain specs suitable for external validator onboarding.

## Checks

- Plain spec generated: `chain-specs/x3-public-testnet-plain.json`
- Raw spec generated: `chain-specs/x3-public-testnet-raw.json`
- No known development authorities (Alice/Bob/Charlie/etc.) in public spec
- Spec can be consumed by node startup command from validator guide

## Notes

- RC6 allows `BOOTNODE_DEPLOYMENT: PENDING` if bootnodes are not yet live.
- Public launch is blocked until bootnodes are deployed and verified.

## Status

- Chain spec generation: PENDING
- Dev-key scrub: PENDING
- Public readiness judgment: PENDING
