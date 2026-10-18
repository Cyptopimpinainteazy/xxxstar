//! Bounded official-source discovery. External documents are inert data; they
//! cannot establish eligibility, authorize submissions or execute agent actions.
use crate::{
    db::Database,
    error::{GatewayError, Result},
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, time::Duration};
use tokio::{sync::Mutex, time::Instant};
use uuid::Uuid;

const SOURCE: &str = "grants.gov";
const NORMALIZATION_VERSION: i32 = 2;
const SOURCE_URL: &str = "https://www.grants.gov/api/api-guide";
const API_BASE: &str = "https://api.grants.gov/v1/api";
// Operator request budgets: at most 30 detailed opportunities per refresh,
// 4 MiB per response, one request every 500 ms per gateway process.
const PAGE_SIZE: u32 = 10;
const MAX_PAGES: u32 = 3;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const REQUEST_SPACING: Duration = Duration::from_millis(500);
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_ATTEMPTS: u32 = 3;
static REQUEST_GATE: Mutex<Option<Instant>> = Mutex::const_new(None);
// Prevent overlapping refreshes in this process; persistence handles dedupe
// across gateway replicas. This is centralized discovery, not a durable queue.
static DISCOVERY_GATE: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRequest {
    pub keyword: String,
    #[serde(default = "one_page")]
    pub pages: u32,
}
fn one_page() -> u32 {
    1
}
impl DiscoveryRequest {
    pub fn validate(&self) -> Result<()> {
        if self.keyword.trim().is_empty()
            || self.keyword.len() > 120
            || self.keyword.chars().any(char::is_control)
            || !(1..=MAX_PAGES).contains(&self.pages)
        {
            return Err(GatewayError::BadRequest(
                "keyword must contain 1–120 bytes without control characters; pages must be 1–3"
                    .into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingOpportunity {
    pub normalization_version: i32,
    pub source: String,
    pub source_opportunity_id: i64,
    pub opportunity_number: String,
    pub program_name: String,
    pub funding_agency: String,
    pub official_url: String,
    pub description: String,
    pub deadline: Option<NaiveDate>,
    pub deadline_original: Option<String>,
    pub eligibility_requirements: Option<String>,
    pub eligibility_status: String,
    pub organization_types: Vec<String>,
    pub geographic_restrictions: Option<String>,
    pub cost_sharing_required: Option<bool>,
    pub award_min_usd_cents: Option<i64>,
    pub award_max_usd_cents: Option<i64>,
    pub award_duration: Option<String>,
    pub reporting_requirements: Option<String>,
    pub last_verified_at: DateTime<Utc>,
    pub expired: bool,
}

fn source_error(message: &str) -> GatewayError {
    GatewayError::Internal(format!("official funding source: {message}"))
}

fn decode_envelope(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(source_error("response exceeds size budget"));
    }
    let mut envelope: Value =
        serde_json::from_slice(bytes).map_err(|_| source_error("invalid JSON"))?;
    if envelope.get("errorcode").and_then(Value::as_i64) != Some(0) {
        return Err(source_error("API reported failure"));
    }
    // The optional service token is deliberately outside the retained data.
    envelope
        .as_object_mut()
        .and_then(|obj| obj.remove("data"))
        .filter(|data| data.is_object())
        .ok_or_else(|| source_error("missing response data"))
}
fn required_text(data: &Value, key: &str) -> Result<String> {
    data.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| source_error("missing required source field"))
}
fn optional_text(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
}

/// Parse decimal USD exactly to cents. Unknown is not zero, and excessive
/// precision, negative values and overflow are rejected rather than rounded.
fn usd_cents(value: Option<&Value>) -> Result<Option<i64>> {
    let Some(value) = value.filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let amount = match value {
        Value::String(text) => text.clone(),
        Value::Number(number) if number.as_u64().is_some() => number.to_string(),
        _ => return Err(source_error("invalid award amount type")),
    };
    if amount.trim().is_empty() || amount.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let mut pieces = amount.split('.');
    let whole = pieces.next().unwrap_or_default();
    let fraction = pieces.next();
    if pieces.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || fraction
            .is_some_and(|f| f.is_empty() || f.len() > 2 || !f.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err(source_error("invalid award amount precision"));
    }
    let whole = whole
        .parse::<i64>()
        .map_err(|_| source_error("award amount overflow"))?;
    let fraction = match fraction {
        None => 0,
        Some(f) => f
            .parse::<i64>()
            .map_err(|_| source_error("invalid award fraction"))?
            .checked_mul(if f.len() == 1 { 10 } else { 1 })
            .ok_or_else(|| source_error("award amount overflow"))?,
    };
    whole
        .checked_mul(100)
        .and_then(|v| v.checked_add(fraction))
        .map(Some)
        .ok_or_else(|| source_error("award amount overflow"))
}

pub fn normalize_opportunity(
    data: Value,
    expected_id: i64,
    observed: DateTime<Utc>,
) -> Result<FundingOpportunity> {
    if expected_id <= 0 || data.get("id").and_then(Value::as_i64) != Some(expected_id) {
        return Err(source_error("opportunity identity mismatch"));
    }
    let synopsis = data
        .get("synopsis")
        .filter(|v| v.is_object())
        .ok_or_else(|| source_error("missing synopsis"))?;
    let deadline = match synopsis.get("responseDateStr") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(match value.len() {
            10 => NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| source_error("invalid deadline"))?,
            // Grants.gov represents local date/time with hyphen-separated fields.
            19 => NaiveDateTime::parse_from_str(value, "%Y-%m-%d-%H-%M-%S")
                .map_err(|_| source_error("invalid deadline"))?
                .date(),
            _ => return Err(source_error("invalid deadline format")),
        }),
        Some(_) => return Err(source_error("invalid deadline type")),
    };
    let award_min = usd_cents(synopsis.get("awardFloor"))?;
    let award_max = usd_cents(synopsis.get("awardCeiling"))?;
    if award_min.zip(award_max).is_some_and(|(min, max)| min > max) {
        return Err(source_error("inverted award range"));
    }
    let agency = data
        .get("agencyDetails")
        .and_then(|details| optional_text(details, "agencyName"))
        .or_else(|| {
            synopsis
                .get("agencyDetails")
                .and_then(|details| optional_text(details, "agencyName"))
        })
        .or_else(|| optional_text(synopsis, "agencyName"))
        .ok_or_else(|| source_error("missing funding agency identity"))?;
    Ok(FundingOpportunity {
        normalization_version: NORMALIZATION_VERSION,
        source: SOURCE.into(),
        source_opportunity_id: expected_id,
        opportunity_number: required_text(&data, "opportunityNumber")?,
        program_name: required_text(&data, "opportunityTitle")?,
        funding_agency: agency,
        official_url: format!("https://www.grants.gov/search-results-detail/{expected_id}"),
        description: optional_text(synopsis, "synopsisDesc").unwrap_or_default(),
        deadline,
        deadline_original: optional_text(synopsis, "responseDate"),
        eligibility_requirements: optional_text(synopsis, "applicantEligibilityDesc"),
        eligibility_status: "UNKNOWN".into(),
        organization_types: synopsis
            .get("applicantTypes")
            .and_then(Value::as_array)
            .map(|types| {
                types
                    .iter()
                    .filter_map(|t| optional_text(t, "description"))
                    .collect()
            })
            .unwrap_or_default(),
        geographic_restrictions: None,
        cost_sharing_required: synopsis.get("costSharing").and_then(Value::as_bool),
        award_min_usd_cents: award_min,
        award_max_usd_cents: award_max,
        award_duration: None,
        reporting_requirements: None,
        last_verified_at: observed,
        expired: deadline.is_some_and(|day| day < observed.date_naive()),
    })
}

async fn official_request(client: &reqwest::Client, operation: &str, body: Value) -> Result<Value> {
    for attempt in 0..MAX_ATTEMPTS {
        let mut gate = REQUEST_GATE.lock().await;
        if let Some(previous) = *gate {
            tokio::time::sleep_until(previous + REQUEST_SPACING).await;
        }
        *gate = Some(Instant::now());
        drop(gate);
        let response = client
            .post(format!("{API_BASE}/{operation}"))
            .json(&body)
            .send()
            .await;
        let mut response = match response {
            Ok(response) => response,
            Err(_) if attempt + 1 < MAX_ATTEMPTS => continue,
            Err(_) => return Err(source_error("transport failed after bounded retries")),
        };
        if response.status().as_u16() == 429 || response.status().is_server_error() {
            if attempt + 1 == MAX_ATTEMPTS {
                return Err(source_error("source unavailable or rate limited"));
            }
            let seconds = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(2);
            if response.headers().contains_key("retry-after")
                && (seconds > 20
                    || response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .is_none())
            {
                return Err(source_error(
                    "source requested a deferred retry; refresh later",
                ));
            }
            tokio::time::sleep(Duration::from_secs(seconds)).await;
            continue;
        }
        if !response.status().is_success() {
            return Err(source_error("HTTP request rejected"));
        }
        if response
            .content_length()
            .is_some_and(|len| len > 4 * 1024 * 1024)
        {
            return Err(source_error("response exceeds size budget"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| source_error("response interrupted"))?
        {
            if chunk.len() > MAX_RESPONSE_BYTES.saturating_sub(bytes.len()) {
                return Err(source_error("response exceeds size budget"));
            }
            bytes.extend_from_slice(&chunk);
        }
        return decode_envelope(&bytes);
    }
    Err(source_error("retry budget exhausted"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_source_failure_missing_fields_and_oversized_documents() {
        for document in [
            json!({"errorcode":1,"data":{}}),
            json!({"errorcode":0}),
            json!({"errorcode":0,"data":[]}),
            json!({"data":{}}),
        ] {
            assert!(decode_envelope(&serde_json::to_vec(&document).unwrap()).is_err());
        }
        assert!(decode_envelope(b"corrupted response").is_err());
        assert!(decode_envelope(&vec![b'x'; MAX_RESPONSE_BYTES + 1]).is_err());
    }

    #[test]
    fn never_retains_the_optional_service_token_as_source_evidence() {
        let data = decode_envelope(
            br#"{"errorcode":0,"token":"fixture-service-token","data":{"id":123}}"#,
        )
        .unwrap();
        assert_eq!(data, json!({"id":123}));
        assert!(!serde_json::to_string(&data)
            .unwrap()
            .contains("fixture-service-token"));
    }
}

#[derive(Debug, Serialize)]
pub struct DiscoveryResult {
    pub imported: u32,
    pub changed: u32,
}

pub async fn discover(db: &Database, request: DiscoveryRequest) -> Result<DiscoveryResult> {
    request.validate()?;
    let _guard = DISCOVERY_GATE
        .try_lock()
        .map_err(|_| GatewayError::BadRequest("a discovery refresh is already running".into()))?;
    sqlx::query("INSERT INTO funding_sources (source_id, official_url) VALUES ($1,$2) ON CONFLICT DO NOTHING")
        .bind(SOURCE).bind(SOURCE_URL).execute(db.pool()).await?;
    let result = tokio::time::timeout(Duration::from_secs(120), discover_inner(db, request))
        .await
        .map_err(|_| source_error("refresh exceeded the 120-second operation budget"))
        .and_then(|result| result);
    match &result {
        Ok(_) => {
            sqlx::query("UPDATE funding_sources SET last_success_at=NOW(), last_error_code=NULL WHERE source_id=$1")
            .bind(SOURCE).execute(db.pool()).await?;
        }
        Err(_) => {
            sqlx::query("UPDATE funding_sources SET last_failure_at=NOW(), last_error_code='refresh_failed' WHERE source_id=$1")
            .bind(SOURCE).execute(db.pool()).await?;
        }
    }
    result
}

async fn discover_inner(db: &Database, request: DiscoveryRequest) -> Result<DiscoveryResult> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(HTTP_TIMEOUT)
        .user_agent("X3FundingDiscovery/0.1 (+https://github.com/Cyptopimpinainteazy/xxxstar)")
        .build()
        .map_err(|_| source_error("HTTP client initialization failed"))?;
    let mut result = DiscoveryResult {
        imported: 0,
        changed: 0,
    };
    let mut seen = BTreeSet::new();
    for page in 0..request.pages {
        let data = official_request(
            &client,
            "search2",
            json!({"keyword":request.keyword,
            "rows":PAGE_SIZE,"startRecordNum":page * PAGE_SIZE,"oppStatuses":"posted|forecasted"}),
        )
        .await?;
        let hits = data
            .get("oppHits")
            .and_then(Value::as_array)
            .ok_or_else(|| source_error("missing search results"))?;
        if hits.len() > usize::try_from(PAGE_SIZE).map_err(|_| source_error("invalid page size"))? {
            return Err(source_error("source exceeded requested page size"));
        }
        for hit in hits {
            let id = required_text(hit, "id")?
                .parse::<i64>()
                .map_err(|_| source_error("invalid opportunity ID"))?;
            if !seen.insert(id) {
                continue;
            }
            let raw =
                official_request(&client, "fetchOpportunity", json!({"opportunityId":id})).await?;
            let normalized = normalize_opportunity(raw.clone(), id, Utc::now())?;
            result.changed += u32::from(db.store_funding_snapshot(&normalized, &raw).await?);
            result.imported += 1;
        }
        if hits.len() < usize::try_from(PAGE_SIZE).map_err(|_| source_error("invalid page size"))? {
            break;
        }
    }
    Ok(result)
}

impl Database {
    /// Atomic normalized record + immutable source snapshot + audit event.
    /// Source refresh never resets human review stages or promotes eligibility.
    pub async fn store_funding_snapshot(
        &self,
        opportunity: &FundingOpportunity,
        raw: &Value,
    ) -> Result<bool> {
        if opportunity.source != SOURCE || opportunity.eligibility_status != "UNKNOWN" {
            return Err(GatewayError::BadRequest(
                "unsupported source or eligibility assertion".into(),
            ));
        }
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(raw)?));
        let metadata = serde_json::to_value(opportunity)?;
        let canonical = normalize_opportunity(
            raw.clone(),
            opportunity.source_opportunity_id,
            opportunity.last_verified_at,
        )?;
        if serde_json::to_value(canonical)? != metadata {
            return Err(GatewayError::BadRequest(
                "normalized opportunity does not match source evidence".into(),
            ));
        }
        let mut tx = self.pool().begin().await?;
        sqlx::query("INSERT INTO funding_sources (source_id, official_url) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(SOURCE).bind(SOURCE_URL).execute(&mut *tx).await?;
        let grant_id: String = sqlx::query_scalar(
            "INSERT INTO funding_swarm_grants (grant_id,external_id,title,sponsor,status,stage,score,amount_usd,metadata)
             VALUES ($1,$2,$3,$4,$5,'discovery',0,0,$6)
             ON CONFLICT (external_id) DO UPDATE SET title=EXCLUDED.title,sponsor=EXCLUDED.sponsor,
               metadata=funding_swarm_grants.metadata || EXCLUDED.metadata,updated_at=NOW(),
               status=CASE WHEN funding_swarm_grants.status IN ('open','expired') THEN EXCLUDED.status ELSE funding_swarm_grants.status END
             RETURNING grant_id")
            .bind(Uuid::new_v4().to_string()).bind(format!("{SOURCE}:{}", opportunity.source_opportunity_id))
            .bind(&opportunity.program_name).bind(&opportunity.funding_agency)
            .bind(if opportunity.expired { "expired" } else { "open" }).bind(&metadata)
            .fetch_one(&mut *tx).await?;
        let inserted = sqlx::query(
            "INSERT INTO funding_source_snapshots (snapshot_id,grant_id,source_id,content_sha256,official_url,retrieved_at,normalized,source_document,normalization_version)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (grant_id,content_sha256,normalization_version) DO NOTHING")
            .bind(Uuid::new_v4().to_string()).bind(&grant_id).bind(SOURCE).bind(&digest)
            .bind(&opportunity.official_url).bind(opportunity.last_verified_at).bind(&metadata).bind(raw).bind(NORMALIZATION_VERSION)
            .execute(&mut *tx).await?.rows_affected() == 1;
        if inserted {
            sqlx::query("INSERT INTO funding_swarm_events (event_id,grant_id,event_type,visibility,detail) VALUES ($1,$2,'official_source_snapshot','public',$3)")
                .bind(Uuid::new_v4().to_string()).bind(&grant_id)
                .bind(json!({"source":SOURCE,"content_sha256":digest,"official_url":opportunity.official_url}))
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(inserted)
    }
}
