# Physical port map

| Endpoint | Observed state | Remote endpoint |
|---|---|---|
| x3star1 eno2, MAC 08:94:ef:37:68:c3 | 1000 Mb/s, full duplex, LAN 192.168.0.70/24 | Unknown switch and port |
| x3star1 eno1 / eno3 / eno4 | Link down | Unknown |
| x3star1 ens1f0 / ens1f1 | Link down | Unknown |
| x3star1 enx0a94ef3768c9 | USB Ethernet link up, no assigned IP observed | Unknown; IBM RNDIS USB device observed |
| OS6860E-P24 U3581156 | Console reset verified; 30 port rows observed; modules UP | No access/uplink assignment approved |
| Remaining two OS6860E units | Not inspected | No access/uplink assignment approved |

Do not infer cable endpoints from neighbor IPs or USB adapter serial numbers.
LLDP is not available locally. Obtain switch LLDP and MAC tables plus physical
cable labels before assigning ports. Verify actual copper/SFP/uplink/VFL capability,
transceiver compatibility, negotiated speed, counters, and whether ports are reserved
for existing VC membership. Do not connect redundant uplinks without loop protection.
No stacking is approved or configured. Fibre Channel equipment is out of scope.

Migration order: retain x3star1's existing eno2 path; prove a parallel management
path; move one reviewed endpoint at a time; verify SSH and services before proceeding.
