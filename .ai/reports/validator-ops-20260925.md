# The validator operator path: one verified, one that nothing ran

2026-09-25. The servers are not here yet, so this cycle went at the part of the roadmap that does not
need them: ROADMAP PRIORITY 9's rule that *"every operational runbook must be tested instead of merely
documented."* Two scripts matter for a new validator host — `install-validator.sh` and
`harden-validator.sh` — and they are in very different states.

## `install-validator.sh` — verified end to end, in five seconds

This one is already right. It was hardened earlier (a published release is required, the `.sha256`
asset is required and verified, a Live genesis with bootnodes is required unless
`--allow-non-live-chain` says otherwise), it has a `--check` mode that needs no root and writes
nothing, and `scripts/mainnet/validator_install_gate.sh` drives that mode through every way the
inputs can be wrong.

I ran it, because "there is a gate" and "the gate passes on this tree" are different claims:

```
[install-gate] node: target/release/x3-chain-node
[install-gate] built binary + Live genesis -> exit 0 (ok)
[install-gate] dev genesis refused -> exit 1 (fail)
[install-gate] dev genesis accepted with --allow-non-live-chain -> exit 0 (ok)
[install-gate] missing binary refused -> exit 1 (fail)
[install-gate] wrong --sha256 refused -> exit 1 (fail)
[install-gate] correct --sha256 accepted -> exit 0 (ok)
[install-gate] --from-release with no published release refused -> exit 1 (fail)
[install-gate] no source flag refused -> exit 1 (fail)
[install-gate] no --chain refused -> exit 1 (fail)
[install-gate] check mode wrote nothing under /tmp/.../prefix
PASS  (5.0 s)
```

So the path the seven hosts will take is exercised on this box. It is reachable through
`make mainnet-check` (`release gate (mainnet-check)`, in `GATES_RELEASE`) rather than the fast set,
because it needs a built `x3-chain-node`; adding it to the fast set would make it order-dependent on
whichever gate builds that binary first. That is a reason, not an excuse, and it is ticket 1 below.

## `harden-validator.sh` — cited as evidence, run by nothing, unsafe in two ways

`feature-matrix/consensus-l1.toml` lists this script under `paths` and `evidence`. Nothing in
`scripts/local-ci.sh` or any other gate ran it, and reading it turned up two things an operator would
have hit on the first host:

* **A live placeholder.** The firewalld branch passed `source address="YOUR-MGMT-CIDR"` to
  `firewall-cmd` — a literal string where a network belongs. The rule would match nothing, and the
  host would look configured.
* **An unconditional `ufw --force reset`.** Running the script discarded whatever firewall the host
  already had, without a flag, a prompt or a mention in the summary.

Both are the shape this repository keeps finding: a claim (a matrix entry, a "hardening complete"
banner) whose supporting script had never been run.

### What changed

`scripts/harden-validator.sh` now has the same shape as the installer:

* **`--check`** — needs no root, writes nothing, prints each of the six steps as a plan, and refuses
  when an input the plan needs is missing.
* **`--mgmt-cidr` / `X3_MGMT_CIDR` is required** for the firewalld branch, and checked for CIDR shape.
  Its absence is a refusal whose message names the placeholder it replaced, so the next reader learns
  why it is required rather than just that it is.
* **`--reset-firewall` / `X3_RESET_FIREWALL=1`** is what allows `ufw --force reset`; without it the
  rules are added to the existing configuration and the plan says so.
* **`X3_FIREWALL_TOOL=auto|ufw|firewalld|none`** to force a branch — which is how the gate exercises
  all four on one machine, and how an operator can see the plan for a host with the other tool.
* **`X3_HARDEN_ROOT`** prefixes every path the script touches, so "check mode wrote nothing" is
  something the gate asserts rather than assumes.
* The firewalld plan now prints the exact `firewall-cmd` command and rule it would apply, CIDR
  included.

`scripts/mainnet/harden_validator_gate.sh` drives all of it — four branches, a malformed CIDR, an
unknown argument, the reset opt-in, and the no-writes check — in about a second, and the new
`harden-validator check` gate runs it in the fast set.

The gate caught an inconsistency of mine on its second run: the plan said *"a rule scoped to
<cidr>"* without showing the rule, so the assertion on the actual `source address=` text failed. The
plan shows the command now. That is the technique working as intended.

`docs/STAGING_TESTNET_SETUP.md` and `docs/X3_DEPLOYMENT_POLICY.md` show the new invocation, including
the `--check` form.

## Tickets

1. **Promote the install gate to the fast set** once something there guarantees a `x3-chain-node`
   binary, or have the gate build one when it is missing. It is five seconds and it covers the path
   every new host takes.
2. **The rest of PRIORITY 9 for the seven servers**: sentry topology and bootnode generation are the
   two named items with no script yet that I could find; key generation/rotation exists
   (`inject-keystore.sh`, `node/src/validator_rotation.rs`, `validator-rotation-drill.sh`).
3. **The hardening script's steps are still only *planned* here.** sysctl, the SSH edits and logrotate
   have never been applied and verified on a real host, which is what the seven servers are for. The
   plan is the reviewable artifact until then.
