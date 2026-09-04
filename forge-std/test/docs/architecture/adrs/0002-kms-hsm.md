# ADR 0002 — KMS/HSM Integration for Bitcoin Signing

Date: 2026-01-11

Status: Proposed

Context

- The atomic swap relayer currently signs Bitcoin PSBTs using WIF private keys stored in configuration and payloads. This is insecure for production: keys in files or env vars are high-value secrets.
- Project policy requires HSM/KMS-backed key management for signing production funds.

Decision

Introduce an abstraction layer (KMS provider interface) for signing Bitcoin PSBTs that supports multiple providers:

- LocalFileKeystore (development/testing): stores encrypted key material in a file and exposes a simple sign interface.
- AWS KMS / Google Cloud KMS / Azure Key Vault adapters (future): use remote KMS APIs to sign digests using managed keys (asymmetric ECDSA/secp256k1 or ECDSA via raw signing where supported).
- HSM connector (PKCS#11) adapter (future): for on-prem HSM integration.

Key Design Points

- API: A KMS provider MUST implement signPsbt(psbt: Psbt, keyId: string): Promise<void> which attaches signatures to the PSBT inputs.
- Key material never leaves the provider. For cloud KMS that do not support PSBT-level signing directly, the provider will compute raw input digests and return signatures to be applied to the PSBT by the orchestrator.
- The relayer will prefer KMS signing when a `lock.kmsKeyId` is present in the settlement payload or a `KMS_KEY_ID` environment variable is set. WIF fallback remains for local testing only.
- Configuration: `RELAYER_KMS_PROVIDER` and `RELAYER_KMS_KEY_ID` environment variables select and configure the default provider.

Security Considerations

- Audit logging: All signing operations will emit audit events (key id, swap id, timestamp, operation result) without logging secrets.
- Rate-limiting / throttling: To protect KMS usage and conform to operation quotas.
- Protection against signing replay: Ensure per-input sighash types and PSBT contexts are validated before signing.
- Key rotation: Key identifiers are external to the code, keep rotation policy in operations docs.

Migration and Rollout

1. Add a local KMS provider implementation for developer and CI usage.
2. Add config flags to enable/disable KMS usage and to specify provider and key id.
3. Update `bitcoin-builder` to prefer KMS signing when available and add robust fallbacks.
4. Add unit and integration tests that exercise KMS signing using the local provider.
5. Post-deployment: run a canary flow with low-value funds and audit logs enabled.

Consequences

- The relayer becomes pluggable for key management and is ready for enterprise-grade key storage.
- Additional operational work will be required to provision and rotate keys in KMS or HSM.

