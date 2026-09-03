# X3 Funding OS Phase 1 — Funding Brain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first production-capable Funding Brain that discovers configured official funding sources, normalizes and deduplicates opportunities, verifies core program facts, extracts requirements, scores opportunities, persists an auditable history, and exposes a basic dashboard/API.

**Architecture:** Add one focused Rust service under `services/x3-funding-os` using Axum, Tokio, SQLx/PostgreSQL, and Reqwest. Phase 1 deliberately keeps discovery, verification, requirements extraction, scoring, and API delivery in one deployable service with clear internal module boundaries; later phases may split these modules behind NATS without changing their domain contracts. Add a small Next.js dashboard under `apps/funding-dashboard` that consumes the Rust API and shows pipeline status and score breakdowns.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1.x, SQLx 0.8/PostgreSQL, Reqwest 0.12, Serde, UUID, Chrono, SHA-256, URL, Scraper, tracing, Next.js/TypeScript, PostgreSQL 16.

**Spec:** `docs/superpowers/specs/2026-09-03-x3-funding-os-design.md`

## Global Constraints

- Canonical repository is `Cyptopimpinainteazy/xxxstar`.
- Only capabilities verified in `xxxstar` may be represented as production-complete unless a proposal explicitly identifies another repository.
- The Funding OS must preserve `LAUNCH_SCOPE.md` as the authority for public X3 status claims.
- No production path may introduce mocks, fake data, placeholder integrations, silent no-ops, or fabricated benchmarks.
- Grant-generated engineering must never directly modify protected `main`.
- Every persisted opportunity, requirement, score, and state transition must retain provenance and timestamps.
- Automatic actions must remain auditable and policy-gated.
- Phase 1 does not send outreach or submit applications; those belong to Phase 5.
- Prefer deterministic extraction and scoring where possible; model-assisted enrichment is added behind a typed interface in a later phase rather than becoming a correctness dependency in Phase 1.

---

## File Structure

Create this production service boundary:

```text
services/x3-funding-os/
├── Cargo.toml
├── src/
│   ├── main.rs                 # process bootstrap only
│   ├── lib.rs                  # module exports + AppState
│   ├── config.rs               # env configuration
│   ├── error.rs                # typed service errors + HTTP mapping
│   ├── domain.rs               # core persisted/value types
│   ├── db.rs                   # PostgreSQL pool + migration bootstrap
│   ├── repositories.rs         # SQLx persistence operations
│   ├── discovery.rs            # official-source crawler and candidate extraction
│   ├── verification.rs         # source/status/deadline/eligibility verification
│   ├── requirements.rs         # deterministic requirement extraction
│   ├── scoring.rs              # Funding Score formula + breakdown
│   ├── pipeline.rs             # discover -> verify -> parse -> score orchestration
│   └── api.rs                  # Axum routes/handlers
├── migrations/
│   └── 0001_funding_brain.sql
└── tests/
    ├── api.rs
    ├── discovery.rs
    ├── requirements.rs
    ├── scoring.rs
    └── fixtures/
        ├── grant-active.html
        ├── grant-expired.html
        └── sponsor-hardware.html
```

Create the dashboard boundary:

```text
apps/funding-dashboard/
├── package.json
├── tsconfig.json
├── next.config.ts
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   └── opportunities/[id]/page.tsx
└── lib/
    ├── api.ts
    └── types.ts
```

Modify:

```text
Cargo.toml                 # add services/x3-funding-os workspace member
.gitignore                 # ignore dashboard build artifacts only if not already covered
```

No other existing X3 runtime, pallet, bridge, compiler, or validator code is changed in Phase 1.

---

### Task 1: Create the Funding OS service shell and typed configuration

**Files:**
- Create: `services/x3-funding-os/Cargo.toml`
- Create: `services/x3-funding-os/src/lib.rs`
- Create: `services/x3-funding-os/src/main.rs`
- Create: `services/x3-funding-os/src/config.rs`
- Create: `services/x3-funding-os/src/error.rs`
- Modify: `Cargo.toml`
- Test: `services/x3-funding-os/tests/api.rs`

**Interfaces:**
- Consumes: `DATABASE_URL`, `FUNDING_BIND_ADDR`, `FUNDING_HTTP_TIMEOUT_SECS`, `FUNDING_USER_AGENT` environment variables.
- Produces: `Config::from_env() -> Result<Config, ConfigError>`, `build_router(AppState) -> Router`, `GET /healthz -> 200 {"status":"ok"}`.

- [ ] **Step 1: Write the failing health-route test**

```rust
// services/x3-funding-os/tests/api.rs
use axum::{body::Body, http::Request};
use tower::ServiceExt;
use x3_funding_os::{api::build_router, AppState};

#[tokio::test]
async fn healthz_returns_ok() {
    let app = build_router(AppState::for_tests());
    let response = app
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
```

- [ ] **Step 2: Run the test and confirm it fails because the crate/router does not exist**

Run:

```bash
cargo test -p x3-funding-os --test api healthz_returns_ok -- --nocapture
```

Expected: FAIL before compilation with missing package/module errors.

- [ ] **Step 3: Add the service crate to the root workspace**

Add this member to the existing `[workspace].members` array in root `Cargo.toml`:

```toml
"services/x3-funding-os",
```

- [ ] **Step 4: Create the service manifest**

```toml
# services/x3-funding-os/Cargo.toml
[package]
name = "x3-funding-os"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
axum = "0.7"
chrono = { version = "0.4", features = ["serde"] }
hex = "0.4"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
scraper = "0.20"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "migrate"] }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal", "time"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
url = "2"
uuid = { version = "1", features = ["v4", "serde"] }

[dev-dependencies]
http-body-util = "0.1"
wiremock = "0.6"
```

- [ ] **Step 5: Implement strict configuration loading**

```rust
// services/x3-funding-os/src/config.rs
use std::{env, net::SocketAddr, time::Duration};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub http_timeout: Duration,
    pub user_agent: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid FUNDING_BIND_ADDR: {0}")]
    BindAddr(String),
    #[error("invalid FUNDING_HTTP_TIMEOUT_SECS: {0}")]
    Timeout(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;
        let bind_addr_raw = env::var("FUNDING_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".into());
        let bind_addr = bind_addr_raw.parse().map_err(|_| ConfigError::BindAddr(bind_addr_raw.clone()))?;
        let timeout_raw = env::var("FUNDING_HTTP_TIMEOUT_SECS").unwrap_or_else(|_| "20".into());
        let timeout_secs = timeout_raw.parse::<u64>().map_err(|_| ConfigError::Timeout(timeout_raw.clone()))?;
        let user_agent = env::var("FUNDING_USER_AGENT").unwrap_or_else(|_| "X3-Funding-OS/0.1 (+official-program-research)".into());

        Ok(Self {
            database_url,
            bind_addr,
            http_timeout: Duration::from_secs(timeout_secs),
            user_agent,
        })
    }
}
```

- [ ] **Step 6: Implement the minimal app state and health router**

```rust
// services/x3-funding-os/src/lib.rs
pub mod api;
pub mod config;
pub mod error;

#[derive(Clone, Default)]
pub struct AppState;

impl AppState {
    pub fn for_tests() -> Self {
        Self
    }
}
```

```rust
// services/x3-funding-os/src/api.rs
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new().route("/healthz", get(healthz)).with_state(state)
}

async fn healthz() -> Json<Value> {
    Json(json!({"status": "ok"}))
}
```

- [ ] **Step 7: Implement bootstrap without hidden fallbacks**

```rust
// services/x3-funding-os/src/main.rs
use x3_funding_os::{api::build_router, config::Config, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    axum::serve(listener, build_router(AppState)).await?;
    Ok(())
}
```

- [ ] **Step 8: Run formatting, package check, and health test**

Run:

```bash
cargo fmt --all -- --check
cargo check -p x3-funding-os
cargo test -p x3-funding-os --test api healthz_returns_ok -- --nocapture
```

Expected: all PASS.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml services/x3-funding-os
git commit -m "feat(funding): add funding OS service shell"
```

---

### Task 2: Add the PostgreSQL schema, domain types, and persistence layer

**Files:**
- Create: `services/x3-funding-os/migrations/0001_funding_brain.sql`
- Create: `services/x3-funding-os/src/domain.rs`
- Create: `services/x3-funding-os/src/db.rs`
- Create: `services/x3-funding-os/src/repositories.rs`
- Modify: `services/x3-funding-os/src/lib.rs`
- Test: `services/x3-funding-os/tests/api.rs`

**Interfaces:**
- Produces: `Opportunity`, `OpportunityClass`, `OpportunityState`, `ProgramRequirement`, `FundingScoreBreakdown`, `OpportunityRepository`.
- Produces: `db::connect(database_url) -> PgPool`, `db::migrate(&PgPool)`.
- Persistence uniqueness: canonical source URL and `content_fingerprint` are both indexed; duplicate ingest updates `last_verified_at` rather than creating a second logical opportunity.

- [ ] **Step 1: Write a failing migration/persistence test**

Add an ignored-by-default integration test that requires `DATABASE_URL_TEST` so CI can enable it explicitly without silently substituting SQLite or in-memory state:

```rust
#[tokio::test]
async fn persisted_opportunity_round_trips() {
    let database_url = std::env::var("DATABASE_URL_TEST").expect("DATABASE_URL_TEST is required");
    let pool = x3_funding_os::db::connect(&database_url).await.unwrap();
    x3_funding_os::db::migrate(&pool).await.unwrap();

    let repo = x3_funding_os::repositories::OpportunityRepository::new(pool);
    let candidate = x3_funding_os::domain::NewOpportunity::fixture("https://foundation.example/grants/alpha");
    let inserted = repo.upsert(candidate).await.unwrap();
    let loaded = repo.get(inserted.id).await.unwrap().unwrap();

    assert_eq!(loaded.source_url, "https://foundation.example/grants/alpha");
}
```

- [ ] **Step 2: Create the first migration with auditable core entities**

```sql
-- services/x3-funding-os/migrations/0001_funding_brain.sql
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE organizations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name text NOT NULL,
    canonical_domain text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE opportunities (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id uuid REFERENCES organizations(id),
    title text NOT NULL,
    class text NOT NULL,
    state text NOT NULL,
    source_url text NOT NULL UNIQUE,
    official_source boolean NOT NULL DEFAULT false,
    description text NOT NULL,
    deadline timestamptz,
    funding_min_usd numeric(18,2),
    funding_max_usd numeric(18,2),
    hardware_value_usd numeric(18,2),
    compute_credit_value_usd numeric(18,2),
    security_value_usd numeric(18,2),
    content_fingerprint text NOT NULL,
    eligibility_confidence double precision NOT NULL DEFAULT 0,
    verification_confidence double precision NOT NULL DEFAULT 0,
    discovered_at timestamptz NOT NULL DEFAULT now(),
    last_verified_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (eligibility_confidence BETWEEN 0 AND 1),
    CHECK (verification_confidence BETWEEN 0 AND 1)
);
CREATE INDEX opportunities_content_fingerprint_idx ON opportunities(content_fingerprint);
CREATE INDEX opportunities_state_deadline_idx ON opportunities(state, deadline);

CREATE TABLE program_requirements (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    opportunity_id uuid NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    category text NOT NULL,
    severity text NOT NULL,
    requirement_text text NOT NULL,
    source_excerpt text NOT NULL,
    source_url text NOT NULL,
    confidence double precision NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (confidence BETWEEN 0 AND 1)
);

CREATE TABLE score_history (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    opportunity_id uuid NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    total_score double precision NOT NULL,
    technical_fit double precision NOT NULL,
    eligibility_confidence double precision NOT NULL,
    award_probability double precision NOT NULL,
    economic_value double precision NOT NULL,
    strategic_value double precision NOT NULL,
    evidence_readiness double precision NOT NULL,
    deadline_urgency double precision NOT NULL,
    contact_quality double precision NOT NULL,
    effort_efficiency double precision NOT NULL,
    formula_version text NOT NULL,
    scored_at timestamptz NOT NULL DEFAULT now(),
    CHECK (total_score BETWEEN 0 AND 100)
);

CREATE TABLE opportunity_state_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    opportunity_id uuid NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    from_state text,
    to_state text NOT NULL,
    reason text NOT NULL,
    actor text NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit_log (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    actor text NOT NULL,
    action text NOT NULL,
    target_type text NOT NULL,
    target_id uuid,
    reason text NOT NULL,
    input_hash text,
    output_json jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);
```

- [ ] **Step 3: Define exact domain enums and score types**

Use serde names matching the approved design values exactly:

```rust
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "text")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpportunityClass {
    Grant, Sponsor, Hardware, Compute, Security, Research,
    Bounty, Hackathon, Accelerator, PublicGoods, Partnership,
    Donation, Credit,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpportunityState {
    Discovered, Verified, Scored, RepoAudit, GapsIdentified,
    Preparing, Verifying, Ready, ProposalAudit, Submittable,
    Submitted, Deprioritized, Blocked, Expired, Rejected,
    Awarded, Withdrawn, HumanReviewRequired,
}
```

Also define `NewOpportunity`, `Opportunity`, `ProgramRequirement`, and `FundingScoreBreakdown` with strongly typed UUID/DateTime fields rather than raw JSON blobs.

- [ ] **Step 4: Implement DB connection and migration helpers**

```rust
pub async fn connect(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &sqlx::PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
```

- [ ] **Step 5: Implement `OpportunityRepository::upsert` with URL dedupe**

Use `INSERT ... ON CONFLICT (source_url) DO UPDATE` and update title, description, fingerprint, deadline, values, confidence, and `updated_at`, while preserving `discovered_at`.

The method signature must be:

```rust
pub async fn upsert(&self, input: NewOpportunity) -> Result<Opportunity, sqlx::Error>
```

- [ ] **Step 6: Run database-backed tests against a real PostgreSQL test database**

Run:

```bash
DATABASE_URL_TEST=postgres://postgres:postgres@127.0.0.1:5432/x3_funding_test \
  cargo test -p x3-funding-os persisted_opportunity_round_trips -- --nocapture
```

Expected: PASS. Do not replace this with an in-memory database.

- [ ] **Step 7: Commit**

```bash
git add services/x3-funding-os

git commit -m "feat(funding): add funding brain persistence model"
```

---

### Task 3: Implement official-source discovery and deterministic deduplication

**Files:**
- Create: `services/x3-funding-os/src/discovery.rs`
- Create: `services/x3-funding-os/tests/discovery.rs`
- Create: `services/x3-funding-os/tests/fixtures/grant-active.html`
- Create: `services/x3-funding-os/tests/fixtures/sponsor-hardware.html`
- Modify: `services/x3-funding-os/src/lib.rs`

**Interfaces:**
- Consumes: a list of configured `DiscoverySource { organization_name, root_url, allowed_hosts, class_hint }`.
- Produces: `DiscoveryCandidate { title, source_url, class_hint, raw_text, content_fingerprint }`.
- Produces: `Hunter::discover_source(&DiscoverySource) -> Result<Vec<DiscoveryCandidate>, DiscoveryError>`.

- [ ] **Step 1: Write a failing discovery test using a real local HTTP server**

```rust
#[tokio::test]
async fn discovers_only_links_on_allowed_host() {
    let server = wiremock::MockServer::start().await;
    let body = include_str!("fixtures/grant-active.html");
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let source = DiscoverySource {
        organization_name: "Example Foundation".into(),
        root_url: url::Url::parse(&server.uri()).unwrap(),
        allowed_hosts: vec![url::Url::parse(&server.uri()).unwrap().host_str().unwrap().to_string()],
        class_hint: OpportunityClass::Grant,
    };

    let hunter = Hunter::new(reqwest::Client::new());
    let results = hunter.discover_source(&source).await.unwrap();
    assert!(results.iter().all(|r| r.source_url.host_str() == source.allowed_hosts.first().map(String::as_str)));
}
```

- [ ] **Step 2: Implement canonical URL normalization**

The function must remove fragments and known tracking parameters but retain application-relevant query parameters:

```rust
pub fn canonicalize_url(mut url: url::Url) -> url::Url {
    url.set_fragment(None);
    let filtered: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !matches!(k.as_ref(), "utm_source" | "utm_medium" | "utm_campaign" | "utm_term" | "utm_content" | "gclid" | "fbclid"))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.set_query(None);
    if !filtered.is_empty() {
        url.query_pairs_mut().extend_pairs(filtered);
    }
    url
}
```

- [ ] **Step 3: Implement content fingerprinting**

Normalize Unicode whitespace to single spaces, lowercase the content, then SHA-256 the normalized title + canonical URL + extracted page text:

```rust
pub fn fingerprint(title: &str, source_url: &url::Url, text: &str) -> String {
    use sha2::{Digest, Sha256};
    let normalized = format!("{}|{}|{}", title.trim().to_lowercase(), source_url, text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase());
    hex::encode(Sha256::digest(normalized.as_bytes()))
}
```

- [ ] **Step 4: Implement HTML link extraction with strict host allow-listing**

Use `scraper::Html` and `Selector::parse("a[href]")`. Resolve relative links against `root_url`. Reject `mailto:`, `javascript:`, non-HTTP(S), and hosts not listed in `allowed_hosts`.

Candidate link text or URL must match at least one configured discovery term from this default set:

```text
grant
funding
sponsor
sponsorship
credits
startup
accelerator
bounty
hackathon
research
public goods
hardware
compute
security
```

- [ ] **Step 5: Add a duplicate-content regression test**

Two URLs with different UTM tags but the same canonical page must produce the same canonical URL/fingerprint and be collapsed before persistence.

- [ ] **Step 6: Run tests**

```bash
cargo test -p x3-funding-os --test discovery -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add services/x3-funding-os/src/discovery.rs services/x3-funding-os/tests

git commit -m "feat(funding): add official source discovery"
```

---

### Task 4: Implement program verification and requirement extraction

**Files:**
- Create: `services/x3-funding-os/src/verification.rs`
- Create: `services/x3-funding-os/src/requirements.rs`
- Create: `services/x3-funding-os/tests/requirements.rs`
- Create: `services/x3-funding-os/tests/fixtures/grant-expired.html`
- Modify: `services/x3-funding-os/src/lib.rs`

**Interfaces:**
- Produces: `ProgramVerifier::verify(&DiscoveryCandidate) -> VerificationReport`.
- Produces: `RequirementExtractor::extract(&VerifiedProgram) -> Vec<ProgramRequirementDraft>`.
- `VerificationReport` includes `official_source`, `active_status`, `deadline`, `eligibility_confidence`, `verification_confidence`, and `warnings`.

- [ ] **Step 1: Write failing tests for active vs expired programs**

```rust
#[test]
fn expired_deadline_is_not_marked_active() {
    let html = include_str!("fixtures/grant-expired.html");
    let report = ProgramVerifier::default().verify_text(
        "https://foundation.example/grants/closed",
        html,
        chrono::Utc::now(),
    ).unwrap();
    assert_eq!(report.active_status, ProgramStatus::Expired);
}
```

- [ ] **Step 2: Implement deterministic deadline parsing**

Support ISO dates plus common English month formats using a bounded parser. If multiple dates are present, label the result ambiguous unless context includes one of `deadline`, `apply by`, `applications close`, or `submission deadline`. Ambiguous dates must lower verification confidence rather than guessing.

- [ ] **Step 3: Implement active-status rules**

`ProgramStatus` must be one of:

```rust
pub enum ProgramStatus { Active, Expired, Closed, Unknown }
```

Rules:

```text
explicit "closed" / "applications closed" -> Closed
verified deadline < now -> Expired
explicit "applications open" and future/no deadline -> Active
otherwise -> Unknown
```

`Unknown` may be persisted but must not transition to `VERIFIED` without a later verification pass.

- [ ] **Step 4: Implement requirement extraction by evidence-bearing sentence**

Split visible page text into sentences/blocks and retain blocks containing requirement signals:

```text
must
required
eligible
eligibility
open source
license
deadline
applicant
team
jurisdiction
milestone
budget
deliverable
security
audit
repository
prototype
demo
```

Each output stores the exact `source_excerpt`, source URL, category, severity, and confidence. Never create a requirement without a source excerpt.

- [ ] **Step 5: Map requirement severity deterministically**

```text
explicit must/required/ineligible condition -> BLOCKER
security/audit/working prototype/core technical deliverable -> CRITICAL
documentation/milestone/reporting requirement -> IMPORTANT
preference/nice-to-have language -> OPTIONAL
```

- [ ] **Step 6: Add tests proving requirements always carry provenance**

```rust
#[test]
fn every_requirement_has_source_evidence() {
    let html = include_str!("fixtures/grant-active.html");
    let requirements = RequirementExtractor::default().extract_text("https://foundation.example/grants/alpha", html).unwrap();
    assert!(!requirements.is_empty());
    assert!(requirements.iter().all(|r| !r.source_excerpt.trim().is_empty()));
}
```

- [ ] **Step 7: Run tests**

```bash
cargo test -p x3-funding-os --test requirements -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add services/x3-funding-os/src/verification.rs services/x3-funding-os/src/requirements.rs services/x3-funding-os/tests

git commit -m "feat(funding): verify programs and extract requirements"
```

---

### Task 5: Implement the versioned Funding Score and state transitions

**Files:**
- Create: `services/x3-funding-os/src/scoring.rs`
- Modify: `services/x3-funding-os/src/repositories.rs`
- Create: `services/x3-funding-os/tests/scoring.rs`
- Modify: `services/x3-funding-os/src/lib.rs`

**Interfaces:**
- Produces: `score_v1(ScoreInputs) -> FundingScoreBreakdown`.
- Produces: `OpportunityRepository::record_score(opportunity_id, breakdown)`.
- Produces: `OpportunityRepository::transition(id, to_state, actor, reason)` with allowed-transition validation.

- [ ] **Step 1: Write the failing exact-weight score test**

```rust
#[test]
fn score_v1_uses_approved_weights() {
    let input = ScoreInputs {
        technical_fit: 100.0,
        eligibility_confidence: 80.0,
        award_probability: 60.0,
        economic_value: 90.0,
        strategic_value: 70.0,
        evidence_readiness: 50.0,
        deadline_urgency: 40.0,
        contact_quality: 20.0,
        effort_efficiency: 80.0,
    };
    let score = score_v1(input).unwrap();
    assert!((score.total_score - 73.0).abs() < f64::EPSILON);
    assert_eq!(score.formula_version, "funding-score-v1");
}
```

The expected total is calculated as:

```text
100*.20 + 80*.15 + 60*.15 + 90*.15 + 70*.10 + 50*.10 + 40*.05 + 20*.05 + 80*.05 = 73
```

- [ ] **Step 2: Implement bounded score inputs**

Reject NaN, infinity, and values outside `0..=100`; do not silently clamp invalid inputs.

- [ ] **Step 3: Implement the approved weighting exactly**

```rust
pub fn score_v1(i: ScoreInputs) -> Result<FundingScoreBreakdown, ScoreError> {
    i.validate()?;
    let total = i.technical_fit * 0.20
        + i.eligibility_confidence * 0.15
        + i.award_probability * 0.15
        + i.economic_value * 0.15
        + i.strategic_value * 0.10
        + i.evidence_readiness * 0.10
        + i.deadline_urgency * 0.05
        + i.contact_quality * 0.05
        + i.effort_efficiency * 0.05;
    Ok(FundingScoreBreakdown::from_inputs("funding-score-v1", total, i))
}
```

- [ ] **Step 4: Implement explicit allowed state transitions**

For Phase 1, allow only:

```text
DISCOVERED -> VERIFIED
DISCOVERED -> EXPIRED
DISCOVERED -> BLOCKED
VERIFIED -> SCORED
VERIFIED -> EXPIRED
SCORED -> DEPRIORITIZED
SCORED -> REPO_AUDIT
any nonterminal -> HUMAN_REVIEW_REQUIRED
```

Reject every other transition with `TransitionError::NotAllowed` and do not write a state event on failure.

- [ ] **Step 5: Persist every score and successful transition**

Every score goes to `score_history`; every state change goes to `opportunity_state_events` with `actor` and non-empty `reason`.

- [ ] **Step 6: Run scoring tests and DB transition tests**

```bash
cargo test -p x3-funding-os --test scoring -- --nocapture
DATABASE_URL_TEST=postgres://postgres:postgres@127.0.0.1:5432/x3_funding_test \
  cargo test -p x3-funding-os transition -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add services/x3-funding-os/src/scoring.rs services/x3-funding-os/src/repositories.rs services/x3-funding-os/tests/scoring.rs

git commit -m "feat(funding): add versioned opportunity scoring"
```

---

### Task 6: Wire the Funding Brain pipeline and REST API

**Files:**
- Create: `services/x3-funding-os/src/pipeline.rs`
- Modify: `services/x3-funding-os/src/api.rs`
- Modify: `services/x3-funding-os/src/lib.rs`
- Modify: `services/x3-funding-os/src/main.rs`
- Modify: `services/x3-funding-os/tests/api.rs`

**Interfaces:**
- Produces: `FundingPipeline::run_source(source_id) -> PipelineRunSummary`.
- API endpoints:
  - `GET /healthz`
  - `GET /api/v1/opportunities?state=&class=&limit=&offset=`
  - `GET /api/v1/opportunities/:id`
  - `GET /api/v1/opportunities/:id/requirements`
  - `GET /api/v1/opportunities/:id/scores`
  - `POST /api/v1/discovery/run`
- `POST /api/v1/discovery/run` accepts only configured source IDs; it never accepts arbitrary crawler URLs from unauthenticated input.

- [ ] **Step 1: Write a failing pipeline test**

The test should mount a local HTTP source, run the pipeline, then assert one persisted opportunity has state `SCORED`, requirements, and one score-history record.

- [ ] **Step 2: Implement `FundingPipeline::run_source` in this exact order**

```text
load configured source
fetch source root
extract candidates
canonicalize/dedupe candidates
fetch candidate page
verify program
persist opportunity as DISCOVERED
if expired -> transition EXPIRED and stop candidate
if verification unknown -> leave DISCOVERED and record warning
if verified active -> transition VERIFIED
extract + persist requirements
calculate score inputs from verified facts and configured X3 component tags
persist score history
transition VERIFIED -> SCORED
record audit log for pipeline completion
```

Do not mark a program `VERIFIED` from a successful HTTP response alone.

- [ ] **Step 3: Implement initial score-input derivation without invented evidence**

Phase 1 may derive these dimensions only from observable/program metadata:

```text
technical_fit       <- keyword/component match confidence
eligibility         <- verifier confidence
award_probability   <- conservative configured baseline per opportunity class
 economic_value      <- normalized disclosed cash/hardware/credit value
strategic_value     <- configured organization/component weighting
 evidence_readiness  <- 0 in Phase 1 until Repo Intelligence exists
 deadline_urgency    <- deterministic time-to-deadline function
 contact_quality     <- 0 in Phase 1 until Contact Intel exists
 effort_efficiency   <- requirement count + application complexity heuristic
```

The two unavailable dimensions remain explicitly `0`; do not guess them.

- [ ] **Step 4: Implement API pagination and stable response envelopes**

Use:

```json
{
  "data": [],
  "meta": {"limit": 50, "offset": 0, "total": 0}
}
```

Reject `limit > 200` with HTTP 400.

- [ ] **Step 5: Add API integration tests**

Test at minimum:

```text
GET unknown opportunity -> 404
GET list default pagination -> 200
limit=201 -> 400
POST discovery unknown source -> 404
successful pipeline record -> visible through list/detail/requirements/scores
```

- [ ] **Step 6: Run service verification**

```bash
cargo fmt --all -- --check
cargo clippy -p x3-funding-os --all-targets -- -D warnings
cargo test -p x3-funding-os -- --nocapture
cargo build --release -p x3-funding-os
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add services/x3-funding-os

git commit -m "feat(funding): wire funding brain pipeline and API"
```

---

### Task 7: Add the basic Funding Command Center dashboard

**Files:**
- Create: `apps/funding-dashboard/package.json`
- Create: `apps/funding-dashboard/tsconfig.json`
- Create: `apps/funding-dashboard/next.config.ts`
- Create: `apps/funding-dashboard/app/layout.tsx`
- Create: `apps/funding-dashboard/app/page.tsx`
- Create: `apps/funding-dashboard/app/opportunities/[id]/page.tsx`
- Create: `apps/funding-dashboard/lib/api.ts`
- Create: `apps/funding-dashboard/lib/types.ts`

**Interfaces:**
- Consumes: Funding OS API base URL from `FUNDING_API_BASE_URL` on the server.
- Produces: dashboard summary table plus opportunity detail page.
- No write actions are added to the dashboard in Phase 1 except triggering discovery if an authenticated internal deployment later enables it; the initial UI is read-only.

- [ ] **Step 1: Create the package manifest with exact scripts**

```json
{
  "name": "x3-funding-dashboard",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "typescript": "^5.6.0"
  }
}
```

- [ ] **Step 2: Define API types matching Rust response fields exactly**

```ts
export type OpportunityState =
  | "DISCOVERED" | "VERIFIED" | "SCORED" | "REPO_AUDIT"
  | "GAPS_IDENTIFIED" | "PREPARING" | "VERIFYING" | "READY"
  | "PROPOSAL_AUDIT" | "SUBMITTABLE" | "SUBMITTED"
  | "DEPRIORITIZED" | "BLOCKED" | "EXPIRED" | "REJECTED"
  | "AWARDED" | "WITHDRAWN" | "HUMAN_REVIEW_REQUIRED";

export interface OpportunitySummary {
  id: string;
  title: string;
  class: string;
  state: OpportunityState;
  source_url: string;
  deadline: string | null;
  latest_score: number | null;
}
```

- [ ] **Step 3: Implement server-side API fetching that fails visibly**

`lib/api.ts` must throw on non-2xx status and must not replace API failures with fake/empty opportunity data.

- [ ] **Step 4: Implement the dashboard home page**

Display:

```text
Total opportunities
Discovered
Verified
Scored
Expired/Blocked
Top opportunities by latest score
Deadline
Class
State
Source link
```

- [ ] **Step 5: Implement opportunity detail**

Display the latest score plus all score dimensions, requirements with source excerpts, verification metadata, and state history. Phase 1 must visibly show that Evidence Readiness and Contact Quality are zero/unavailable rather than inventing values.

- [ ] **Step 6: Run dashboard verification**

```bash
cd apps/funding-dashboard
npm install
npm run typecheck
npm run build
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/funding-dashboard

git commit -m "feat(funding): add funding command center dashboard"
```

---

### Task 8: Add Phase 1 deployment, seed configuration, and end-to-end acceptance test

**Files:**
- Create: `services/x3-funding-os/config/sources.example.json`
- Create: `services/x3-funding-os/README.md`
- Create: `scripts/funding-os-e2e.sh`
- Create: `docs/funding/FUNDING_OS_PHASE1_OPERATIONS.md`
- Modify: `.github/workflows/ci.yml` only if the existing workflow structure can absorb focused package checks without changing protected status-check semantics; otherwise create `/.github/workflows/funding-os.yml`.

**Interfaces:**
- Source config entries: stable `id`, organization, root URL, allowed hosts, class hint, component tags, strategic weight.
- Acceptance script proves a real PostgreSQL-backed service can ingest a controlled official-style source, persist it, score it, and serve it over HTTP.

- [ ] **Step 1: Add explicit source configuration format**

```json
[
  {
    "id": "example-foundation-grants",
    "organization_name": "Example Foundation",
    "root_url": "https://foundation.example/grants",
    "allowed_hosts": ["foundation.example"],
    "class_hint": "GRANT",
    "component_tags": ["interoperability", "security", "developer-tooling"],
    "strategic_weight": 80
  }
]
```

This file is an example schema only and is not shipped as a production source list containing invented active opportunities.

- [ ] **Step 2: Document production startup exactly**

The README must include:

```bash
export DATABASE_URL=postgres://x3_funding:<password>@127.0.0.1:5432/x3_funding
export FUNDING_BIND_ADDR=127.0.0.1:8787
export FUNDING_SOURCES_FILE=/etc/x3/funding-sources.json
cargo run --release -p x3-funding-os
```

Document migration behavior, backup requirements, required network egress, and how to disable a source without deleting historical records.

- [ ] **Step 3: Add the E2E script with hard failures**

`scripts/funding-os-e2e.sh` must use `set -euo pipefail`, require a real `DATABASE_URL_TEST`, launch the service, wait for `/healthz`, run discovery against a controlled local fixture server, query `/api/v1/opportunities`, assert at least one `SCORED` record, and exit non-zero on any failed assertion.

- [ ] **Step 4: Add focused CI checks**

Required checks:

```bash
cargo fmt --all -- --check
cargo clippy -p x3-funding-os --all-targets -- -D warnings
cargo test -p x3-funding-os
cargo build --release -p x3-funding-os
cd apps/funding-dashboard && npm ci && npm run typecheck && npm run build
```

Database-backed integration tests run in a PostgreSQL service container and use `DATABASE_URL_TEST`; no in-memory replacement is allowed.

- [ ] **Step 5: Run the complete Phase 1 acceptance gate locally**

```bash
cargo fmt --all -- --check
cargo clippy -p x3-funding-os --all-targets -- -D warnings
cargo test -p x3-funding-os -- --nocapture
cargo build --release -p x3-funding-os
DATABASE_URL_TEST=postgres://postgres:postgres@127.0.0.1:5432/x3_funding_test bash scripts/funding-os-e2e.sh
cd apps/funding-dashboard && npm ci && npm run typecheck && npm run build
```

Expected: every command PASS.

- [ ] **Step 6: Inspect outputs for prohibited placeholder behavior**

Run:

```bash
rg -n "TODO|FIXME|todo!\(|unimplemented!\(|placeholder|fake data|mock opportunity|hardcoded score|silent fallback" \
  services/x3-funding-os apps/funding-dashboard docs/funding scripts/funding-os-e2e.sh
```

Expected: no production-path placeholder implementation. Fixture/test references must be clearly confined to test files or example configuration.

- [ ] **Step 7: Commit**

```bash
git add services/x3-funding-os apps/funding-dashboard scripts/funding-os-e2e.sh docs/funding .github/workflows

git commit -m "test(funding): add phase 1 production acceptance gate"
```

---

## Phase 1 Exit Criteria

Phase 1 is complete only when all of the following are evidenced by fresh command output:

```text
[PASS] Funding OS release binary builds.
[PASS] PostgreSQL migration applies to an empty test database.
[PASS] Official-source crawler enforces host allow lists.
[PASS] Canonical URL/content dedupe works.
[PASS] Expired/closed programs do not become VERIFIED.
[PASS] Every extracted requirement contains source evidence.
[PASS] Funding Score v1 matches the approved formula exactly.
[PASS] Invalid state transitions are rejected.
[PASS] Every successful state transition is persisted.
[PASS] API pagination and detail endpoints pass integration tests.
[PASS] Dashboard typecheck and production build pass.
[PASS] End-to-end test persists and serves at least one SCORED controlled test opportunity.
[PASS] Production-path placeholder scan is clean.
```

The phase must not claim autonomous grant submission, Contact Intel, repository readiness, proposal generation, award tracking, or outreach. Those capabilities are intentionally outside this plan.

## Follow-On Plan Boundaries

After Phase 1 passes, create separate implementation plans in this order:

1. `x3-funding-os-phase-2-repo-intelligence` — GitHub evidence ingestion, Claim Ledger, launch-scope reconciliation, readiness scoring.
2. `x3-funding-os-phase-3-grant-aware-engineering` — gap mapper, funding-weighted backlog, isolated branches, verification, PR generation.
3. `x3-funding-os-phase-4-proposal-factory` — X3 Matcher, evidence-constrained proposal generation, budgets, claim auditor.
4. `x3-funding-os-phase-5-autonomous-acquisition` — Contact Intel, policy-gated email, permitted forms/browser automation, follow-ups.
5. `x3-funding-os-phase-6-award-operations` — awards, hardware registry, restricted budgets, deliverables, reports, conversion analytics.

Each follow-on plan must preserve the same evidence, provenance, stop-condition, and protected-main constraints from the design spec.
