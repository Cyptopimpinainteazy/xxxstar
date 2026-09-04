# CosmWasm / IBC Rules — Knowledge Core

## Overview

These are the mandatory security rules for all CosmWasm smart contracts and IBC integrations in the X3 ecosystem. CosmWasm's actor model, IBC's packet lifecycle, and the interaction between them require specific security patterns. Every CosmWasm contract and IBC handler must comply with these rules.

## CosmWasm Rules

### Rule CW-1: Permission Checks

- Every contract entry point (`ExecuteMsg`, `SudoMsg`, `MigrateMsg`) must verify the sender's permissions before modifying state.
- Use `info.sender` for authentication. Do not trust `env` parameters that can be manipulated.
- Admin/owner actions must be gated by an access control check (`if info.sender != config.admin`).
- Use the `cw-ownable` or `cw-access-control` libraries for role-based access control.
- Never assume a message comes from a trusted source. Validate the sender against the expected address.
- Contract migrations must be gated by an admin check. Only the contract admin can migrate.

### Rule CW-2: Error Types

- All errors must be well-defined, documented, and exhaustive.
- Use `cosmwasm_std::StdError` for standard errors and define custom error types for contract-specific errors.
- Error types must implement `std::fmt::Display` and `std::fmt::Debug`.
- Errors must not leak sensitive information (private keys, internal state).
- Use `cosmwasm_std::StdResult<T>` for fallible operations. Do not unwrap or expect in production code.
- Error messages must be actionable. Include enough context for the caller to understand and fix the issue.

### Rule CW-3: Unchecked Math

- All arithmetic must use checked operations. No `+`, `-`, `*` without overflow/underflow checking.
- Use `Uint128`, `Uint256`, `Int128`, `Int256` for token amounts. These types implement checked arithmetic.
- Do not cast between integer types without bounds checking. `Uint128::u128()` can overflow if the value exceeds `u64` range.
- Use `checked_add`, `checked_sub`, `checked_mul` for all financial calculations.
- Use `Uint128::try_from()` or `u64::try_from()` for type conversions. Never use `as` for numeric casting.

### Rule CW-4: Reply and Callback Safety

- `Reply` callbacks are called when a sub-message returns a result. They must handle both success and failure cases.
- `Reply` must validate the sub-message ID to ensure it corresponds to the expected operation.
- `Reply` must not assume the sub-message succeeded. Always check `reply.result`.
- `Reply` must not leave the contract in an inconsistent state if the sub-message fails. Use the `reply` handler to clean up or revert state changes.
- `Reply` must not create reentrancy. Do not dispatch new sub-messages from a reply handler that modifies the same state.
- IBC callbacks (`ibc_channel_connect`, `ibc_channel_open`, `ibc_packet_ack`, `ibc_packet_timeout`) must validate all parameters before modifying state.

### Rule CW-5: Storage and State

- Use `cw-storage-plus` for storage. It provides type-safe maps and items.
- All storage keys must be namespaced to avoid collisions with other contracts.
- Use `Map::keys()` and `Map::range()` for iteration. Be aware of gas costs for large maps.
- Do not store unbounded lists in storage. Use pagination or bounded maps.
- State changes must be committed atomically. If a transaction fails partway through, no state changes should persist.
- Use `cw2::set_contract_version` to track contract version for migrations.

## IBC Rules

### Rule IBC-1: Channel and Sequence Validation

- Every IBC message must validate the channel ID and port ID against the expected values.
- Channel IDs must match the format `channel-N` where N is a non-negative integer.
- Port IDs must match the expected port (e.g., `transfer` for ICS-20, `wasm.<contract_address>` for custom ports).
- Sequence numbers must be monotonically increasing. A sequence number that is out of order indicates a replay or a bug.
- Do not accept IBC packets from unknown channels. Verify the channel against the channel state.

### Rule IBC-2: Timeout Handling

- Every IBC packet must have a timeout. No packet may remain in flight indefinitely.
- Timeout can be specified as a block height (`timeout_height`) or a timestamp (`timeout_timestamp`). At least one must be set.
- On timeout, the sending chain must revert the escrow and refund the sender. The `ibc_packet_timeout` handler must be implemented.
- Timeout values must account for the latency and finality of both chains. A timeout that is too short may cause valid packets to expire; a timeout that is too long may lock funds.
- ICS-20 transfers must use the `timeout_height` and `timeout_timestamp` fields. Do not set them to zero (which means no timeout).

### Rule IBC-3: IBC Packet Lifecycle

The IBC packet lifecycle has the following stages:

1. **Send**: The sending chain escrows the token and sends a packet.
2. **Receive**: The receiving chain processes the packet and writes an acknowledgement.
3. **Acknowledge**: The sending chain processes the acknowledgement.
4. **Timeout** (if no acknowledgement): The sending chain refunds the escrow.

Rules for each stage:

- **Send**: Escrow the token before sending the packet. The escrow must be atomic with the packet send.
- **Receive**: Validate the packet (denom, amount, sender, receiver). Mint the voucher token or credit the escrow. Write the acknowledgement.
- **Acknowledge**: If the acknowledgement is successful, the transfer is complete. If it is an error, refund the escrow.
- **Timeout**: If the packet times out (no acknowledgement received), refund the escrow. This must be deterministic and automatic.

### Rule IBC-4: Counterparty Verification

- The sending chain must not trust the receiving chain's state. It must verify counterparty information using IBC proofs.
- IBC proofs must be verified against the trusted header (client state) of the counterparty chain.
- The client state must be updated regularly. Stale clients may accept invalid proofs.
- Connection hops must be validated. A packet must travel through the expected connection path.
- Denom traces must be validated. The full denom trace (`ibc/<port>/<channel>/<original_denom>`) must be checked, not just the base denom.

### Rule IBC-5: ICS-20 Token Transfer

- ICS-20 transfers must use the standard `FungibleTokenPacketData` format.
- The `denom` field must be the full denom trace, not just the base denom.
- The `amount` field must be a valid Uint256 string (no negative amounts, no overflow).
- The `sender` and `receiver` fields must be valid bech32 addresses on their respective chains.
- Do not send ICS-20 packets with empty or invalid fields. Validate all fields before sending.
- When receiving tokens, escrow the source chain tokens and mint voucher tokens (or credit the escrow if the token originated on the receiving chain).
- When sending tokens back (burning voucher tokens), burn the voucher and un-escrow the source chain tokens.

## Cross-VM Integration

### Rule CW-6: X3 CosmWasm Integration

- CosmWasm contracts on X3 must comply with the UAK's canonical supply invariant.
- Cross-VM transfers from CosmWasm to other VMs must go through the UAK's validation layer.
- IBC packets from X3's CosmWasm to external chains must follow the IBC rules above.
- IBC packets from external chains to X3's CosmWasm must be verified against the counterparty's client state.
- The `pending` term in the UAK must include IBC packets in flight.

## Testing Requirements

### Rule CW-7: Testing Requirements

Every CosmWasm contract must have:

- **Unit tests** for all entry points (`instantiate`, `execute`, `query`, `migrate`, `sudo`).
- **Integration tests** for IBC handlers (`ibc_channel_open`, `ibc_channel_connect`, `ibc_channel_close`, `ibc_packet_receive`, `ibc_packet_ack`, `ibc_packet_timeout`).
- **Reply tests** for all sub-message reply handlers.
- **Fuzz tests** for functions that accept user input.
- **Migration tests** for contract upgrades.
- **Invariant tests** for critical invariants (total supply, escrow balance, channel state).

### Rule CW-8: Test Frameworks

- Use `cw-multi-test` for unit and integration testing.
- Use `ibc-test-kit` for IBC testing.
- Use `cosmwasm-vm` for VM-level testing.
- Use `proptest` for property-based testing.
- Use `cargo fuzz` for fuzz testing.

## Deployment Checklist

1. Contract binary is compiled in `cosmwasm-vm` compatible mode (no `std`).
2. Contract size is within the chain's limit (typically 800KB-1.5MB).
3. All entry points are tested and verified.
4. IBC channels are correctly configured.
5. Access control is set up (admin, owner, roles).
6. Contract version is set for future migrations.
7. UAK integration is tested for bridge contracts.
8. Events are emitted for all state changes.

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — Defines the canonical supply invariant. CosmWasm balances are the `cosmwasm` term.
- **UNIVERSAL_ASSET_KERNEL.md** — CosmWasm contracts must call the UAK for every asset movement.
- **CROSS_VM_ROUTING.md** — IBC and cross-VM routes must follow route specifications.
- **TRADING_SAFETY_KERNEL.md** — DEX and arb contracts on CosmWasm must comply with trading safety rules.
- **FORBIDDEN_PATTERNS.md** — Explicit list of forbidden contract patterns.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*