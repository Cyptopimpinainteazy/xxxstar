# X3 CI Autopilot — foundation (v0.1)

This directory provides a **read-only diagnostic foundation** for X3's CI control plane.
It complements the existing `scripts/x3-cluster/x3jobs.py` scheduler, Ansible inventory,
`scripts/local-ci.sh` gate and GitHub Actions workflows. It does **not** replace them.

## Commands

```bash
python3 -m unittest discover -s scripts/ci -p test_autopilot.py -v
python3 scripts/ci/autopilot.py --local-only --output /tmp/x3-ci-report.json
# Inspect previously exported runner JSON without making a network request:
python3 scripts/ci/autopilot.py --runners-json /path/to/runners.json
# For operators with appropriate GitHub runner-list access:
GH_TOKEN="$(gh auth token)" python3 scripts/ci/autopilot.py --output /tmp/x3-ci-report.json
```

GitHub runner-list access may require repository administration permissions.
If unavailable, run `--local-only`; UNKNOWN does not mean healthy.
No tokens or credentials are included in the JSON report.
The GitHub workflow runs in local-only mode; it **does not prove runner registry status**.
The inventory reflects an earlier topology and must be reconciled with the newer
x3star2/x3star3 naming before automated placement or recovery is enabled.
No inventory hosts are contacted by this collector. `--local-only` is safe on
any machine with Python 3.10+.

## Operational safety

- Output is observations only. No service restarts, SSH, sudo, installs, cleanup or API writes.
- A GitHub runner being OFFLINE is **not** proof that the server is powered off.
- A host absent from the inventory can still be registered on GitHub.
- No self-hosted workflow on fork pull requests. Restrict secrets and permissions independently.
- Never trust a pull request workflow to approve its own promotion to production.
- Maintain existing required gates; this new workflow cannot replace them.

## Next stages — acceptance gates

1. **Reconcile inventory and runner registry:** operator-confirmed identities, unique per-host labels,
   SSH host key verification and Ansible ping. Never guess LAN IP addresses.
2. **Capacity service:** historical telemetry and measured job durations; schedule only within
   reserve CPU/RAM/disk limits and never over validator workloads.
3. **Build cache:** isolated `sccache` cache with measured hit rates, access policy and rollback.
4. **Execution isolation:** trusted-branch policies, ephemeral runner VMs/containers as appropriate,
   no production secrets in untrusted jobs.
5. **Failure investigator:** consume exact-run logs, classify CI vs infrastructure vs flakes,
   provide reproducer and evidence; no write authority by default.
6. **Candidate repair agent:** isolated checkout, test-first proof and **draft PR only**.
   Require independent reviewer and exact-head gates.
7. **Blockchain lab:** seeded fault injection, multi-node replay, consensus state-root checks,
   X3-Lang interpreter/JIT/AOT comparison and deterministic gas/state parity.
8. **Release admission:** preserve independent security, consensus and human approval gates.
   No missing runner or skipped test can count as PASS.

## Existing project integrations

- `scripts/x3-cluster/README.md` documents role-based job routing and evidence.
- `.github/workflows/local-ci.yml` is the existing required local CI path.
- `.github/workflows/x3-local-runner-smoke.yml` provides manual host smoke checks.
- `.github/workflows/x3-ci-autopilot.yml` tests this new collector on a trusted runner.

**Do not merge or deploy to rack servers until the unit test, workflow check,
and hardware/inventory review pass.**
