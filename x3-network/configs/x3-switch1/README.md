# x3-switch1 — proposed role: core

Observed chassis: OS6860E-P24, serial U3581156, AOS 8.1.1.497.R01.
Startup configuration reset verified after software reload; firmware retained.
Original saved configurations: /flash/x3-pre-reset-U3581156-20261021 (0700).
Raw exports and administrator credential stay outside Git in the private host state.

Old custom accounts removed. Generated administrator credential verified with a
fresh login. Only admin and built-in non-login default template remain. Remote
management AAA is denied; console access works. Defaults are saved and certified.
Final reboot-persistence verification PASSED: generated administrator credential,
account cleanup and clean settings survived restart; chassis is Master/Running. The unit is not X3 deployment accepted: management address, VLANs,
SSH provisioning, topology and live forwarding/isolation tests remain outstanding.
No inter-switch stacking was configured.
