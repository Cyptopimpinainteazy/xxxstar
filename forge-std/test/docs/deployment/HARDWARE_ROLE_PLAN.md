# X3 Hardware Role Plan

## Purpose

This document maps the currently available hardware to concrete X3 testnet roles. It is not a generic sizing guide. It is the operator plan for the machines that are actually on hand right now, plus the minimum rented infrastructure required if the network is going to be presented as a real public testnet instead of a single-site lab.

## Current Reality

The owned hardware is enough to run a real multi-node lab network immediately. It is not enough by itself to claim geographic distribution or site-level fault isolation, because the machines are concentrated in one physical location. A credible public testnet still needs at least two additional remote hosts in separate regions or facilities.

The role assignments below assume the following inventory supplied by the operator:

| Node ID | Machine | Confirmed Hardware | Current Confidence | Assigned Role |
| --- | --- | --- | --- | --- |
| `x3-lab-val-01` | Lenovo System x3550 M5 #1 | Large RAM, enterprise server chassis | Medium | Primary local validator and initial bootnode |
| `x3-lab-val-02` | Lenovo System x3550 M5 #2 | Large RAM, enterprise server chassis | Medium | Secondary local validator |
| `x3-lab-val-03` | Threadripper 1900X workstation | 8 cores, 64 GB RAM, 4x GTX 1070, about 2 TB NVMe | High | Third local validator during lab proving, or GPU canary host when not participating in consensus |
| `x3-lab-rpc-01` | HP DL380p Gen8 | Enterprise server, exact CPU and disk profile still needs confirmation | Low | RPC, Prometheus, Grafana, log shipping |
| `x3-lab-dr-01` | Dell R710 | Older enterprise server, exact CPU and disk profile still needs confirmation | Low | Cold standby, restore drills, snapshot validation |
| `x3-lab-ops-01` | Apple Xserve | Older platform, exact software support depends on installed OS | Low | Offline backups, artifact mirror, long-retention logs if the OS is still supportable |

The Lenovo systems are the safest consensus candidates in the current rack because they are server-class machines and separate from the GPU workstation. The Threadripper host is the most flexible machine in the inventory, but it should not carry both production validator duty and experimental GPU workloads at the same time.

## Role Assignments For The Local Lab

This section defines the topology that can be deployed now with only owned hardware. It is suitable for proving validator startup, peer discovery, finality progression, RPC exposure, monitoring, and rollback procedures on a real multi-node network.

| Logical Role | Host | Why This Host | Notes |
| --- | --- | --- | --- |
| Bootnode plus Validator 1 | `x3-lab-val-01` | Stable dedicated server, clean separation from workstation noise | Use only one validator as the initial bootnode to keep peer discovery simple during first bring-up |
| Validator 2 | `x3-lab-val-02` | Second server-class node for consensus diversity inside the rack | Keep the same software and chainspec as Validator 1 |
| Validator 3 | `x3-lab-val-03` | Enough CPU, RAM, and fast local storage to carry validator load | Disable GPU experiments while this node is an authority |
| RPC plus Monitoring | `x3-lab-rpc-01` | Keeps public RPC and observability off authority nodes | Do not expose validator RPC ports publicly |
| Backup and Log Sink | `x3-lab-ops-01` | Useful for low-write operator tasks if still operationally sound | If the Xserve is unstable, move this role to the DL380p and demote the Xserve to archive storage only |
| Disaster Recovery Target | `x3-lab-dr-01` | Separate restore target for snapshot rehearsal and node replacement | Do not treat the R710 as a primary validator unless it proves stable under sustained sync |

The local lab network should stay at three authorities, one non-authority RPC and monitoring node, and one cold or warm recovery target. That is enough to validate the operator path honestly without pretending the network is internet-grade.

## Local Lab Topology

The first production-like topology should look like this:

```text
Rack / single-site lab

  x3-lab-val-01  -> bootnode + validator-01
  x3-lab-val-02  -> validator-02
  x3-lab-val-03  -> validator-03
  x3-lab-rpc-01  -> rpc-01 + prometheus + grafana
  x3-lab-ops-01  -> backup mirror + artifact store + long-retention logs
  x3-lab-dr-01   -> standby restore target
```

This topology is appropriate for the next real step: bring the network up on bare metal, run the existing operator SOP against it, prove that all three validators stay in consensus, then exercise restart, rollback, and restore procedures against the support hosts.

## Public Testnet Topology

A public testnet has a different standard. Three validators in one building are still one power event, one switch failure, and one ISP outage away from disappearing together. If the network is going to be published as a real external testnet, use the owned rack for one authority and support roles, then add at least two remote validators in separate regions.

The minimum credible public topology is this:

| Logical Role | Host | Failure Domain | Notes |
| --- | --- | --- | --- |
| Validator 1 | `x3-lab-val-01` | Local site | Keep this one as an authority only if local power and networking are stable |
| Validator 2 | Remote VPS A | Remote site A | This can be the existing far-away VPS if it has enough CPU, RAM, storage, and stable networking |
| Validator 3 | Remote VPS B | Remote site B | This must be a second remote site, not another VM in the same provider region |
| Bootnode / seed | `x3-lab-val-02` or remote non-authority VPS | Local site or remote site C | Prefer a non-authority bootnode once the network is stable |
| Public RPC | `x3-lab-rpc-01` | Local site | Put rate limits and firewall policy in front of it |
| Monitoring and alerting | `x3-lab-rpc-01` | Local site | Mirror alerts to a second destination if possible |
| GPU canary and build host | `x3-lab-val-03` | Local site | Do not make this node an authority while it is running GPU experiments |
| Hot spare / validator replacement | `x3-lab-val-02` or `x3-lab-dr-01` | Local site | Use this to replace a failed validator after controlled maintenance |

This public topology deliberately moves the Threadripper workstation out of the authority set. The workstation is valuable, but mixing GPU validation experiments, build work, and consensus duty on one host is the wrong trade if the goal is a stable public network.

## Machine-Specific Guidance

### Lenovo System x3550 M5 nodes

These two machines should carry the heaviest always-on X3 duties inside the local rack. Use one as the first bootnode and validator, and keep the second ready as either a second validator for lab proving or a hot spare for public operations. Before assigning permanent authority roles, confirm CPU model, total RAM, disk health, and whether each machine has SSD-backed storage rather than old spinning disks.

### Threadripper 1900X workstation

This host is the best place to keep the X3 build toolchain, release artifacts, GPU-specific validation experiments, and any CUDA or swarm work that must run on real hardware. It can serve as the third authority in the local proving cluster because the CPU and storage profile are sufficient, but that should be a temporary proving role. Once the public testnet starts, move it to non-authority GPU canary duty and keep validator traffic off the same machine.

### HP DL380p Gen8

Use this box for non-consensus services first. RPC, Prometheus, Grafana, and log ingestion benefit from server hardware without risking consensus if the machine turns out to be noisy or storage-constrained. Validate disk reliability and network throughput before putting public RPC load on it.

### Dell R710

Treat this as recovery infrastructure until it proves otherwise. It is old enough that power draw, storage age, and firmware condition matter more than raw core count. It is still useful for restore drills, snapshot validation, and rehearsing node replacement. Do not count it as a public authority until it survives sustained sync and restart testing.

### Apple Xserve

This machine is only worth keeping in the plan if its OS, storage, and remote-management story are still healthy. If it is reliable, use it for artifact mirroring, offline backups, and long-retention logs. If it is brittle, remove it from the operational path entirely and keep it out of anything the network depends on.

## Network Layout

Keep consensus traffic and public RPC separated. Validators need stable east-west connectivity on the P2P port. RPC and metrics should terminate on the support node instead of on authority hosts.

Use the following split:

| Service | Hosts | Exposure |
| --- | --- | --- |
| P2P `30333/tcp` | Validators, bootnode, sentries | Open only where peer connectivity requires it |
| RPC `9944/tcp` | RPC node only for public use; validator RPC stays private | Public only behind firewall and rate limiting |
| Metrics `9615/tcp` | Validators and RPC node | Private management network or VPN only |
| SSH `22/tcp` | All hosts | Restricted to operator source IPs |

If the rack has multiple switches or separate NICs available, keep validator P2P on one VLAN and management plus metrics on another. That reduces the chance that an RPC or scraping mistake interferes with consensus traffic.

## Bring-Up Order

Start with the local lab and do not skip straight to the public topology. The sequence should be:

1. Rack and cable the Lenovo pair, Threadripper, and DL380p.
2. Install one supported Linux image on all X3-critical hosts.
3. Run the three-authority local lab with the existing operator SOP.
4. Prove block production, finality, restart recovery, RPC health, and backup restore on owned hardware.
5. Add Remote VPS A as a validator.
6. Add Remote VPS B as a validator in a second region.
7. Remove the Threadripper from the authority set and move it to GPU canary duty.
8. Publish public RPC only after the remote validator set is stable.

If only one remote VPS is available, stop at the lab phase or call the result a private test environment. Do not market that configuration as a resilient public testnet.

## Immediate Next Work

Before deployment, confirm the exact CPU, RAM, disk type, and NIC speed for the two Lenovo servers, the DL380p, and the R710. That hardware truth pass decides whether the spare role stays on the R710 or moves to the DL380p.

After the hardware truth pass, the next concrete deliverables are an inventory file with real hostnames and IPs, systemd unit mappings for each role, firewall rules per host, and a chainspec plus bootnode plan that matches the final authority set.