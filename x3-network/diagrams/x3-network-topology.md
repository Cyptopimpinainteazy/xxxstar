# Observed and proposed topology

Observed (remote switch identity unknown):

```mermaid
flowchart LR
  H["x3star1 eno2: 192.168.0.70/24, 1 Gb/s full"] --- U["Unknown LAN switch/port"]
  U -. "IP reachable, physical path unverified" .-> G["Gateway 192.168.0.1"]
  U -. "IP reachable, resolver label only" .-> GPU["x3gpu1: 192.168.0.30"]
  H --- C["CP2104 USB UART: ttyUSB0; console verified"]
  C -. "9600/8N1 console" .-> S["OS6860E-P24: U3581156; reset verified"]
```

Proposed, not cabled/approved/implemented:

```mermaid
flowchart TD
  F["Firewall: approved zone policy required"] --- C["x3-switch1: core"]
  C --- V["x3-switch2: validators / development"]
  C --- G["x3-switch3: GPU / infrastructure"]
  V --- VN["Validators, CI, testnet: separate access VLANs"]
  G --- GN["GPU and supporting infrastructure: separate access VLANs"]
  M["Private management clients / VLAN 10"] --- C
```

Lines in the proposal do not specify ports, LAGs or link speeds. All require physical
inspection. No routing or stacking is implied. Fibre Channel is excluded.
