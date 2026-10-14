# X3 Guardian — checklist truth audit (independent verification)

- Repo: `/home/lojak/Desktop/xxxstar-main`
- Branch / HEAD audited: `feat/x3-guardian` @ `fb75f265a`
- Auditor: independent verification pass (`/root/sgate2`), read-only on source
- Tracker audited: `docs/guardian/checklist.json` (194 items, 17 marked DONE)
- Spec of record: `/home/lojak/.codex/attachments/58a984aa-f333-4e95-9d15-eaa9d1346d5a/pasted-text-1.txt`

## 1. Verdict

`17/194 = 8%` is **not** an overstated number, but it is an **upper bound**, for two
independent reasons:

1. The denominator is incomplete — 9 of the spec's 66 sections have no checklist
   item at all (§1, §2, §4, §6, §29, §39, §57, §65, §66), including the section
   that defines the Security Gate / Trust Gate split (§2) and the "no mocks for
   production claims" rule (§39).
2. The numerator is *implemented-and-unit-tested but unwired*. Nothing that runs
   on X3 can consult Guardian today (§4 below).

No fake-green was found in the 17 DONE items: every DONE item is backed by real
code plus a real, named, passing test. One evidence string is mislabeled (§3).

## 2. DONE-item verification (each claim re-checked against code and tests)

Live evidence run:

```
$ cargo test -p pallet-x3-app-registry
test result: ok. 17 passed; 0 failed; 0 ignored
```

Test names are real (not renamed assertions): `only_guardian_can_certify_or_restrict`,
`certify_binds_to_exact_hashes`, `certify_only_accepts_current_version`,
`revoke_records_artifacts_and_blocks_recertification`,
`revoked_bytecode_is_tracked_fleet_wide`, `restrict_removes_privileges_and_keeps_history`,
`add_version_resets_to_under_review`, `submit_for_review_only_from_experimental`,
`address_binding_is_exclusive_and_releasable`, `unknown_application_is_rejected`,
`version_cap_enforced`, `register_rejects_overlong_name`, plus 4 framework/boilerplate
tests (`runtime_integrity_tests`, `test_genesis_config_builds`, ...). 13 behavioural,
4 boilerplate.

| id | claim | verdict | evidence |
|---|---|---|---|
| A1 | app-registry implements identity/hashes/status/versions/revocations/upgrades | SUPPORTED | `pallets/x3-app-registry/src/lib.rs` (829 lines) + `src/tests.rs` (485 lines); 17/17 tests pass |
| T1 | tier EXPERIMENTAL is permissionless/unendorsed | SUPPORTED | `lib.rs:96` `CertificationTier::Experimental`; `tests.rs` `register_creates_experimental_app` |
| T2 | tier UNDER_REVIEW | SUPPORTED | `lib.rs:98`; `tests.rs` `submit_for_review_only_from_experimental` |
| T3 | tier X3_VERIFIED only via GuardianOrigin | SUPPORTED | `lib.rs:100`, `lib.rs:602-609` (`T::GuardianOrigin::ensure_origin`); `tests.rs:291-302` proves a *signed* origin is rejected |
| T4 | tier RESTRICTED keeps history | SUPPORTED | `lib.rs:102`; `tests.rs:304-321` |
| R1 | app id/name/owner/version/VM/addresses | SUPPORTED | `ApplicationRecord` (`lib.rs:230-250`), `AddressOwners` index (`lib.rs:319-321`) |
| R2 | bytecode/source/manifest hashes | SUPPORTED | `ArtifactHashes`; `certify_application` rejects a hash mismatch with `Error::ArtifactHashMismatch` (`lib.rs:614`) |
| R6 | upgrade history | SUPPORTED | `Versions` DoubleMap (`lib.rs:302-313`); `add_version` resets tier to UNDER_REVIEW |
| BD1 | certification bound to artifact hash, never to name/owner | SUPPORTED | `lib.rs:616-621` (`VersionNotCurrent` + `ArtifactHashMismatch`) |
| FS1 | fail-safe: unknown != verified | SUPPORTED (in-pallet) | `lib.rs:791-795` `is_restricted` returns false for unknown; `lib.rs:797-803` `is_certified_artifact` returns **false** for unknown/revoked/unverified. But see §5 — these helpers have no caller outside the pallet |
| SP1 | certification cannot be forged | SUPPORTED | `tests.rs:291-302` |
| SP2 | certification cannot transfer to different bytecode | SUPPORTED | `tests.rs` `certify_binds_to_exact_hashes` |
| SP3 | old certification cannot validate new code | SUPPORTED | `add_version` → `UnderReview`; `certify_only_accepts_current_version` |
| SP4 | revoked apps cannot use Guardian privileges | SUPPORTED (in-pallet, unwired) | `lib.rs:805-819`; 3 tests. No external consumer — see §4 |

Every test name referenced in a DONE evidence string was confirmed to exist in
`src/tests.rs` (automated cross-check: zero unresolved references).

Not fake-green, worth noting as honest: `src/weights.rs` explicitly states the
weights are **not** benchmark-measured and must be replaced before mainnet. That
is the right way to ship provisional weights.

## 3. One mislabeled claim

| id | claim | problem |
|---|---|---|
| AUD-3 | "Baseline: workspace compiles" | Evidence is `cargo check -p pallet-x3-invariants finished clean in 26.6s` — that is a *single pallet*, not the workspace. The claim overstates its own proof. |

Action: replace the evidence with a real workspace check, or retitle the item to
"baseline: one pallet compiles".

## 4. Wiring: Guardian is not reachable from anything that runs

```
$ rg -n "x3_app_registry|x3_security_gate|x3_trust_gate|X3AppRegistry|X3SecurityGate|X3TrustGate" --glob '!target'
pallets/x3-app-registry/src/weights.rs:21   (its own doc comment)
pallets/x3-app-registry/src/tests.rs:24,32  (its own mock runtime)
```

```
$ rg -n "pallet-x3-app-registry|pallet-x3-security-gate|pallet-x3-trust-gate" --glob '*.toml'
  -> only the three crate manifests themselves
```

Consequences:

- No `construct_runtime!` anywhere includes the Guardian pallets.
- No runtime, node service, wallet, explorer, or other pallet depends on them.
- The fail-closed read helpers (`has_guardian_privileges`, `is_certified_artifact`,
  `is_artifact_revoked`, `is_restricted`) have callers only inside the pallet and
  inside its own tests. So "a revoked application cannot use Guardian-controlled
  privileges" is true as a *library* statement and unenforced as a *chain*
  statement.
- At `fb75f265a`, `pallets/x3-security-gate/src/lib.rs` and
  `pallets/x3-trust-gate/src/lib.rs` were empty skeletons (empty `Event` enum, no
  storage, no calls, no tests) that still compiled into the workspace as
  members-of-nothing. (Two parallel agents are implementing them now.)

## 5. Fail-open default worth hardening

`lib.rs:791`:

```rust
fn is_restricted(app_id: ApplicationId) -> bool {
    match Applications::<T>::get(app_id) {
        Some(app) => app.revoked || app.tier == CertificationTier::Restricted,
        None => false, // <-- unknown application reads as "not restricted"
    }
}
```

`is_certified_artifact` and `has_guardian_privileges` return `false` for an unknown
application (correct fail-closed). `is_restricted` returns `false` too, which reads
as *permissive* for a caller that only asks "is it restricted?". Either make the
return type `Option<bool>`/a typed error, or document that callers must first
establish existence. Not a false claim today (no caller), but it is a live footgun
for the wallet/explorer integration that §45/§46 require.

## 6. Checklist completeness vs the spec

Sections 1..66 of the spec, sections with **zero** checklist items:

| § | title | why it matters |
|---|---|---|
| 1 | Primary Principle | certification binds to an exact artifact + ruleset version |
| 2 | Core Security Model | defines the Security Gate / Trust Gate split the whole design rests on |
| 4 | Never Destroy Permissionlessness | the EXPERIMENTAL tier's reason to exist |
| 6 | Security Testing Pipeline | intake → reproducible build phases |
| 29 | User-Facing Security Display | must show provenance + limits, not a "safe" badge |
| 39 | No Mocks for Production Claims | matches this repo's NO FAKE GREEN rule |
| 57 | User Protection Without False Promises | anti-false-confidence requirements |
| 65, 66 | closing sections | uncategorized |

The denominator also has no item for the *integration* of the three pallets into a
runtime — `IN1..IN10` cover integrations with `x3-kernel`, `atomic-trade-engine`,
etc., but nothing owns "wire the Guardian pallets into `construct_runtime!` and
have at least one real path consult `has_guardian_privileges`".

## 7. `scripts/guardian_status.py` honesty check

```
$ python3 scripts/guardian_status.py
X3 Guardian: 17/194 complete  [##..........................]  8%
  done 17   doing 0   todo 177   blocked 0
  REMAINING: 177
```

- Denominator comes from the checklist file, not from what is finished. Correct.
- Percent uses floor division (`17/194` → 8%, not 9%). Conservative. Correct.
- `--mark ID DONE` with no `--evidence` leaves the evidence string empty and still
  writes `DONE`. A tracker whose whole purpose is "the count cannot drift" should
  refuse `DONE` without evidence. Recommended guard:

  ```python
  if status == "DONE" and not (args.evidence or match[0].get("evidence")):
      sys.exit(f"refusing to mark {item_id} DONE with no evidence")
  ```

- `--by-section` sorts section keys by string length then lexicographically, so the
  ordering is cosmetic-only, not numeric. Minor.
- `--mark` rewrites the entire file with `json.dumps(..., indent=2)`, which is the
  same formatting the file already uses. No churn observed.

## 8. Commands run

```
cargo test -p pallet-x3-app-registry              -> 17 passed / 0 failed
python3 scripts/guardian_status.py                -> 17/194, 8%
python3 scripts/guardian_status.py --by-section   -> per-section counts
rg -n "x3_app_registry|x3_security_gate|..." --glob '!target'
rg -n "pallet-x3-app-registry|..." --glob '*.toml'
python3 (ad-hoc) checklist cross-check vs src/tests.rs, section coverage 1..66
```

## 9. Remaining blockers (ranked)

1. `pallet-x3-security-gate` — was an empty skeleton (P0: forbidden placeholder).
2. `pallet-x3-trust-gate` — was an empty skeleton (P0: forbidden placeholder).
3. No runtime wiring for any Guardian pallet (P1: nothing enforces).
4. Checklist denominator omits 9 spec sections incl. §2 and §39 (P1: honest count).
5. AUD-3 evidence mislabeled (P2).
6. `is_restricted` fail-open for unknown app (P2: latent footgun).

## 10. Next 10 tasks

1. Land real `pallet-x3-security-gate` (storage/calls/events/errors/tests/weights).
2. Land real `pallet-x3-trust-gate` (privilege map + category trust policy).
3. Add the 9 missing spec sections as checklist items so the denominator stops
   omitting core requirements.
4. Add a checklist item owning "wire Guardian into a runtime".
5. Replace AUD-3 evidence with a real `cargo check --workspace` result.
6. Guard `guardian_status.py --mark DONE` to require evidence.
7. Type the `is_restricted` unknown-application case instead of `false`.
8. `x3-guardian-runner` (A4) — deterministic off-chain coordinator.
9. `x3-exploit-corpus` (A5) with the §41 per-entry fields.
10. §40 testing matrices (TM1/TM2/TM3) per VM.

---

# Post-landing addendum

Written after the implementation pass that followed this audit. Two things
changed underneath the audit: HEAD moved (a sibling agent landed §23 Security
Manifest work), and the two skeleton pallets were implemented.

## What landed

- `feat/x3-guardian` @ `d80b021ce` — "*feat(x3-guardian): real security gate and
  trust gate pallets*".
  - `pallets/x3-security-gate`: 18 tests pass, `--no-default-features` clean,
    clippy `-D warnings` clean, `cargo fmt --check` clean.
  - `pallets/x3-trust-gate`: 16 tests pass, same three checks clean.
  - Both reuse `pallet_x3_app_registry::{GuardianVm, ApplicationId}` rather than
    defining a second vocabulary for the same concepts.
- HEAD also advanced from `fb75f265a` to `48d989a22` while this audit was in
  progress: `e9376f82a` (lock the three new workspace members), `aade124f6`
  (§23 Security Manifest + canonical hashing + registration, +131 lines and +96
  test lines in the registry), `48d989a22` (M1–M4 marked done). The registry is
  now 960 lines, so the file:line citations in §2 of this report are accurate as
  of `fb75f265a`, not `48d989a22`.
- Nothing in those commits wire anything into a runtime, so finding §4 stands.

## Tracker changes made

| change | reason |
|---|---|
| A2 -> DONE | real pallet with 18 tests; evidence records the commit and the counts |
| A3 -> DONE | real pallet with 16 tests; evidence records the commit and the counts |
| R3 -> DONE | `StandardRefs` is stored per version and supplied on `add_version` — the code was there, the checkbox was not |
| R4 -> DONE | `tier` / `certified_at` / `next_review` are written by `certify_application` |
| R5 -> DONE | `RestrictionReason` + `restriction` + `restrict`/`revoke` extrinsics, with tests |
| 10 new items | the 9 spec sections that had no item at all (§1, §2, §4, §6, §29, §39, §57, §65, §66) plus an explicit `WIRE-1` item owning "wire the Guardian pallets into a runtime" |

Result: the denominator went 194 -> 204 and the numerator 17 -> 26, i.e. the
report line moved from `17/194 (8%)` to `26/204 (12%)`. Both directions of
correction matter: R3–R5 were finished work that was not claimed, and the ten new
items are required work that was not counted. Do not quote either number as
progress toward production readiness — §4 of this report (nothing enforces
Guardian on chain) and the absence of any live-node, EVM/SVM/X3VM or exploit
corpus evidence are what actually gate that.

## Revised blocker order

1. No runtime wiring for any Guardian pallet (P0: the gates cannot refuse anything).
2. `x3-guardian-runner` (A4) and `x3-exploit-corpus` (A5) do not exist (P1).
3. No live-chain evidence for any tier transition; all 26 DONE items are unit-level (P1).
4. `is_restricted` still returns `false` for an unknown application (P2).
5. `guardian_status.py --mark DONE` still accepts an empty evidence string (P2).
6. Weights for all three pallets are provisional, not benchmarked (P2).

## Commands run in this pass

```
cargo test -p pallet-x3-security-gate                    18 passed / 0 failed
cargo test -p pallet-x3-trust-gate                       16 passed / 0 failed
cargo check -p pallet-x3-security-gate --no-default-features   clean
cargo check -p pallet-x3-trust-gate --no-default-features      clean
cargo clippy -p pallet-x3-security-gate -p pallet-x3-trust-gate --all-targets -- -D warnings   clean
cargo fmt -p pallet-x3-security-gate -p pallet-x3-trust-gate -- --check   clean
cargo check --workspace                                  (recorded with the AUD-3 evidence)
python3 scripts/guardian_status.py                        26/204, 12%
```
