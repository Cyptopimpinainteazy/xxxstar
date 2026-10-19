# X3 lost-work recovery audit — 2026-10-18

Start with [the master report](MASTER_RECOVERY_REPORT.md) and [the ordered recovery queue](RECOVERY_QUEUE.md).

This is the forensic audit snapshot, published for review. It is **partial**, not a mainnet-readiness or integration claim. No original repository was repaired, reset, merged or cleaned up. The report's “no commit” statement refers to engineering/recovery activity at collection time; this publication commit contains audit documents and evidence only.

The inventory covers 177 candidate paths, 90 object stores and 617 primary-repository PRs. The queue contains 2,845 overlapping candidate records, not 2,845 distinct missing features. Unavailable hosts, invalid Git metadata, status timeouts and bounded catalogues remain explicit. Recovery actions are proposals and have not been executed.

## Evidence format

Large JSON/JSONL/CSV evidence files are gzip-compressed to preserve exact original bytes without adding hundreds of megabytes of repeated metadata to Git history. A filename mentioned in the report may therefore appear here with `.gz` appended. The complete queue is [RECOVERY_QUEUE.json.gz](RECOVERY_QUEUE.json.gz); decompress with `gzip -dc RECOVERY_QUEUE.json.gz` or Python's standard-library `gzip` module.

[EVIDENCE_INDEX.json](EVIDENCE_INDEX.json) records original and published sizes and SHA-256 hashes. Markdown navigation links and terminal-output trailing whitespace were adjusted for publication; exact originals of adjusted files are retained under `originals/`. [PUBLICATION_VALIDATION.json](PUBLICATION_VALIDATION.json) records publication checks. REPORT_VALIDATION.json and MANIFEST_VERIFICATION.json describe the earlier local audit validation, not new runtime or feature tests.

This package includes every queue item's cited evidence plus the canonical report analyses. The original SHA256_MANIFEST.json also inventories other local diagnostic/intermediate files and the borrowed-object Git evidence repository; those are not all included in this publication. No Git object database, private key or credential material is intentionally published here. Full original working trees and indexes remain on their source machines; report hashes are not source backups.

The original evidence directory is `/home/lojak/x3-recovery-sources/reports/audit-20261018/`. Source code recovery, source/index repair, branch integration, ref repair and cleanup require separate authorization. Read-only verification and investigation can continue.
