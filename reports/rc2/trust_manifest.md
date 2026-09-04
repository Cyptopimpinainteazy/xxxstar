# RC2 Trust Manifest

Generated: 2026-05-12

## Status

RC2 internal settlement proof is backed by a fresh remote-sourced backup bundle and evidence archive.

## Commit And Tag

- Repository: `Cyptopimpinainteazy/x3-atomic-star`
- Branch: `main`
- RC2 commit: `5a45ce4dbfc26cfd25ecb9e1c5ce88f63bac7106`
- RC2 tag: `x3-atomic-star-rc2-internal-settlement-smoke`
- Tag target: `5a45ce4dbfc26cfd25ecb9e1c5ce88f63bac7106`

## Backup Location

Local backup directory:

```text
/home/lojak/Desktop/X3_RC2_BACKUPS
```

Durable backup artifacts:

- `x3-atomic-star-rc2-main.bundle`
- `x3-atomic-star-rc2-evidence.tgz`
- `SHA256SUMS`
- `bundle-verify.txt`
- `RC2_TRUST_MANIFEST.md`
- `remote-refs.txt`

## Verification Performed

- Fresh shallow bare clone was created from the remote `main` branch.
- `git fsck --full --strict` on that independent clone exited `0`.
- `git bundle verify` on the RC2 bundle exited `0`.
- Bundle contains `refs/heads/main` at `5a45ce4dbfc26cfd25ecb9e1c5ce88f63bac7106`.
- Bundle contains annotated tag `x3-atomic-star-rc2-internal-settlement-smoke`; the peeled tag target is `5a45ce4dbfc26cfd25ecb9e1c5ce88f63bac7106`.

## RC2 Smoke Evidence

`reports/rc2/internal_settlement_smoke_report.md` records:

- Verdict: `PASS`
- All six internal settlement routes: `PASS`
- External Ethereum route rejected: `PASS`
- Final supply invariant: `PASS`
- Blockers: none

## Checksums

```text
8538b4bfda9780ad3d4c1e22db44c98b28ba0b1bc5799400d5d382abe12635a0  x3-atomic-star-rc2-main.bundle
b4dbd1b2631645f25d23f997e56e6b75b9a6f86215f3fd9b63249a72f536810e  x3-atomic-star-rc2-evidence.tgz
f6bd5d7c1066789f51fca8f12a4d269e5202708047f48a6aedbae02a24310d2f  /home/lojak/Desktop/X3_ATOMIC_STAR/target/rc2-current-node/debug/x3-chain-node
ae113dd11c83cc0c4261e4e87d4b254d0eb01f3a7b6ad945fa128733f2030ecd  bundle-verify.txt
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  rc2-shallow-fsck.txt
0c45bc47b20ad5e4732911987ae49d956e5c9bb50841fee197ef71b464302eaf  remote-refs.txt
```

## Local Trust Note

`/home/lojak/Desktop/X3_ATOMIC_STAR_RECOVERED` is clean at the pushed RC2 commit, but it shares the original repo common Git object directory at `/home/lojak/Desktop/X3_ATOMIC_STAR/.git`. Because the original object store previously showed corruption symptoms, use the remote-sourced bundle under `/home/lojak/Desktop/X3_RC2_BACKUPS` as the local trust anchor.
