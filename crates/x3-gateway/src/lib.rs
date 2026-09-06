//! x3-gateway: REST and GraphQL API gateway for X3 Chain indexed data.
//!
//! The package is a binary-first crate. `lib.rs` exists so the real server
//! wiring in [`crate::rest`] and its data layers are compiled and testable
//! as a unit, and so the thin binary entry point in `main.rs` can delegate to
//! the same setup functions exercised by the test-suite.
//!
//! Layout:
//! - [`config`] validated process configuration (listen addr, optional DB,
//!   optional Redis, optional orchestra control-plane).
//! - [`db`] Postgres models + queries/migrations.
//! - [`cache`] optional Redis response cache.
//! - [`error`] the shared gateway error type.
//! - [`rest`] the axum router and every REST + GraphQL HTTP handler.
//! - [`graphql`] the executable GraphQL schema.
//! - [`orchestra`] workflow coordination between the gateway DB and the
//!   (optional) orchestra control-plane.

pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod graphql;
pub mod orchestra;
pub mod rest;
