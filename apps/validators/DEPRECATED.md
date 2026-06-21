# apps/validators — DEPRECATED

Absorbed into `apps/x3-desktop/src/ui/panels/ValidatorGlobePanel.tsx`.

**Previous RPC client** (`src/lib/validatorRpcClient.ts`) → replaced by Tauri `invoke('get_validators')` which calls `validator_getValidators` on node RPC.

**Date:** 2026-06-17

All new validator UI should be added in `apps/x3-desktop/src/ui/panels/`.