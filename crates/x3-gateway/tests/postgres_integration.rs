use x3_gateway::{
    config::DatabaseConfig,
    db::{Database, NewFundingSwarmGrant},
};

const TEST_DATABASE_URL_ENV: &str = "X3_GATEWAY_TEST_DATABASE_URL";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a real Postgres instance; run via scripts/gateway-postgres-integration.sh"]
async fn migrations_and_funding_swarm_roundtrip_against_postgres() {
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

    assert!(
        before.total_grants >= 3,
        "migration seed data should create the three demo grants"
    );

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
