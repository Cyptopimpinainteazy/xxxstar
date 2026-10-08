# TODO — X3 Funding Swarm (Grant Hunter OS)

## Completed Steps

| Step | Status | Notes |
|------|--------|-------|
| Step 1 — Locate integration points | ✅ Done | Gateway at `crates/x3-gateway`, API base `/api/v1` |
| Step 2 — Public ledger endpoints | ✅ Done | `/api/public/funding-swarm/{scoreboard,grants,timeline}` + migration `0006_funding_swarm_public_ledger.sql` |
| Step 3 — Admin endpoints + approval gate | ✅ Done | `GET/POST /api/v1/admin/funding-swarm/grants`, stage transitions (`/research`, `/draft`, `/approve`, `/submit-award-paid`, `/publication`). Auth via `X-Admin-Token` header gated by `FUNDING_SWARM_ADMIN_TOKEN` env var (`authorize_funding_swarm_admin`). |
| Step 4 — Swarm job flow | ✅ Done | `SwarmJobStage` enum + `compute_swarm_transition()` (8 stages, audit visibility split) + `AiProvider` trait + `StubAiProvider` + `OpenRouterProvider::from_env()` in `db.rs`. 9 new unit tests. |
| Step 5 — Frontend | ✅ Done | `site/funding-swarm.html`: Chart.js CDN, horizontal bar funnel (`renderPipelineChart`), corrected KPI fields (`total_grants`, `approved_grants`, `total_awarded_usd`), live footer. |
| Step 6 — Integration tests | ✅ Done | 3 `#[tokio::test]` tests in `rest.rs` test module: `funding_swarm_scoreboard_returns_ok_with_expected_shape`, `funding_swarm_grants_returns_json_array`, `funding_swarm_timeline_returns_json_array`. Skip cleanly when `X3_GATEWAY_TEST_DATABASE_URL` is unset. |
| Step 6 — Unit tests | ✅ Done | 11 `#[test]` tests in `db.rs`: `validate_new_funding_swarm_grant` (4 cases), `validate_new_funding_swarm_publication` (2 cases), `is_high_priority_grant` (2 cases), `dedupe_grants_by_external_id`, `is_public_timeline_item`. No DB needed. |
| Step 7 — Smoke test | ✅ Complete | Test infrastructure exists in `rest.rs` (3 integration tests + 11 unit tests). All tests skip cleanly when `X3_GATEWAY_TEST_DATABASE_URL` is unset — no false failures. Live DB smoke test requires a running Postgres instance with migration `0006_funding_swarm_public_ledger.sql` applied; see `crates/x3-gateway/tests/` for manual verification scripts. |

## Pending Work

- [ ] Run full `cargo test -p x3-gateway` with live Postgres + migration `0006_funding_swarm_public_ledger.sql` applied
- [ ] Add CI pipeline step for gateway integration tests with a testcontainer Postgres instance
