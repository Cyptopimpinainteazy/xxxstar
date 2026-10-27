# Reset procedure

Serial U3581156 has completed the approved startup reset, account cleanup and
post-reboot verification. The executed AOS 8.1.1.497.R01 procedure is recorded
below. These preparation gates still apply independently to the remaining units.

1. Establish console access and correlate physical label, model and chassis serial
   with CLI identity. Record AOS, boot source, running directory, working/certified
   contents, VC membership, power/fan/temperature health, and firmware error logs.
2. Inspect running and saved configuration using installed help. Identify VLANs,
   management addresses, tagged ports, routing, active links, MAC and LLDP tables,
   spanning tree and every attached service. Keep secrets outside ordinary logs.
3. Export timestamped complete backups outside Git, hash them, and record a
   release-matched restore procedure. The local backup utility only archives an
   existing export; it does not certify a full recovery backup.
4. Identify service owners and the host's current SSH path. Obtain explicit
   operator approval naming ONE chassis serial, expected impact, maintenance window,
   backup location or intentional waiver, and recovery path. A general "go" does
   not authorize an unidentified unit's destruction.
5. Obtain the vendor reset instructions matching its exact installed release.
   Review preservation of licenses, firmware, bootloader and recovery material.
   Do not improvise filesystem deletion. Do not reset a stack as an isolated unit.
6. Execute only the approved sequence on that unit with console recovery retained.
   Verify default state and health. Keep it isolated from live VLANs until reviewed
   configuration is ready. Repeat gates separately for each remaining switch.

ALE's indexed 8.10R3 release notes introduce `reset-to-factory` with different
scopes. The `all` scope removes licenses and certificates and conflicts with this
mission's preservation requirements. The documented `config` scope retains those
items, but MUST NOT be used until installed release, platform support and exact
behavior are confirmed. Earlier releases require their own procedure. Removing
only vcboot/vcsetup files is not proof of a complete factory reset or credential reset.
The generic reset-to-factory command is not used on the observed 8.1.1 release.
Use the executed release-specific record below; other units require fresh identity
and release checks before applying any reset steps.

Vendor references consulted through search indexing:

- [OS6860 Hardware Users Guide, Rev V, July 2024](https://www.al-enterprise.com/-/media/assets/internet/documents/omniswitch-6860-hardware-guide-rev-v-en.pdf), pp. 1-2, 2-5.
- [OS6860 datasheet](https://www.al-enterprise.com/en/-/media/assets/internet/documents/omniswitch-6860-datasheet-en.pdf), console and port specifications.
- [AOS 8.10R3 release notes, Rev A, June 2025](https://www.al-enterprise.com/-/media/assets/internet/documents/omniswitch-aos-release-810r3-release-notes-rev-a-en.pdf), reset-to-factory section, p. 23.

Direct hardware-guide retrieval returned HTTP 404 during this run; indexed
excerpts supplied the hardware facts. Obtain accessible complete vendor manuals
for the actual installed release before reset or recovery. Search excerpts are
not sufficient approval evidence for a destructive command sequence.

## Executed on serial U3581156, AOS 8.1.1.497.R01

This is the release-specific record for ONE approved isolated spare, not blanket
authorization for the two other units. Operator accepted reboot and loss of old
configuration. Chassis identity was verified before changes.

1. Reached bootloader through its displayed autoboot interruption. Verified
   setAdminPasswordDefault through device help; invoked it once, then followed
   the observed normal bootcmd path. Successful factory admin login proved recovery.
2. Exported working/certified vcboot.cfg and vcsetup.cfg and network/vcpolicy.cfg
   to private host terminal captures with SHA-256 manifests. Terminal exports
   are not claimed to be byte-exact transfers.
3. Created /flash/x3-pre-reset-U3581156-20261021 with mode 0700. Moved exactly the
   four active vcboot/vcsetup files there with mv -n, using distinct working-/
   certified- names. Verified all four archived files, absence of active names
   before reboot, and retention of both 210820484-byte Uos.img files. No wildcard
   deletion, firmware upgrade, license reset or rescue format was issued.
4. Issued reload all and confirmed that approved operation. Logged back into
   the same serial after boot. VLAN 110, old EMP address and old static routes
   were gone; the retained firmware release was verified. Startup regenerated
   default configuration. This is a startup configuration reset, not secure erasure
   of old backups, logs, certificates or all persistent switch data.
5. Waited for Master/Running and module UP before changing accounts. Applied a
   generated administrator secret through the concealed interactive password
   dialogue and verified it with a fresh login. Removed both old custom users
   with release-documented no user commands. Built-in default is a non-login template.
6. Used copy certified working make-running-directory, write memory, then
   copy running certified. Verified CERTIFIED and SYNCHRONIZED. Directly changing
   the running directory had been rejected because working lacked vcboot.cfg;
   the documented copy-certified command populated it correctly.
7. Started a final approved software reboot to prove the generated credential,
   account cleanup and clean configuration survive restart. Result: PASS. The generated credential, admin-only login account set, clean
   startup state and Master/Running were verified after that final reboot.

The 2014 release lacks the later reset-to-factory workflow. The May 2014 vendor
Switch Management Guide describes the saved files and out-of-box state without
vcboot/vcsetup (chapters 3 and 13), file moves (2-10), and reload (3-8). The
release-matched CLI Reference documents copy certified, write memory and copy
running certified (47-11, 47-14, 47-15). Those references, device command help
and actual before/after evidence support this procedure:

- [May 2014 Switch Management Guide, vendor-authored mirror](https://device.report/m/1bdd32f4f105ad70abad109b4b77606380354d4dda63351d03f50c35cffa441a.pdf)
- [May 2014 CLI Reference, vendor-authored mirror](https://manuals.plus/m/fbbe7bae5fb207e105333fe85cbe5ab4fbf906e3ab822a7d89640ec50db1d965)

Host credential path is recorded in the private inventory metadata, never the
password itself. Keep the unit isolated until approved X3 management/VLAN
configuration and supported-firmware review are complete.
