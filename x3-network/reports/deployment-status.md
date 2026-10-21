# Current state — October 21, 2026 (America/Denver)

First console-connected spare: OS6860E-P24, serial U3581156, AOS 8.1.1.497.R01.
Approved startup configuration reset completed. Original working/certified
vcboot.cfg and vcsetup.cfg are archived at
/flash/x3-pre-reset-U3581156-20261021 (0700). Private host terminal exports have
SHA-256 manifests. Firmware and licensing material were not deleted. This is not
a secure erase of backups or logs.

Before/after checks: old VLAN 110, EMP 192.168.115.105 and old static routes are
absent. Old custom accounts were removed; only admin and built-in non-login
default template remain. A generated administrator credential passed a fresh
login and is stored ONLY outside Git:
/home/lojak/.local/state/x3-network/credentials-U3581156-r1n0kf1e/credentials.json
(mode 0600 in a 0700 directory). Never paste its content into chat or Git.

Defaults were saved and certified using the release-matched commands. Remote
management authentication is denied; console authentication is local. No X3
management IP, VLAN plan or inter-switch topology has been applied. The 2014
firmware is retained; supported firmware/security review remains necessary before
production deployment. Temperatures are 35°C, fan functional, PSU1 UP, PSU2 UNPLUG.
Master/Running and both modules UP were observed before the final reboot.

Final reboot persistence: PASS. Generated administrator login, account cleanup,
clean VLAN/address state, unchanged firmware version, certified/synchronized state
and Master/Running chassis were verified after the final reboot. Both modules
are UP. Full proof is in switch-U3581156-reset-proof.json; raw account and boot
responses remain private. The initial checker timed out amid startup messages;
its completion was explicitly rechecked, not treated as a password rejection.

Three-switch deployment acceptance: 0/3 (0%). Startup reset verified on 1/3 units;
this does not imply live network or production acceptance. No configuration change
or reset has been made on the other two units.

Files changed: x3-network only. Commands run and exact check states are recorded
in switch-U3581156-reset-proof.json and utility-verification.json; raw exports,
boot captures, account details and credentials remain in private host state.
No Rust/JS code changed; workspace application builds are outside this task's scope.
Utility verification: 8 unittest tests PASS, shell syntax PASS, inventory/report
parsing PASS, whitespace PASS and generated-credential exclusion PASS. Pytest is
not installed in the selected Python; unittest ran the same utility tests.
Live VLAN forwarding/isolation, SSH login, inter-switch links and server service
connectivity: NOT RUN. Mainnet or production readiness is not claimed.

Next 10 tasks:

1. PASS: final reboot-persistence and readiness verification for U3581156.
2. PASS: saved-state/security/console evidence and hashes recorded.
3. Connect and identify second physical switch without resetting an unknown unit.
4. Back up and obtain that second unit's isolated-reset approval; recover/reset it.
5. Identify, back up and obtain separate approval to recover/reset the third unit.
6. Reconcile actual server NICs, roles and all physical link endpoints.
7. Review supported firmware and upgrade path with vendor images and recovery proof.
8. Approve VLAN/IPAM/firewall/management/DNS topology after wider conflict checks.
9. Apply approved secure configuration sequentially and preserve console/SSH recovery.
10. Validate live forwarding, isolation, monitoring, authorized service paths and
    saved-config persistence across all three before deployment acceptance.

Historical checkpoints follow. Current observations above supersede earlier
statements about missing serial access, unknown identity, default passwords or no reset.

---

# Deployment checkpoint — October 20, 2026 (America/Denver)

Final objective remains BLOCKED. Latest resume: host serial prerequisites were supplied, but USB UART initialization fails. No switch has been reset or configured. No
release-specific reset sequence has been verified on hardware. There is no claim
of mainnet readiness. All work remains within the requested network mission.

| Phase | State | Verified evidence / next action |
|---|---|---|
| A discovery | BLOCKED, partial observations collected | UART now works at 9600/8N1; OS6860 login prompt observed; authentication required |
| B recovery/reset | NOT RUN | Identity, complete backup, release-matched instructions and per-unit approval required |
| C configuration | NOT RUN | VLAN and topology proposals prepared; port map and firewall approval missing |
| D deployment | NOT RUN | Requires B/C prerequisites and explicit disruptive-change approval |
| E switch validation | NOT RUN | Requires real deployed devices and approved persistence/failover window |
| F final documentation | NOT RUN | Preparation documents exist; deployed configs and final proof do not |

Verified facts:

- x3star1: Ubuntu 22.04.5 LTS, Lenovo product identifier -[546345Z]-.
- eno2: 192.168.0.70/24, MAC 08:94:ef:37:68:c3, 1000 Mb/s full duplex.
  Default gateway 192.168.0.1; physical switch/port unknown. Preserve this SSH path.
- eno1/3/4 and ens1f0/1 are link-down. USB Ethernet is up without an IP observed.
- CP2104 10c4:ea60, adapter serial 011C3902, cp210x driver, /dev/ttyUSB0.
  This is NOT the switch's serial number. Initial device open failed errno 13.
  Operator subsequently granted lojak rw ACL; latest open failed errno 5 before any bytes were sent.
- User lojak still lacks dialout membership, but a named rw ACL now grants access.
  Picocom 3.1 is now installed. Passwordless sudo remains unavailable.
- Kernel journal: `cp210x ttyUSB0: failed set request 0x0 status: -71` and
  `cp210x_open - Unable to enable UART`. Linux error 71 is EPROTO. USB enumeration
  and cp210x binding are present; failure occurs while enabling UART. Exact physical
  cause (cable, connector, adapter or other USB communication fault) is not established.
  ModemManager is active, but no port holder was reported by fuser; this does not prove
  it never probed the device. No service was stopped or USB controller reset.
- Docker subnets 172.17.0.0/16 and 172.18.0.0/16; Tailscale present.
- Resolver maps x3gpu1 to 192.168.0.30. Both that address and gateway answered 3/3
  ICMP packets, 0% loss, average RTT 0.260 ms and 0.636 ms respectively. No hardware,
  role or physical topology identification follows from these ICMP results.
- Other server interfaces and roles have not been remotely verified. Repository
  ansible example/test inventories are not a live server inventory.

Implemented / files changed:

Only new `x3-network/` files: three inventory files, three configuration README
files, four shell entrypoints and Python helper, utility tests, topology diagram,
reset/recovery/VLAN/port-map guides, README and reports. Existing staged and unstaged
user changes were not modified. No plaintext credentials were collected or stored.

Commands run / proof result:

- Git status, branch, HEAD and last ten commits inspected. Branch feat/x3-guardian,
  HEAD 1beb644603a0f1b6de8bb5882e699fdc99bb2e58; extensive pre-existing changes.
- lsusb, udevadm, device/sysfs inspection, id, read-only ip address/link/routes/rules/
  neighbors, ethtool eno2, DMI vendor/model, Docker network/IPAM, filtered Tailscale
  state, resolver and bounded 3-packet ICMP checks executed.
- dmesg: BLOCKED by kernel permissions; sudo -n: BLOCKED by password requirement.
  Read-only kernel journal access succeeded during resume and exposed the UART enable error.
- Initial discover-console.sh: exit 2, BLOCKED permissions/tools. After operator
  preparation, exit 0 with console_access NOT RUN: only file permissions/tool presence
  passed. The subsequent actual port open failed with EIO. No switch login occurred.
- validate-network.sh: exit 0, PASS only for local address/route/Docker overlap check;
  zero overlap with the nine proposals. Wider network approval remains BLOCKED.
- Python unittest: PASS, 8 tests, exit 0. Backup byte preservation/hash/private
  permissions, empty/symlink rejection, unreviewed/injected SSH input rejection,
  overlap rejection, scoped success, and failed-command handling covered.
- Shell syntax, Python compilation and inventory parsing: PASS, exit 0.
  Command results retained in utility-verification.json.
- `python3 -m pytest -q x3-network/tests`: BLOCKED, exit 1, pytest not installed in
  selected Python. Equivalent eight utility tests executed with unittest.
- Scoped marker scan found only unittest.mock in the isolated test module, allowed
  test infrastructure. No production stand-in or fabricated device result added.
- Rust/JS workspace checks: NOT RUN, not applicable to these standalone network
  scripts; no Rust or JS changed by this task. Shellcheck is unavailable.
- SSH switch collector and real configuration export/restore: NOT RUN. Backup
  utility is tested with isolated test data, not a real switch backup.

Private evidence (not Git):

- ~/.local/state/x3-network/console-20261021T052751Z-66m9frm7/discovery.json
- ~/.local/state/x3-network/address-check-20261021T052752Z-habzgemk/address-check.json

Timestamps in artifact names use UTC; report date uses America/Denver.

Remaining blockers / risks:

USB UART enable failure; unknown chassis identity, AOS, credentials,
active services, port map and saved state; unavailable full release-matched manuals;
remote IPAM/VPN/Kubernetes conflict checks; approved firewall policy and topology;
complete tested backups and recovery; per-unit reset and disruptive-change approval.
None can be inferred from a USB adapter or ping response. Physical and credential
inputs must be obtained locally, without passwords in chat or repository files.

Next 10 tasks, in dependency order:

1. PASS: operator granted this session serial rw access and installed picocom.
   Acceptance met for host prerequisites only; UART open itself is BLOCKED.
2. BLOCKED: restore USB UART communication, then inspect first chassis. Acceptance:
   UART opens without USB protocol errors, physical serial matches CLI, AOS and boot
   directories captured without reboot. Physical operator should reseat the console
   data cable, then try a known-good data cable/direct USB port if needed. Do not
   reboot the switch or unplug any server Ethernet/power cable.
3. Map active ports/services. Acceptance: cable labels, LLDP/MAC/VLAN/STP evidence
   identify management and production dependencies.
4. Export full recovery material. Acceptance: timestamped private backups with hashes
   and release-specific restore instructions, or explicit operator waiver.
5. Verify release-matched reset procedure. Acceptance: accessible complete vendor
   instructions preserve images, licenses, bootloader and recovery state.
6. Obtain named first-unit reset approval. Acceptance: chassis serial, service impact,
   window and recovery path explicitly authorized.
7. Reset first unit and verify defaults. Acceptance: real console/health evidence;
   no second-unit reset without its separate approval.
8. Approve addressing/topology/security policy. Acceptance: remote allocations checked,
   explicit physical port map, management path and firewall allowlist recorded.
9. Deploy sequentially across three approved units. Acceptance: independent console
   recovery, verified secure management and saved release-correct configurations.
10. Validate and finalize. Acceptance: VLAN forwarding/isolation, permitted service
    connectivity, LLDP/STP/links, health and approved reboot persistence proven on all
    three; sanitized final inventory, diagram, reports and private backups retained.

Completion percent: 0% of target switches accepted (0/3). Acceptance requires
verified identity, approved reset, secure configuration, recoverability and live
validation. This is a deployment acceptance count, not a project-completion estimate.
Commit: none; new preparation files remain untracked for review, hardware mission
still blocked. No agent reset, topology change, package installation or group change ran.
Operator-provided picocom installation and device ACL were observed during resume.

Previously completed operator host preparation (do not repeat unless ACL is lost):

```bash
sudo setfacl -m u:lojak:rw /dev/ttyUSB0
sudo apt install -y picocom
```

This narrow device ACL lasts until the device is recreated/unplugged. For durable
access, use the dialout/fresh-login procedure in the recovery guide. Never send the
sudo or switch password into chat. Once access is available, inspect the first unit;
request reset approval only after identity, backup and impact evidence are concrete.

Latest private USB diagnostic evidence: `/home/lojak/.local/state/x3-network/usb-uart-failure-20261021T055947Z-63n9cye3/evidence.json`.

Next executable action: operator reseats or replaces the USB console data cable
without disturbing switch power or server Ethernet; re-enumeration may require
reapplying the device ACL. Then resume the port-open check before any console
commands. Tasks 3–10 remain dependent on actual chassis inspection and approvals.
Existing utility test evidence is reused because no utility source changed.

## October 21, 2026 retry (America/Denver)

Adapter re-enumerated as USB device 33 at /dev/ttyUSB0 with the same CP2104
adapter serial. The previous named ACL was lost: root:dialout 0660, no lojak ACL.
Actual open now returns errno 13 (Permission denied). No bytes sent.
This permission failure prevents retesting the earlier UART EPROTO fault; that fault
is neither confirmed ongoing nor proven resolved. Noninteractive sudo remains
unavailable. Reapply the narrow device ACL locally, then retry console access.
Private evidence: `/home/lojak/.local/state/x3-network/console-retry-20261021T060057Z-ar0nnks5/retry.json`. No reset or device configuration changes.

## October 21, 2026 console success (America/Denver)

The latest operator ACL is present. The actual UART open and 9600/8N1 setup
succeeded. After one carriage return, 17 bytes arrived: `OS6860 login:` plus
line endings. The connection closed cleanly. No credentials, show commands, reset
commands or configuration commands were sent. Earlier EIO/EPROTO did not recur
in this attempt; this establishes current console communication, not long-term
USB stability. The banner identifies a family label, not exact model/serial/AOS.

Task 1 host prerequisites and console communication: PASS. Task 2 chassis
inspection remains BLOCKED at authentication. Tasks 3–10 remain dependent on
identity, backups, release instructions, topology and approvals. Three-switch
acceptance remains 0/3 (0%). No code changes or new commit in this retry.

Private proof: `/home/lojak/.local/state/x3-network/console-attempt-20261021T060335Z-uz7m2bre/result.json`
and adjacent response.raw (17 bytes). This supersedes earlier console blockers.

Next action: operator authenticates locally using known switch credentials:

```bash
picocom --baud 9600 --databits 8 --parity n --stopbits 1 --flow n /dev/ttyUSB0
```

After login, exit picocom with Ctrl-A then Ctrl-X without logging out of the
switch, then resume inspection. Do not send passwords into chat. If credentials
are unknown, stop at login and obtain official release-matched recovery guidance;
do not guess passwords or initiate a reset. The UART remains closed until resumed.

## New cable check — October 21, 2026

Operator reported a new cable. After a bounded 10-second device detection wait: BLOCKED: no USB serial device enumerated.
Previously successful console access is historical evidence, not proof of the new
cable connection. No login, reset or configuration action was attempted.
Private proof: `/home/lojak/.local/state/x3-network/new-cable-check-20261021T060532Z-0z05akfr/evidence.json`.

## Unknown password and reset request — October 21, 2026

Operator reports the switch password is unknown and requests a complete reset.
One ALE-documented factory-login attempt was rejected with an explicit invalid
login/password response. No repeated credential guesses, boot interruption,
reboot, password change or erase was attempted.

Proof: `/home/lojak/.local/state/x3-network/factory-login-check-20261021T061428Z-f7ydjucu/capture.json`.

The target is a configuration reset retaining firmware, licenses and recovery
material. Installed AOS and chassis serial remain unverified. The generic
reset-to-factory all scope described for newer AOS removes licenses/certificates,
so it does not satisfy this mission. Publicly indexed ALE documentation verifies
the factory credential and notes password persistence, but no installed-release
unauthenticated recovery procedure has been established. Bootloader protection
may independently restrict recovery; its presence on this unit is unknown.

Before any service interruption, obtain physical chassis model/serial, confirmation
that this specific console-connected unit is isolated from servers and inter-switch
links, and explicit acceptance of losing its old configuration without a backup.
Then arrange an approved boot capture to identify AOS/bootloader, obtain matching
vendor recovery guidance and choose the exact reset steps. No claim is made that
reset is currently executable or guaranteed to recover access.

References: ALE OS6860 Hardware Users Guide Rev V pp. 2-6 through 2-8 and AOS
8.10R3 Release Notes Rev A reset-to-factory section, as linked in reset-procedure.md.

## Reset authorization — October 21, 2026

Operator answered yes to: this is a spare switch that can be rebooted/erased
without affecting running servers, and loss of old configuration is acceptable.
This approval applies to the currently console-connected spare; do not request
the same authorization again. Preserve firmware, licenses and recovery material.
Exact recovery commands still depend on bootloader/AOS identification.

Latest preflight: console adapter present, but its named user ACL was lost again
(device recreated at 00:17 host time); noninteractive sudo setfacl failed because
a password is required. Boot capture has NOT started. No reboot or reset ran.
Next action: operator restores serial permission locally; establish durable
dialout membership to avoid repeated device-ACL loss, then arm passive boot capture
and guide one approved power cycle of this spare. Never power-cycle all switches.

## Approved boot capture — October 21, 2026

Operator power-cycled the approved isolated spare while passive capture was active.
Observed boot model OS6860E-P24M, U-Boot 2012.10 8.1.1.R01.70 armv7
(Jan 13 2014), boot image certified/Uos.img. Both FIT image CRC32 checks
reported OK; kernel decompression succeeded; switch returned to OS6860 login.
CRC is an observed integrity check, not authenticity proof. Running AOS release
and chassis serial were not exposed by this boot output. The observed model
differs from reported OS6860E-P24 and requires inventory reconciliation.

Private boot proof: `/home/lojak/.local/state/x3-network/boot-20261021T061925Z-xf_ozeky/boot.raw`; SHA-256 `17f09399325a415116a7ad26a5d3d449bc484487ab422e163233b6afab53d2e3`.
Passive capture result exited 0, 3507 received bytes, zero transmitted bytes.
No factory reset has run. A subsequent approved reboot capture is armed to
interrupt the displayed autoboot and issue only bootloader help.
