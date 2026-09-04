# X3 Secret Management Policy

## Critical Rule
**No secret material shall ever be committed to this repository.**

Secrets include:
- Private keys
- Mnemonic phrases
- API tokens
- Password/key material for bootnodes, validators, RPC endpoints, databases
- Cloud provider credentials
- CI/CD tokens

## Key Rotation (P0 — Execute Before Testnet)

### What was removed

The following files contained committed secret material and have been replaced with placeholders:

| File | Action | Replacement |
|---|---|---|
| `deployment/keys/bootnode-keys.json` | Secrets replaced with `REPLACED_RUN_KEY_ROTATION_SCRIPT` | Generate new keys, inject via env/secret manager |
| `deployment/keys/bootnode-node-key` | Raw hex key replaced | Generate new key, inject via env/secret manager |

### Rotation procedure

1. **Generate new bootnode keys** using `subkey`:
   ```bash
   subkey generate-node-key --file bootnode-node-key
   # public key printed to stdout
   ```

2. **Generate new p2p secret keys** for each bootnode:
   ```bash
   subkey generate --scheme sr25519
   ```

3. **Store generated keys** in a secure secrets manager (HashiCorp Vault, AWS Secrets Manager, Azure Key Vault, 1Password CLI, etc.) — NOT in this repo.

4. **Update Kubernetes Secrets** or environment variables with the new key material.

5. **Verify** that `deployment/keys/` contains no real key material.

### Git history purge

All prior commits containing secrets have been removed via git filter-branch:
```bash
git filter-branch --force --index-filter \
  "git rm --cached --ignore-unmatch deployment/keys/bootnode-keys.json deployment/keys/bootnode-node-key" \
  --prune-empty --tag-name-filter cat -- --all
```

If the repo is forked or mirrored, force-push the cleaned history:
```bash
git push origin --force --all
git push origin --force --tags
```

## Secret Detection

### Pre-commit hook (recommended)
Install `git-secrets` or `trufflehog` as a pre-commit hook to block accidental commits:
```bash
# git-secrets
git secrets --install
git secrets --register-aws

# Scan staged files
git secrets --scan
```

### CI gate
A secret scanning step runs in CI (`.github/workflows/ci.yml` or equivalent) that:
- Scans all files for regex patterns matching private keys, mnemonic phrases, API tokens
- Checks for `REPLACE_ME` or `FILL_IN` placeholders (allows them only in template files)
- Fails the build if any secret-like pattern is detected outside exempted directories

### Exempted directories
These directories are allowed to contain template/placeholder key files (no real secrets):
- `deployment/keys/` — template only, real keys injected at deploy time
- Any `.env.example` files — template only, `.env` is gitignored

## Secret Injection Methods

| Environment | Method | Example |
|---|---|---|
| Local dev | `.env` file (gitignored) | `BOOTNODE_KEY=<generated-key>` |
| Docker/K8s | Kubernetes Secrets mounted as env vars | `spec.containers[].envFrom[].secretRef` |
| CI/CD | GitHub Actions secrets | `${{ secrets.BOOTNODE_KEY }}` |
| Testnet | Vault agent injector sidecar | `vault.hashicorp.com/agent-inject: "true"` |

## Incident Response

If a secret is accidentally committed:
1. **Immediately rotate** the exposed secret — generate new key material
2. **Revoke** the old credential on any service or network that accepted it
3. **Purge** the secret from git history (see procedure above)
4. **Announce** via security channel — document what was exposed, when, and what was done
5. **Review** whether the exposure window requires upgrading the affected network

## Enforcement

This policy is enforced by:
- CI gate `check-secrets` step
- Dockerfile.mainnet-check Gate 6 (no hardcoded secrets check)
- `scripts/mainnet_release_gate.py` — secret validation