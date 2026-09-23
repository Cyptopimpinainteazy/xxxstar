# The runtime's privileged origins were a compiled-in dev key

Date: 2026-09-23. Found while looking at where to put the operator's paid DRPC key (GAP-SECRETS-COMMITTED),
one level up from the credentials that were removed the same day.

## What the runtime did

```rust
pub type EnsureX3LangGateway =
    frame_system::EnsureSignedBy<X3LangGatewayAccount, AccountId>;
```

`X3LangGatewayAccount` is a 32-byte constant. `node/src/atomic_gateway.rs` has a test that derives the
account from the phrase `//x3-atomic-gateway` and asserts the two are equal:

```rust
#[test]
fn gateway_account_matches_runtime_constant() {
    assert_eq!(gateway().account(), expected);
}
```

`SettlementGatewayAccount` is the same thing for `//x3-settlement-gateway`. The node's comment says the
seed is "held by the node's atomic gateway service (`//x3-atomic-gateway` by default, overridable via
CLI/env)" — which is true of the *node* and false of the *runtime*. A chain spec cannot change a
constant compiled into the WASM, so every chain this runtime builds — the dev chain, the testnet, and
the `mainnet-rc1` narrow variant, which includes `X3AtomicKernel` in its `construct_runtime!` —
accepted a signature from those accounts for:

| gate | what it opens |
| --- | --- |
| atomic kernel `X3LangOrigin` | `assign_bundle_executor`, `finalize_atomic_bundle`, `rollback_bundle`, bundle submission |
| cross-VM router `X3LangOrigin`, `VmAdapterOrigin` | routing and VM-adapter entry points |
| atomic kernel `SettlementOrigin` | `finalize_with_settlement` |

The seed is a public dev phrase, so the account is anybody's, and funding it is a faucet away. This is
the same defect class as the four credentials removed from `crates/external-chains` on 2026-09-23,
except that this one is the authorization root rather than a provider key, and it could not be rotated
— it had to be changed in the runtime.

## What it is now

* `pallet-x3-custody` gained `AuthorizedGateways: StorageDoubleMap<GatewayRole, AccountId, ()>`, two
  genesis items (`x3_lang_gateways`, `settlement_gateways`), and `authorize_gateway`/`revoke_gateway`
  behind `GovernanceOrigin` (call indices 8 and 9).
* `EnsureAuthorizedGateway<T, R>` is a signed origin **and** a member of `R`'s role.
  `try_successful_origin` returns `Err(())`: no origin is authorized as a property of the code, so a
  pallet that wants to act as a gateway has to go through a call the chain authorized.
* The runtime's two aliases are that type. `X3LangGatewayAccount` and `SettlementGatewayAccount`
  remain, documented as the *dev* accounts, and are authorized only where a spec's genesis names them.
* Dev and local specs are unchanged in behaviour (`dev_gateway_genesis` names the two accounts the
  node's service already signs with).
* Staging, testnet and production parse `X3_{STAGING,TESTNET,PRODUCTION}_ATOMIC_GATEWAYS` and
  `..._SETTLEMENT_GATEWAYS` (JSON array of SS58, required, non-empty) and
  `assert_no_dev_gateway_accounts` refuses the published dev seeds plus everything already in the
  forbidden-seed list. A live chain cannot opt back into the old posture by hand.
* A chain that names no gateway can no longer assign or finalize a bundle. That is the intended
  failure mode for a chain that has not said who may.

`spec_version` 18 → 19. Two calls, one storage map, two genesis items, no existing key changes shape,
so no migration.

## Two things testing caught that reading would not have

**1. Every committed plain chain spec would have stopped loading.** The two new genesis fields
de-serialize as required by default (`genesis_config` generates `#[serde(deny_unknown_fields)]` and no
per-field default), and `chain-specs/x3-local3-plain.json` and
`deployment/chain-specs/fresh/x3-testnet-plain.json` both name the custody pallet without them. Measured
rather than assumed: deleting one *existing* field from `x3Custody` in a copy of the local3 spec made
`build-spec --raw` fail with

```
missing field `initialSignerLimits` at line 1191 column 3
```

so the same thing would have happened to `x3LangGateways` for every plain spec in the tree. Both fields
now carry `#[serde(default)]`: an old spec loads, and it authorizes nobody — the same fail-closed
posture as a chain that never named a gateway. Verified with the rebuilt node:

```
chain-specs/x3-local3-plain.json                     -> exit=0, 17,214,308 bytes raw
deployment/chain-specs/fresh/x3-testnet-plain.json    -> exit=0,  2,852,996 bytes raw
```

**2. Two e2e files are not test targets.** `tests/e2e/safety_tests.rs` and
`tests/e2e/real_finality_proofs.rs` live in the `e2e_tests` package but are not declared in its
`Cargo.toml`, and `safety_tests.rs` opens with `mod mock;` for a file that does not exist. Neither
compiles, so neither runs; both drive `finalize_atomic_bundle` with `RuntimeOrigin::signed(1)`, an origin
this runtime has never accepted. They read as coverage of the atomic lifecycle and are not. Ticket
below.

## Evidence

```
cargo test -p pallet-x3-custody                         35 passed (13 new)
cargo test -p x3-chain-node --lib gateway_origin_tests   4 passed
cargo check -p x3-chain-node                             ok
cargo check -p x3-chain-runtime --features mainnet-rc1   ok
cargo clippy -p pallet-x3-custody --all-targets --all-features -- -D warnings   clean
cargo fmt --all --check                                  clean
```

End to end, with a node built from this revision:

```
X3_NODE_BIN=… bash scripts/mainnet/production_genesis_gate.sh
  [fixture-spec] spec ok: 17437793 bytes, 3 authorities, 3 bootnodes
  [genesis-gate] all three connected (peers: 2/2/2)
  [genesis-gate] all three finalized at least block 124 (a=124 b=124 c=124)
  [genesis-gate] PASS — production genesis builds, boots, finalizes and agrees at height 124
                 (0x03df83e142af8495fa5df705d514e0211f9fb3d80f1f58a3f79e34268e19007f)

python3 scripts/testnet/build-x3-testnet-spec.py 3 --skip-raw
  [spec] x3-testnet-plain.json written
  [spec] x3-testnet-plain.json loads in the node (build-spec --chain … -> ok)
  [OK] SUCCESS
```

The new custody tests are the ones that matter: an empty registry admits nobody; a role grants only
its own gate; `RuntimeOrigin::none()` and `RuntimeOrigin::root()` never pass a gate; `authorize_gateway`
and `revoke_gateway` are governance-only; revoking takes the privilege back immediately. The node tests
pin the relationship the other way round: the dev gateway accounts *are* the accounts of the two
published seeds, so a live chain naming them is refused.

## What this does not fix

* **TICKET-097.** `submit_finalization_result` is `ensure_none` and does not consult `X3LangOrigin`, so
  an anonymous peer can still plant a certificate anchor for the current block and finalize an
  `Executing` bundle against it. This change removes the *key*; that one is the *unsigned path*.
* **TICKET-105.** The node's atomic service still defaults its signing URI to `//x3-atomic-gateway`.
  Against a live chain whose genesis names a different account, the service's extrinsics are rejected —
  fail-closed, but silent until someone reads the log.
