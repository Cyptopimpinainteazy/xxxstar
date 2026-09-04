# X3 Benchmark Report Schema

## Purpose

This document defines the canonical partner-facing report schema for X3 Benchmark Cloud and X3 Connect onboarding.

The schema is designed for:

- passive chain analysis
- traffic replay
- shadow-mode comparison
- signed benchmark certification

## Report Envelope

```json
{
  "report_version": "1.0",
  "report_id": "uuid",
  "generated_at": "RFC3339 timestamp",
  "signer": {
    "service": "x3-benchmark-cloud",
    "key_id": "string",
    "signature": "hex"
  },
  "partner": {},
  "baseline": {},
  "x3_replay": {},
  "recommendation": {},
  "artifacts": {}
}
```

## Partner Section

Required fields:

- `chain_name`
- `chain_type`
- `environment`
- `rpc_endpoints`
- `benchmark_window`
- `software_versions`
- `traffic_source`

Example:

```json
{
  "chain_name": "PartnerChain",
  "chain_type": "evm",
  "environment": "testnet",
  "benchmark_window": {
    "start": "2026-04-01T00:00:00Z",
    "end": "2026-04-01T01:00:00Z"
  },
  "software_versions": {
    "node": "v1.2.3",
    "client": "v2.0.1"
  },
  "traffic_source": "rpc+trace-replay"
}
```

## Baseline Section

Metrics to collect from the partner chain as-is:

- `accepted_tps`
- `finalized_tps`
- `p50_latency_ms`
- `p95_latency_ms`
- `p99_latency_ms`
- `failed_tx_rate`
- `block_fullness_avg`
- `hotspots`
- `conflict_profile`

Hotspot entry fields:

- `type`
- `identifier`
- `share_of_load`
- `failure_rate`

Conflict profile fields:

- `low_conflict_ratio`
- `medium_conflict_ratio`
- `high_conflict_ratio`
- `estimated_serial_fraction`

## X3 Replay Section

Metrics from replaying or shadowing the workload through X3 models:

- `mode`
- `replayed_operations`
- `x3_ingress_ops_per_sec`
- `x3_settled_receipts_per_sec`
- `x3_canonical_state_transitions_per_sec`
- `p50_receipt_latency_ms`
- `p95_receipt_latency_ms`
- `p99_receipt_latency_ms`
- `projected_failure_rate`
- `compression_ratio`
- `cross_lane_ratio`
- `gpu_assist_used`
- `workload_profile`

Workload profile fields:

- `total_transactions`
- `total_receipts`
- `total_logs`
- `active_lanes`
- `active_log_lanes`
- `low_conflict_ratio`
- `medium_conflict_ratio`
- `high_conflict_ratio`
- `estimated_serial_fraction`
- `log_classes`

Log class entry fields:

- `class_name`
- `count`
- `share_of_logs`
- `unique_contracts`
- `unique_transactions`

Supported modes:

- `passive-analysis`
- `trace-replay`
- `shadow-sidecar`
- `turbo-lane-projection`

## Recommendation Section

Required fields:

- `integration_tier`
- `expected_gain_summary`
- `required_components`
- `commercial_model`
- `confidence`

Valid `integration_tier` values:

- `benchmark-only`
- `sidecar-mode`
- `turbo-lane-mode`
- `shared-settlement-mode`

Example:

```json
{
  "integration_tier": "turbo-lane-mode",
  "expected_gain_summary": {
    "hot_path_throughput_gain": "4.2x",
    "p95_latency_delta": "-61%",
    "failure_rate_delta": "-73%"
  },
  "required_components": [
    "x3-sidecar",
    "x3-connect-sdk",
    "x3-proof-bus-adapter"
  ],
  "commercial_model": "integration+metered-throughput",
  "confidence": 0.88
}
```

## Artifacts Section

Attach digests and references for:

- raw trace bundle
- normalized replay bundle
- report JSON digest
- dashboard snapshot
- hotspot heatmap
- conflict heatmap

Example:

```json
{
  "trace_bundle_hash": "hex",
  "replay_bundle_hash": "hex",
  "report_digest": "hex",
  "dashboard_url": "https://...",
  "heatmap_artifacts": [
    {
      "type": "conflict-heatmap",
      "url": "https://..."
    }
  ]
}
```

## Truthfulness Rules

Every report must clearly separate:

1. `Ingress TPS`
2. `Settled Receipt TPS`
3. `Canonical State-Transition TPS`

No X3 report may collapse these into one vanity number.

## CI / API Use

This schema should be consumable by:

- partner dashboards
- CI pipelines
- onboarding portal
- sales certification workflows
- metering systems

## Next Implementation Targets

The first code-paths that should implement this schema are:

- `crates/x3-rpc`
- `crates/x3-sidecar`
- `crates/x3-gateway`

These crates should expose:

- report-generation APIs
- signed-report retrieval
- benchmark job status
- artifact references
