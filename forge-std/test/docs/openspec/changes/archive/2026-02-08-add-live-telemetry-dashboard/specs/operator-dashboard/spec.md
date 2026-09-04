## ADDED Requirements
### Requirement: Live telemetry streaming
The desktop application SHALL provide a live telemetry stream for swarm, network, storage, and IDE runtime data, delivering updates at least every 2 seconds.

#### Scenario: Telemetry updates are published
- **WHEN** the telemetry update loop runs
- **THEN** the backend emits a telemetry update event and the latest snapshot can be retrieved via IPC

### Requirement: Live telemetry panel
The desktop application SHALL render a live telemetry panel with a GPU swarm heatmap and a storage utilization graph fed by the telemetry stream.

#### Scenario: Panel renders live telemetry
- **WHEN** a user opens the live telemetry panel
- **THEN** the heatmap and storage graph update with the latest telemetry values

### Requirement: Typed telemetry contract
The telemetry payloads SHALL conform to a stable, typed contract shared between the backend IPC responses and the frontend hooks.

#### Scenario: Payloads match the contract
- **WHEN** a telemetry snapshot is returned from IPC
- **THEN** the payload fields match the frontend telemetry types without transformation
