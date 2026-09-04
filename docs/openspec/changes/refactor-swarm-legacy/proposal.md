# Change: Deprecate Legacy Swarm Logic in Inferstructor Core

## Why
The legacy swarm stack should be preserved for reference, but it is not part of the near‑term production path. We need to reduce operational surface area and avoid accidental coupling while retaining the codebase for future reuse.

## What Changes
- Mark legacy swarm modules as deprecated and excluded from default build/run paths.
- Preserve the legacy swarm codebase for reference; do not delete or archive it.
- Preserve reusable primitives (metrics, adapters) only when they directly support Inferstructor.
- Update documentation to reflect the new operational baseline.

## Impact
- Affected specs: new `inferstructor-runtime` requirement for default execution path.
- Affected code: `swarm/`, `crates/gpu-swarm/`, related docs and scripts.
- Risk: removing dependencies may require refactoring imports and build configuration.
