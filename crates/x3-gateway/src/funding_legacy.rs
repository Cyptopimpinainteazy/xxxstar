//! Quarantined, private research leads from X3's historical Markdown catalog.
//!
//! Import does not establish that a program is open, that X3 is eligible,
//! that a funder approved an application, or that any money changed hands.
//! These records cannot enter the public funding ledger automatically.

use crate::{
    db::Database,
    error::{GatewayError, Result},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use uuid::Uuid;

const MAX_BATCH: usize = 32;
const MAX_FIELD_BYTES: usize = 8 * 1024;
const MAX_TOTAL_FIELD_BYTES: usize = 32 * 1024;
const MAX_URLS: usize = 20;
const MAX_URL_BYTES: usize = 2048;

fn bad_input(message: &str) -> GatewayError {
    GatewayError::BadRequest(message.to_owned())
}

fn valid_legacy_id(value: &str) -> bool {
    let Some(tail) = value.strip_prefix('C') else {
        return false;
    };
    let mut parts = tail.split('.');
    let first = parts.next().unwrap_or("");
    first.len() <= 3 && !first.is_empty() && first.bytes().all(|ch| ch.is_ascii_digit()) && {
        let segments: Vec<_> = parts.collect();
        !segments.is_empty()
            && segments.iter().all(|part| {
                !part.is_empty()
                    && part.len() <= 16
                    && part.bytes().all(|ch| ch.is_ascii_alphanumeric())
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyLead {
    pub legacy_id: String,
    pub name: String,
    pub category: String,
    pub raw_fields: Vec<String>,
    pub candidate_urls: Vec<String>,
    pub historical_status: String,
    pub historical_verification_date: Option<String>,
    pub source_file: String,
    pub source_line: i64,
    pub origin: String,
    pub current_availability: String,
    pub current_eligibility: String,
    pub application_authorized: bool,
}

impl LegacyLead {
    fn validate(&self) -> Result<Option<NaiveDate>> {
        if !(4..=48).contains(&self.legacy_id.len())
            || !valid_legacy_id(&self.legacy_id)
            || self.name.trim().is_empty()
            || self.name.len() > 240
            || self.name.chars().any(char::is_control)
            || self.source_line <= 0
            || self.source_line > i32::MAX.into()
        {
            return Err(bad_input(
                "invalid archived lead identifier, name or source line",
            ));
        }
        // Only the archived repository catalog is accepted. No arbitrary
        // filesystem paths, traversal or unverifiable source imports.
        let Some(filename) = self.source_file.strip_prefix("grants/") else {
            return Err(bad_input("source must be a grants/*.md catalog file"));
        };
        if !filename.ends_with(".md")
            || filename.len() < 4
            || !filename
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
            || filename.strip_suffix(".md") != Some(self.category.as_str())
            || self.category.len() > 128
            || self.origin != "legacy_markdown_research"
        {
            return Err(bad_input("invalid archived catalog provenance"));
        }
        if self.current_availability != "UNKNOWN"
            || self.current_eligibility != "UNKNOWN"
            || self.application_authorized
        {
            return Err(bad_input(
                "archived leads must remain unverified, ineligible-for-automatic-action",
            ));
        }
        if !matches!(
            self.historical_status.as_str(),
            "RESEARCH"
                | "QUALIFIED"
                | "APPLIED"
                | "AWARDED"
                | "REJECTED"
                | "NOT-A-GRANT"
                | "DEFUNCT"
                | "CLOSED"
                | "INACTIVE"
                | "UNKNOWN"
        ) {
            return Err(bad_input("invalid historical status"));
        }
        if self.raw_fields.is_empty()
            || self.raw_fields.len() > 20
            || self
                .raw_fields
                .iter()
                .any(|field| field.len() > MAX_FIELD_BYTES)
            || self.raw_fields.iter().map(String::len).sum::<usize>() > MAX_TOTAL_FIELD_BYTES
        {
            return Err(bad_input("archived lead fields exceed bounds"));
        }
        if self.candidate_urls.len() > MAX_URLS {
            return Err(bad_input("too many candidate source URLs"));
        }
        for url in &self.candidate_urls {
            if url.len() > MAX_URL_BYTES
                || url.chars().any(char::is_control)
                || url.chars().any(char::is_whitespace)
            {
                return Err(bad_input("invalid candidate source URL"));
            }
            let parsed =
                reqwest::Url::parse(url).map_err(|_| bad_input("invalid candidate source URL"))?;
            if !matches!(parsed.scheme(), "https" | "http")
                || parsed.host_str().is_none()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
            {
                return Err(bad_input(
                    "candidate URLs must be anonymous HTTP(S) sources",
                ));
            }
        }
        let date = self
            .historical_verification_date
            .as_deref()
            .map(|value| {
                NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .map_err(|_| bad_input("invalid historical verification date"))
            })
            .transpose()?;
        Ok(date)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyImport {
    pub leads: Vec<LegacyLead>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyListQuery {
    #[serde(default = "default_list_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub search: String,
}

fn default_list_limit() -> i64 {
    20
}

impl LegacyImport {
    fn validate(&self) -> Result<Vec<Option<NaiveDate>>> {
        if self.leads.is_empty() || self.leads.len() > MAX_BATCH {
            return Err(bad_input(
                "archived lead import requires 1 to 32 records per batch",
            ));
        }
        let mut seen = HashSet::new();
        let mut dates = Vec::with_capacity(self.leads.len());
        for lead in &self.leads {
            dates.push(lead.validate()?);
            if !seen.insert((&lead.source_file, &lead.legacy_id)) {
                return Err(bad_input("duplicate legacy lead in request"));
            }
        }
        Ok(dates)
    }
}

#[derive(Debug, Serialize)]
pub struct LegacyImportResult {
    pub inserted: usize,
    pub unchanged: usize,
    pub total: usize,
    pub current_opportunities_verified: usize,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LegacyLeadRow {
    pub source_file: String,
    pub legacy_id: String,
    pub name: String,
    pub category: String,
    pub source_line: i32,
    pub historical_status: String,
    pub historical_verification_date: Option<NaiveDate>,
    pub content_sha256: String,
    pub raw_record: Value,
    pub current_availability: String,
    pub current_eligibility: String,
    pub review_state: String,
    pub created_at: DateTime<Utc>,
}

impl Database {
    /// Idempotent, all-or-nothing private import. A changed legacy record
    /// raises a conflict instead of overwriting source evidence silently.
    pub async fn import_legacy_leads(&self, input: LegacyImport) -> Result<LegacyImportResult> {
        let dates = input.validate()?;
        let mut tx = self.pool().begin().await?;
        let mut inserted = 0usize;
        for (lead, verification_date) in input.leads.iter().zip(dates) {
            let raw = serde_json::to_value(lead)?;
            let digest = hex::encode(Sha256::digest(serde_json::to_vec(&raw)?));
            let result = sqlx::query(
                "INSERT INTO funding_legacy_leads
                 (source_file, legacy_id, name, category, source_line,
                  historical_status, historical_verification_date, content_sha256, raw_record)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                 ON CONFLICT (source_file,legacy_id) DO NOTHING",
            )
            .bind(&lead.source_file)
            .bind(&lead.legacy_id)
            .bind(&lead.name)
            .bind(&lead.category)
            .bind(lead.source_line as i32)
            .bind(&lead.historical_status)
            .bind(verification_date)
            .bind(&digest)
            .bind(&raw)
            .execute(&mut *tx)
            .await?;
            if result.rows_affected() == 1 {
                inserted += 1;
            } else {
                let existing: String = sqlx::query_scalar(
                    "SELECT content_sha256 FROM funding_legacy_leads WHERE source_file=$1 AND legacy_id=$2",
                )
                .bind(&lead.source_file)
                .bind(&lead.legacy_id)
                .fetch_one(&mut *tx)
                .await?;
                if existing != digest {
                    return Err(bad_input(
                        "conflicting historical research record; manual review and versioning required",
                    ));
                }
            }
        }
        if inserted > 0 {
            sqlx::query(
                "INSERT INTO funding_swarm_events(event_id,grant_id,event_type,visibility,detail)
                 VALUES($1,NULL,'historical_leads_staged','private',$2)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(json!({"inserted":inserted,"total":input.leads.len(),
                "verified_live_opportunities":0,"actor":"funding_operator"}))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(LegacyImportResult {
            inserted,
            unchanged: input.leads.len() - inserted,
            total: input.leads.len(),
            current_opportunities_verified: 0,
        })
    }

    /// Private research queue only; never used in public funding totals.
    pub async fn list_legacy_leads(
        &self,
        limit: i64,
        offset: i64,
        search: &str,
    ) -> Result<Vec<LegacyLeadRow>> {
        if search.len() > 120 || search.chars().any(char::is_control) {
            return Err(bad_input(
                "historical lead search must be bounded plain text",
            ));
        }
        sqlx::query_as::<_, LegacyLeadRow>(
            "SELECT * FROM funding_legacy_leads
             WHERE $3 = '' OR POSITION(LOWER($3) IN LOWER(name || ' ' || category || ' ' || legacy_id)) > 0
             ORDER BY category,legacy_id LIMIT $1 OFFSET $2",
        )
        .bind(limit.clamp(1, 100))
        .bind(offset.max(0))
        .bind(search.trim())
        .fetch_all(self.pool())
        .await
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lead() -> LegacyLead {
        LegacyLead {
            legacy_id: "C27.1.1".into(),
            name: "Example blockchain research".into(),
            category: "C27-chain-grants".into(),
            source_file: "grants/C27-chain-grants.md".into(),
            source_line: 42,
            raw_fields: vec!["Research".into(), "https://example.org/grants".into()],
            candidate_urls: vec!["https://example.org/grants".into()],
            historical_status: "RESEARCH".into(),
            historical_verification_date: Some("2026-09-05".into()),
            origin: "legacy_markdown_research".into(),
            current_availability: "UNKNOWN".into(),
            current_eligibility: "UNKNOWN".into(),
            application_authorized: false,
        }
    }

    #[test]
    fn accepts_archival_source_without_conferring_live_status() {
        let item = lead();
        assert_eq!(item.validate().unwrap().unwrap().to_string(), "2026-09-05");
        assert!(!item.application_authorized);
    }
    #[test]
    fn rejects_forged_status_and_automatic_action() {
        let mut item = lead();
        item.current_availability = "OPEN".into();
        assert!(item.validate().is_err());
        item.current_availability = "UNKNOWN".into();
        item.application_authorized = true;
        assert!(item.validate().is_err());
    }
    #[test]
    fn rejects_paths_and_bad_urls() {
        let mut item = lead();
        item.source_file = "grants/../secret.md".into();
        assert!(item.validate().is_err());
        item.source_file = "grants/C27-chain-grants.md".into();
        item.candidate_urls = vec!["file:///etc/passwd".into()];
        assert!(item.validate().is_err());
        item.candidate_urls = vec!["https://user:pass@example.org".into()];
        assert!(item.validate().is_err());
    }
    #[test]
    fn supports_alphanumeric_ids_and_rejects_duplicate_records() {
        let mut item = lead();
        item.legacy_id = "C25.A1".into();
        item.source_file = "grants/C25-vc-accelerators.md".into();
        item.category = "C25-vc-accelerators".into();
        assert!(item.validate().is_ok());
        assert!(LegacyImport {
            leads: vec![item.clone(), item]
        }
        .validate()
        .is_err());
    }
    #[test]
    fn rejects_large_batches_and_malformed_dates() {
        let mut item = lead();
        item.historical_verification_date = Some("2026-02-30".into());
        assert!(item.validate().is_err());
        let original = lead();
        assert!(LegacyImport { leads: vec![] }.validate().is_err());
        let oversize = (0..33)
            .map(|i| {
                let mut r = original.clone();
                r.legacy_id = format!("C27.1.{i}");
                r
            })
            .collect();
        assert!(LegacyImport { leads: oversize }.validate().is_err());
    }
}
