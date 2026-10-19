# Jacktheripper missing staged blob recovery

Source: x3star1 `/home/lojak/Desktop/jacktheripper`, HEAD `32062ebd7384f374a3b7f4a2e6ad85d8df805b9a`.

Implemented: preserved the exact raw index, all 190 present working files corresponding to index entries, 184 available indexed blobs and six missing indexed blobs recovered from `/home/lojak/x3-recovery/mirrors/x3jacktheripper.git`. Raw payloads and configuration remain private in `/home/lojak/x3-recovery-sources/preserved/jack-index-v2-20261019T025452Z`.

The first preservation attempt failed when `git cat-file` could not read an indexed blob; its partial archive remains at `/home/lojak/x3-recovery-sources/preserved/jack-index-20261019T025329Z`. Subsequent enumeration identified six missing blobs, not merely an invalid cache-tree pointer. The empty working log file does not match its staged version. A bounded read-only lookup across 90 captured local object stores completed without store-query failures and found all six exact blob IDs in the recovery mirror.

| Affected file | Recovered blob SHA |
|---|---|
| `.ai/runlogs/venue-expansion-and-the-quiet-ingest.md` | `780f6515d9fe031fbca73270c9f7302d89793f14` |
| `GAP_MATRIX.md` | `5e8466a8eca37b131a29331af5f1ff7922b3572f` |
| `config/jack.toml` | `b282d34e7fd245d841d49298d6e02ca44f368d64` |
| `crates/jack_chains/tests/venue_fees.rs` | `2ad8ae3e5cea0d07c7ee70aedd94d65939932b5f` |
| `crates/jack_graph/src/routes.rs` | `89149ff33c6925d315a384c7a129e9bf7f42b87f` |
| `crates/jack_ripper/src/main.rs` | `93b73725b4db111057e9c0bf0c25460e3427cdb2` |

Proof result: PASS for this limited recovery scope. In a new independent bare repository, all available blobs matched byte-for-byte, recovered payload hashes matched the six exact Git IDs, all 190 index entries matched the source index, and the restored raw index successfully wrote tree `1c518ee0a588e5f8497482c148f1261480fd7fad`. Archive SHA256 and private-permission checks passed. No source object store or index was repaired. Original HEAD, index, status and the 190 scoped working-file bytes were unchanged.

Commands run: read-only `git status`, `git ls-files --stage -z`, `git cat-file blob`, bounded local-store `git cat-file --batch-check`, and `git pack-objects`; `git index-pack`, `git hash-object -w`, and `git write-tree` only in a new verification repository. Source cached diff still exits 128 because the original remains damaged; restored write-tree exits 0. There is no claim that the original repository is fixed or that full repository fsck passed.

Files changed: new private archive and verification repository plus this sanitized metadata package. No original files or refs changed.

Remaining blockers: the source has 22,970 staged deletions. These are not instructions to delete working files. Full baseline history and staged-deleted working-file paths are excluded from this index-only preservation. The original repository still needs a separately reviewed repair strategy; production/runtime tests are NOT RUN. This local copy has no off-host disaster guarantee.

[Exact six-item recovery queue](RECOVERY_QUEUE.json) and [verification result](RESULT.json) supply provenance, affected files, risks and validation. The next 10 tasks and broader incomplete status are listed in the [primary recovery checkpoint](../2026-10-18-primary-working-and-object-preservation/README.md#recovery-queue-and-handoff). Overall completion percent is not assigned.
