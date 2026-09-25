# X3 Alpha economic policy — authoritative

This file is authoritative for **Public Testnet Alpha**. `scripts/mainnet/x3_economic_model_gate.sh`
checks that the code still matches every claim below, and fails the build when it does not. If the
code and this file disagree, the code is wrong *or* this file is wrong — either way the gate stops
the launch until one of them is corrected deliberately.

> Libraries that implement inflation, staking analytics, EIP-1559 simulation, reward models or any
> other future economic mechanism **do not constitute active runtime economics** unless they are
> wired through `runtime/src/lib.rs`. `crates/x3-economics` and `crates/x3-fees` are not runtime
> dependencies today; nothing they compute can move a balance.

## Frozen parameters

| Parameter | Alpha value | Where it lives |
| --- | --- | --- |
| Symbol | `X3` | `pallets/x3-coin/src/lib.rs` (`X3_SYMBOL`) |
| Decimals | 12 | `pallets/x3-coin/src/lib.rs` (`X3_DECIMALS`); `node/src/chain_spec.rs:23` endows genesis balances at `1 X3 = 10^12` |
| Initial total supply | 8,888,888,888 X3 | `X3_TOTAL_SUPPLY_X3`, mirrored in `deployment/genesis/x3-testnet-allocations.json` |
| Inflation | **0** — fixed genesis supply, no issuance path | no mint increases supply; `X3Coin::mint` is treasury-funded and asserts `verify_supply_invariant()` |
| Permissionless staking | **disabled** — `pallet_staking` is not in the runtime | `runtime/Cargo.toml` |
| Validator set | governed / launch-operator controlled | `Aura` + `Grandpa` + `Session` authorities, set by chain spec and `X3Consensus` |

## Genesis allocation buckets

Every bucket below is expressed in whole X3 and must sum to the total supply
(`X3_ALLOCATION_SUM == X3_TOTAL_SUPPLY`, a compile-time assertion in the pallet):

| Bucket | X3 | Purpose |
| --- | --- | --- |
| `treasury` | 2,222,222,222 | protocol treasury |
| `validators_staking` | 1,777,777,778 | **validator/security reserve** — funds validator compensation for Alpha |
| `ecosystem_grants` | 1,777,777,778 | ecosystem and grants |
| `presale_early_investors` | 1,333,333,333 | presale |
| `bonus_pool` | 888,888,889 | community bonus pool |
| `team_core_contributors` | 888,888,888 | team, vesting over ~1 year with a ~6 month cliff |

Faucet funds are **not** a genesis bucket and must never be paid from the validator/security reserve.
Validator signing keys, treasury multisig keys and atomic-gateway/settlement keys stay separate.

## Transaction fees

* **Calculation**: `WeightToFee = IdentityFee<Balance>` (1 base unit per unit of weight) and
  `LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>` with
  `TransactionByteFee = 10 * MICRO_ATLAS`; `OperationalFeeMultiplier = 5`.
* **Fee multiplier**: `FeeMultiplierUpdate = ()` — pricing is **static and deterministic**. There is
  no congestion adjustment in Alpha, and no document may claim EIP-1559-style runtime pricing.
* **Destination**: **100% burned.** `DealWithFees::on_unbalanced` drops the `NegativeImbalance`, so a
  collected fee reduces issuance and reaches neither the treasury nor the block author.
* **Priority fee / tip**: not implemented in Alpha. There is no tip-routing path, so no document may
  claim block authors receive tips.

## Validator compensation (Alpha)

Validators are **not** paid by staking rewards, because there is no staking pallet. They are intended
to be paid from the `validators_staking` reserve above, by governance-approved periodic payouts.

**Status: the reserve is frozen as a constant (`X3_VALIDATOR_SECURITY_RESERVE`) but the payout path
is not implemented yet.** Until it is, Alpha validators are uncompensated by the runtime. That is
stated here rather than papered over: no automatic reward is faked to make the documentation true.

Transition path to permissionless staking (post-Alpha, requires a runtime upgrade and a governance
motion): wire `pallet_staking` (or a purpose-built X3 staking pallet), move the reserve into it as the
initial stake/era pot, replace the fixed validator set with the staking-derived one, and only then
enable inflation policy — which must be re-attested through this gate before it can mint anything.

## Accounting identity

```
treasury.allocation.amount_base + Σ allocations[].amount_base == token.total_supply_base
```

This is asserted by `scripts/ci/verify_genesis_allocations_baseline.sh` against the pinned baseline
and re-asserted by the economic gate. It may only change with an accepted governance/economic-policy
update, which means updating this file, the manifest, the pallet constants and the baseline together.

## Known open items (ticketed, not waived)

1. Validator payout path from the security reserve (`X3_VALIDATOR_SECURITY_RESERVE`) — not implemented.
2. Tip/priority-fee routing — not implemented; Alpha burns 100%.
3. Dynamic fee multiplier — deliberately deferred; Alpha is static.
4. `node/src/chain_spec.rs` fee constants use a `NANO_ATLAS` ladder (1 X3 = 10^9) while the token
   decimals above are 12; `crates/x3-rpc/src/wallet_service_rpc.rs` carries a `10^18` literal labelled
   "1 X3". Neither is reconciled by this policy and both are launch hazards.
5. `pallets/x3-coin` `X3_ASSET_ID = 0` while the deployment manifest says `asset_id: 1000`.
