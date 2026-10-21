# Switch 2 management — U3581140

Startup reset and generated administrator reboot persistence: PASS. SSH/IP
configuration saved/certified: PASS. Live SSH: BLOCKED by missing Ethernet link.
Management-specific reboot persistence: NOT RUN after the SSH/IP change.

Connect the rear EMP of U3581140 to x3star1 eno4. Leave switch 1 on eno3, the
existing LAN on eno2 and USB console on switch 2. Management target is
10.30.10.12/24. Host profile `x3-switch2-management` is prepared with eno4 MAC
08:94:ef:37:68:c5, address 10.30.10.2/32 and a specific 10.30.10.12/32 route.
It has no gateway. The specific route avoids competing broad subnet routes across
the two direct management links. Existing host forwarding settings are unchanged;
firewall isolation is not claimed.

Switch SSH service is enabled with local authentication; FTP, Telnet, HTTP,
HTTPS and SNMP services are disabled. Console authentication remains local.
The configuration is saved and certified; its management settings have not yet
been reboot-tested. The earlier generated-password/reset-state reboot test passed.

Public host keys were obtained over the authenticated console and pinned in
`~/.local/state/x3-network/ssh-U3581140/known_hosts`. Both differ from switch 1:

- ECDSA: SHA256:5F48Wsi6JF96wsp6LYUNivf81VNsJKC4pjfwIiMUER0
- RSA: SHA256:Pyc0uLF4m2o0YewGpP0irDtbR1Dig0QlndVq/RyzejI

After carrier appears, activate the existing profile if necessary. Confirm the
route to 10.30.10.12 uses eno4/source 10.30.10.2, the route to 10.30.10.11 remains
on eno3, and the default route remains on eno2. Only then use the prepared pinned
SSH command and private askpass helper. Never invoke askpass directly in a log;
it emits the private credential. The route guard blocked the initial check, so
no credential was sent out eno3. Real SSH must verify chassis serial U3581140.
Then perform the authorized management reboot-persistence check with console kept
available. No production VLAN acceptance or firmware security readiness is claimed.

Evidence: ../reports/switch-U3581140-reset-proof.json and
../reports/switch-U3581140-ssh-setup-proof.json. Raw captures and credentials remain
in private host storage outside Git.
