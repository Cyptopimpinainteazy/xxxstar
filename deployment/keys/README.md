# Deployment Keys

## ⚠️ IMPORTANT — READ FIRST

**Do NOT commit real key material to this repository.**

This directory contains **templates** with placeholder values (`REPLACE_ME`, `REPLACED_RUN_KEY_ROTATION_SCRIPT`). Real keys must be generated locally and injected via:

- Environment variables
- Kubernetes Secrets
- HashiCorp Vault
- AWS Secrets Manager
- GitHub Actions secrets (for CI)

## Key Rotation Procedure

See `docs/SECRET_MANAGEMENT_POLICY.md` for complete instructions.

## Files

| File | Purpose | Production Source |
|---|---|---|
| `bootnode-keys.json` | Bootnode P2P identity keys | Generate with `subkey generate-node-key` |
| `bootnode-node-key` | Bootnode node key (raw hex) | Generate with `subkey generate-node-key` |

Both files are gitignored — never un-ignore them.