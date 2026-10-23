//! Route shapes used across the crate.
//!
//! `RouteFinder` and `RouteConstraints` used to be declared here as empty structs
//! with no methods: a name that looked like a multi-hop router and could not route
//! anything. Route search now lives in `quote_engine` (`NativePool` for one pool,
//! `MultiHopPath` for a chain of them), and the pallet-level graph search in
//! `crates/x3-dex/src/route_finder.rs`. Nothing implemented the old stubs, so
//! they are gone rather than left as a second, hollow entry point.

use crate::mev_protection::{Hop, Route};

pub type SwapRoute = Route;
pub type HopInfo = Hop;
