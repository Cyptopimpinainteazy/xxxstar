# sTPS baseline (single validator)

Method: `scripts/stps/stps.js`, after [paritytech/polkadot-stps](https://github.com/paritytech/polkadot-stps)
(Unlicense): keep-alive balance transfers, each from a distinct untouched sender (nonce 0) to a distinct
receiver, both endowed at genesis; counted from `balances.Transfer` events in **finalized** blocks,
divided by on-chain time (`pallet_timestamp` of the block before the first one with a transfer to the
last one). Submitted/accepted counts are reported separately and never called sTPS.

| Run | Transfers | Finalized | Blocks | On-chain span | **sTPS** | Per block avg / max |
|---|---|---|---|---|---|---|
| burst | 2,000 | 2,000 | 17 | 3.4 s | **588.2** | 118 / 136 |
| sustained | 10,000 | 10,000 | 94 | 25.6 s | **390.6** | 106 / 214 |

Environment (2026-10-13): x3star1, 2× Xeon E5-2620 v4 (32 threads), one validator
(`x3-chain-node --chain <dev spec + 20k endowed accounts> --alice --validator --force-authoring`,
release build of the mission branch, spec_version 22, dev chain block time ~200 ms). The machine was
also running the GitHub runners' CI (load ≈ 24), so these are conservative. Not a multi-validator or
networked figure: the 7-validator measurement is a separate run.

Reproduce:
```
x3-chain-node build-spec --chain dev > base.json
node scripts/stps/stps.js spec base.json stps.json 10000
x3-chain-node --chain stps.json --alice --validator --force-authoring --tmp --node-key <hex> --pool-limit 50000 --pool-kbytes 200000
node scripts/stps/stps.js run ws://127.0.0.1:9944 10000
```
