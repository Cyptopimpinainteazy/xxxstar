# X3 OmniSwitch deployment

Status: two isolated units reset and verified after reboot:

- U3581156: generated administrator credential and saved state verified; SSH
  10.30.10.11 through x3star1 eno3 verified after reboot.
- U3581140: generated administrator credential, clean startup configuration and
  Master/Running with both modules UP verified after reboot. SSH/IP settings saved
  at 10.30.10.12; live SSH testing and management reboot
  persistence await the rear EMP-to-eno4 cable. See
  [switch 2 management](docs/ssh-management-switch2.md).

Both retain AOS 8.1.1.497.R01; firmware review remains necessary. The third unit
is uninspected. Production VLAN deployment is pending approval and live testing.
See [deployment status](reports/deployment-status.md) and
[SSH management for U3581156](docs/ssh-management.md).

Run from the repository root:

```bash
x3-network/scripts/discover-console.sh
x3-network/scripts/validate-network.sh
```

Exit codes: 0 means the stated local check or capture completed; 1 means an
address overlap; 2 means prerequisites are blocked or collection failed.
A successful capture does not establish switch health or deployment readiness.
Validation checks local addressing, all IPv4 routing tables and Docker IPAM.
It does not validate VLAN forwarding, firewall isolation or remote allocations.

Private evidence is saved under `~/.local/state/x3-network/` in mode 0700
folders with mode 0600 files. Do not commit raw exports or captures. The YAML
inventory files use JSON syntax, which is valid YAML 1.2; null means unknown.
Reported hardware and proposed roles are distinguished from observed facts.

To archive an existing operator-exported configuration:

```bash
x3-network/scripts/backup-switch.sh /absolute/path/to/export x3-switch1
```

This preserves bytes and records SHA-256, not completeness or recoverability.
Before reset, separately export running and saved configurations, VC state,
licenses/certificates as required by the release, and verify the recovery plan.

`collect-switch-status.sh HOST USER --reviewed-cli` captures a fixed set of
read-only status commands over SSH. First check every STATUS_COMMANDS entry
against installed AOS help. It requires an existing trusted host key and key/agent
authentication; it neither enrolls keys nor prompts for credentials. No credentials
are accepted on the command line. Output stays private. AOS command rejection may
arrive with SSH exit 0: review each response before drawing conclusions.
This collector has NOT RUN; the separate pinned-key/password SSH test and reboot
verification are recorded in reports/switch-U3581156-ssh-reboot-proof.json.

See [deployment status](reports/deployment-status.md), [console and recovery](docs/recovery-procedure.md),
[reset gates](docs/reset-procedure.md), and [VLAN proposal](docs/vlan-plan.md).
