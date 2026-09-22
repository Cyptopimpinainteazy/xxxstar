# The public-launch kit, and a lesson about editing running scripts

Date: 2026-09-22. Base: `origin/master` = `4c4e21c417`.

The local testnet is complete and gated. A *public* one needs artifacts that exist
before anything launches — a bootnode address its validators can dial, a spec carrying
that address, and a runbook an operator can follow. That is what this adds, plus the
discovery that the previous turn's soak had died for a silly reason.

## The kit

* **`scripts/testnet/public-node-id.sh`** — creates (0600, gitignored) or reuses a
  bootnode identity, derives its peer id with the same helper the spec builder uses, and
  prints the `/dns4/<host>/tcp/<port>/p2p/<peer>` address to publish. Same key file,
  same peer id across runs.
* **`PUBLIC_BOOTNODES=/dns4/…/p2p/…`** (comma-separated) — `build-x3-testnet-spec.py`
  now uses those published addresses as the spec's `bootNodes` instead of the derived
  loopback entries:

  ```
  $ OUT_DIR=/tmp/x3-public-spec PUBLIC_BOOTNODES=/dns4/bootnode.testnet.example/tcp/30333/p2p/12D3KooWHMKP… \
      python3 scripts/testnet/build-x3-testnet-spec.py 3
  [spec] using 1 published bootnode(s) from PUBLIC_BOOTNODES
  bootNodes: ['/dns4/bootnode.testnet.example/tcp/30333/p2p/12D3KooWHMKP…']
  ```

* **`SKIP_BOOTNODE_MEMBERSHIP_CHECK=1`** — the multi-host case: the spec's bootnode is a
  host these validators dial, not one of them, so requiring each local validator's peer
  id to be among the bootnodes is wrong there. The authority check stays on: verified by
  starting three validators from the published-bootnode spec on their own ports —
  `[validate] ok: … every launcher key (Aura 3, GRANDPA 3) is in the authority sets`,
  then `[validate] bootnode membership check skipped (…)` — all three came up and formed
  a mesh through the launcher's CLI bootnode.
* **`docs/reports/PUBLIC_TESTNET_LAUNCH.md`** — the operator runbook: identity → spec →
  per-host validator start → ceremony record/verify → gates, with the remaining
  infrastructure (hosts, DNS, TLS, monitoring, faucet, explorer, signed publication)
  named as outstanding. `TESTNET_DEPLOYMENT_CHECKLIST.md` points at it now and keeps its
  168 unchecked infrastructure boxes.

## The lesson: do not edit a script that is running

The two-hour soak died at launch with
`run-7-validators-local.sh: line 650: syntax error near unexpected token 'do'` — *after*
it had started four validators. The cause: I patched that script **while the soak's
launcher instance was still executing it**. Bash reads scripts incrementally, so the
running shell's file offset no longer pointed at a statement boundary.

Nothing was lost except the soak's window (the nodes were cleaned up by its trap), and
it is now restarted on isolated ports. The rule for this repo, where runs last minutes
to hours: patch source in a different file, or wait.

## Rows

| row | before | after |
| --- | --- | --- |
| `X3-L1-010` bootnode / peer discovery ops | 65/45/35 | **75/50/40** — publishable identity + published-bootnode specs; no host is running one yet |
| `X3-OPS-003` public testnet gate | 75/60/45 | **80/65/45** — the operator path up to "point the gate at a hosted endpoint" exists and is exercised; mainnet-ready unchanged |
