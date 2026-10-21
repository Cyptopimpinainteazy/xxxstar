# SSH management — U3581156

Current state: PASS for live SSH authentication, chassis identity and management
configuration restart persistence. After an actual reboot, SSH returned with the
same trusted host key and generated administrator credential. EMP is UP at
10.30.10.11/24, chassis Master/Running, both modules UP, and configuration
CERTIFIED/SYNCHRONIZED. See ../reports/switch-U3581156-ssh-reboot-proof.json.

The approved direct link now uses x3star1 **eno3**, MAC 08:94:ef:37:68:c4,
10.30.10.1/24, to switch rear EMP 10.30.10.11/24. Moving the cable from eno1 to
eno3 established carrier. The cause of eno1's lack of carrier is not established.
The existing NetworkManager profile `x3-switch-management` was moved to eno3;
no duplicate profile was created. No management gateway was configured. The
existing default route remains via 192.168.0.1 on eno2. Existing host forwarding
settings were not changed; this is not a claim of firewall isolation.

Verification on 2026-10-21: carrier=1, profile activation succeeded, target route
uses eno3 and source 10.30.10.1, ping returned 3/3 replies, and real OpenSSH
exited 0 with chassis serial U3581156. Host-key checking was strict against public
keys obtained over the console. Modern SSH defaults worked without compatibility
exceptions. The password remained in private storage outside Git.

Applied switch configuration: EMP address 10.30.10.11/24, SSH enabled with local
AAA; FTP, Telnet, HTTP, HTTPS and SNMP services disabled. Configuration was saved
in working and copied to certified. These settings survived an actual reboot. This management check does not prove production VLANs or X3
traffic deployment.

Private live proof:
`~/.local/state/x3-network/ssh-U3581156/live-ssh-check.txt`.
Private save proof:
`~/.local/state/x3-network/ssh-save-U3581156-r2sizzc4/capture.json`.

Observed console-authenticated public-key fingerprints:

| Algorithm | Bits | SHA-256 fingerprint |
|---|---|---|
| ECDSA | 256 | cUeKdAQ7xncHe3XYV3ANNtiPfowBtOw0Dly28Og/OGo |
| RSA | 2048 | Pq12DSv9/r80G59yES86Prey2sQ8gzBBNzM0/y95cI0 |

These are existing keys; uniqueness across the other uninspected units has not
been established. A supported key-rotation procedure should be reviewed before
production use. Firmware remains the observed 2014 AOS release and requires a
supported-firmware review before production deployment.

Private discovery proof: ~/.local/state/x3-network/ssh-preflight-wgbit0pk/capture.json.
Public-key capture/fingerprint proof: ~/.local/state/x3-network/aos-read-kvo4cjn1/.
No generated administrator password is stored in this document or Git.

Reference: [May 2014 vendor Switch Management Guide](https://device.report/m/1bdd32f4f105ad70abad109b4b77606380354d4dda63351d03f50c35cffa441a.pdf),
EMP addressing section 1-9, chassis EMP section 13-21, and switch authentication
chapter 7. Operator selected direct management and identified the rear EMP cable.
