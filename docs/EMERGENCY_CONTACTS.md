# Emergency Contacts — X3 Atomic Star Mainnet

> **Status: Operational contacts published via on-chain identity + secure out-of-band channel.**
>
> This document does NOT contain plaintext identities, phone numbers, or email addresses.
> Real emergency contacts are published through the following secure processes:
>
> 1. **On-chain validator identities**: Each validator's `identity(display = ..., email = ...)` is set via
>    the Identity pallet and verified by the Council. Query at:
>    `./target/release/x3-chain-node --chain production --execution NativeElseWasm query pallet_identity identities`
>    (The production chain spec must be running or the raw spec used.)
>
> 2. **Encrypted operational file**: `emergency-contacts.gpg` (GPG-encrypted) is stored in a secure
>    repository accessible to the governance multisig signers. Decryption key is shared during the
>    genesis ceremony key ceremony.
>
> 3. **Incident escalation**: If the chain is halted and on-chain identity is unreachable, use the
>    secure messaging channels established during the genesis ceremony signer onboarding.

## Tier 1 — Immediate Technical Response (24/7 On-Call)

On-call rotation is managed through the validator set's on-chain identity.
In case of chain halt or critical vulnerability:

- **Primary channel**: On-chain `pallet_identity` → filter by `has_role: oncall`
- **Secondary channel**: Secure Matrix room (invite-only, established at genesis ceremony)
- **Fallback**: GitHub Security Advisory via `SECURITY.md`

**Do not store unencrypted phone numbers, email addresses, or messaging handles in this repository.**

## Tier 2 — Governance Multisig Signers (3-of-5 minimum)

Multisig signers are the five Council members elected on-chain.
Queries:

```bash
# Query current Council members (requires a running production node or archived state):
# ./target/release/x3-chain-node --chain production query pallet_council members

# Multisig address:
# ./target/release/x3-chain-node --chain production query pallet_multisig multisigs
```

## Tier 3 — Validator Operators

All active validators are discoverable via the Session pallet.
Each validator maintains an on-chain identity with an `email` field set to a PGP-encrypted
contact address.

```bash
# ./target/release/x3-chain-node --chain production query pallet_session validators
# ./target/release/x3-chain-node --chain production query pallet_identity identityOf <validator_account>
```

## Communication Channels

| Channel | Purpose | Access Control |
|---|---|---|
| On-chain identity (`pallet_identity`) | Discovery, contact info | Public (read), Council-verified (write) |
| Secure Matrix room | Real-time incident coordination | Invite-only, genesis ceremony participants |
| GitHub Security Advisory | Vulnerability disclosure | `SECURITY.md` process |
| GPG-encrypted file | Full contact roster | Governance multisig signers |

## Activation Criteria

Any of the following triggers emergency escalation:

- Chain halt (no blocks for >5 minutes)
- Security vulnerability (critical or above per CVSS)
- Bridge compromise (unauthorized mint or lock mismatch)
- Governance attack (malicious proposal execution)
- >1/3 validators offline simultaneously

## Escalation Procedure

1. **Detection**: Monitoring alert, validator operator report, or user report
2. **Verification**: Cross-check against at least 2 independent monitoring sources
3. **Activation**: Primary on-call acknowledges via on-chain remark or Matrix room
4. **Coordination**: Follow `docs/INCIDENT_RUNBOOK.md`
5. **Resolution**: Implement fix, test, and deploy via governance

## Key Rotation

PGP keys used for emergency contact encryption are rotated every 12 months or immediately after
any signer change. Rotation is coordinated during a governance motion.

## Testnet Contacts

Testnet validators use the same on-chain identity mechanism. For testnet-specific issues,
use the `#x3-testnet-operators` Matrix room.