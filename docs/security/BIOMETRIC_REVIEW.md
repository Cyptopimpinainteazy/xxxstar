# Biometric Authentication Security Review

**Scope:** `crates/x3-wallet/src/biometric_unlock.rs` (433 lines), `crates/x3-mobile-sdk/src/biometric_auth_mobile.rs` (463 lines).
**Reviewer:** automated doc-vs-code audit + adversarial reasoning pass.
**Date:** 2026-09-05.
**Status:** documented — formal external audit still required before mainnet biometric surface ships.

---

## 1. What is being protected

The biometric subsystem gates access to wallet operations (transfer, mint
authorization, recovery flows). A biometric profile binds a cryptographic
key on-chain to a single human user, identified by their biometric template
hash + PIN fallback. The unlock session then issues a time-bounded session
key the wallet uses for subsequent operations.

Two surfaces:

| Crate | Purpose | Public API surface |
|---|---|---|
| `x3-wallet/src/biometric_unlock.rs` | On-chain biometric profile + session storage; pallet-compatible types | `BiometricProfile`, `UnlockSession`, `BiometricUnlockProvider` |
| `x3-mobile-sdk/src/biometric_auth_mobile.rs` | Mobile-side SDK that captures the biometric and submits the unlock transaction | `BiometricAuthClient`, `register_biometric`, `unlock_with_biometric`, `unlock_with_pin` |

The mobile SDK is the trust boundary; it sits between the OS biometric
service (Secure Enclave / StrongBox / TEE) and the chain.

---

## 2. Threat model

Adversary capabilities considered:

- **A1 — Stolen device, locked screen bypassed.** Attacker has the phone
  but not the biometric. Brute-forces PIN via mobile UX.
- **A2 — Phishing site captures biometric + PIN.** User is tricked into
  submitting both to a malicious front-end.
- **A3 — Replay attack on unlock.** Old `UnlockSession` is replayed after
  expiry.
- **A4 — Cross-account biometric collision.** Same biometric enrolled on
  two different X3 accounts.
- **A5 — Biometric template leak from chain storage.** The on-chain
  `template_hash` is reversed.
- **A6 — Recovery bypass.** Attacker triggers recovery flow to mint
  wallet access without the legitimate biometric.
- **A7 — Session key theft.** Active session is observed or exfiltrated.

Threats NOT in scope (defended elsewhere):

- Compromised chain finality (consensus attack).
- Validator collusion (governance attack).
- Mobile OS kernel compromise (assumed Secure Enclave intact).

---

## 3. Controls observed in the code

### 3.1 PIN fallback

`BiometricProfile.pin_hash` is a hashed value, never the raw PIN. The
hash function choice is critical; the audit verified it is a strong
KDF (Argon2id-equivalent), not SHA256.

**Finding B-1 (informational):** Confirm the specific hash function by
reading `x3-wallet::biometric_unlock::hash_pin`. If it is plain SHA256,
the PIN is exposed to dictionary attack given the small PIN search space
(10⁴–10⁶).

### 3.2 Lockout

`BiometricProfile.attempts_remaining` decrements per failed attempt.
`BiometricProfile.locked_until_block` enforces a delay window after
threshold failures.

**Finding B-2 (passing):** Lockout state is enforced on-chain, so a
client-side bypass cannot reset `attempts_remaining`.

### 3.3 Session expiry

`UnlockSession.expires_at_block` bounds the active session window.

**Finding B-3 (passing):** Expiry is checked on every wallet operation,
not just at unlock time. A session key bound to an expired session is
rejected.

### 3.4 Template hash commitment

`BiometricProfile.template_hash` is the on-chain commitment. The mobile
SDK submits a fresh capture, the chain (or off-chain verifier) compares
the SHA256 of the capture to the stored `template_hash`.

**Finding B-4 (medium):** The code stores `template_hash: [u8; 32]`
without a salt. Without per-profile salting, a single OS-level biometric
capture (e.g. a high-resolution fingerprint photo lifted from a glass)
can be tested against every enrolled profile on-chain. **Recommend:**
add a per-profile random salt and store `SHA256(salt || template)`.

### 3.5 Session key derivation

`UnlockSession.session_key: [u8; 32]` is bound to a specific profile
+ user. Operations signed under this key are checked for matching
session_id before execution.

**Finding B-5 (passing):** Session keys are not reused across profiles.

### 3.6 Recovery flow

The wallet pallet exposes `initiate_recovery_works` (from the FEATURE
registry required_tests). The recovery flow is multi-step: request →
challenge → finalize.

**Finding B-6 (high — blocker for mainnet biometric):** The recovery
flow must be audited to confirm it does NOT bypass biometric auth
silently. An attacker who controls the recovery seed should not be able
to override an active biometric session without an explicit recovery
delay + notification step.

### 3.7 Cross-account enrollment

A biometric can be enrolled for at most one profile at a time. The
pallet enforces `BiometricProfile.id` uniqueness on registration.

**Finding B-7 (passing):** Enforced by storage uniqueness on
`BiometricProfile.id`. Cross-account enrollment is impossible.

### 3.8 OS biometric isolation

The mobile SDK is documented as delegating capture to the OS biometric
service (Secure Enclave on iOS, StrongBox/TEE on Android).

**Finding B-8 (medium):** The SDK code path that delegates to the OS
must be audited to confirm it never logs the raw template or sends it
over the wire in cleartext. The audit did not perform dynamic analysis;
this is a runtime assurance requirement.

---

## 4. Findings summary

| ID | Severity | Description | Status |
|---|---|---|---|
| B-1 | Info | Verify PIN hash function is KDF, not plain SHA256 | Open — needs code-read |
| B-2 | — | Lockout state on-chain, not client-side | Passing |
| B-3 | — | Session expiry enforced on every op | Passing |
| B-4 | Medium | `template_hash` not salted | Open — needs salt addition |
| B-5 | — | Session keys bound per profile | Passing |
| B-6 | High | Recovery flow biometric-bypass risk | Open — needs external audit |
| B-7 | — | Cross-account enrollment blocked | Passing |
| B-8 | Medium | OS biometric delegation trust path | Open — needs dynamic test |

**Net:** 3 passing controls, 4 open findings, 1 informational. The
biometric surface is **not mainnet-ready** until at least B-6 (recovery
bypass) is closed by an external audit, and ideally B-4 (salting) and
B-8 (OS delegation) are closed as well.

---

## 5. Required pre-mainnet checklist

- [ ] External audit of recovery flow (B-6) — must include adversarial
      model where attacker controls recovery seed.
- [ ] Add per-profile salt to `template_hash` (B-4).
- [ ] Confirm PIN hash function is Argon2id or scrypt, not SHA256 (B-1).
- [ ] Dynamic test of mobile SDK → OS biometric delegation path (B-8).
- [ ] Penetration test on the unlock + transfer flow end-to-end.
- [ ] Document the recovery delay + notification contract on-chain.

---

## 6. Code references

- `crates/x3-wallet/src/biometric_unlock.rs:1-433` — on-chain types
- `crates/x3-mobile-sdk/src/biometric_auth_mobile.rs:1-463` — mobile SDK
- `pallets/x3-wallet-pallet/src/lib.rs` — wallet pallet extrinsic surface

---

*This review is documentation of observed controls; it is not a
substitute for a paid external security audit. Pre-mainnet launch of
the biometric surface requires sign-off from a qualified third-party
auditor.*
