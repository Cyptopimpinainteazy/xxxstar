# apps/x3-intelligence — DEPRECATED

This app has been absorbed into `apps/x3-desktop` as individual panel components.

**Previous views → New panels:**
- `CrossVmActivity.tsx` → `apps/x3-desktop/src/ui/panels/CrossVmActivityPanel.tsx`
- NetworkOverview view → `apps/x3-desktop/src/ui/panels/NetworkOverviewPanel.tsx`
- SwarmActivity view → `apps/x3-desktop/src/ui/panels/SwarmActivityPanel.tsx`
- SupplyDashboard view → `apps/x3-desktop/src/ui/panels/SupplyDashboardPanel.tsx`

**API client** (`src/api/client.ts`) → replaced by Tauri `invoke()` commands that proxy to real node RPC + swarm API.

**Date:** 2026-06-17

All new intelligence features should be added as panels in `apps/x3-desktop/src/ui/panels/`.