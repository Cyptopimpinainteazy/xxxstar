# X3 Atomic Star — Emergency Contacts

**Version:** 1.0
**Status:** Active
**Date:** 2026-06-09
**Classification:** Confidential — Distribution restricted to node operators and governance members

---

## 1. Purpose

This document lists the emergency contact roster for X3 Atomic Star mainnet operations. In the event of a critical incident (chain halt, security breach, bridge compromise, or governance emergency), the contacts below are authorized to coordinate response actions.

---

## 2. Contact Tiers

### 2.1 Tier 1 — Immediate Technical Response (24/7 On-Call)

These individuals have the authority to:
- Halt block production (via `scheduler.pause` if available)
- Trigger emergency governance proposals
- Coordinate with validator operators

| Role | Name / Handle | Signal / Keybase | Email | PGP Fingerprint |
|---|---|---|---|---|
| Lead Core Developer | TBD — placeholder | TBD | TBD | TBD |
| Security Lead | TBD — placeholder | TBD | TBD | TBD |
| Infrastructure Lead | TBD — placeholder | TBD | TBD | TBD |

### 2.2 Tier 2 — Governance Multisig Signers (3-of-5 Minimum)

These keyholders control the emergency governance multisig:

| Name / Handle | Address | Verification Method |
|---|---|---|
| Signer 1 | TBD — placeholder | PGP-signed address proof |
| Signer 2 | TBD — placeholder | PGP-signed address proof |
| Signer 3 | TBD — placeholder | PGP-signed address proof |
| Signer 4 | TBD — placeholder | PGP-signed address proof |
| Signer 5 | TBD — placeholder | PGP-signed address proof |

### 2.3 Tier 3 — Validator Operators

Primary validator operators for the public testnet and mainnet genesis set:

| Operator | Node Identity | Contact | Status |
|---|---|---|---|
| TBD — placeholder | TBD | TBD | TBD |
| TBD — placeholder | TBD | TBD | TBD |
| TBD — placeholder | TBD | TBD | TBD |

---

## 3. Communication Channels

| Channel | Purpose | Access |
|---|---|---|
| Signal Group: X3 Emergency | Real-time incident coordination | Tier 1 + multisig signers |
| Matrix Room: #x3-ops:matrix.org | Async coordination + post-mortems | All node operators |
| Email: security@x3atomicstar.io | External vulnerability reports | Public |
| GitHub Security Advisory | Coordinated disclosure | Restricted to repo admins |

---

## 4. Activation Criteria

An emergency contact MAY be activated when:

1. The chain has halted (no blocks produced for >5 minutes on mainnet, >30 minutes on testnet).
2. A critical security vulnerability has been confirmed (e.g., supply invariance violation, bridge double-spend).
3. An external bridge has been compromised.
4. A governance attack is in progress.
5. >1/3 of validators are offline simultaneously.

---

## 5. Escalation Procedure

1. **Detection:** Monitoring alerts or manual report.
2. **Verification:** At least two Tier 1 contacts independently verify the incident.
3. **Activation:** Tier 1 contacts notify the Signal group and initiate the incident runbook (`INCIDENT_RUNBOOK.md`).
4. **Coordination:** Tier 1 leads coordinate the response; multisig signers may be called to execute on-chain emergency actions.
5. **Resolution:** Incident is resolved; post-mortem is published within 72 hours.

---

## 6. Key Rotation and Access Control

- PGP keys must be rotated every 12 months or immediately upon suspected compromise.
- Multisig signer addresses must be verified on-chain and rotated if a keyholder changes.
- This document must be updated within 24 hours of any contact change.

---

## 7. Document Custody

- **Primary:** Committed to `docs/EMERGENCY_CONTACTS.md` in the repository (public version with placeholder identities).
- **Confidential version:** Stored in a separate encrypted repository with actual contact details.
- **Access:** Tier 1 contacts and governance multisig signers only.

---

## 8. Testnet Contacts

For public testnet incidents, the same escalation procedure applies but with relaxed time thresholds (e.g., chain halt triggers after 30 minutes instead of 5).

| Role | Name / Handle | Signal | Email |
|---|---|---|---|
| Testnet Lead | TBD — placeholder | TBD | TBD |
| Testnet Security | TBD — placeholder | TBD | TBD |