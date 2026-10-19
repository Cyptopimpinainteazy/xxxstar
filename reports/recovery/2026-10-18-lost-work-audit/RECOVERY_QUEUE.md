# X3 recovery queue

Generated 2026-10-18T20:11:32.712077-06:00. All actions below are proposals. No recovery, merge, ref repair or cleanup was executed.

Full queue: [RECOVERY_QUEUE.json](RECOVERY_QUEUE.json.gz). Counts are candidate records, not independent bugs or guaranteed unique features: {'P0': 1045, 'P1': 494, 'P2': 701, 'P3': 21, 'P4': 584}. Commit aliases, shared object stores and overlapping PR stacks intentionally retain provenance.

| Priority | Ordered action | Host/path | Exact commit(s) | Expected value, risk and validation |
|---|---|---|---|---|
| P0 | Preserve primary staged compiler/runtime and unstaged gateway/router work | x3star1 / /home/lojak/Desktop/xxxstar-main | 1beb644603a0f1b6de8bb5882e699fdc99bb2e58 | 188 status entries at initial capture; 58 staged and 122 untracked. Preserve index and worktree separately; compare with #623 before porting. |
| P0 | Preserve unreferenced history and broken copied worktrees | x3star1 / See unreferenced-commits-final.jsonl and repository-inventory.json | Exact SHAs per item | Protect object bytes and copied source before any ref repair, GC or cleanup; 15 copied worktree pointers target missing /home/x3star paths. |
| P0 | Preserve and diagnose jacktheripper index | x3star1 / /home/lojak/Desktop/jacktheripper | 32062ebd7384f374a3b7f4a2e6ad85d8df805b9a | Invalid cache-tree pointer; staged deletions are not authorization to remove files or reset the index. |
| P1 | Review duplicate local finality fixes | x3star1 / /home/lojak/Desktop/xxxstar-main | aa69621f2563dfaaf747c23370f891116844600d; 7c03615c5bddb44a71e35f8b671d2d27676775ca | Compare both patches and #592/#625/#634; validate tests/test_testnet_finality_status.py and real seven-validator evidence; port once. |
| P2 | Recover canonical trading changes through their stack | x3star1 / GitHub #623, then #628 | 2ae185d7e0f2ff19da48744a4acc425d9f67ec08; 218f44d58ea7b268c43cbdbd85c2a8442ea4acf2 | #628 descends from #623. Fix root/nested/parity/storage checks before proposing integration; compare primary staged work. |
| P2 | Review independent atomic rollback implementation | x3star1 / GitHub #624 | 324a9871decf9469d7db1927f327bac752e6b67a | Not an ancestor of #628; reconcile shared settlement and DEX behavior explicitly and prove rollback and conservation. |
| P2 | Review volunteer stack in dependency order | x3star1 / GitHub #626 → #627 → #629 | 9094f71e297323ef000c6c4e47945f79af17735a; eff2ec8b98ea133125bd9d081ab5d1d0668c0a5a; 53fd11bb584c12524ace858b22801775a5686bc7 | Ancestry confirmed. Reproduce lease failures, verify explicit consent and signed manifest rejection paths. |
| P2 | Review funding stack and published Grant Hunter v3 | x3star1 / GitHub #630 → #632 → #633 and codex/x3-grant-hunter-v3 | 6c2eec195e2068a49065f60b7dae7e4f2d474f6a; e394823a33fcbe4ed1106cb09ca45f401538a37f; 3afc3a6a7fbaa5adada8bf200faef16d62540253; 3ddedd61df18aca449b3aff9751e8020a8f32a90 | Local 1b6be1b has identical tools/x3-grant-hunter contents to published 3ddedd6. #631 is not an ancestor of #632; inspect closed variant separately. |
| P3 | Restore remote audit coverage | x3star1 / x3star2, x3star3, x3gpu1, x3gpu2, Tailscale x3star | UNKNOWN: hosts unavailable | Restore authorized access, verify identity, then capture refs and status before fetch; do not infer remote emptiness from connection failure. |
| P4 | Keep equivalent branches and duplicate sources pending approval | x3star1 / Prelaunch economics and copy/scaffold inventories | d1ea105588ee6678e5d51791778995a92fc171c8 | Both economics commits are patch-equivalent to master. Desktop x3-lang includes Cargo scaffold examples, not proof of a recovered compiler. No cleanup authorized. |

For each enumerated queue item, the JSON supplies host, exact source paths, SHA (or explicit null when no commit exists), affected files, expected value, risk, recommended action, validation procedure and evidence file. Empty affected-file lists mean the ref/host has not yielded a safe file diff; this is unverified, not an empty implementation.

Approval is required before any recovery action. The audit does not grant permission to change original refs, working trees, indexes or infrastructure.
