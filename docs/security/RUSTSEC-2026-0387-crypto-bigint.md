# RUSTSEC-2026-0387: residual `crypto-bigint 0.5.5`

## Exposure

`RUSTSEC-2026-0387` reports secret-dependent control flow in `crypto-bigint`
operations including subtraction, zero counting, division, and Montgomery
multiplication. The advisory has no patched release in the `0.5` line; the fix
starts at `0.7.6`.

The root workspace now resolves two copies:

- `crypto-bigint 0.7.8` and `0.7.10` through `k256 0.14` for X3-owned
  signing and verification crates.
- `crypto-bigint 0.5.5` through the unmodified `polkadot-sdk` stable2512 and
  Frontier dependency lines, which pin `k256 0.13`.

This split was measured with `cargo tree -i crypto-bigint@0.5.5` and
`cargo tree -i crypto-bigint@0.7.8` and
`cargo tree -i crypto-bigint@0.7.10`. X3 directly updated every owning manifest
that it controls: `x3-atomic-swap`, `x3-bitcoin-vault`, `gpu-sig-verifier`,
`cross-chain-gpu-validator`, `x3-mobile-sdk`, and the desktop Tauri wallet.

## Why it is accepted

The residual upstream copy is reached by Substrate's ECDSA crypto traits and
Frontier's Ethereum transaction recovery. Those node-facing operations consume
public transactions, signatures, and public keys. X3's private-key signing
paths use the updated direct `k256 0.14` dependency. No X3-authored code signs
with the upstream `0.5.5` arithmetic.

The advisory says practical exploitability depends on the downstream algorithm,
compiler, target CPU, and observability of the timing channel. It does not
publish a CVSS score.

## Re-review trigger

Remove this exception as soon as a supported `polkadot-sdk`/Frontier revision
accepts `k256 0.14`. Re-open the risk immediately if any X3-owned production path
starts signing with the upstream `k256 0.13` dependency, or if an audit
demonstrates a practical key-recovery channel in the public verification path.
