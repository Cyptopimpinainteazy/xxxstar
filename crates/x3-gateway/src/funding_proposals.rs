//! Private, immutable proposal packages. Source facts and operator self-reports
//! remain distinct. Saving a package never approves eligibility or submits it.
use crate::{
    db::Database,
    error::{GatewayError, Result},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// Bound private draft size and history responses without silently truncating edits.
const MAX_SECTION_BYTES: usize = 16 * 1024;
const HISTORY_LIMIT: i64 = 25;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalInput {
    pub expected_version: i32,
    pub source_snapshot_id: String,
    pub project_title: String,
    pub research_question: String,
    pub technical_approach: String,
    pub evaluation_plan: String,
    pub organization_facts: String,
    pub requested_usd_cents: Option<i64>,
}
impl ProposalInput {
    pub fn validate(&self) -> Result<()> {
        if self.expected_version < 0
            || self.expected_version == i32::MAX
            || self
                .requested_usd_cents
                .is_some_and(|n| !(0..=9_007_199_254_740_991).contains(&n))
            || self.source_snapshot_id.is_empty()
            || self.source_snapshot_id.len() > 128
            || self.project_title.len() > 200
        {
            return Err(GatewayError::BadRequest(
                "invalid proposal identity, version or monetary units".into(),
            ));
        }
        for section in [
            &self.project_title,
            &self.research_question,
            &self.technical_approach,
            &self.evaluation_plan,
            &self.organization_facts,
        ] {
            if section.trim().is_empty()
                || section.len() > MAX_SECTION_BYTES
                || section
                    .chars()
                    .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            {
                return Err(GatewayError::BadRequest(
                    "proposal sections must be nonempty bounded text".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProposalVersion {
    pub grant_id: String,
    pub version: i32,
    pub source_snapshot_id: String,
    pub package: Value,
    pub package_sha256: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProposalSource {
    pub snapshot_id: String,
    pub content_sha256: String,
    pub normalization_version: i32,
    pub official_url: String,
    pub retrieved_at: DateTime<Utc>,
    pub normalized: Value,
}

#[derive(Serialize)]
pub struct ProposalContext {
    pub source: ProposalSource,
    pub versions: Vec<ProposalVersion>,
    pub history_limit: i64,
}

impl Database {
    pub async fn proposal_context(&self, grant_id: &str) -> Result<ProposalContext> {
        // A consistent snapshot avoids mixing a new source with old history
        // during concurrent discovery. Saves still require optimistic bindings.
        let mut tx = self.pool().begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        let source = sqlx::query_as::<_, ProposalSource>(
            "SELECT s.snapshot_id,s.content_sha256,s.normalization_version,s.official_url,s.retrieved_at,s.normalized
             FROM funding_source_snapshots s JOIN funding_swarm_grants g ON g.grant_id=s.grant_id
             WHERE s.grant_id=$1 AND NOT g.is_example AND s.snapshot_id=g.metadata->>'source_snapshot_id'")
            .bind(grant_id).fetch_optional(&mut *tx).await?
            .ok_or_else(|| GatewayError::NotFound("grant has no current source reference; refresh official source records first".into()))?;
        let versions = sqlx::query_as::<_, ProposalVersion>(
            "SELECT * FROM funding_proposal_versions WHERE grant_id=$1 ORDER BY version DESC LIMIT $2")
            .bind(grant_id).bind(HISTORY_LIMIT).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(ProposalContext {
            source,
            versions,
            history_limit: HISTORY_LIMIT,
        })
    }

    pub async fn save_proposal(
        &self,
        grant_id: &str,
        input: ProposalInput,
    ) -> Result<ProposalVersion> {
        input.validate()?;
        let mut tx = self.pool().begin().await?;
        let state: Option<(String, bool)> = sqlx::query_as(
            "SELECT stage,is_example FROM funding_swarm_grants WHERE grant_id=$1 FOR UPDATE",
        )
        .bind(grant_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((stage, example)) = state else {
            return Err(GatewayError::NotFound("grant not found".into()));
        };
        if example || !matches!(stage.as_str(), "research" | "draft") {
            return Err(GatewayError::BadRequest(
                "proposal editing requires an active research or drafting record".into(),
            ));
        }
        let source = sqlx::query_as::<_, ProposalSource>(
            "SELECT s.snapshot_id,s.content_sha256,s.normalization_version,s.official_url,s.retrieved_at,s.normalized
             FROM funding_source_snapshots s JOIN funding_swarm_grants g ON g.grant_id=s.grant_id
             WHERE s.grant_id=$1 AND s.snapshot_id=g.metadata->>'source_snapshot_id'")
            .bind(grant_id).fetch_optional(&mut *tx).await?
            .ok_or_else(|| GatewayError::BadRequest("official source evidence required".into()))?;
        if source.snapshot_id != input.source_snapshot_id {
            return Err(GatewayError::BadRequest(
                "source changed; reload and review the current evidence before saving".into(),
            ));
        }
        let current: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version),0) FROM funding_proposal_versions WHERE grant_id=$1",
        )
        .bind(grant_id)
        .fetch_one(&mut *tx)
        .await?;
        if current != input.expected_version {
            return Err(GatewayError::BadRequest(
                "proposal version conflict; reload before editing".into(),
            ));
        }
        let version = current
            .checked_add(1)
            .ok_or_else(|| GatewayError::BadRequest("proposal version exhausted".into()))?;
        let package = json!({"schema_version":1,"kind":"research_application_working_draft",
            "grant_id":grant_id,"version":version,"eligibility_status":"UNKNOWN",
            "source_evidence":source,"operator_self_report":input,
            "unverified_requirements":["organizational eligibility","legal identity","funding terms","budget approval","technical capability evidence"],
            "external_submission":false});
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(&package)?));
        let row = sqlx::query_as::<_, ProposalVersion>(
            "INSERT INTO funding_proposal_versions(grant_id,version,source_snapshot_id,package,package_sha256)
             VALUES($1,$2,$3,$4,$5) RETURNING *")
            .bind(grant_id).bind(version).bind(&input.source_snapshot_id).bind(&package).bind(&digest)
            .fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO funding_swarm_events(event_id,grant_id,event_type,visibility,detail)
            VALUES($1,$2,'proposal_version_saved','private',$3)")
            .bind(Uuid::new_v4().to_string()).bind(grant_id)
            .bind(json!({"version":version,"package_sha256":digest,"source_snapshot_id":input.source_snapshot_id,"actor":"funding_operator"}))
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(row)
    }
}
