# X3 Chain RPC Operational Policy

**Version:** 1.0.0
**Effective date:** 2026-05-09
**Owner:** X3 Chain Infrastructure Team
**Review cadence:** Quarterly (see §14)

---

## Table of Contents

1. [Purpose and Scope](#1-purpose-and-scope)
2. [Provider Classification — Self-Host vs Outsource Decision Matrix](#2-provider-classification--self-host-vs-outsource-decision-matrix)
3. [Provider Tiers](#3-provider-tiers)
4. [Health Scoring Model](#4-health-scoring-model)
5. [Failover Routing Matrix](#5-failover-routing-matrix)
6. [Degraded-Mode Operation Rules](#6-degraded-mode-operation-rules)
7. [Cache Rules](#7-cache-rules)
8. [Rate Limit Policy](#8-rate-limit-policy)
9. [Monetization Decision Points](#9-monetization-decision-points)
10. [Provider Health Scoring Runbook](#10-provider-health-scoring-runbook)
11. [Key Rotation and Credential Management](#11-key-rotation-and-credential-management)
12. [Incident Response Playbook](#12-incident-response-playbook)
13. [Metrics and Alerting](#13-metrics-and-alerting)
14. [Review Cadence](#14-review-cadence)

---

## 1. Purpose and Scope

### 1.1 Purpose

This document is the single authoritative operational policy governing all RPC connectivity for the X3 Chain network. It establishes mandatory rules for:

- Classifying whether a given RPC workload is self-hosted or outsourced to a managed provider.
- Defining the three-tier provider hierarchy and the promotion/demotion logic between tiers.
- Specifying the 0–100 health score formula used by `node/src/metrics.rs` and the `x3-rpc-policy` crate.
- Governing health-based failover order, degraded-mode serving, cache lifetimes, and rate limits per method class.
- Setting the free-tier and premium-tier SLA thresholds that drive monetization.
- Providing actionable runbooks for score breaches, key rotation, and provider outages.

### 1.2 Scope

This policy applies to:

- All X3 Chain mainnet, testnet, and devnet RPC surface area (Substrate JSON-RPC, EVM JSON-RPC, SVM JSON-RPC).
- All internal services that call upstream RPC providers (settlement engine, solvency sidecar, rebalance pallet, proof-forge).
- All external-facing RPC endpoints published to dApps and partner chains.
- The `node/src/rpc_middleware.rs` rate limiter, the `node/src/metrics.rs` health tracker, and the `crates/x3-rpc-policy` types crate.

This policy does NOT govern:

- Database connectivity.
- Internal gRPC between node services.
- WebSocket subscriptions beyond what is explicitly specified in §7.

### 1.3 Definitions

| Term | Definition |
|------|-----------|
| Provider | Any upstream RPC node or managed service that X3 Chain sends requests to. |
| Endpoint | A single HTTP or WebSocket URL belonging to a provider. |
| Health score | 0–100 integer computed per endpoint per 30-second window. |
| Failover | The act of routing requests away from a degraded/frozen/offline endpoint. |
| Block drift | The difference in block height between an endpoint and the canonical X3 tip. |
| BPS | Basis points (1 BPS = 0.01%). Error rates are expressed in BPS. |
| TTL | Time to live for a cached response, in seconds unless otherwise stated. |

---

## 2. Provider Classification — Self-Host vs Outsource Decision Matrix

### 2.1 Classification Criteria

Every RPC workload must be evaluated annually against the following criteria. The outcome determines whether the workload runs on a Tier 0 (X3-owned) node or is delegated to Tier 1 managed providers.

| Criterion | Self-Host (Tier 0) | Outsource (Tier 1/2) |
|-----------|--------------------|----------------------|
| Daily request volume | > 50 million requests/day | ≤ 50 million requests/day |
| p99 latency requirement | < 100 ms (latency-critical) | ≤ 500 ms acceptable |
| Custom method support | Requires non-standard JSON-RPC methods | Standard JSON-RPC only |
| Data sensitivity | Contains proof data, custody state, or slashing evidence | Public read-only queries |
| Finality requirement | Hard finality required (settlement, slashing) | Soft/eventual finality acceptable |
| SLA ownership | Must own the SLA (cannot delegate accountability) | SLA can be inherited from provider |
| Compliance | Subject to data-residency or jurisdiction constraints | No constraint |
| Estimated annual cost delta | Self-hosting is cheaper at scale | Provider is cheaper at low volume |

**Rule:** A workload must be self-hosted (Tier 0) if it satisfies ANY of the following:
- Daily volume > 50 million requests, OR
- Latency requirement < 100 ms, OR
- Requires custom non-standard methods, OR
- Contains proof, custody, or slashing data.

**Rule:** A workload MAY be outsourced (Tier 1) if ALL of the following hold:
- Daily volume ≤ 50 million requests, AND
- Standard JSON-RPC only, AND
- p99 latency ≤ 500 ms acceptable, AND
- No data-residency constraint.

### 2.2 Reclassification Trigger

Reclassification from outsourced to self-hosted is automatically triggered when:

- Volume exceeds 40 million requests/day for two consecutive weeks (80% of threshold, proactive).
- A provider's 30-day rolling uptime drops below 99.5%.
- A provider's 30-day rolling p99 latency exceeds 450 ms.
- A compliance or legal event requires data-residency changes.

Reclassification must be approved by the Infrastructure Lead and documented as a Quarterly Review action item.

### 2.3 Current Classification (mainnet)

| Workload | Classification | Rationale |
|----------|---------------|-----------|
| X3 Substrate block import/finality | Tier 0 — self-host | Hard finality, custom methods |
| Proof verification queries | Tier 0 — self-host | Proof data sensitivity |
| Settlement engine external chains (Arbitrum) | Tier 1 (Alchemy primary) | Low volume, standard EVM JSON-RPC |
| Public explorer reads | Tier 1 (dRPC/Nodecore) | High volume, latency-tolerant |
| Price oracle feeds | Tier 1 (Ankr secondary) | Burst-tolerant, standard reads |
| Emergency fallback reads | Tier 2 (public) | Last-resort only |

---

## 3. Provider Tiers

### 3.1 Tier 0 — X3-Owned Infrastructure

**Definition:** Nodes operated directly by X3 Chain, either bare-metal or dedicated cloud instances running the X3 node binary.

**Failover priority:** 0 (highest, always tried first).

**Properties:**

- No external rate limits; governed only by node hardware capacity.
- Full custom method support including `x3_submitProof`, `x3_querySolvency`, `x3_getInvariantState`.
- Direct access to archive state, pending pool, and private RPC methods.
- Must maintain synchrony within 2 blocks of network tip at all times.
- Minimum two geographically distinct Tier 0 nodes required for mainnet (e.g., US-East + EU-West).
- Health score target: ≥ 90 at all times. Alert at < 80.

**Current Tier 0 endpoints:**

```
wss://rpc.x3chain.io/substrate     — primary Substrate WS
https://rpc.x3chain.io/substrate   — primary Substrate HTTP
wss://rpc-eu.x3chain.io/substrate  — EU failover Substrate WS
https://rpc-eu.x3chain.io/substrate — EU failover Substrate HTTP
https://rpc.x3chain.io/evm         — EVM JSON-RPC (Frontier)
```

### 3.2 Tier 1 — Managed Provider Infrastructure

**Definition:** Third-party managed RPC providers operating under a commercial SLA. X3 Chain holds an account and API key with each provider.

**Failover priority:** 1 (tried after all Tier 0 endpoints fail).

**Provider roster (ordered by priority within Tier 1):**

| Priority | Provider | Chain Families | Rate Limit (free) | Rate Limit (paid) |
|----------|----------|---------------|-------------------|-------------------|
| 1 | Alchemy | EVM | 300 req/s | Custom |
| 2 | dRPC / Nodecore | EVM, Substrate | 100 req/s | Custom |
| 3 | Ankr | EVM, SVM | 50 req/s | 500 req/s |
| 4 | QuickNode | EVM, SVM | 25 req/s | Custom |

**Properties:**

- API keys managed in secrets vault (see §11).
- Health scoring applies; a Tier 1 provider with score < 60 triggers failover to the next Tier 1 provider.
- Tier 1 providers are used exclusively for standard JSON-RPC. No custom X3 methods.
- Block drift tolerance for Tier 1: ≤ 10 blocks behind canonical tip. Exceeding triggers degraded mode for that endpoint.

**dRPC / Nodecore specifics:**

The dRPC stack runs locally via `infra/drpc/docker-compose.drpc.yml` when self-hosting the provider plane is desired. Nodecore exposes `http://127.0.0.1:9090/queries/{chain}` and Dshackle exposes `http://127.0.0.1:8545/{chain}`. In production, these are replaced by the hosted dRPC endpoint with a scoped API key per partner chain (see `docs/DRPC_NODECORE_DSHACKLE_INTEGRATION.md`).

### 3.3 Tier 2 — Public Fallback

**Definition:** Unauthenticated or lightly-authenticated public RPC endpoints. Used exclusively as last-resort fallback when all Tier 0 and Tier 1 providers are degraded or offline.

**Failover priority:** 2 (tried only after all Tier 0 and Tier 1 endpoints are unavailable or score < 30).

**Provider roster:**

| Provider | URL | Chain Families |
|----------|-----|---------------|
| Llamarpc | https://eth.llamarpc.com | EVM |
| Ankr public | https://rpc.ankr.com/{chain} | EVM, SVM |
| Cloudflare | https://cloudflare-eth.com | EVM |

**Properties:**

- Rate limits: 5–10 req/s per IP. Treated as severely capacity-constrained.
- Tier 2 is read-only. Transaction submission to public RPC is prohibited unless explicitly overridden per incident (see §12).
- Maximum consecutive Tier 2 serving time: 15 minutes before mandatory incident escalation (PagerDuty page).
- Block drift tolerance for Tier 2: ≤ 25 blocks. Beyond that, serve cached data only.
- Health score still tracked, but Tier 2 endpoints are not expected to meet Tier 0/1 SLAs.

---

## 4. Health Scoring Model

### 4.1 Score Formula

Health score is an unsigned integer in [0, 100]. It is computed independently per endpoint every 30 seconds using the following penalty-based formula:

```
health_score = 100
             − finality_lag_penalty
             − error_rate_penalty
             − latency_penalty
             − block_drift_penalty
             (floor at 0)
```

### 4.2 Penalty Schedules

**Finality Lag Penalty** (max 30 points deducted)

Measures blocks since last observed finalized block on the endpoint.

| Finality lag (blocks) | Penalty |
|-----------------------|---------|
| 0 – 3 | 0 |
| 4 – 5 | 5 |
| 6 – 10 | 15 |
| 11 – 20 | 20 |
| > 20 | 30 |

**Error Rate Penalty** (max 30 points deducted)

Measured as failed requests / total requests in the last 60-second sliding window, expressed in basis points (BPS).

| Error rate (BPS) | Penalty |
|------------------|---------|
| 0 – 100 | 0 |
| 101 – 300 | 5 |
| 301 – 500 | 10 |
| 501 – 1000 | 20 |
| > 1000 | 30 |

**Latency Penalty** (max 25 points deducted)

Measured as p95 response time in milliseconds for the last 30-second window.

| p95 Latency (ms) | Penalty |
|------------------|---------|
| 0 – 200 | 0 |
| 201 – 500 | 5 |
| 501 – 1000 | 10 |
| 1001 – 2000 | 20 |
| > 2000 | 25 |

**Block Drift Penalty** (max 15 points deducted)

Measures the absolute difference between the endpoint's reported best block and the canonical chain tip observed from Tier 0 nodes.

| Block drift (blocks) | Penalty |
|----------------------|---------|
| 0 – 2 | 0 |
| 3 – 5 | 5 |
| 6 – 10 | 10 |
| > 10 | 15 |

### 4.3 Score Thresholds and Status Classification

| Score range | Status | Behavior |
|-------------|--------|----------|
| 60 – 100 | Healthy | Normal routing, all methods served |
| 30 – 59 | Degraded | Read-only, cached responses preferred, new writes routed elsewhere |
| 1 – 29 | Frozen | No new traffic. Existing connections drained and closed. Failover mandatory. |
| 0 | Offline | Endpoint unreachable or returning non-JSON. Complete removal from routing pool. |

### 4.4 Failover Trigger Conditions

An endpoint triggers failover (regardless of score) if ANY of the following hold:

- `health_score < 60` (score-based failover).
- `block_drift > 10 blocks` (drift-based failover, independent of score).
- `error_rate_bps > 500` (error-based failover, independent of score).
- Three consecutive HTTP 5xx responses within a 10-second window (circuit-breaker trip).
- No response within `timeout_ms` (configurable per endpoint; default 30 000 ms for proofs, 5 000 ms for reads).

These thresholds are the authoritative constants in `crates/x3-rpc-policy/src/lib.rs`:

```rust
pub const FAILOVER_THRESHOLD: u8 = 60;
pub const FREEZE_THRESHOLD: u8 = 30;
pub const MAX_BLOCK_DRIFT: u32 = 10;
pub const MAX_ERROR_RATE_BPS: u32 = 500;
pub const DEGRADED_BLOCK_DRIFT: u32 = 5;
```

### 4.5 Score Recovery

- An endpoint marked Frozen (score 1–29) re-enters the routing pool only after its score remains ≥ 60 for two consecutive 30-second windows (1 full minute of health).
- An endpoint marked Offline re-enters only after a successful connectivity probe AND score ≥ 60 for two windows.
- Recovery is automatic; no manual intervention required unless score oscillates (see §10.4).

---

## 5. Failover Routing Matrix

### 5.1 General Routing Rule

Requests are always sent to the highest-priority healthy endpoint for the applicable chain family. "Healthy" means score ≥ 60, block drift ≤ 10, and error rate ≤ 500 BPS.

### 5.2 Substrate Chain Family

| Priority | Endpoint | Tier | Notes |
|----------|----------|------|-------|
| 0 | `wss://rpc.x3chain.io/substrate` | 0 | Primary |
| 1 | `wss://rpc-eu.x3chain.io/substrate` | 0 | EU geographic failover |
| 2 | dRPC/Nodecore Substrate endpoint | 1 | Managed, standard JSON-RPC only |
| 3 | (none configured) | 2 | No public Substrate fallback in production |

**Special rule:** If all Tier 0 Substrate endpoints are offline simultaneously, the system enters HARD DEGRADED mode: no new extrinsics are accepted, all reads are served from the local cached state, and PagerDuty is paged immediately (see §12).

### 5.3 EVM Chain Family (Arbitrum, and X3 EVM via Frontier)

| Priority | Endpoint | Tier | Notes |
|----------|----------|------|-------|
| 0 | `https://rpc.x3chain.io/evm` | 0 | X3 EVM (Frontier) |
| 1 | Alchemy Arbitrum | 1 | priority 100 in RpcEndpoint config |
| 2 | dRPC Arbitrum | 1 | priority 90 |
| 3 | Ankr Arbitrum | 1 | priority 80 |
| 4 | QuickNode Arbitrum | 1 | priority 70 |
| 5 | Llamarpc (public) | 2 | Last resort, read-only |
| 6 | Cloudflare ETH (public) | 2 | Last resort, read-only |

**Retry policy:** Max 3 retries per endpoint with exponential backoff starting at 100 ms. After 3 failures, advance to next priority. Total retry budget per request: 10 seconds for reads, 30 seconds for settlement calls.

### 5.4 SVM Chain Family

| Priority | Endpoint | Tier | Notes |
|----------|----------|------|-------|
| 0 | X3 SVM bridge endpoint | 0 | If running local SVM runtime |
| 1 | Helius | 1 | Primary SVM managed provider |
| 2 | Ankr SVM | 1 | Secondary |
| 3 | QuickNode SVM | 1 | Tertiary |
| 4 | Ankr public SVM | 2 | Last resort, read-only |

### 5.5 Load Balancing Within a Priority Group

When two endpoints share the same `failover_priority` value and both are Healthy, requests are distributed using weighted round-robin, where the weight is proportional to `health_score`. An endpoint with score 95 receives approximately twice the traffic of one with score 50 (which would already be in Degraded mode, but this rule applies for scores 60–100).

---

## 6. Degraded-Mode Operation Rules

### 6.1 Mode Definitions

| Mode | Trigger | Active Duration |
|------|---------|----------------|
| NORMAL | All Tier 0 healthy (score ≥ 60) | Default |
| DEGRADED | Any Tier 0 endpoint score < 60, OR block drift 5–10 | Until Tier 0 recovers |
| FROZEN | All Tier 0 endpoints score < 30, OR block drift > 10 | Until Tier 0 recovers + 1 min |
| HARD DEGRADED | All Tier 0 AND all Tier 1 endpoints offline | Manual incident resolution required |

### 6.2 What Degrades (Reduced Capability)

In DEGRADED mode, the following continue to operate but with restrictions:

- **Read queries** (`chain_getBlock`, `state_getStorage`, `eth_call`, `eth_getLogs`): Served normally, but responses include a `X-X3-Provider-Status: degraded` header. Cache TTLs are extended by 2x.
- **Price feeds**: Served from cache, cache TTL extended from default to 10 seconds (normally 3 seconds).
- **Historical queries** (archive): Served normally from Tier 1 or Tier 2 if Tier 0 archive is unavailable.
- **Block subscriptions**: Re-routed to the next healthy Tier endpoint. Subscription lag is expected to increase.

### 6.3 What Freezes (Suspended Operations)

In DEGRADED or FROZEN mode, the following are suspended:

- **Extrinsic submission** (`author_submitExtrinsic`, `eth_sendRawTransaction`): Queued locally for up to 60 seconds. If DEGRADED mode persists beyond 60 seconds, the queue is rejected with error code `-32003` (Service temporarily unavailable).
- **Settlement finalization calls**: Suspended. The settlement engine enters hold mode. Outstanding settlements are not confirmed until Tier 0 recovers.
- **Proof verification calls** (`x3_submitProof`): Suspended. Proof submission is queued.
- **Solvency checks that require live state**: Return last-known-good cached result with `stale: true` flag.

### 6.4 What Still Serves (No Interruption)

The following continue unaffected in all degraded modes:

- **Cached block headers** (within TTL, see §7).
- **Metrics and health check endpoints** (`/health`, `/metrics`, `/api/drpc/status`).
- **WebSocket connections already established**: Maintained. New subscriptions are rejected during FROZEN.
- **Read-only archive queries** with TTL > 60 seconds (historical data).
- **Rate limit enforcement**: Continues to operate independently of provider health.

### 6.5 Block Drift Degradation Rules

| Drift | Mode Change |
|-------|------------|
| 1 – 4 blocks | No mode change. Normal operation. |
| 5 – 10 blocks | DEGRADED. Cache TTL extended 2x. Writes queued. |
| > 10 blocks | FROZEN for that endpoint. Mandatory failover to next tier. |
| > 25 blocks (any tier) | Endpoint removed from pool entirely until manual health check. |

---

## 7. Cache Rules

### 7.1 Cache Principles

- Caching is a performance optimization, never a correctness shortcut.
- Cached responses include a `X-X3-Cache-Status: HIT` or `MISS` header.
- Cache keys incorporate the full JSON-RPC method name and parameter hash.
- Stale entries are served during degraded mode with `X-X3-Cache-Stale: true`.

### 7.2 Method-Class Cache Rules

**Class A — Block Headers**

Methods: `chain_getHeader`, `eth_getBlockByNumber`, `eth_getBlockByHash`, `chain_getBlock`

| Parameter | Value |
|-----------|-------|
| Default TTL | 400 ms |
| Degraded TTL | 800 ms |
| Cache eviction | On new block arrival (subscription-driven) |
| Consistency | Strong (pinned block hash in key) for finalized blocks; eventual for "latest" |
| Max entries | 1 000 (LRU eviction) |

**Class B — State Queries**

Methods: `state_getStorage`, `state_queryStorageAt`, `eth_call`, `eth_getBalance`, `eth_getCode`

| Parameter | Value |
|-----------|-------|
| Default TTL | 1 000 ms (1 second) |
| Degraded TTL | 2 000 ms |
| Cache eviction | By block number tag. Stale on next block. |
| Consistency | Eventually consistent. State may lag 1 block. |
| Max entries | 5 000 (LRU eviction) |

**Class C — Transaction Status**

Methods: `eth_getTransactionByHash`, `eth_getTransactionReceipt`, `author_pendingExtrinsics`

| Parameter | Value |
|-----------|-------|
| Default TTL | 2 000 ms for pending, 0 ms (no cache) for receipts of finalized txs |
| Degraded TTL | 5 000 ms for pending |
| Cache eviction | On receipt confirmation or transaction drop |
| Consistency | Must verify finality before caching receipt |
| Max entries | 500 |

**Class D — Transaction Submission**

Methods: `author_submitExtrinsic`, `eth_sendRawTransaction`, `eth_sendTransaction`

| Parameter | Value |
|-----------|-------|
| TTL | No cache. Every call is a pass-through. |
| Degraded behavior | Queue for 60 s, then reject (see §6.3) |
| Idempotency | Caller is responsible. Duplicate detection not performed. |

**Class E — Subscription Methods**

Methods: `author_submitAndWatchExtrinsic`, `state_subscribeStorage`, `eth_subscribe`

| Parameter | Value |
|-----------|-------|
| TTL | No cache. WebSocket streams are not cached. |
| Degraded behavior | Re-route to next healthy endpoint. Subscription re-established automatically. |
| Max concurrent subscriptions per connection | 10 (configurable via rate limit policy) |

**Class F — Health and Metadata**

Methods: `/health`, `/metrics`, `system_health`, `system_version`, `/api/drpc/status`

| Parameter | Value |
|-----------|-------|
| Default TTL | 10 000 ms (10 seconds) |
| Degraded TTL | 30 000 ms |
| Consistency | Stale is acceptable for monitoring consumers |

**Class G — Proof and Custody Queries**

Methods: `x3_submitProof`, `x3_querySolvency`, `x3_getInvariantState`, `x3_getCustodyState`

| Parameter | Value |
|-----------|-------|
| TTL | No cache for write paths. 5 000 ms for read-only custody/solvency reads. |
| Degraded behavior | Return last-known-good with `stale: true` flag (see §6.4) |
| Consistency | Hard consistency required for all proof submissions |

---

## 8. Rate Limit Policy

### 8.1 Method Classes for Rate Limiting

| Class | Methods | Notes |
|-------|---------|-------|
| READ_LIGHT | `eth_blockNumber`, `eth_chainId`, `system_health`, `chain_getHeader` | Very cheap, high allowed rate |
| READ_HEAVY | `eth_getLogs`, `state_queryStorageAt`, `trace_*`, archive queries | Expensive; lower rate |
| WRITE | `eth_sendRawTransaction`, `author_submitExtrinsic` | Submit path; moderate limit |
| SUBSCRIPTION | `eth_subscribe`, `state_subscribeStorage` | WebSocket; per-connection |
| ADMIN | `system_addReservedPeer`, `x3_submitProof` | Restricted to whitelisted IPs |

### 8.2 Per-Connection Rate Limits

| Class | Free Tier | Premium Tier | Internal / Tier 0 |
|-------|-----------|--------------|-------------------|
| READ_LIGHT | 60 req/min per conn | 3 000 req/min per conn | Unlimited |
| READ_HEAVY | 10 req/min per conn | 300 req/min per conn | Unlimited |
| WRITE | 20 req/min per conn | 600 req/min per conn | Unlimited |
| SUBSCRIPTION | 5 concurrent subs per conn | 50 concurrent subs per conn | Unlimited |
| ADMIN | Not available | Not available | IP-whitelisted only |

### 8.3 Burst Allowance

Free tier: 2x the per-minute rate for up to 5 seconds, then hard throttle.
Premium tier: 3x the per-minute rate for up to 15 seconds, then hard throttle.

Burst counters reset on a rolling 60-second window.

### 8.4 Global Rate Limits (Aggregate, Per Provider Tier)

| Tier | Max aggregate req/s across all connections |
|------|--------------------------------------------|
| Tier 0 (X3-owned) | Hardware-bound; target 10 000 req/s per node |
| Tier 1 — Alchemy | 300 req/s (standard plan) |
| Tier 1 — dRPC | 100 req/s (managed plan) |
| Tier 1 — Ankr | 50 req/s (free) / 500 req/s (paid) |
| Tier 2 — Public | 5 req/s per IP (enforced by provider) |

When a Tier 1 provider's aggregate rate approaches 80% of limit, the router begins routing new requests to the next Tier 1 provider before the limit is hit. This is proactive load shedding, not failover.

### 8.5 HTTP Status Codes Returned on Rate Limit

| Scenario | HTTP Status | JSON-RPC Error Code |
|----------|-------------|---------------------|
| Per-connection limit exceeded | 429 | -32005 |
| Global limit exceeded | 429 | -32005 |
| Admin method from non-whitelisted IP | 403 | -32006 |
| Provider upstream rate limit | 429 (pass-through) | -32005 |

---

## 9. Monetization Decision Points

### 9.1 Free Tier Definition

The X3 Chain free RPC tier is available to all developers and dApps without registration.

| Parameter | Free Tier Limit |
|-----------|----------------|
| Request rate | 100 req/min per API key or IP |
| Daily request volume | 1 000 000 requests/day |
| WebSocket connections | 2 concurrent |
| Concurrent subscriptions | 5 total |
| Archive access | No (latest 128 blocks only) |
| Custom method access | No (`x3_*` methods blocked) |
| SLA guarantee | None (best effort) |
| Support | Community Discord only |

Free-tier traffic is routed to Tier 1 managed providers only. Tier 0 nodes do not serve free-tier traffic.

### 9.2 Premium Tier Definition

Premium tier requires registration and billing agreement.

| Parameter | Premium Tier Limit |
|-----------|-------------------|
| Request rate | 10 000 req/min per API key |
| Daily request volume | 100 000 000 requests/day (100M) |
| WebSocket connections | 100 concurrent |
| Concurrent subscriptions | 50 total |
| Archive access | Full archive |
| Custom method access | Restricted set (excludes admin methods) |
| SLA | 99.9% monthly uptime |
| p99 Latency SLA | ≤ 300 ms |
| Support | Dedicated Slack channel + 4-hour response SLA |

Premium-tier traffic may be routed to Tier 0 nodes for latency-critical paths when Tier 0 capacity allows.

### 9.3 Partner / Institutional Tier

Partner chains and institutional users negotiate bespoke SLAs. The baseline requirements for partner tier:

- Volume: Negotiated per contract, minimum 500M req/day commitment.
- Routing: Dedicated Tier 0 capacity allocation.
- SLA: 99.99% monthly uptime or better.
- Support: Named account manager + 1-hour incident SLA.
- Custom methods: Full access including `x3_submitProof`, `x3_querySolvency`.

### 9.4 Overage Policy

| Tier | Overage Action |
|------|---------------|
| Free | Hard throttle at limit. No overage charges. |
| Premium | Soft throttle to 50% of limit rate for remainder of billing day. Next day resets. |
| Partner | Negotiated; typically 10% overage allowed, then throttle. |

### 9.5 SLA Breach Credits

For Premium and Partner tiers, monthly uptime SLA breaches trigger service credits:

| Uptime | Credit |
|--------|--------|
| 99.0% – 99.9% | 10% of monthly fee |
| 95.0% – 98.9% | 25% of monthly fee |
| < 95.0% | 50% of monthly fee |

SLA breach credits are calculated at the end of each calendar month and applied to the next invoice.

---

## 10. Provider Health Scoring Runbook

### 10.1 Score Update Cycle

Health scores are computed every 30 seconds by the health monitor in `node/src/metrics.rs`. The monitor:

1. Issues a probe request to each configured endpoint (`eth_blockNumber` for EVM, `chain_getHeader` for Substrate, with `id: "health_probe"`).
2. Records response latency, HTTP status, and reported block number.
3. Computes the penalty schedule (§4.2) using rolling-window metrics.
4. Emits the new score to the internal health bus.
5. The RPC router reads scores from the health bus before each routing decision.

Probe timeout: 5 000 ms. A probe timeout counts as one error in the error rate window.

### 10.2 Score Breach Response Matrix

| Score drops to | Automated action | Human action required |
|----------------|-----------------|----------------------|
| < 80 | Log WARNING. Increase probe frequency to every 10 seconds. | No |
| < 70 | Log ERROR. Notify on-call via Slack `#rpc-alerts`. | Review within 30 minutes. |
| < 60 | Log CRITICAL. Initiate failover to next endpoint. Page PagerDuty. | Acknowledge within 15 minutes. |
| < 30 | Log CRITICAL. Freeze endpoint. Force all traffic to next tier. Page PagerDuty URGENT. | Acknowledge within 5 minutes. |
| = 0 | Log CRITICAL. Remove endpoint from pool. Emit `RpcProviderOffline` on-chain event if Tier 0. | Immediate investigation. |

### 10.3 SLA Breach Actions

A Tier 1 provider is in SLA breach when its 30-day rolling uptime falls below the contracted level (typically 99.5% for managed providers). On SLA breach:

1. Log the breach with the provider name, breach window, and measured uptime.
2. Notify Infrastructure Lead and Legal/Procurement team.
3. File a formal SLA breach report with the provider within 48 hours.
4. Evaluate whether to reclassify the workload to self-hosted (apply §2.2 trigger criteria).
5. If breach is ongoing, begin provider replacement process: identify alternative Tier 1 provider, update keys and configuration, run 24-hour shadow test routing before full switchover.

### 10.4 Oscillating Score (Flapping)

If an endpoint's score crosses the 60-point failover threshold more than 3 times in any 10-minute window:

1. Mark the endpoint as FLAPPING.
2. Remove it from the routing pool for 10 minutes regardless of score.
3. After 10 minutes, re-admit with a 5-minute observation window. If it flaps again, extend removal to 60 minutes.
4. Alert on-call with `FLAPPING` severity (Slack `#rpc-alerts`, no PagerDuty page unless it causes routing disruption).

### 10.5 Score Reporting

- Scores are exposed as Prometheus gauges at `/metrics` with label `{provider, tier, chain_family}`.
- The Chainbench API endpoint `/api/drpc/status` returns current scores for all configured Tier 1 endpoints.
- A weekly score summary is sent to `#rpc-weekly-report` Slack channel (automated).

---

## 11. Key Rotation and Credential Management

### 11.1 Provider API Key Storage

All provider API keys (Alchemy, dRPC, Ankr, QuickNode, Helius) are stored in the secrets management system. Keys must never appear in:

- Source code or configuration files committed to version control.
- Plaintext environment files (`.env`) on production hosts.
- Log output or metrics labels.
- Error messages returned to callers.

The `.env.example` file in the repository contains placeholder values. Production keys are injected at deploy time via the secrets manager (e.g., HashiCorp Vault or cloud-native secrets manager).

### 11.2 Rotation Schedule

| Key type | Rotation interval | Trigger for emergency rotation |
|----------|-------------------|-------------------------------|
| Tier 1 provider API keys | 90 days | Suspected compromise, provider breach notification |
| Tier 0 node authentication tokens | 180 days | Node compromise or key exposure |
| Internal RPC middleware signing keys | 365 days | Compromise |
| WebSocket TLS certificates | Before expiry (auto-renewed) | Certificate revocation |

### 11.3 Emergency Key Rotation Procedure

On suspected compromise:

1. Immediately generate new API key from provider dashboard.
2. Update the secrets manager entry (do NOT delete old key yet).
3. Deploy new key to all running nodes via secrets manager hot reload — target completion within 4 hours.
4. Verify new key is active by observing successful probe responses.
5. Revoke old key in provider dashboard.
6. Document the incident in the key rotation log with timestamp, trigger, and affected systems.

### 11.4 Key Rotation Log

Maintain a key rotation log in the internal security wiki with the following fields per entry:

- Date rotated
- Key type and provider
- Rotation trigger (scheduled / emergency)
- Rotated by (name)
- Verification method
- Next scheduled rotation date

### 11.5 Per-Partner Nodecore Keys

Per `docs/DRPC_NODECORE_DSHACKLE_INTEGRATION.md` §Next Phase Recommendations, Nodecore auth must be implemented with scoped keys per partner chain. Each partner receives a Nodecore key scoped to:

- Their specific chain family.
- Read-only methods unless write access is explicitly granted.
- A request rate matching their contracted tier.

Partner Nodecore keys follow the same 90-day rotation schedule and the same emergency rotation procedure.

---

## 12. Incident Response Playbook

### 12.1 Severity Classification

| Severity | Definition | Response target |
|----------|-----------|----------------|
| P1 — Critical | All Tier 0 endpoints offline OR all EVM failover paths exhausted | 5 minutes to acknowledge, 30 minutes to mitigate |
| P2 — High | Single Tier 0 endpoint offline OR all Tier 1 endpoints degraded | 15 minutes to acknowledge, 60 minutes to mitigate |
| P3 — Medium | Any Tier 1 endpoint score < 60 sustained > 10 minutes | 30 minutes to acknowledge, 4 hours to mitigate |
| P4 — Low | Tier 2 endpoint degraded OR provider rate limits approaching | Next business day |

### 12.2 Provider Outage Response Sequence

The following is the mandatory action sequence on a Tier 1 provider outage:

**Step 1 — Detection (automated, t = 0)**

Health monitor detects score < 30 for 2 consecutive 30-second windows. Failover to next Tier 1 provider is automatic. PagerDuty alert fires.

**Step 2 — Acknowledgment (on-call, t ≤ 15 min)**

On-call engineer acknowledges the PagerDuty alert. Checks:
- Which provider is down (from `/metrics` dashboard).
- Whether failover is already active (check `X-X3-Provider-Tier` response header).
- Whether Tier 2 fallback is being used (log: `WARN rpc_router: serving from Tier 2`).

**Step 3 — Diagnosis (on-call, t ≤ 30 min)**

Investigate root cause:
- Provider status page (Alchemy, dRPC, Ankr).
- Direct curl probe: `curl -X POST https://provider-endpoint -d '{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}'`
- Check provider API key validity (probe with new key if suspicious).
- Check network connectivity from node to provider.

**Step 4 — Mitigation (on-call, t ≤ 60 min for P2, t ≤ 30 min for P1)**

Options in order of preference:
1. If provider has regional outage: switch to provider's alternate region endpoint.
2. If provider API key issue: rotate key (see §11.3).
3. If provider is globally down: ensure traffic is fully on next Tier 1 provider (confirm automatic failover worked).
4. If all Tier 1 down: confirm Tier 2 serving is active. Notify all Premium and Partner tier customers of degraded service within 30 minutes.
5. If Tier 2 also unavailable: escalate to P1. Bring up emergency Tier 0 capacity. Suspend writes system-wide.

**Step 5 — Recovery (automated + on-call)**

When provider recovers:
- Score rises above 60 for two consecutive windows.
- Automatic re-admission to routing pool.
- On-call confirms traffic has returned to primary provider (monitor `provider_health_score` metric).
- On-call closes the incident in PagerDuty with a post-incident comment.

**Step 6 — Post-Incident Review (within 48 hours)**

For P1 and P2 incidents: mandatory 30-minute post-incident review with:
- Timeline reconstruction.
- Root cause.
- Customer impact (number of failed requests, affected tiers).
- Action items to prevent recurrence.

For P3 and P4: async written summary in `#rpc-incidents` Slack channel.

### 12.3 Specific Scenario: All Tier 0 Substrate Nodes Offline

This is a HARD DEGRADED event and requires immediate P1 escalation:

1. System automatically suspends all extrinsic submission.
2. System serves reads from last-known-good cache (with `stale: true` flag).
3. On-call pages secondary on-call engineer.
4. Initiate emergency Tier 0 node restart or scale-out.
5. If node restart fails within 15 minutes, evaluate rolling back to last stable node binary.
6. Notify partner chains of settlement hold within 10 minutes of P1 declaration.

---

## 13. Metrics and Alerting

### 13.1 Core Prometheus Metrics

| Metric name | Type | Labels | Description |
|-------------|------|--------|-------------|
| `x3_rpc_provider_health_score` | Gauge | `provider, tier, chain_family` | Current 0–100 health score |
| `x3_rpc_provider_block_drift` | Gauge | `provider, tier, chain_family` | Block drift from canonical tip |
| `x3_rpc_provider_error_rate_bps` | Gauge | `provider, tier, chain_family` | Error rate in basis points |
| `x3_rpc_provider_latency_p95_ms` | Gauge | `provider, tier, chain_family` | p95 response latency in ms |
| `x3_rpc_request_total` | Counter | `method, tier, status` | Total requests routed |
| `x3_rpc_failover_total` | Counter | `from_provider, to_provider, reason` | Failover events |
| `x3_rpc_cache_hit_total` | Counter | `method_class` | Cache hits |
| `x3_rpc_cache_miss_total` | Counter | `method_class` | Cache misses |
| `x3_rpc_rate_limit_total` | Counter | `tier, method_class` | Rate limit rejections |
| `x3_rpc_degraded_mode_seconds` | Gauge | `mode` | Seconds spent in each degraded mode |

### 13.2 Alert Rules (PagerDuty — Pages)

The following conditions trigger an immediate PagerDuty page (interrupting):

| Alert | Condition | Severity |
|-------|-----------|----------|
| `RpcTier0AllDown` | ALL Tier 0 endpoints have score < 30 for > 60 s | P1 |
| `RpcAllTiersFailed` | No healthy endpoint in any tier for > 30 s | P1 |
| `RpcServedFromTier2` | Any traffic served from Tier 2 for > 15 min | P2 |
| `RpcSingleTier0Down` | Any single Tier 0 endpoint score < 30 for > 2 min | P2 |
| `RpcAllTier1Degraded` | All Tier 1 endpoints score < 60 simultaneously | P2 |
| `RpcHardDegradedMode` | System enters HARD DEGRADED mode | P1 |

### 13.3 Alert Rules (Slack #rpc-alerts — Warnings, No Page)

| Alert | Condition | Severity |
|-------|-----------|----------|
| `RpcProviderLowScore` | Any provider score < 70 for > 5 min | WARNING |
| `RpcHighLatency` | Any provider p95 latency > 1 000 ms for > 5 min | WARNING |
| `RpcHighErrorRate` | Any provider error rate > 300 BPS for > 5 min | WARNING |
| `RpcProviderFlapping` | Any provider flapping (see §10.4) | WARNING |
| `RpcRateLimitApproaching` | Provider aggregate at > 80% of rate limit | WARNING |
| `RpcCacheHitRateLow` | Cache hit rate < 40% for READ_LIGHT over 15 min | INFO |

### 13.4 Alerting Silence Windows

Planned maintenance windows may silence specific alerts. Silence must be:

- Pre-approved by Infrastructure Lead.
- Duration-limited (maximum 4 hours per silence).
- Documented in the maintenance log.

Silencing `RpcTier0AllDown` or `RpcAllTiersFailed` is prohibited during production hours (06:00–23:00 UTC).

---

## 14. Review Cadence

### 14.1 Quarterly Provider Performance Review

Conducted every 90 days. Agenda:

1. Review each provider's 90-day rolling uptime, p99 latency, and error rate from Prometheus data.
2. Compare against SLA thresholds (§3).
3. Evaluate reclassification triggers (§2.2).
4. Review API key rotation schedule (§11.2). Rotate all keys due within the next 30 days.
5. Update provider tier assignments if performance or cost warrants.
6. Review failover events: how many, which providers, average recovery time.
7. Review any SLA breach credits claimed or owed.

### 14.2 Monthly Rate Limit Review

Conducted monthly by the Infrastructure team:

1. Review 30-day rate limit hit counts per tier and method class.
2. If free-tier limit hits > 5% of total free-tier traffic, evaluate raising limits or tightening abuse detection.
3. If premium-tier burst limit hits > 1% of traffic, evaluate raising burst allowance or advising customer optimization.
4. Update `node/src/rpc_middleware.rs` method class limits if warranted.

### 14.3 Annual Policy Review

Conducted annually:

1. Full review of all thresholds in §4 (health score penalties).
2. Review of all cache TTLs in §7.
3. Review of monetization tier limits in §9.
4. Review of provider roster in §3 — add, remove, or reprioritize providers.
5. Publish updated policy version with changelog.

### 14.4 Ad-Hoc Review Triggers

This policy must be reviewed out-of-cycle if:

- A P1 incident occurs (review within 48 hours, see §12.2 Step 6).
- A provider announces a breaking change to their API or rate limit structure.
- A new chain family is added to X3 Chain.
- Regulatory or compliance requirements change.
- A security vulnerability is discovered in the RPC middleware.

---

## Appendix A — Constants Reference

The authoritative values for thresholds used in this policy are encoded in
`crates/x3-rpc-policy/src/lib.rs`. Any update to thresholds must be reflected
in both that crate and the corresponding section of this document.

| Constant | Value | Policy section |
|----------|-------|---------------|
| `FAILOVER_THRESHOLD` | 60 | §4.3, §4.4 |
| `FREEZE_THRESHOLD` | 30 | §4.3 |
| `MAX_BLOCK_DRIFT` | 10 blocks | §4.4, §6.5 |
| `MAX_ERROR_RATE_BPS` | 500 BPS | §4.4 |
| `DEGRADED_BLOCK_DRIFT` | 5 blocks | §6.5 |

## Appendix B — Related Documents

- `docs/RPC_CONFIGURATION.md` — Arbitrum mainnet RPC endpoint configuration and priority strategy.
- `docs/rpc.md` — X3 Chain RPC API reference (all 60+ methods).
- `docs/DRPC_NODECORE_DSHACKLE_INTEGRATION.md` — dRPC/Nodecore/Dshackle provider plane setup.
- `node/src/metrics.rs` — Health score implementation.
- `node/src/rpc_middleware.rs` — Rate limit enforcement implementation.
- `crates/x3-rpc-policy/src/lib.rs` — Types and constants used across node and sidecar.
