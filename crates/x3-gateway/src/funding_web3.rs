//! Operator-controlled official Web3 funding-source observations.
//! A fetched program page is evidence of what the page said, not evidence
//! that a grant is presently open, X3 qualifies, or funding has been awarded.
//! Documents are inert private inputs; this module has no submission/outreach.

use crate::{
    db::Database,
    error::{GatewayError, Result},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

const MAX_DOCUMENT: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);
static REFRESH_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Web3DiscoveryRequest {
    pub provider: String,
}

#[derive(Debug, Clone, Copy)]
struct OfficialSource {
    id: &'static str,
    url: &'static str,
    title: &'static str,
    required_phrases: &'static [&'static str],
}

const SOLANA: OfficialSource = OfficialSource {
    id: "solana.foundation.web",
    url: "https://solana.org/grants-funding",
    title: "Solana Foundation Grants and Funding",
    required_phrases: &["solana foundation", "funding", "grant"],
};
const STELLAR: OfficialSource = OfficialSource {
    id: "stellar.scf.web",
    url: "https://communityfund.stellar.org/awards",
    title: "Stellar Community Fund Build Awards",
    required_phrases: &["stellar community fund", "build", "award"],
};

fn source(provider: &str) -> Result<OfficialSource> {
    match provider {
        "solana" => Ok(SOLANA),
        "stellar" => Ok(STELLAR),
        _ => Err(GatewayError::BadRequest(
            "unsupported Web3 source; choose solana or stellar".into(),
        )),
    }
}

fn source_error(message: &str) -> GatewayError {
    GatewayError::Internal(format!("official Web3 funding source: {message}"))
}

/// Keep script/style content out of evidence text and never execute HTML.
/// This is *not* a general web browser: it extracts plain text from fixed,
/// manually approved official public funding-program pages.
fn visible_text(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut text = String::new();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find('<') {
        let start = cursor + relative;
        text.push_str(&html[cursor..start]);
        let Some(end_relative) = lower[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let tag = lower[start..end].trim_start_matches('<').trim();
        if tag.starts_with("script") || tag.starts_with("style") {
            let closing = if tag.starts_with("script") {
                "</script"
            } else {
                "</style"
            };
            if let Some(close_relative) = lower[end..].find(closing) {
                let closing_start = end + close_relative;
                if let Some(close_end) = lower[closing_start..].find('>') {
                    cursor = closing_start + close_end + 1;
                    text.push(' ');
                    continue;
                }
            }
            break;
        }
        cursor = end;
        text.push(' ');
    }
    if cursor < html.len() {
        text.push_str(&html[cursor..]);
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract only an explicitly labelled SCF submission round and date.
/// A different page layout leaves round/deadline UNKNOWN, never invented.
fn stellar_round(text: &str) -> (Option<String>, Option<NaiveDate>) {
    let Some(begin) = text.find("SCF #") else {
        return (None, None);
    };
    let suffix = &text[begin + 5..];
    let digits: String = suffix.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() > 4 {
        return (None, None);
    }
    let round = format!("SCF #{digits}");
    let section = &suffix[digits.len()..];
    let bounded = &section[..section
        .char_indices()
        .nth(500)
        .map(|(index, _)| index)
        .unwrap_or(section.len())];
    if !bounded.to_ascii_lowercase().contains("submission") {
        return (Some(round), None);
    }
    let Some(deadline) = bounded.find("Deadline to submit:") else {
        return (Some(round), None);
    };
    let following = &bounded[deadline + "Deadline to submit:".len()..];
    let date = following
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    let parsed = NaiveDate::parse_from_str(&date, "%B %e, %Y").ok();
    (Some(round), parsed)
}

fn normalize_page(src: OfficialSource, html: &str, checked: DateTime<Utc>) -> Result<Value> {
    if html.len() > MAX_DOCUMENT {
        return Err(source_error("page exceeded maximum size"));
    }
    let plain = visible_text(html);
    let lower = plain.to_ascii_lowercase();
    if !src
        .required_phrases
        .iter()
        .all(|phrase| lower.contains(phrase))
    {
        return Err(source_error("official page did not expose required program identity"));
    }
    let (round, deadline) = if src.id == STELLAR.id {
        stellar_round(&plain)
    } else {
        (None, None)
    };
    let status = if src.id == STELLAR.id {
        match deadline {
            Some(day) if day < checked.date_naive() => "HISTORICAL_DEADLINE",
            Some(_) if round.is_some() => "SUBMISSION_ROUND_DESCRIBED",
            _ => "UNKNOWN",
        }
    } else if lower.contains("rolling basis") && lower.contains("application") {
        "ROLLING_APPLICATION_GUIDANCE"
    } else {
        "UNKNOWN"
    };
    Ok(json!({
        "source_id":src.id,
        "program_name":src.title,
        "official_url":src.url,
        "funding_kind":"ecosystem_funding",
        "source_verification":"OFFICIAL_PAGE_OBSERVED",
        "application_guidance_status":status,
        "round":round,
        "deadline":deadline,
        "eligibility_status":"UNKNOWN",
        "award_status":"UNKNOWN",
        "approved_for_submission":false,
        "verified_at":checked,
        "plain_text_excerpt": plain.chars().take(3000).collect::<String>()
    }))
}

async fn retrieve_public_page(src: OfficialSource) -> Result<String> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
        .user_agent("X3FundingSourceReview/0.1 (+https://github.com/Cyptopimpinainteazy/xxxstar)")
        .build()
        .map_err(|_| source_error("client initialization failed"))?;
    let mut response = client
        .get(src.url)
        .send()
        .await
        .map_err(|_| source_error("official source request failed"))?;
    if !response.status().is_success() {
        return Err(source_error("official source returned unsuccessful HTTP status"));
    }
    if !response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/html"))
    {
        return Err(source_error("unexpected official source content type"));
    }
    if response.content_length().is_some_and(|size| size > MAX_DOCUMENT as u64) {
        return Err(source_error("official page exceeded size budget"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| source_error("official source response interrupted"))?
    {
        if chunk.len() > MAX_DOCUMENT.saturating_sub(bytes.len()) {
            return Err(source_error("official page exceeded size budget"));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| source_error("official page was not UTF-8"))
}

#[derive(Debug, Serialize)]
pub struct Web3DiscoveryResult {
    pub source_id: String,
    pub official_url: String,
    pub snapshot_id: String,
    pub new_snapshot: bool,
    pub application_guidance_status: String,
    pub eligibility_status: &'static str,
    pub awarded_funding_verified: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Web3SourceRow {
    pub snapshot_id: String,
    pub source_id: String,
    pub official_url: String,
    pub content_sha256: String,
    pub normalized: Value,
    pub retrieved_at: DateTime<Utc>,
    pub last_observed_at: DateTime<Utc>,
}

pub async fn discover(db: &Database, request: Web3DiscoveryRequest) -> Result<Web3DiscoveryResult> {
    let src = source(&request.provider)?;
    let _guard = REFRESH_LOCK
        .try_lock()
        .map_err(|_| GatewayError::BadRequest("official Web3 refresh is already running".into()))?;
    // Only two manually selected official public pages. Operators must review
    // site policies before enabling this separate, disabled-by-default feature.
    let html = retrieve_public_page(src).await?;
    let retrieved = Utc::now();
    let normalized = normalize_page(src, &html, retrieved)?;
    let digest = hex::encode(Sha256::digest(html.as_bytes()));
    let mut tx = db.pool().begin().await?;
    sqlx::query(
        "INSERT INTO funding_sources(source_id,official_url) VALUES($1,$2) ON CONFLICT DO NOTHING",
    )
    .bind(src.id)
    .bind(src.url)
    .execute(&mut *tx)
    .await?;
    let id = Uuid::new_v4().to_string();
    let snapshot: (String, bool) = sqlx::query_as(
        "WITH saved AS (
            INSERT INTO funding_web3_source_snapshots
            (snapshot_id,source_id,official_url,content_sha256,source_document,
             normalized,retrieved_at,last_observed_at)
            VALUES($1,$2,$3,$4,$5,$6,$7,$7)
            ON CONFLICT(source_id,content_sha256) DO NOTHING
            RETURNING snapshot_id
         )
         SELECT snapshot_id,TRUE FROM saved
         UNION ALL
         SELECT snapshot_id,FALSE FROM funding_web3_source_snapshots
          WHERE source_id=$2 AND content_sha256=$4
          AND NOT EXISTS (SELECT 1 FROM saved)
         LIMIT 1",
    )
    .bind(id)
    .bind(src.id)
    .bind(src.url)
    .bind(&digest)
    .bind(&html)
    .bind(&normalized)
    .bind(retrieved)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE funding_web3_source_snapshots SET last_observed_at=$3
         WHERE source_id=$1 AND content_sha256=$2",
    )
    .bind(src.id)
    .bind(&digest)
    .bind(retrieved)
    .execute(&mut *tx)
    .await?;
    if snapshot.1 {
        sqlx::query(
            "INSERT INTO funding_swarm_events(event_id,grant_id,event_type,visibility,detail)
             VALUES($1,NULL,'web3_official_page_observed','private',$2)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(json!({"source":src.id,"snapshot_id":snapshot.0,
            "content_sha256":digest,"funding_awarded":false}))
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "UPDATE funding_sources SET last_success_at=NOW(),last_error_code=NULL WHERE source_id=$1",
    )
    .bind(src.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Web3DiscoveryResult {
        source_id: src.id.into(),
        official_url: src.url.into(),
        snapshot_id: snapshot.0,
        new_snapshot: snapshot.1,
        application_guidance_status: normalized["application_guidance_status"]
            .as_str()
            .unwrap_or("UNKNOWN")
            .into(),
        eligibility_status: "UNKNOWN",
        awarded_funding_verified: false,
    })
}

impl Database {
    pub async fn list_web3_sources(&self) -> Result<Vec<Web3SourceRow>> {
        sqlx::query_as::<_, Web3SourceRow>(
            "SELECT DISTINCT ON (source_id) snapshot_id,source_id,official_url,
                    content_sha256,normalized,retrieved_at,last_observed_at
             FROM funding_web3_source_snapshots
             ORDER BY source_id,last_observed_at DESC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unapproved_sources() {
        assert!(source("solana").is_ok());
        assert!(source("stellar").is_ok());
        assert!(source("http://127.0.0.1").is_err());
        assert!(source("rootstock").is_err());
    }

    #[test]
    fn recognizes_solana_official_grants_without_claiming_eligibility() {
        let html = "<html><body><h1>Grants and Funding</h1><p>Solana Foundation funding \
            includes milestone-based grants. All applications reviewed on a rolling basis.</p>\
            <script>AWARDED MILLIONS TO X3</script></body></html>";
        let n = normalize_page(SOLANA, html, Utc::now()).unwrap();
        assert_eq!(n["application_guidance_status"], "ROLLING_APPLICATION_GUIDANCE");
        assert_eq!(n["eligibility_status"], "UNKNOWN");
        assert_eq!(n["approved_for_submission"], false);
        assert!(!n["plain_text_excerpt"].as_str().unwrap().contains("AWARDED MILLIONS"));
    }

    #[test]
    fn extracts_stellar_round_without_fabricated_award() {
        let html = "<h1>Stellar Community Fund Build Awards</h1>\
            <p>SCF #46 Submission Deadline to submit: November 8, 2026 View</p>";
        let now = DateTime::parse_from_rfc3339("2026-10-18T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let n = normalize_page(STELLAR, html, now).unwrap();
        assert_eq!(n["round"], "SCF #46");
        assert_eq!(n["deadline"], "2026-11-08");
        assert_eq!(n["application_guidance_status"], "SUBMISSION_ROUND_DESCRIBED");
        assert_eq!(n["award_status"], "UNKNOWN");
    }

    #[test]
    fn treats_expired_and_missing_stellar_dates_conservatively() {
        let html = "<h1>Stellar Community Fund Build Awards</h1>\
            <p>SCF #45 Submission Deadline to submit: August 16, 2026</p>";
        let now = DateTime::parse_from_rfc3339("2026-10-18T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            normalize_page(STELLAR, html, now).unwrap()["application_guidance_status"],
            "HISTORICAL_DEADLINE"
        );
        let html = "<h1>Stellar Community Fund Build Awards</h1><p>SCF #47 details coming soon</p>";
        assert_eq!(normalize_page(STELLAR, html, now).unwrap()["deadline"], Value::Null);
    }

    #[test]
    fn rejects_unrelated_and_oversized_pages() {
        assert!(normalize_page(SOLANA, "<p>Marketing landing page</p>", Utc::now()).is_err());
        assert!(normalize_page(SOLANA, &"x".repeat(MAX_DOCUMENT + 1), Utc::now()).is_err());
    }
}
