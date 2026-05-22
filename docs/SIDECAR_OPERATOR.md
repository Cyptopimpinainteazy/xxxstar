# X3 Sidecar Operator Guide

Operational reference for running the `x3-sidecar` binary — the Cross-VM bridge
daemon that relays Solana escrow events into the X3 runtime.

---

## 1. What the sidecar does

1. **Polls Solana** every `--poll-interval-secs` (default 30 s) for escrow
   program accounts via `getProgramAccounts`.
2. **Checks finality** — skips accounts whose `created_slot` is less than 32
   slots old.  When `created_slot` is unknown (zero), uses global slot as proxy.
3. **Derives a Blake2b-256 state root** over `account_data || created_slot_le`.
4. **Submits a SCALE V4 signed extrinsic** calling
   `X3CrossVmRouter::register_external_root` on the X3 node.
5. **Re-fetches chain metadata** (`spec_version`, `transaction_version`,
   `genesis_hash`) every `X3_META_REFRESH_SECS` seconds (default 300 s) and
   logs a warning when a runtime upgrade is detected.

---

## 2. Who can submit: `ExternalExecutorOrigin`

The runtime configures `ExternalExecutorOrigin = EnsureRootOrHalfCouncil`.
This means the signer account in `X3_SIGNER_SEED_HEX` **must** be one of:

| Option | Requirement |
|--------|-------------|
| **Sudo / root key** | The account that holds sudo privileges on the chain |
| **Council member** | Any account that is a current council member; must meet ≥ half-council threshold for batch calls |

If neither condition is met, the transaction pool will reject the extrinsic with
`BadOrigin`.  Contact the chain governance team to add your sidecar key to the
council, or to rotate the sudo key.

---

## 3. Signer key setup

### 3.1 Generate a key

```bash
# Using subkey (Substrate tool)
subkey generate --scheme sr25519
# Output includes:
#   Secret seed:       0xABCD...
#   Public key (hex):  0x1234...
#   SS58 Address:      5GrwvaEF...
```

Use the **Secret seed** (32 bytes, 64 hex chars) as `X3_SIGNER_SEED_HEX`.

### 3.2 Fund the account

The signer account needs a small balance to pay transaction fees.  Send at least
`0.01 X3` (adjust for your fee market) to the SS58 address before starting the
sidecar.

### 3.3 Grant authority

**Sudo path** (devnet / testnet only):
```bash
# Via Polkadot.js Apps → Extrinsics → sudo → sudo(call)
# Call: council.setMembers([<sidecar_ss58>])
```

**Council governance path** (production):
Submit a council motion to add the sidecar account, then have council members
vote.  The sidecar's first extrinsic will succeed once the motion passes.

---

## 4. Environment variables

| Variable | Default | Required | Description |
|----------|---------|----------|-------------|
| `X3_SIGNER_SEED_HEX` | *(none)* | **Yes** (production) | 32-byte sr25519 mini-secret as 64 hex chars. Without this the sidecar sends unsigned extrinsics — accepted only on patched devnets. |
| `X3_SOLANA_RPC_URL` | `https://api.mainnet-beta.solana.com` | Yes | Solana JSON-RPC endpoint |
| `X3_NODE_RPC_URL` | `http://127.0.0.1:9944` | Yes | X3 node WebSocket/HTTP RPC |
| `X3_ESCROW_PROGRAM` | *(empty)* | Yes | Base58 program ID of the Solana escrow program. If empty, Solana polling is disabled. |
| `X3_SOLANA_CHAIN_ID` | `2` | No | chain_id passed to `register_external_root` |
| `X3_BRIDGE_PALLET_INDEX` | `26` | No | SCALE pallet index of `X3CrossVmRouter` |
| `X3_BRIDGE_CALL_INDEX` | `4` | No | SCALE call index of `register_external_root` |
| `X3_META_REFRESH_SECS` | `300` | No | How often to refresh chain metadata (spec_version etc) |
| `X3_SIDECAR_BIN` | auto | No | Override binary path used by the node launcher |
| `RUST_LOG` | `info` | No | Log filter (`debug` for verbose output) |

### 4.1 Secure secret storage

**Never hard-code `X3_SIGNER_SEED_HEX` in source or config files.**

Recommended approaches (in order of preference):

1. **HashiCorp Vault / AWS Secrets Manager** — inject at container start:
   ```bash
   export X3_SIGNER_SEED_HEX="$(vault kv get -field=seed secret/x3-sidecar)"
   ```

2. **systemd `EnvironmentFile`** (Linux service):
   ```ini
   [Service]
   EnvironmentFile=/etc/x3/sidecar.env   # chmod 600, owned by x3-sidecar user
   ```

3. **Docker secrets**:
   ```bash
   docker run --secret x3_seed \
     -e X3_SIGNER_SEED_HEX="$(cat /run/secrets/x3_seed)" \
     x3-sidecar
   ```

---

## 5. Key rotation without downtime

1. **Generate a new sr25519 key** (Section 3.1).
2. **Fund the new account** before the switchover.
3. **Grant authority** to the new key (council/sudo — Section 3.3).
4. **Update the secret store** with the new seed.
5. **Restart the sidecar** — it picks up the new key on startup.
6. **Revoke authority** from the old key via governance.

During steps 4–6 there is a brief gap where the old key is no longer running
and the new key is not yet submitting.  For production systems with strict SLAs,
run two sidecar instances (old + new) for one poll cycle overlap before
removing the old one.

---

## 6. Runtime upgrades

When the X3 runtime is upgraded, `spec_version` and `transaction_version`
change.  Extrinsics signed with stale values are rejected.

The sidecar handles this automatically via `MetaCache`:
- Re-fetches `state_getRuntimeVersion` and `chain_getBlockHash(0)` every
  `X3_META_REFRESH_SECS` seconds (default 5 minutes).
- Logs a `⚡ Runtime upgrade detected: spec_version X → Y` warning on change.

**To force immediate refresh** after a scheduled runtime upgrade:
```bash
# Send SIGTERM to gracefully stop, then restart:
systemctl restart x3-sidecar
# OR reduce refresh interval temporarily:
X3_META_REFRESH_SECS=10 x3-sidecar ...
```

---

## 7. Health monitoring

Log lines to watch:

| Pattern | Meaning |
|---------|---------|
| `✅ tx=0x...` | Extrinsic accepted by X3 node |
| `❌ submit failed: ...` | Extrinsic rejected — check origin/nonce/spec_version |
| `⚡ Runtime upgrade detected` | Spec version changed; metadata refreshed |
| `X3_SIGNER_SEED_HEX not set` | Running in unsigned mode (devnet only) |
| `chain meta unavailable` | Node RPC unreachable at startup; using defaults |
| `getSlot: ...` | Solana RPC unreachable; poll cycle skipped |

Recommended alert: fire on `❌ submit failed` in the last 10 minutes.

---

## 8. Pallet indices reference

These are hardcoded in the runtime `construct_runtime` macro:

| Pallet | Index | Relevant call | Call index |
|--------|-------|---------------|------------|
| `X3CrossVmRouter` | 26 | `register_external_root` | 4 |

Override via `X3_BRIDGE_PALLET_INDEX` / `X3_BRIDGE_CALL_INDEX` if these change
in a future runtime upgrade.

---

## 9. Troubleshooting

### `BadOrigin` on submit
- Signer account is not sudo or council member → Section 3.3.
- Account has insufficient balance for fees → fund it.

### Extrinsic rejected: `InvalidTransaction(Transaction is outdated)`
- Stale `spec_version` — trigger a metadata refresh (Section 6).

### `nonce fetch: RPC error`
- X3 node RPC is unreachable.  Check `X3_NODE_RPC_URL`.

### Sidecar exits immediately on startup
- `X3_SIGNER_SEED_HEX` contains invalid hex or wrong length → regenerate key.

### `getSlot: HTTP getSlot: error sending request`
- Solana RPC endpoint is unreachable.  Check `X3_SOLANA_RPC_URL` and network.

### All accounts are skipped with `age N < 32`
- Solana is healthy but no accounts have reached finality yet — normal at startup.
  Wait 32 slots (~14 seconds on mainnet-beta).
