# VLAN proposal — not applied

| VLAN | Purpose | Proposed subnet |
|---|---|---|
| 10 | Management | 10.30.10.0/24 |
| 20 | Validators | 10.30.20.0/24 |
| 30 | Full nodes / RPC | 10.30.30.0/24 |
| 40 | CI / development | 10.30.40.0/24 |
| 50 | GPU compute | 10.30.50.0/24 |
| 60 | Monitoring / logs | 10.30.60.0/24 |
| 70 | Testnet services | 10.30.70.0/24 |
| 80 | Ethernet storage | 10.30.80.0/24 |
| 90 | Infrastructure | 10.30.90.0/24 |

Local check: PASS, no overlap against x3star1 addresses, IPv4 routes from all
routing tables and Docker IPAM (172.17.0.0/16 and 172.18.0.0/16).
Observed LAN: 192.168.0.0/24; Tailscale interface: 100.89.236.89/32.
This does not establish availability across other hosts, upstream IPAM, VPN
advertised but uninstalled routes, or Kubernetes pod/service allocations.
Deployment remains BLOCKED pending those checks and topology approval.

Reserve management addresses only after DHCP/IPAM review. No gateway or switch IP
has been assigned. Do not enable inter-zone routing on the switches. Use an approved
firewall allowlist; deny inter-zone access by default, including IPv6. Separate
validator management from RPC exposure and untrusted testnet traffic. Approve exact
validator peer/service ports from actual running configuration, not assumed defaults.
Keep switch administration private, accessible only from approved management clients.

After AOS identity and topology approval, generate release-specific configuration for
hostnames, explicit access/tagged VLAN membership, SSH, secure administrator
provisioning, disabling unused insecure services, NTP, syslog, LLDP, and SNMPv3 if
required. Review STP root/edge/BPDU guards and storm-control thresholds per port role;
never enable edge protections blindly on switch uplinks. Save and certify with the
release's supported procedure, then verify persistence in an approved reboot window.

Internal DNS zone choice requires domain control verification for x3star.com,
x3star.net or x3star.org. Use split DNS independently of Cloudflare public records;
publish no management records publicly. No DNS ownership or changes are asserted.
