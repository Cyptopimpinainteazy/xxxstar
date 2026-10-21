# X3 OmniSwitch deployment

Status: first isolated unit identified and reset: OS6860E-P24, serial U3581156,
AOS 8.1.1.497.R01. Administrator access recovered, old custom accounts removed,
generated administrator password stored privately, defaults saved and certified.
Final reboot-persistence validation PASSED: the generated credential, account
cleanup and clean startup settings survived restart; chassis is Master/Running. The other two units have not been inspected. X3 topology and
production VLAN deployment remain pending operator review. Management SSH at
10.30.10.11 through x3star1 eno3 is verified after reboot; see
[SSH management](docs/ssh-management.md).

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
