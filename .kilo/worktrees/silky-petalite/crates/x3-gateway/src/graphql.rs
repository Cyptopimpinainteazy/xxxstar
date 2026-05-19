//! GraphQL schema and resolvers.

use crate::db::{
    Account, ApprovalCase, Block, ChainStats, ComitTransaction, Database, Event, EvidenceBundle,
    Extrinsic, NewApprovalCase, NewEvidenceBundle, NewOrchestraIntent, NewVoteReceipt,
    NewVoteWindow, OrchestraIntent, VoteReceipt, VoteWindow,
};
use crate::orchestra;
use async_graphql::{Context, EmptySubscription, InputObject, Json, Object, Schema, SimpleObject};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use x3_orchestra_control_plane::{ControlPlaneClient, DispatchEvidenceRequest};
use x3_rpc::benchmark::{BenchmarkLogClassStat, BenchmarkReport, BenchmarkWorkloadProfile};

/// GraphQL query root.
pub struct QueryRoot;

/// GraphQL mutation root.
pub struct MutationRoot;

#[Object]
impl QueryRoot {
    // ========================================================================
    // Block queries
    // ========================================================================

    /// Get block by number.
    async fn block(&self, ctx: &Context<'_>, number: i64) -> async_graphql::Result<Option<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block(number).await?)
    }

    /// Get block by hash.
    async fn block_by_hash(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> async_graphql::Result<Option<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block_by_hash(&hash).await?)
    }

    /// Get latest block.
    async fn latest_block(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_latest_block().await?)
    }

    /// Get recent blocks.
    async fn blocks(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_recent_blocks(limit.min(100), offset).await?)
    }

    /// Get blocks in a range.
    async fn blocks_range(
        &self,
        ctx: &Context<'_>,
        from: i64,
        to: i64,
    ) -> async_graphql::Result<Vec<Block>> {
        let db = ctx.data::<Database>()?;
        // Limit range to 100 blocks
        let limited_to = (from + 100).min(to);
        Ok(db.get_blocks_range(from, limited_to).await?)
    }

    // ========================================================================
    // Extrinsic queries
    // ========================================================================

    /// Get extrinsic by hash.
    async fn extrinsic(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> async_graphql::Result<Option<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_extrinsic(&hash).await?)
    }

    /// Get extrinsics for a block.
    async fn block_extrinsics(
        &self,
        ctx: &Context<'_>,
        block_number: i64,
    ) -> async_graphql::Result<Vec<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block_extrinsics(block_number).await?)
    }

    /// Get recent extrinsics.
    async fn extrinsics(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_recent_extrinsics(limit.min(100), offset).await?)
    }

    /// Get extrinsics by account.
    async fn account_extrinsics(
        &self,
        ctx: &Context<'_>,
        address: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_account_extrinsics(&address, limit.min(100), offset)
            .await?)
    }

    // ========================================================================
    // Event queries
    // ========================================================================

    /// Get events for a block.
    async fn block_events(
        &self,
        ctx: &Context<'_>,
        block_number: i64,
    ) -> async_graphql::Result<Vec<Event>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block_events(block_number).await?)
    }

    /// Get events by pallet.
    async fn events_by_pallet(
        &self,
        ctx: &Context<'_>,
        pallet: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Event>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_events_by_pallet(&pallet, limit.min(100), offset)
            .await?)
    }

    /// Get events by pallet and variant.
    async fn events_by_type(
        &self,
        ctx: &Context<'_>,
        pallet: String,
        variant: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Event>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_events_by_type(&pallet, &variant, limit.min(100), offset)
            .await?)
    }

    // ========================================================================
    // Comit queries
    // ========================================================================

    /// Get Comit by hash.
    async fn comit(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> async_graphql::Result<Option<ComitTransaction>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_comit(&hash).await?)
    }

    /// Get recent Comits.
    async fn comits(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<ComitTransaction>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_recent_comits(limit.min(100), offset).await?)
    }

    /// Get Comits by origin account.
    async fn account_comits(
        &self,
        ctx: &Context<'_>,
        address: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<ComitTransaction>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_account_comits(&address, limit.min(100), offset)
            .await?)
    }

    // ========================================================================
    // Account queries
    // ========================================================================

    /// Get account by address.
    async fn account(
        &self,
        ctx: &Context<'_>,
        address: String,
    ) -> async_graphql::Result<Option<Account>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_account(&address).await?)
    }

    /// Get top accounts by balance.
    async fn top_accounts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
    ) -> async_graphql::Result<Vec<Account>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_top_accounts(limit.min(100)).await?)
    }

    /// Search accounts.
    async fn search_accounts(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 10)] limit: i64,
    ) -> async_graphql::Result<Vec<Account>> {
        let db = ctx.data::<Database>()?;
        Ok(db.search_accounts(&query, limit.min(50)).await?)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get chain statistics.
    async fn stats(&self, ctx: &Context<'_>) -> async_graphql::Result<ChainStats> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_stats().await?)
    }

    /// Get benchmark reports with optional contention filters and sorting.
    async fn benchmark_reports(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<String>,
        min_high_conflict_ratio: Option<f64>,
        min_serial_fraction: Option<f64>,
        log_class: Option<String>,
        sort_by: Option<String>,
        sort_order: Option<String>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<BenchmarkReportView>> {
        let db = ctx.data::<Database>()?;
        let mut reports = db
            .get_benchmark_reports(tenant_id.as_deref(), limit.min(100), offset)
            .await?;

        reports.retain(|report| {
            benchmark_report_matches(
                report,
                min_high_conflict_ratio,
                min_serial_fraction,
                log_class.as_deref(),
            )
        });

        sort_benchmark_reports(&mut reports, sort_by.as_deref(), sort_order.as_deref());

        Ok(reports
            .into_iter()
            .map(BenchmarkReportView::from)
            .take(limit.min(100) as usize)
            .collect())
    }

    /// Get a single benchmark report.
    async fn benchmark_report(
        &self,
        ctx: &Context<'_>,
        report_id: String,
    ) -> async_graphql::Result<Option<BenchmarkReportView>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_benchmark_report(&report_id)
            .await?
            .map(BenchmarkReportView::from))
    }

    /// Get workflow intents.
    async fn orchestra_intents(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<OrchestraIntentView>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .list_orchestra_intents(limit.clamp(1, 100), offset.max(0))
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Get workflow intent by id.
    async fn orchestra_intent(
        &self,
        ctx: &Context<'_>,
        intent_id: String,
    ) -> async_graphql::Result<Option<OrchestraIntentView>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_orchestra_intent(&intent_id).await?.map(Into::into))
    }

    /// Get approval cases.
    async fn approval_cases(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<ApprovalCaseView>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .list_approval_cases(limit.clamp(1, 100), offset.max(0))
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Get approval case by id.
    async fn approval_case(
        &self,
        ctx: &Context<'_>,
        case_id: String,
    ) -> async_graphql::Result<Option<ApprovalCaseView>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_approval_case(&case_id).await?.map(Into::into))
    }

    /// Get vote windows.
    async fn vote_windows(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<VoteWindowView>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .list_vote_windows(limit.clamp(1, 100), offset.max(0))
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Get vote window by id.
    async fn vote_window(
        &self,
        ctx: &Context<'_>,
        window_id: String,
    ) -> async_graphql::Result<Option<VoteWindowView>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_vote_window(&window_id).await?.map(Into::into))
    }

    /// Get evidence bundles.
    async fn evidence_bundles(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<EvidenceBundleView>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .list_evidence_bundles(limit.clamp(1, 100), offset.max(0))
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Get evidence bundle by id.
    async fn evidence_bundle(
        &self,
        ctx: &Context<'_>,
        bundle_id: String,
    ) -> async_graphql::Result<Option<EvidenceBundleView>> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        Ok(
            orchestra::get_evidence_bundle(db, orchestra_client.as_ref(), &bundle_id)
                .await?
                .map(Into::into),
        )
    }
}

#[Object]
impl MutationRoot {
    /// Create a workflow intent.
    async fn create_orchestra_intent(
        &self,
        ctx: &Context<'_>,
        input: CreateOrchestraIntentInput,
    ) -> async_graphql::Result<OrchestraIntentView> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        Ok(
            orchestra::create_orchestra_intent(db, orchestra_client.as_ref(), input.into())
                .await?
                .into(),
        )
    }

    /// Create an approval case.
    async fn create_approval_case(
        &self,
        ctx: &Context<'_>,
        input: CreateApprovalCaseInput,
    ) -> async_graphql::Result<ApprovalCaseView> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        Ok(
            orchestra::create_approval_case(db, orchestra_client.as_ref(), input.into())
                .await?
                .into(),
        )
    }

    /// Create a vote window.
    async fn create_vote_window(
        &self,
        ctx: &Context<'_>,
        input: CreateVoteWindowInput,
    ) -> async_graphql::Result<VoteWindowView> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        Ok(
            orchestra::create_vote_window(db, orchestra_client.as_ref(), input.into())
                .await?
                .into(),
        )
    }

    /// Record a vote receipt.
    async fn create_vote_receipt(
        &self,
        ctx: &Context<'_>,
        window_id: String,
        input: CreateVoteReceiptInput,
    ) -> async_graphql::Result<VoteReceiptView> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        Ok(
            orchestra::create_vote_receipt(db, orchestra_client.as_ref(), &window_id, input.into())
                .await?
                .into(),
        )
    }

    /// Create an evidence bundle.
    async fn create_evidence_bundle(
        &self,
        ctx: &Context<'_>,
        input: CreateEvidenceBundleInput,
    ) -> async_graphql::Result<EvidenceBundleView> {
        let db = ctx.data::<Database>()?;
        Ok(db.create_evidence_bundle(input.into()).await?.into())
    }

    /// Dispatch a workflow intent through the control plane.
    async fn dispatch_orchestra_intent(
        &self,
        ctx: &Context<'_>,
        intent_id: String,
        input: DispatchIntentInput,
    ) -> async_graphql::Result<IntentDispatchView> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        let (intent, evidence) = orchestra::dispatch_orchestra_intent(
            db,
            orchestra_client.as_ref(),
            &intent_id,
            DispatchEvidenceRequest {
                artifact_uri: input.artifact_uri,
                digest: input.digest,
                detail: input.detail.0,
            },
        )
        .await?;
        Ok(IntentDispatchView {
            intent: intent.into(),
            evidence: evidence.into(),
        })
    }

    /// Close a vote window through the control plane.
    async fn close_vote_window(
        &self,
        ctx: &Context<'_>,
        window_id: String,
    ) -> async_graphql::Result<VoteWindowClosureView> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        let (vote_window, approval_case, evidence) =
            orchestra::close_vote_window(db, orchestra_client.as_ref(), &window_id).await?;
        Ok(VoteWindowClosureView {
            vote_window: vote_window.into(),
            approval_case: approval_case.into(),
            evidence: evidence.into(),
        })
    }

    /// Import a closed-window tally from the control plane CRM bridge.
    async fn import_vote_window_tally(
        &self,
        ctx: &Context<'_>,
        window_id: String,
    ) -> async_graphql::Result<VoteTallyView> {
        let db = ctx.data::<Database>()?;
        let orchestra_client = ctx
            .data_opt::<Option<Arc<ControlPlaneClient>>>()
            .and_then(|client| client.clone());
        let tally =
            orchestra::import_vote_window_tally(db, orchestra_client.as_ref(), &window_id).await?;
        Ok(tally.into())
    }
}

/// GraphQL schema type.
pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Create the GraphQL schema.
pub fn create_schema(db: Database, orchestra_client: Option<Arc<ControlPlaneClient>>) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(db)
        .data(orchestra_client)
        .finish()
}

#[derive(Clone, Debug, SimpleObject)]
struct BenchmarkLogClassStatView {
    class_name: String,
    count: u64,
    share_of_logs: f64,
    unique_contracts: u64,
    unique_transactions: u64,
}

#[derive(Clone, Debug, SimpleObject)]
struct BenchmarkWorkloadProfileView {
    total_transactions: u64,
    total_receipts: u64,
    total_logs: u64,
    active_lanes: u64,
    active_log_lanes: u64,
    low_conflict_ratio: f64,
    medium_conflict_ratio: f64,
    high_conflict_ratio: f64,
    estimated_serial_fraction: f64,
    log_classes: Vec<BenchmarkLogClassStatView>,
}

#[derive(Clone, Debug, SimpleObject)]
struct BenchmarkReportView {
    report_id: String,
    chain_name: String,
    signer: String,
    generated_at_unix: u64,
    workload_profile: BenchmarkWorkloadProfileView,
}

#[derive(Clone, Debug, SimpleObject)]
struct OrchestraIntentView {
    intent_id: String,
    tenant_id: String,
    kind: String,
    status: String,
    risk_class: String,
    submitter: String,
    requires_approval: bool,
    payload: Json<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, SimpleObject)]
struct ApprovalCaseView {
    case_id: String,
    intent_id: String,
    status: String,
    review_kind: String,
    requested_by: String,
    summary: String,
    metadata: Json<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, SimpleObject)]
struct VoteWindowView {
    window_id: String,
    approval_case_id: String,
    title: String,
    status: String,
    opens_at_unix: i64,
    closes_at_unix: i64,
    electorate: Json<serde_json::Value>,
    tally: Json<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, SimpleObject)]
struct VoteReceiptView {
    receipt_id: String,
    window_id: String,
    voter_id: String,
    vote_choice: String,
    rationale: Option<String>,
    cast_at_unix: i64,
    created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, SimpleObject)]
struct VoteTallyView {
    approvals: u64,
    rejections: u64,
    abstentions: u64,
}

#[derive(Clone, Debug, SimpleObject)]
struct EvidenceBundleView {
    bundle_id: String,
    intent_id: Option<String>,
    approval_case_id: Option<String>,
    vote_window_id: Option<String>,
    artifact_uri: String,
    digest: String,
    summary: Json<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, SimpleObject)]
struct IntentDispatchView {
    intent: OrchestraIntentView,
    evidence: EvidenceBundleView,
}

#[derive(Clone, Debug, SimpleObject)]
struct VoteWindowClosureView {
    vote_window: VoteWindowView,
    approval_case: ApprovalCaseView,
    evidence: EvidenceBundleView,
}

#[derive(Debug, InputObject)]
struct CreateOrchestraIntentInput {
    tenant_id: String,
    kind: String,
    status: String,
    risk_class: String,
    submitter: String,
    requires_approval: bool,
    payload: Json<serde_json::Value>,
}

#[derive(Debug, InputObject)]
struct CreateApprovalCaseInput {
    intent_id: String,
    status: String,
    review_kind: String,
    requested_by: String,
    summary: String,
    metadata: Json<serde_json::Value>,
}

#[derive(Debug, InputObject)]
struct CreateVoteWindowInput {
    approval_case_id: String,
    title: String,
    status: String,
    opens_at_unix: i64,
    closes_at_unix: i64,
    electorate: Json<serde_json::Value>,
}

#[derive(Debug, InputObject)]
struct CreateVoteReceiptInput {
    voter_id: String,
    vote_choice: String,
    rationale: Option<String>,
    cast_at_unix: i64,
}

#[derive(Debug, InputObject)]
struct CreateEvidenceBundleInput {
    intent_id: Option<String>,
    approval_case_id: Option<String>,
    vote_window_id: Option<String>,
    artifact_uri: String,
    digest: String,
    summary: Json<serde_json::Value>,
}

#[derive(Debug, InputObject)]
struct DispatchIntentInput {
    artifact_uri: String,
    digest: String,
    detail: Json<serde_json::Value>,
}

impl From<BenchmarkLogClassStat> for BenchmarkLogClassStatView {
    fn from(value: BenchmarkLogClassStat) -> Self {
        Self {
            class_name: value.class_name,
            count: value.count,
            share_of_logs: value.share_of_logs,
            unique_contracts: value.unique_contracts,
            unique_transactions: value.unique_transactions,
        }
    }
}

impl From<BenchmarkWorkloadProfile> for BenchmarkWorkloadProfileView {
    fn from(value: BenchmarkWorkloadProfile) -> Self {
        Self {
            total_transactions: value.total_transactions,
            total_receipts: value.total_receipts,
            total_logs: value.total_logs,
            active_lanes: value.active_lanes,
            active_log_lanes: value.active_log_lanes,
            low_conflict_ratio: value.low_conflict_ratio,
            medium_conflict_ratio: value.medium_conflict_ratio,
            high_conflict_ratio: value.high_conflict_ratio,
            estimated_serial_fraction: value.estimated_serial_fraction,
            log_classes: value.log_classes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<BenchmarkReport> for BenchmarkReportView {
    fn from(value: BenchmarkReport) -> Self {
        Self {
            report_id: value.report_id,
            chain_name: value.chain_name,
            signer: value.signer,
            generated_at_unix: value.generated_at_unix,
            workload_profile: value.workload_profile.into(),
        }
    }
}

impl From<OrchestraIntent> for OrchestraIntentView {
    fn from(value: OrchestraIntent) -> Self {
        Self {
            intent_id: value.intent_id,
            tenant_id: value.tenant_id,
            kind: value.kind,
            status: value.status,
            risk_class: value.risk_class,
            submitter: value.submitter,
            requires_approval: value.requires_approval,
            payload: Json(value.payload),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<ApprovalCase> for ApprovalCaseView {
    fn from(value: ApprovalCase) -> Self {
        Self {
            case_id: value.case_id,
            intent_id: value.intent_id,
            status: value.status,
            review_kind: value.review_kind,
            requested_by: value.requested_by,
            summary: value.summary,
            metadata: Json(value.metadata),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<VoteWindow> for VoteWindowView {
    fn from(value: VoteWindow) -> Self {
        Self {
            window_id: value.window_id,
            approval_case_id: value.approval_case_id,
            title: value.title,
            status: value.status,
            opens_at_unix: value.opens_at_unix,
            closes_at_unix: value.closes_at_unix,
            electorate: Json(value.electorate),
            tally: Json(value.tally),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<VoteReceipt> for VoteReceiptView {
    fn from(value: VoteReceipt) -> Self {
        Self {
            receipt_id: value.receipt_id,
            window_id: value.window_id,
            voter_id: value.voter_id,
            vote_choice: value.vote_choice,
            rationale: value.rationale,
            cast_at_unix: value.cast_at_unix,
            created_at: value.created_at,
        }
    }
}

impl From<x3_orchestra_control_plane::VoteTally> for VoteTallyView {
    fn from(value: x3_orchestra_control_plane::VoteTally) -> Self {
        Self {
            approvals: value.approvals,
            rejections: value.rejections,
            abstentions: value.abstentions,
        }
    }
}

impl From<EvidenceBundle> for EvidenceBundleView {
    fn from(value: EvidenceBundle) -> Self {
        Self {
            bundle_id: value.bundle_id,
            intent_id: value.intent_id,
            approval_case_id: value.approval_case_id,
            vote_window_id: value.vote_window_id,
            artifact_uri: value.artifact_uri,
            digest: value.digest,
            summary: Json(value.summary),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<CreateOrchestraIntentInput> for NewOrchestraIntent {
    fn from(value: CreateOrchestraIntentInput) -> Self {
        Self {
            tenant_id: value.tenant_id,
            kind: value.kind,
            status: value.status,
            risk_class: value.risk_class,
            submitter: value.submitter,
            requires_approval: value.requires_approval,
            payload: value.payload.0,
        }
    }
}

impl From<CreateApprovalCaseInput> for NewApprovalCase {
    fn from(value: CreateApprovalCaseInput) -> Self {
        Self {
            intent_id: value.intent_id,
            status: value.status,
            review_kind: value.review_kind,
            requested_by: value.requested_by,
            summary: value.summary,
            metadata: value.metadata.0,
        }
    }
}

impl From<CreateVoteWindowInput> for NewVoteWindow {
    fn from(value: CreateVoteWindowInput) -> Self {
        Self {
            approval_case_id: value.approval_case_id,
            title: value.title,
            status: value.status,
            opens_at_unix: value.opens_at_unix,
            closes_at_unix: value.closes_at_unix,
            electorate: value.electorate.0,
        }
    }
}

impl From<CreateVoteReceiptInput> for NewVoteReceipt {
    fn from(value: CreateVoteReceiptInput) -> Self {
        Self {
            voter_id: value.voter_id,
            vote_choice: value.vote_choice,
            rationale: value.rationale,
            cast_at_unix: value.cast_at_unix,
        }
    }
}

impl From<CreateEvidenceBundleInput> for NewEvidenceBundle {
    fn from(value: CreateEvidenceBundleInput) -> Self {
        Self {
            intent_id: value.intent_id,
            approval_case_id: value.approval_case_id,
            vote_window_id: value.vote_window_id,
            artifact_uri: value.artifact_uri,
            digest: value.digest,
            summary: value.summary.0,
        }
    }
}

fn benchmark_report_matches(
    report: &BenchmarkReport,
    min_high_conflict_ratio: Option<f64>,
    min_serial_fraction: Option<f64>,
    log_class: Option<&str>,
) -> bool {
    if let Some(min_high_conflict_ratio) = min_high_conflict_ratio {
        if report.workload_profile.high_conflict_ratio < min_high_conflict_ratio {
            return false;
        }
    }

    if let Some(min_serial_fraction) = min_serial_fraction {
        if report.workload_profile.estimated_serial_fraction < min_serial_fraction {
            return false;
        }
    }

    if let Some(log_class) = log_class {
        if !report
            .workload_profile
            .log_classes
            .iter()
            .any(|entry| entry.class_name.eq_ignore_ascii_case(log_class))
        {
            return false;
        }
    }

    true
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BenchmarkReportSortField {
    GeneratedAt,
    HighConflictRatio,
    SerialFraction,
    TotalTransactions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortOrder {
    Asc,
    Desc,
}

impl BenchmarkReportSortField {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "generated_at" => Some(BenchmarkReportSortField::GeneratedAt),
            "high_conflict_ratio" => Some(BenchmarkReportSortField::HighConflictRatio),
            "serial_fraction" => Some(BenchmarkReportSortField::SerialFraction),
            "total_transactions" => Some(BenchmarkReportSortField::TotalTransactions),
            _ => None,
        }
    }
}

impl SortOrder {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "desc" => SortOrder::Desc,
            _ => SortOrder::Asc,
        }
    }
}

fn sort_benchmark_reports(
    reports: &mut Vec<BenchmarkReport>,
    sort_by: Option<&str>,
    sort_order: Option<&str>,
) {
    let field = sort_by
        .and_then(BenchmarkReportSortField::from_str)
        .unwrap_or(BenchmarkReportSortField::GeneratedAt);
    let order = sort_order
        .map(SortOrder::from_str)
        .unwrap_or(SortOrder::Desc);

    match (field, order) {
        (BenchmarkReportSortField::GeneratedAt, SortOrder::Asc) => {
            reports.sort_by_key(|r| r.generated_at_unix);
        }
        (BenchmarkReportSortField::GeneratedAt, SortOrder::Desc) => {
            reports.sort_by_key(|r| std::cmp::Reverse(r.generated_at_unix));
        }
        (BenchmarkReportSortField::HighConflictRatio, SortOrder::Asc) => {
            reports.sort_by(|a, b| {
                a.workload_profile
                    .high_conflict_ratio
                    .partial_cmp(&b.workload_profile.high_conflict_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::HighConflictRatio, SortOrder::Desc) => {
            reports.sort_by(|a, b| {
                b.workload_profile
                    .high_conflict_ratio
                    .partial_cmp(&a.workload_profile.high_conflict_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::SerialFraction, SortOrder::Asc) => {
            reports.sort_by(|a, b| {
                a.workload_profile
                    .estimated_serial_fraction
                    .partial_cmp(&b.workload_profile.estimated_serial_fraction)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::SerialFraction, SortOrder::Desc) => {
            reports.sort_by(|a, b| {
                b.workload_profile
                    .estimated_serial_fraction
                    .partial_cmp(&a.workload_profile.estimated_serial_fraction)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::TotalTransactions, SortOrder::Asc) => {
            reports.sort_by_key(|r| r.workload_profile.total_transactions);
        }
        (BenchmarkReportSortField::TotalTransactions, SortOrder::Desc) => {
            reports.sort_by_key(|r| std::cmp::Reverse(r.workload_profile.total_transactions));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_rpc::benchmark::{
        BenchmarkChainType, BenchmarkIntegrationTier, BenchmarkLogClassStat, BenchmarkMetrics,
        BenchmarkProfile, BenchmarkReportArtifact, BenchmarkReportSummary,
    };

    fn sample_report() -> BenchmarkReport {
        BenchmarkReport {
            report_id: "report-1".to_string(),
            generated_at_unix: 1,
            profile: BenchmarkProfile::Standard,
            chain_name: "PartnerChain".to_string(),
            chain_type: BenchmarkChainType::Evm,
            baseline: BenchmarkMetrics {
                avg_tps: 90.0,
                p50_latency_ms: 900,
                p95_latency_ms: 1800,
                p99_latency_ms: 2600,
                failure_rate: 0.02,
            },
            x3_replay: BenchmarkMetrics {
                avg_tps: 210.0,
                p50_latency_ms: 280,
                p95_latency_ms: 620,
                p99_latency_ms: 1000,
                failure_rate: 0.004,
            },
            recommendation: BenchmarkIntegrationTier::TurboLaneMode,
            summary: BenchmarkReportSummary {
                projected_soft_confirmation_improvement: "3.2x faster".to_string(),
                projected_app_throughput_improvement: "2.3x higher".to_string(),
                projected_route_latency_delta: "61% lower".to_string(),
                projected_bridge_latency_delta: "57% lower".to_string(),
            },
            workload_profile: BenchmarkWorkloadProfile {
                total_transactions: 12,
                total_receipts: 12,
                total_logs: 7,
                active_lanes: 4,
                active_log_lanes: 3,
                low_conflict_ratio: 0.25,
                medium_conflict_ratio: 0.45,
                high_conflict_ratio: 0.30,
                estimated_serial_fraction: 0.38,
                log_classes: vec![BenchmarkLogClassStat {
                    class_name: "bridge-event".to_string(),
                    count: 3,
                    share_of_logs: 0.42,
                    unique_contracts: 1,
                    unique_transactions: 3,
                }],
            },
            artifacts: vec![BenchmarkReportArtifact {
                artifact_type: "report-json".to_string(),
                uri: "benchmark://reports/report-1".to_string(),
                digest: "report-1".to_string(),
                metadata: None,
                signature: None,
            }],
            signer: "x3-sidecar".to_string(),
        }
    }

    #[test]
    fn benchmark_report_view_preserves_workload_profile() {
        let view = BenchmarkReportView::from(sample_report());
        assert_eq!(view.report_id, "report-1");
        assert_eq!(view.workload_profile.total_logs, 7);
        assert_eq!(
            view.workload_profile.log_classes[0].class_name,
            "bridge-event"
        );
        assert!(view.workload_profile.estimated_serial_fraction > 0.0);
    }

    #[test]
    fn graphql_benchmark_report_matches_filters() {
        let report = sample_report();
        assert!(benchmark_report_matches(
            &report,
            Some(0.25),
            Some(0.35),
            Some("bridge-event")
        ));
        assert!(!benchmark_report_matches(&report, Some(0.40), None, None));
        assert!(!benchmark_report_matches(
            &report,
            None,
            None,
            Some("erc20-transfer")
        ));
    }

    #[test]
    fn graphql_sort_benchmark_reports_by_generated_at_desc() {
        let mut reports = vec![
            BenchmarkReport {
                report_id: "report-1".to_string(),
                generated_at_unix: 100,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-2".to_string(),
                generated_at_unix: 300,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-3".to_string(),
                generated_at_unix: 200,
                ..sample_report()
            },
        ];
        sort_benchmark_reports(&mut reports, Some("generated_at"), Some("desc"));
        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-1");
    }

    #[test]
    fn graphql_sort_benchmark_reports_by_generated_at_asc() {
        let mut reports = vec![
            BenchmarkReport {
                report_id: "report-1".to_string(),
                generated_at_unix: 100,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-2".to_string(),
                generated_at_unix: 300,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-3".to_string(),
                generated_at_unix: 200,
                ..sample_report()
            },
        ];
        sort_benchmark_reports(&mut reports, Some("generated_at"), Some("asc"));
        assert_eq!(reports[0].report_id, "report-1");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-2");
    }

    #[test]
    fn graphql_sort_benchmark_reports_by_high_conflict_ratio_desc() {
        let mut report1 = sample_report();
        report1.report_id = "report-1".to_string();
        report1.workload_profile.high_conflict_ratio = 0.2;

        let mut report2 = sample_report();
        report2.report_id = "report-2".to_string();
        report2.workload_profile.high_conflict_ratio = 0.5;

        let mut report3 = sample_report();
        report3.report_id = "report-3".to_string();
        report3.workload_profile.high_conflict_ratio = 0.3;

        let mut reports = vec![report1, report2, report3];
        sort_benchmark_reports(&mut reports, Some("high_conflict_ratio"), Some("desc"));

        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-1");
    }

    #[test]
    fn graphql_sort_benchmark_reports_by_total_transactions_asc() {
        let mut report1 = sample_report();
        report1.report_id = "report-1".to_string();
        report1.workload_profile.total_transactions = 100;

        let mut report2 = sample_report();
        report2.report_id = "report-2".to_string();
        report2.workload_profile.total_transactions = 30;

        let mut report3 = sample_report();
        report3.report_id = "report-3".to_string();
        report3.workload_profile.total_transactions = 50;

        let mut reports = vec![report1, report2, report3];
        sort_benchmark_reports(&mut reports, Some("total_transactions"), Some("asc"));

        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-1");
    }

    #[test]
    fn graphql_sort_benchmark_reports_default_is_generated_at_desc() {
        let mut reports = vec![
            BenchmarkReport {
                report_id: "report-1".to_string(),
                generated_at_unix: 100,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-2".to_string(),
                generated_at_unix: 300,
                ..sample_report()
            },
        ];
        sort_benchmark_reports(&mut reports, None, None);
        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-1");
    }
}

// ============================================================================
// Object implementations for complex types
// ============================================================================

#[Object]
impl Block {
    async fn number(&self) -> i64 {
        self.number
    }

    async fn hash(&self) -> &str {
        &self.hash
    }

    async fn parent_hash(&self) -> &str {
        &self.parent_hash
    }

    async fn state_root(&self) -> &str {
        &self.state_root
    }

    async fn extrinsics_root(&self) -> &str {
        &self.extrinsics_root
    }

    async fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    async fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    async fn extrinsic_count(&self) -> i32 {
        self.extrinsic_count
    }

    async fn event_count(&self) -> i32 {
        self.event_count
    }
}

#[Object]
impl Extrinsic {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn block_number(&self) -> i64 {
        self.block_number
    }

    async fn extrinsic_index(&self) -> i32 {
        self.extrinsic_index
    }

    async fn hash(&self) -> &str {
        &self.hash
    }

    async fn pallet(&self) -> &str {
        &self.pallet
    }

    async fn call(&self) -> &str {
        &self.call
    }

    async fn signer(&self) -> Option<&str> {
        self.signer.as_deref()
    }

    async fn success(&self) -> bool {
        self.success
    }

    async fn fee(&self) -> Option<&str> {
        self.fee.as_deref()
    }
}

#[Object]
impl Event {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn block_number(&self) -> i64 {
        self.block_number
    }

    async fn extrinsic_index(&self) -> Option<i32> {
        self.extrinsic_index
    }

    async fn event_index(&self) -> i32 {
        self.event_index
    }

    async fn pallet(&self) -> &str {
        &self.pallet
    }

    async fn variant(&self) -> &str {
        &self.variant
    }

    async fn data(&self) -> &serde_json::Value {
        &self.data
    }
}

#[Object]
impl ComitTransaction {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn block_number(&self) -> i64 {
        self.block_number
    }

    async fn comit_hash(&self) -> &str {
        &self.comit_hash
    }

    async fn origin(&self) -> &str {
        &self.origin
    }

    async fn evm_payload_size(&self) -> i32 {
        self.evm_payload_size
    }

    async fn svm_payload_size(&self) -> i32 {
        self.svm_payload_size
    }

    async fn evm_gas_used(&self) -> Option<i64> {
        self.evm_gas_used
    }

    async fn svm_compute_used(&self) -> Option<i64> {
        self.svm_compute_used
    }

    async fn fee_paid(&self) -> &str {
        &self.fee_paid
    }

    async fn success(&self) -> bool {
        self.success
    }

    async fn evm_success(&self) -> Option<bool> {
        self.evm_success
    }

    async fn svm_success(&self) -> Option<bool> {
        self.svm_success
    }

    async fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

#[Object]
impl Account {
    async fn address(&self) -> &str {
        &self.address
    }

    async fn native_balance(&self) -> &str {
        &self.native_balance
    }

    async fn nonce(&self) -> i64 {
        self.nonce
    }

    async fn is_authorized(&self) -> bool {
        self.is_authorized
    }

    async fn first_seen_block(&self) -> i64 {
        self.first_seen_block
    }

    async fn last_seen_block(&self) -> i64 {
        self.last_seen_block
    }

    async fn total_transactions(&self) -> i64 {
        self.total_transactions
    }
}

#[Object]
impl ChainStats {
    async fn total_blocks(&self) -> i64 {
        self.total_blocks
    }

    async fn latest_block(&self) -> Option<i64> {
        self.latest_block
    }

    async fn total_extrinsics(&self) -> i64 {
        self.total_extrinsics
    }

    async fn total_events(&self) -> i64 {
        self.total_events
    }

    async fn total_comits(&self) -> i64 {
        self.total_comits
    }

    async fn successful_comits(&self) -> i64 {
        self.successful_comits
    }

    async fn failed_comits(&self) -> i64 {
        self.failed_comits
    }

    async fn total_accounts(&self) -> i64 {
        self.total_accounts
    }
}
