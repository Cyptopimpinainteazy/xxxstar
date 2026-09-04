# Generated Frontend Route Allowlist

This file is generated from rpc_consumer_contracts.json. Do not hand-edit it.

Generated at: 2026-05-30T11:04:51.375842667+00:00

| Route | Allowed Methods | Rationale |
| --- | --- | --- |
| `bridge-status` | `-` | Only read-side proof and validation signals are allowed directly; settlement and cross-VM submission remain sidecar-owned. |
| `explorer` | `-` | No stable direct-read explorer contract is carved out yet from the current RPC surface. |
| `governance` | `-` | Direct-read governance dispute and finality visibility is allowed; proposal actions stay behind sidecar and signing boundaries. |
| `network-overview` | `-` | Public network posture can bind to direct-read validator and operational health endpoints only. |
| `wallet-home` | `-` | Direct-read wallet and account posture data only; no relayer, queue, or signer-owned mutations. |
