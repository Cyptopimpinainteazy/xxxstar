# Change: Add live telemetry dashboard

## Why
Operators need a live telemetry surface in the desktop app to monitor GPU swarm health, network activity, storage utilization, and IDE runtime status in near real time.

## What Changes
- Add a live telemetry panel with streaming updates and visualizations (heatmap + storage utilization graph).
- Extend the Tauri backend to publish telemetry update events and provide snapshot IPC commands.
- Wire frontend hooks to Tauri events with a typed telemetry payload contract.

## Impact
- Affected specs: operator-dashboard
- Affected code: apps/x3-desktop (frontend + src-tauri backend)
