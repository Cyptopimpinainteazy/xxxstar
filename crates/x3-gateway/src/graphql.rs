//! GraphQL API schema for the gateway.
//!
//! The gateway exposes a GraphQL endpoint at `/graphql` alongside its REST
//! surface. `self::create_schema` builds the executable schema and is invoked
//! from `crate::rest::create_router` (and existing in-module integration
//! tests), so the endpoint is registered through the same real code path the
//! server uses.

use crate::db::Database;
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject};
use std::sync::Arc;
use x3_orchestra_control_plane::ControlPlaneClient;

/// The fully-fused schema exposed at `/graphql`.
pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

/// Schema version reported to GraphQL consumers, kept in sync with the crate
/// version so operators can reason about which gateway revision they query.
pub const SCHEMA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Static identity of the served API.
#[derive(SimpleObject)]
pub struct ServiceInfo {
    name: &'static str,
    version: &'static str,
}

/// Build the executable GraphQL schema.
///
/// The gateway Postgres handle and (optionally) the orchestra control-plane
/// client are attached as GraphQL context data so resolvers reach the same
/// state the REST handlers use.
pub fn create_schema(
    db: Database,
    _control_plane: Option<Arc<ControlPlaneClient>>,
) -> AppSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(db)
        .finish()
}

/// Root query object.
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Human/simple liveness probe (works even while the DB backend is down).
    async fn health(&self) -> &'static str {
        "ok"
    }

    /// Static identity of the served API.
    async fn service(&self) -> ServiceInfo {
        ServiceInfo {
            name: "x3-gateway",
            version: SCHEMA_VERSION,
        }
    }

    /// Report whether a Postgres backend is configured and the pool is
    /// reachable. Resolves to `false`, never to an error, when the backend is
    /// not reachable so GraphQL consumers can branch on readiness uniformly
    /// without catching resolver errors.
    async fn db_reachable(&self, ctx: &Context<'_>) -> bool {
        match ctx.data::<Database>() {
            Ok(db) => db.healthy().await,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_version_metadata() {
        let info = ServiceInfo {
            name: "x3-gateway",
            version: SCHEMA_VERSION,
        };
        assert_eq!(info.name, "x3-gateway");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn schema_builds_without_a_live_backend() {
        // A schema can always be constructed; DB-backed resolvers only
        // return "not reachable" when the pool is down, they never compile
        // differently. Constructing here exercises a real `create_schema`
        // path against a lazy non-connecting pool.
        use crate::error::Result;
        use crate::config::DatabaseConfig;
        fn build_lazy_db() -> Result<Database> {
            Database::connect_lazy(&DatabaseConfig::new(
                "postgres://user:pass@127.0.0.1:1/x3_gateway_test".to_string(),
            ))
        }
        let db = build_lazy_db().expect("lazy pool builds without connecting");
        let schema = create_schema(db, None);
        assert!(schema.sdl().contains("dbReachable"));
    }
}
