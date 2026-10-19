use x3_gateway::{
    config::DatabaseConfig,
    db::{Database, NewFundingSwarmGrant},
};

const TEST_DATABASE_URL_ENV: &str = "X3_GATEWAY_TEST_DATABASE_URL";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a real Postgres instance; run via scripts/gateway-postgres-integration.sh"]
async fn migrations_and_funding_swarm_roundtrip_against_postgres() {
    legacy_staging_preserves_public_funding_totals().await;
    funding_workflow_enforces_state_and_preserves_audit().await;
    funding_source_snapshots_are_durable_and_preserve_review().await;
    proposal_versions_bind_source_and_serialize_edits().await;
    fresh_schema_supports_chain_queries().await;
    gateway_schema_upgrades_both_legacy_histories().await;
    control_plane_roundtrip_and_duplicate_vote_rejection().await;
    control_plane_sync_upserts_are_idempotent().await;
    benchmark_reports_survive_upserts_and_reconnect().await;
    let database_url = std::env::var(TEST_DATABASE_URL_ENV).unwrap_or_else(|_| {
        panic!(
            "{TEST_DATABASE_URL_ENV} must point at the disposable Postgres instance owned by the integration gate"
        )
    });

    let db = Database::connect(&DatabaseConfig::new(database_url))
        .await
        .expect("connect to disposable Postgres and run gateway migrations");

    assert!(db.healthy().await, "Postgres must answer a real SELECT 1");

    let before = db
        .get_funding_swarm_scoreboard()
        .await
        .expect("read seeded funding swarm scoreboard");

    let examples: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM funding_swarm_grants WHERE grant_id LIKE 'grant-demo-%'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(examples, 3, "historical examples must remain inspectable");
    assert_eq!(before.funded_grants, 0, "examples are not verified awards");
    assert_eq!(before.total_awarded_usd, 0.0);
    assert!(db
        .list_funding_swarm_grants(100, 0)
        .await
        .unwrap()
        .iter()
        .all(|g| !g.grant_id.starts_with("grant-demo-")));
    assert!(db
        .list_funding_swarm_timeline(100, 0)
        .await
        .unwrap()
        .iter()
        .all(|item| !item.item_id.starts_with("event-demo-")
            && !item.item_id.starts_with("pub-demo-")));

    let external_id = format!("IT-{}", uuid::Uuid::new_v4());
    let created = db
        .admin_create_funding_swarm_grant(NewFundingSwarmGrant {
            external_id: external_id.clone(),
            title: "Gateway Postgres integration test".to_string(),
            sponsor: "X3 CI".to_string(),
            amount_usd: Some(1_234.0),
            metadata: Some(serde_json::json!({"source":"postgres-integration-gate"})),
        })
        .await
        .expect("insert a grant through the production database API");

    assert_eq!(created.external_id, external_id);
    assert_eq!(created.status, "open");
    assert_eq!(created.stage, "discovery");

    let grants = db
        .admin_list_funding_swarm_grants(100, 0)
        .await
        .expect("read grants back through the production database API");

    assert!(
        grants
            .iter()
            .any(|grant| grant.grant_id == created.grant_id),
        "newly inserted grant must be visible through the admin list path"
    );

    let after = db
        .get_funding_swarm_scoreboard()
        .await
        .expect("read scoreboard after inserting a grant");

    assert_eq!(after.total_grants, before.total_grants + 1);
    assert_eq!(after.open_grants, before.open_grants + 1);
}

async fn legacy_staging_preserves_public_funding_totals() {
    use x3_gateway::funding_legacy::LegacyImport;
    let database_url = std::env::var(TEST_DATABASE_URL_ENV).unwrap();
    let db = Database::connect(&DatabaseConfig::new(database_url))
        .await
        .unwrap();
    let before = db.get_funding_swarm_scoreboard().await.unwrap();
    let id = format!("C27.1.{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    let lead = serde_json::json!({
        "legacy_id":id,"name":"Archived research test",
        "category":"C27-chain-grants",
        "source_file":"grants/C27-chain-grants.md","source_line":14,
        "raw_fields":["Historical reference only","https://example.org/archive"],
        "candidate_urls":["https://example.org/archive"],
        "historical_status":"AWARDED",
        "historical_verification_date":"2026-09-05",
        "origin":"legacy_markdown_research",
        "current_availability":"UNKNOWN",
        "current_eligibility":"UNKNOWN",
        "application_authorized":false
    });
    let make_request = |lead: serde_json::Value| -> LegacyImport {
        serde_json::from_value(serde_json::json!({"leads":[lead]})).unwrap()
    };
    let first = db.import_legacy_leads(make_request(lead.clone())).await.unwrap();
    assert_eq!((first.inserted, first.unchanged), (1, 0));
    assert_eq!(first.current_opportunities_verified, 0);
    let replay = db.import_legacy_leads(make_request(lead.clone())).await.unwrap();
    assert_eq!((replay.inserted, replay.unchanged), (0, 1));
    let mut altered = lead.clone();
    altered["name"] = serde_json::json!("Changed evidence");
    assert!(db.import_legacy_leads(make_request(altered)).await.is_err());
    let mut forged = lead.clone();
    forged["current_availability"] = serde_json::json!("OPEN");
    assert!(db.import_legacy_leads(make_request(forged)).await.is_err());
    let row = db
        .list_legacy_leads(100, 0, &id)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.legacy_id == id)
        .expect("archived record remains private and searchable");
    assert_eq!(row.historical_status, "AWARDED");
    assert_eq!(row.current_availability, "UNKNOWN");
    assert_eq!(row.current_eligibility, "UNKNOWN");
    assert_eq!(row.review_state, "UNVERIFIED");
    assert_eq!(row.raw_record["application_authorized"], false);
    let after = db.get_funding_swarm_scoreboard().await.unwrap();
    assert_eq!(before.total_grants, after.total_grants);
    assert_eq!(before.funded_grants, after.funded_grants);
    assert_eq!(before.total_awarded_usd, after.total_awarded_usd);
}

async fn funding_source_snapshots_are_durable_and_preserve_review() {
    use x3_gateway::funding_discovery::normalize_opportunity;
    let url = std::env::var(TEST_DATABASE_URL_ENV).unwrap();
    let db = Database::connect(&DatabaseConfig::new(url.clone()))
        .await
        .unwrap();
    let mut source = serde_json::json!({"id":456,"opportunityNumber":"SOURCE-FIXTURE",
        "opportunityTitle":"Source snapshot fixture","synopsis":{"agencyName":"Fixture agency",
        "synopsisDesc":"Original source description", "responseDateStr":"2030-01-01-00-00-00"}});
    let opportunity = normalize_opportunity(source.clone(), 456, chrono::Utc::now()).unwrap();
    assert!(db
        .store_funding_snapshot(&opportunity, &source)
        .await
        .unwrap());
    assert!(
        !db.store_funding_snapshot(&opportunity, &source)
            .await
            .unwrap(),
        "same source document is idempotent"
    );
    let grant_id: String = sqlx::query_scalar(
        "SELECT grant_id FROM funding_swarm_grants WHERE external_id='grants.gov:456'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    db.admin_advance_funding_swarm_stage(&grant_id, "research", "open")
        .await
        .unwrap();
    source["synopsis"]["synopsisDesc"] = serde_json::json!("Changed source description");
    let changed = normalize_opportunity(source.clone(), 456, chrono::Utc::now()).unwrap();
    assert!(db.store_funding_snapshot(&changed, &source).await.unwrap());
    let mut forged = changed.clone();
    forged.official_url = "https://example.invalid/forged".into();
    assert!(
        db.store_funding_snapshot(&forged, &source).await.is_err(),
        "normalized evidence must bind to the original source"
    );
    let open_before_clock = db.get_funding_swarm_scoreboard().await.unwrap().open_grants;
    // Availability must age out without waiting for another upstream refresh.
    sqlx::query("UPDATE funding_swarm_grants SET metadata=jsonb_set(metadata,'{deadline}','\"2000-01-01\"') WHERE grant_id=$1")
        .bind(&grant_id).execute(db.pool()).await.unwrap();
    assert_eq!(
        db.get_funding_swarm_scoreboard().await.unwrap().open_grants,
        open_before_clock - 1
    );
    sqlx::query("UPDATE funding_swarm_grants SET metadata=jsonb_set(metadata,'{deadline}','\"2030-01-01\"') WHERE grant_id=$1")
        .bind(&grant_id).execute(db.pool()).await.unwrap();
    let open_before = db.get_funding_swarm_scoreboard().await.unwrap().open_grants;
    source["synopsis"]["responseDateStr"] = serde_json::json!("2000-01-01-00-00-00");
    let expired = normalize_opportunity(source.clone(), 456, chrono::Utc::now()).unwrap();
    assert!(db.store_funding_snapshot(&expired, &source).await.unwrap());
    assert_eq!(
        db.get_funding_swarm_scoreboard().await.unwrap().open_grants,
        open_before - 1,
        "expired research opportunities must leave the open count"
    );
    let reopened = Database::connect(&DatabaseConfig::new(url)).await.unwrap();
    let stage: String =
        sqlx::query_scalar("SELECT stage FROM funding_swarm_grants WHERE grant_id=$1")
            .bind(&grant_id)
            .fetch_one(reopened.pool())
            .await
            .unwrap();
    assert_eq!(
        stage, "research",
        "source refresh must not reset human review"
    );
    let snapshots: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM funding_source_snapshots WHERE grant_id=$1")
            .bind(&grant_id)
            .fetch_one(reopened.pool())
            .await
            .unwrap();
    assert_eq!(snapshots, 3);
    let original: String = sqlx::query_scalar("SELECT source_document->'synopsis'->>'synopsisDesc' FROM funding_source_snapshots WHERE grant_id=$1 ORDER BY retrieved_at LIMIT 1")
        .bind(&grant_id).fetch_one(reopened.pool()).await.unwrap();
    assert_eq!(original, "Original source description");
    let audit_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM funding_swarm_events WHERE grant_id=$1 AND event_type='official_source_snapshot'")
        .bind(&grant_id).fetch_one(reopened.pool()).await.unwrap();
    assert_eq!(
        audit_events, 3,
        "only changed source documents emit discovery events"
    );
}

async fn funding_workflow_enforces_state_and_preserves_audit() {
    use x3_gateway::error::GatewayError;
    let url = std::env::var(TEST_DATABASE_URL_ENV).unwrap();
    let db = Database::connect(&DatabaseConfig::new(url.clone()))
        .await
        .unwrap();
    let grant = db
        .admin_create_funding_swarm_grant(NewFundingSwarmGrant {
            external_id: uuid::Uuid::new_v4().to_string(),
            title: "Funding security regression".into(),
            sponsor: "X3 test".into(),
            amount_usd: Some(100.0),
            metadata: None,
        })
        .await
        .unwrap();
    let id = &grant.grant_id;
    assert!(
        matches!(
            db.admin_advance_funding_swarm_stage(id, "approved", "approved")
                .await,
            Err(GatewayError::BadRequest(_))
        ),
        "cannot approve before research and drafting"
    );
    assert!(
        matches!(
            db.admin_advance_funding_swarm_stage(id, "submitted", "award_paid")
                .await,
            Err(GatewayError::BadRequest(_))
        ),
        "cannot invent an award or payment"
    );
    assert!(
        matches!(
            db.admin_advance_funding_swarm_stage(id, "research", "funded")
                .await,
            Err(GatewayError::BadRequest(_))
        ),
        "stage/status pair must be canonical"
    );
    let (left, right) = tokio::join!(
        db.admin_advance_funding_swarm_stage(id, "research", "open"),
        db.admin_advance_funding_swarm_stage(id, "research", "open")
    );
    assert_eq!(
        usize::from(left.is_ok()) + usize::from(right.is_ok()),
        1,
        "exactly one concurrent transition may commit"
    );
    let events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM funding_swarm_events WHERE grant_id = $1")
            .bind(id)
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(events, 1, "rejected actions cannot emit success events");

    // Make only the audit insertion fail. State and audit must roll back together.
    sqlx::raw_sql("CREATE FUNCTION reject_funding_audit_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'audit unavailable'; END $$;
        CREATE TRIGGER reject_funding_audit_test BEFORE INSERT ON funding_swarm_events FOR EACH ROW EXECUTE FUNCTION reject_funding_audit_test();")
        .execute(db.pool()).await.unwrap();
    assert!(db
        .admin_advance_funding_swarm_stage(id, "draft", "pending_human_approval")
        .await
        .is_err());
    sqlx::raw_sql("DROP TRIGGER reject_funding_audit_test ON funding_swarm_events; DROP FUNCTION reject_funding_audit_test();")
        .execute(db.pool()).await.unwrap();
    let stage: String =
        sqlx::query_scalar("SELECT stage FROM funding_swarm_grants WHERE grant_id=$1")
            .bind(id)
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(stage, "research", "failed audit must not advance state");
    db.admin_advance_funding_swarm_stage(id, "draft", "pending_human_approval")
        .await
        .unwrap();
    db.admin_advance_funding_swarm_stage(id, "approved", "approved")
        .await
        .unwrap();
    assert!(db
        .admin_advance_funding_swarm_stage(id, "approved", "approved")
        .await
        .is_err());
    let reopened = Database::connect(&DatabaseConfig::new(url)).await.unwrap();
    let events: Vec<(String, String)> = sqlx::query_as("SELECT event_type, visibility FROM funding_swarm_events WHERE grant_id=$1 ORDER BY created_at, event_id")
        .bind(id).fetch_all(reopened.pool()).await.unwrap();
    assert_eq!(
        events,
        vec![
            ("ai_research_started".into(), "private".into()),
            ("ai_draft_ready".into(), "private".into()),
            ("human_approved".into(), "public".into())
        ]
    );
    assert!(reopened
        .admin_advance_funding_swarm_stage("unknown", "research", "open")
        .await
        .unwrap()
        .is_none());
    sqlx::query("UPDATE funding_swarm_grants SET stage='corrupted' WHERE grant_id=$1")
        .bind(id)
        .execute(db.pool())
        .await
        .unwrap();
    assert!(reopened
        .admin_advance_funding_swarm_stage(id, "research", "open")
        .await
        .is_err());
}

async fn fresh_schema_supports_chain_queries() {
    let url = std::env::var(TEST_DATABASE_URL_ENV).expect("integration database URL");
    let admin = sqlx::PgPool::connect(&url).await.expect("admin pool");
    let name = format!("gateway_fresh_{}", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .expect("isolated fresh database");
    let mut isolated_url = reqwest::Url::parse(&url).expect("database URL");
    isolated_url.set_path(&format!("/{name}"));
    let config = DatabaseConfig::new(isolated_url.to_string());
    let (first, second) = tokio::join!(Database::connect(&config), Database::connect(&config));
    let db = first.expect("migrate isolated database concurrently");
    second
        .expect("concurrent migration follower")
        .pool()
        .close()
        .await;
    assert!(db
        .get_latest_block()
        .await
        .expect("query fresh blocks")
        .is_none());
    let stats = db
        .get_stats()
        .await
        .expect("query statistics on an empty chain");
    assert_eq!(stats.latest_block, None);
    assert_eq!(stats.total_blocks, 0);
    assert!(db
        .get_account("absent")
        .await
        .expect("query accounts")
        .is_none());
    assert!(db
        .get_recent_extrinsics(10, 0)
        .await
        .expect("query extrinsics")
        .is_empty());
    assert!(db
        .get_events_by_pallet("Balances", 10, 0)
        .await
        .expect("query events")
        .is_empty());
    assert!(db
        .get_recent_comits(10, 0)
        .await
        .expect("query comits")
        .is_empty());
    assert!(db
        .list_orchestra_intents(10, 0)
        .await
        .expect("query intents")
        .is_empty());
    assert!(db
        .list_approval_cases(10, 0)
        .await
        .expect("query approvals")
        .is_empty());
    assert!(db
        .list_vote_windows(10, 0)
        .await
        .expect("query vote windows")
        .is_empty());
    assert!(db
        .list_evidence_bundles(10, 0)
        .await
        .expect("query evidence")
        .is_empty());
    assert!(db
        .get_benchmark_reports(None, 10, 0)
        .await
        .expect("query benchmark reports")
        .is_empty());
    sqlx::raw_sql("INSERT INTO blocks (number, hash, parent_hash, state_root, extrinsics_root, timestamp, extrinsic_count, event_count) VALUES (1, 'block-1', 'genesis', 'state-1', 'extrinsics-1', NOW(), 1, 1);
        INSERT INTO extrinsics (block_number, extrinsic_index, hash, pallet, call, signer, success, fee) VALUES (1, 0, 'extrinsic-1', 'Balances', 'transfer', 'account-1', true, '7');
        INSERT INTO events (block_number, extrinsic_index, event_index, pallet, variant, data) VALUES (1, 0, 0, 'Balances', 'Transfer', '{\"amount\":5}');
        INSERT INTO comit_transactions (block_number, extrinsic_index, comit_hash, origin, fee_paid, success) VALUES (1, 0, 'comit-1', 'account-1', '7', true);
        INSERT INTO accounts (address, native_balance, first_seen_block, last_seen_block) VALUES ('account-1', '12345', 1, 1);")
        .execute(db.pool()).await.expect("persist indexed chain fixture");
    assert_eq!(db.get_block(1).await.unwrap().unwrap().hash, "block-1");
    assert_eq!(
        db.get_block_by_hash("block-1")
            .await
            .unwrap()
            .unwrap()
            .number,
        1
    );
    assert_eq!(
        db.get_block_extrinsics(1).await.unwrap()[0].fee.as_deref(),
        Some("7")
    );
    assert_eq!(
        db.get_extrinsic("extrinsic-1")
            .await
            .unwrap()
            .unwrap()
            .signer
            .as_deref(),
        Some("account-1")
    );
    assert_eq!(db.get_block_events(1).await.unwrap()[0].data["amount"], 5);
    assert!(db.get_comit("comit-1").await.unwrap().unwrap().success);
    assert_eq!(
        db.get_top_accounts(1).await.unwrap()[0].native_balance,
        "12345"
    );
    let stats = db.get_stats().await.unwrap();
    assert_eq!(stats.latest_block, Some(1));
    assert_eq!(
        (
            stats.total_blocks,
            stats.total_extrinsics,
            stats.total_events,
            stats.total_comits,
            stats.total_accounts
        ),
        (1, 1, 1, 1, 1)
    );
    assert_eq!((stats.successful_comits, stats.failed_comits), (1, 0));
    db.pool().close().await;
    sqlx::query(&format!("DROP DATABASE {name}"))
        .execute(&admin)
        .await
        .expect("remove isolated database");
    admin.close().await;
}

async fn gateway_schema_upgrades_both_legacy_histories() {
    use std::borrow::Cow;
    let url = std::env::var(TEST_DATABASE_URL_ENV).expect("integration database URL");
    let admin = sqlx::PgPool::connect(&url).await.expect("admin pool");
    for legacy_version in [6, 20241205] {
        let name = format!("gateway_upgrade_{}", uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {name}"))
            .execute(&admin)
            .await
            .expect("create isolated upgrade database");
        let mut isolated_url = reqwest::Url::parse(&url).expect("database URL");
        isolated_url.set_path(&format!("/{name}"));
        let config = DatabaseConfig::new(isolated_url.to_string());
        let pool = sqlx::PgPool::connect(&config.url)
            .await
            .expect("legacy pool");
        let mut legacy = sqlx::migrate!("./migrations");
        legacy.migrations = Cow::Owned(
            legacy
                .iter()
                .filter(|m| m.version == legacy_version)
                .cloned()
                .collect(),
        );
        assert_eq!(legacy.iter().count(), 1, "legacy migration must exist");
        legacy
            .run(&pool)
            .await
            .expect("install actual legacy migration and checksum");
        // Retain user data across both supported starting points.
        if legacy_version == 6 {
            sqlx::query("UPDATE funding_swarm_grants SET title = 'retained legacy grant' WHERE external_id = (SELECT external_id FROM funding_swarm_grants LIMIT 1)")
                .execute(&pool).await.expect("legacy grant update");
        } else {
            sqlx::query("INSERT INTO accounts (address, native_balance, first_seen_block, last_seen_block) VALUES ('retained-account', '12345', 0, 0)")
                .execute(&pool).await.expect("legacy account");
        }
        pool.close().await;
        let db = Database::connect(&config)
            .await
            .expect("upgrade legacy schema");
        db.get_stats().await.expect("statistics after upgrade");
        db.list_orchestra_intents(10, 0)
            .await
            .expect("new control-plane table");
        if legacy_version == 6 {
            assert!(db
                .admin_list_funding_swarm_grants(100, 0)
                .await
                .unwrap()
                .iter()
                .any(|g| g.title == "retained legacy grant"));
        } else {
            assert_eq!(
                db.get_account("retained-account")
                    .await
                    .unwrap()
                    .unwrap()
                    .native_balance,
                "12345"
            );
        }
        // Indexer and gateway use the same migration versions and checksums.
        sqlx::migrate!("../x3-indexer/migrations")
            .run(db.pool())
            .await
            .expect("indexer reconnect after gateway migration");
        let second = Database::connect(&config)
            .await
            .expect("idempotent reconnect");
        second.pool().close().await;
        db.pool().close().await;
        sqlx::query(&format!("DROP DATABASE {name}"))
            .execute(&admin)
            .await
            .expect("remove isolated upgrade database");
    }
    admin.close().await;
}

async fn control_plane_roundtrip_and_duplicate_vote_rejection() {
    use x3_gateway::db::{
        NewApprovalCase, NewEvidenceBundle, NewOrchestraIntent, NewVoteReceipt, NewVoteWindow,
    };
    let url = std::env::var(TEST_DATABASE_URL_ENV).expect("integration database URL");
    let db = Database::connect(&DatabaseConfig::new(url))
        .await
        .expect("migrate database");
    let intent = db
        .create_orchestra_intent(NewOrchestraIntent {
            tenant_id: "integration".into(),
            kind: "validation".into(),
            status: "pending".into(),
            risk_class: "low".into(),
            submitter: "operator".into(),
            requires_approval: true,
            payload: serde_json::json!({"network":"internal"}),
        })
        .await
        .expect("persist intent");
    let case = db
        .create_approval_case(NewApprovalCase {
            intent_id: intent.intent_id.clone(),
            status: "pending".into(),
            review_kind: "quorum".into(),
            requested_by: "operator".into(),
            summary: "integration review".into(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("persist approval");
    let window = db
        .create_vote_window(NewVoteWindow {
            approval_case_id: case.case_id.clone(),
            title: "review".into(),
            status: "open".into(),
            opens_at_unix: 1,
            closes_at_unix: 100,
            electorate: serde_json::json!(["validator-1"]),
        })
        .await
        .expect("persist vote window");
    let vote = NewVoteReceipt {
        voter_id: "validator-1".into(),
        vote_choice: "approve".into(),
        rationale: None,
        cast_at_unix: 2,
    };
    db.create_vote_receipt(&window.window_id, vote.clone())
        .await
        .expect("persist vote");
    assert!(matches!(
        db.create_vote_receipt(&window.window_id, vote).await,
        Err(x3_gateway::error::GatewayError::BadRequest(_))
    ));
    let evidence = db
        .create_evidence_bundle(NewEvidenceBundle {
            intent_id: Some(intent.intent_id.clone()),
            approval_case_id: Some(case.case_id.clone()),
            vote_window_id: Some(window.window_id.clone()),
            artifact_uri: "audit-artifacts/integration".into(),
            digest: "sha256:test".into(),
            summary: serde_json::json!({"verified":true}),
        })
        .await
        .expect("persist evidence");
    assert_eq!(
        db.get_evidence_bundle(&evidence.bundle_id)
            .await
            .unwrap()
            .unwrap()
            .intent_id,
        Some(intent.intent_id)
    );
    assert_eq!(
        db.get_approval_case(&case.case_id)
            .await
            .unwrap()
            .unwrap()
            .case_id,
        case.case_id
    );
    assert_eq!(
        db.get_vote_window(&window.window_id)
            .await
            .unwrap()
            .unwrap()
            .window_id,
        window.window_id
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vote_receipts WHERE window_id=$1")
        .bind(&window.window_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(count, 1, "duplicate vote must not write another receipt");
}

async fn benchmark_reports_survive_upserts_and_reconnect() {
    use x3_gateway::db::StoredBenchmarkReport;
    let url = std::env::var(TEST_DATABASE_URL_ENV).expect("integration database URL");
    let config = DatabaseConfig::new(url);
    let db = Database::connect(&config).await.expect("migrate database");
    let report_id = uuid::Uuid::new_v4().to_string();
    let report = serde_json::from_value(serde_json::json!({
        "report_id": report_id, "generated_at_unix": 1700000000,
        "chain_name": "integration", "chain_type": "substrate", "recommendation": "benchmark-only",
        "signer": "integration-validator",
        "baseline": {"avg_tps": 1.0, "p50_latency_ms": 10, "p95_latency_ms": 20, "p99_latency_ms": 30, "failure_rate": 0.0},
        "x3_replay": {"avg_tps": 2.0, "p50_latency_ms": 5, "p95_latency_ms": 10, "p99_latency_ms": 15, "failure_rate": 0.0},
        "summary": {"projected_soft_confirmation_improvement": "test fixture", "projected_app_throughput_improvement": "test fixture", "projected_route_latency_delta": "test fixture", "projected_bridge_latency_delta": "test fixture"},
        "workload_profile": {"total_transactions": 1, "total_receipts": 1, "total_logs": 0, "active_lanes": 1, "active_log_lanes": 0, "low_conflict_ratio": 1.0, "medium_conflict_ratio": 0.0, "high_conflict_ratio": 0.0, "estimated_serial_fraction": 1.0, "log_classes": []},
        "artifacts": []
    })).expect("typed benchmark report");
    let mut stored = StoredBenchmarkReport {
        tenant_id: "integration-benchmarks".into(),
        report,
    };
    db.insert_benchmark_report(&stored)
        .await
        .expect("insert report");
    stored.report.x3_replay.avg_tps = 3.0;
    db.insert_benchmark_report(&stored)
        .await
        .expect("upsert same report");
    db.pool().close().await;
    let db = Database::connect(&config)
        .await
        .expect("reconnect and revalidate migrations");
    let fetched = db.get_benchmark_report(&report_id).await.unwrap().unwrap();
    assert_eq!(fetched.x3_replay.avg_tps, 3.0);
    let reports = db
        .get_benchmark_reports(Some(&stored.tenant_id), 100, 0)
        .await
        .unwrap();
    assert_eq!(
        reports.iter().filter(|r| r.report_id == report_id).count(),
        1
    );
    assert!(db
        .get_benchmark_reports(Some("absent-tenant"), 100, 0)
        .await
        .unwrap()
        .is_empty());
    db.pool().close().await;
}

async fn control_plane_sync_upserts_are_idempotent() {
    use x3_orchestra_control_plane as control;
    let url = std::env::var(TEST_DATABASE_URL_ENV).expect("integration database URL");
    let db = Database::connect(&DatabaseConfig::new(url))
        .await
        .expect("migrate database");
    let id = || uuid::Uuid::new_v4().to_string();
    let mut intent = control::Intent {
        intent_id: id(),
        tenant_id: "integration-sync".into(),
        kind: control::IntentKind::Validation,
        status: control::IntentStatus::PendingApproval,
        risk_class: control::RiskClass::Low,
        submitter: "operator".into(),
        requires_approval: true,
        payload: serde_json::json!({}),
        created_at_unix: 1700000000,
        updated_at_unix: 1700000000,
    };
    let mut case = control::ApprovalCase {
        case_id: id(),
        intent_id: intent.intent_id.clone(),
        status: control::ApprovalStatus::Open,
        review_kind: "quorum".into(),
        requested_by: "operator".into(),
        summary: "synchronized review".into(),
        metadata: serde_json::json!({}),
        created_at_unix: 1700000000,
        updated_at_unix: 1700000000,
    };
    let mut window = control::VoteWindow {
        window_id: id(),
        approval_case_id: case.case_id.clone(),
        title: "synchronized window".into(),
        status: control::VoteWindowStatus::Open,
        opens_at_unix: 1,
        closes_at_unix: 100,
        electorate: vec!["validator-sync".into()],
        tally: control::VoteTally::default(),
        created_at_unix: 1700000000,
        updated_at_unix: 1700000000,
    };
    let receipt = control::VoteReceipt {
        receipt_id: id(),
        window_id: window.window_id.clone(),
        voter_id: "validator-sync".into(),
        vote_choice: control::VoteChoice::Approve,
        rationale: Some("verified".into()),
        cast_at_unix: 2,
    };
    let evidence = control::EvidenceBundle {
        bundle_id: id(),
        intent_id: Some(intent.intent_id.clone()),
        approval_case_id: Some(case.case_id.clone()),
        vote_window_id: Some(window.window_id.clone()),
        artifact_uri: "audit-artifacts/integration-sync".into(),
        digest: "sha256:test-sync".into(),
        summary: control::EvidenceSummary {
            action: "validation".into(),
            detail: serde_json::json!({"persisted":true}),
        },
        created_at_unix: 1700000000,
    };
    for _ in 0..2 {
        db.upsert_orchestra_intent_from_control_plane(&intent)
            .await
            .expect("sync intent");
        db.upsert_approval_case_from_control_plane(&case)
            .await
            .expect("sync approval");
        db.upsert_vote_window_from_control_plane(&window)
            .await
            .expect("sync window");
        db.upsert_vote_receipt_from_control_plane(&receipt)
            .await
            .expect("sync receipt");
        db.upsert_evidence_bundle_from_control_plane(&evidence)
            .await
            .expect("sync evidence");
        intent.status = control::IntentStatus::Ready;
        intent.updated_at_unix += 1;
        case.status = control::ApprovalStatus::Approved;
        case.updated_at_unix += 1;
        window.tally.approvals = 1;
        window.updated_at_unix += 1;
    }
    let tally = control::VoteTally {
        approvals: 2,
        rejections: 1,
        abstentions: 0,
    };
    let updated = db
        .update_vote_window_tally(&window.window_id, &tally)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.tally["approvals"], 2);
    let stored = db
        .get_orchestra_intent(&intent.intent_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, "ready");
    assert_eq!(stored.created_at.timestamp(), 1700000000);
    assert_eq!(stored.updated_at.timestamp(), 1700000001);
    assert_eq!(
        db.get_approval_case(&case.case_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        "approved"
    );
    assert_eq!(
        db.get_evidence_bundle(&evidence.bundle_id)
            .await
            .unwrap()
            .unwrap()
            .summary["detail"]["persisted"],
        true
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vote_receipts WHERE window_id=$1")
        .bind(&window.window_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        count, 1,
        "replayed control-plane sync must not duplicate votes"
    );
    db.pool().close().await;
}

async fn proposal_versions_bind_source_and_serialize_edits() {
    use sha2::{Digest, Sha256};
    use x3_gateway::funding_discovery::normalize_opportunity;
    use x3_gateway::funding_proposals::ProposalInput;
    let url = std::env::var(TEST_DATABASE_URL_ENV).unwrap();
    let db = Database::connect(&DatabaseConfig::new(url.clone()))
        .await
        .unwrap();
    let mut source = serde_json::json!({"id":789,"opportunityNumber":"PROPOSAL-FIXTURE",
        "opportunityTitle":"Proposal source fixture","synopsis":{"agencyName":"Fixture agency","synopsisDesc":"Original evidence"}});
    db.store_funding_snapshot(
        &normalize_opportunity(source.clone(), 789, chrono::Utc::now()).unwrap(),
        &source,
    )
    .await
    .unwrap();
    let id: String = sqlx::query_scalar(
        "SELECT grant_id FROM funding_swarm_grants WHERE external_id='grants.gov:789'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    let context = db.proposal_context(&id).await.unwrap();
    let original_source = source.clone();
    source["synopsis"]["synopsisDesc"] = serde_json::json!("Intermediate source B");
    db.store_funding_snapshot(
        &normalize_opportunity(source.clone(), 789, chrono::Utc::now()).unwrap(),
        &source,
    )
    .await
    .unwrap();
    let superseded_snapshot = db.proposal_context(&id).await.unwrap().source.snapshot_id;
    source = original_source;
    assert!(
        !db.store_funding_snapshot(
            &normalize_opportunity(source.clone(), 789, chrono::Utc::now()).unwrap(),
            &source
        )
        .await
        .unwrap(),
        "revisited source A reuses its immutable document"
    );
    assert_eq!(
        db.proposal_context(&id).await.unwrap().source.snapshot_id,
        context.source.snapshot_id,
        "A to B to A must select current A, not most recently created B"
    );
    let make_input = |expected_version, snapshot_id: String| ProposalInput {
        expected_version,
        source_snapshot_id: snapshot_id,
        project_title: "Fixture project".into(),
        research_question: "Fixture research question".into(),
        technical_approach: "Compare with a sequential reference".into(),
        evaluation_plan: "Publish actual measurements".into(),
        organization_facts: "Fixture self-report, not verified eligibility".into(),
        requested_usd_cents: Some(12345),
    };
    assert!(
        db.save_proposal(&id, make_input(0, context.source.snapshot_id.clone()))
            .await
            .is_err(),
        "discovery is not an editable proposal stage"
    );
    db.admin_advance_funding_swarm_stage(&id, "research", "open")
        .await
        .unwrap();
    assert!(db
        .save_proposal(&id, make_input(0, "foreign-source".into()))
        .await
        .is_err());
    assert!(
        db.save_proposal(&id, make_input(0, superseded_snapshot))
            .await
            .is_err(),
        "a newer-created B snapshot must not supersede current A"
    );
    let first = db
        .save_proposal(&id, make_input(0, context.source.snapshot_id.clone()))
        .await
        .unwrap();
    assert_eq!(first.version, 1);
    assert_eq!(first.package["eligibility_status"], "UNKNOWN");
    assert_eq!(first.package["external_submission"], false);
    assert_eq!(
        first.package_sha256,
        hex::encode(Sha256::digest(serde_json::to_vec(&first.package).unwrap()))
    );
    assert!(
        db.save_proposal(&id, make_input(0, context.source.snapshot_id.clone()))
            .await
            .is_err(),
        "stale editor must not overwrite a version"
    );
    let (a, b) = tokio::join!(
        db.save_proposal(&id, make_input(1, context.source.snapshot_id.clone())),
        db.save_proposal(&id, make_input(1, context.source.snapshot_id.clone()))
    );
    assert_eq!(
        usize::from(a.is_ok()) + usize::from(b.is_ok()),
        1,
        "exactly one competing edit commits"
    );
    sqlx::query("CREATE FUNCTION reject_proposal_event() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected proposal audit failure'; END $$").execute(db.pool()).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_proposal_event BEFORE INSERT ON funding_swarm_events FOR EACH ROW WHEN (NEW.event_type='proposal_version_saved') EXECUTE FUNCTION reject_proposal_event()").execute(db.pool()).await.unwrap();
    assert!(db
        .save_proposal(&id, make_input(2, context.source.snapshot_id.clone()))
        .await
        .is_err());
    sqlx::query("DROP TRIGGER reject_proposal_event ON funding_swarm_events")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_proposal_event()")
        .execute(db.pool())
        .await
        .unwrap();
    source["synopsis"]["synopsisDesc"] = serde_json::json!("Changed source evidence");
    db.store_funding_snapshot(
        &normalize_opportunity(source.clone(), 789, chrono::Utc::now()).unwrap(),
        &source,
    )
    .await
    .unwrap();
    assert!(
        db.save_proposal(&id, make_input(2, context.source.snapshot_id))
            .await
            .is_err(),
        "an upstream change requires another evidence review"
    );
    let reopened = Database::connect(&DatabaseConfig::new(url)).await.unwrap();
    let context = reopened.proposal_context(&id).await.unwrap();
    assert_eq!(
        context.versions.len(),
        2,
        "failed audit insert leaves no partial version"
    );
    assert_eq!(
        context.versions[1].package_sha256, first.package_sha256,
        "older versions remain intact"
    );
    assert_ne!(context.source.snapshot_id, first.source_snapshot_id);
    let audit:i64=sqlx::query_scalar("SELECT COUNT(*) FROM funding_swarm_events WHERE grant_id=$1 AND event_type='proposal_version_saved' AND visibility='private'").bind(&id).fetch_one(reopened.pool()).await.unwrap();
    assert_eq!(audit, 2);
}
