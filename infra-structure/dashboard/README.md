# Inferstructor Dashboard — DEPRECATED

This app has been absorbed into `apps/tauri-os` as part of the frontend consolidation effort.

**Previous location:** `infra-structure/dashboard/src-tauri/`
**New location:** `apps/tauri-os/` (OS shell Tauri app)

## Migration

- `check_services` → `apps/tauri-os/src-tauri/src/main.rs` → `inferstructor_check_services`
- `get_app_info` → handled by `apps/tauri-os` OS shell
- Admin dashboard components → absorbed into `apps/tauri-os/src/ui/panels/`

## Date of deprecation

2026-06-17

## Do not add new features here

All new inferstructor features should be added to `apps/tauri-os/src/inferstructor/`.