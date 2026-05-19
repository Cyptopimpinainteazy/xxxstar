# Tier‑1 Panel Plugin Dependencies

This table tracks the Tauri command + plugin surface each operator panel depends on. It describes what telemetry the frontend expects so the backend wiring can be prioritized.

| Panel | Tauri Command | Plugin Dependency | Expected Data |
| --- | --- | --- | --- |
| Swarm Health | `launch_swarm_health` | `tauri-plugin-system-info` (GPU/temperature/memory probes) + custom GPU job queue service | Node-level GPU utilization, VRAM use/capacity, temperature, uptime, SLA, queued jobs, live timestamp |
| Network Control | `launch_network_control` | `taurpc` (tcp/udp/mqtt stack) + RPC middleware | Peer discovery & statuses, RPC endpoint metrics, gossip logs, latency/packet stats |
| Storage Manager | `launch_storage_monitor` | `tauri-plugin-fs` / `tauri-plugin-ota` (pinning + proof submissions) | Pin table (CID, size, replicas, proof age/status), recent proofs/challenges, storage capacity vs used bytes |
| Dev Tools / IDE | `launch_ide_ipc` | `rpc` + `auth` (RPC channels, build service telemetry) | Build queue status, contract deployments, replay traces, IDE log stream / compile output |
| Live Telemetry | `telemetry_update` + `launch_*` snapshots | `tauri-plugin-system-info` + `taurpc` + `tauri-plugin-fs` | Combined swarm heatmap + storage utilization from streaming telemetry events |
