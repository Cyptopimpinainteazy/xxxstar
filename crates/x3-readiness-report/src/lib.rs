//! X3 Atomic Star V0.4 Readiness Report Generator
//!
//! This crate provides infrastructure for gathering and reporting on the production
//! readiness status of the X3 Atomic Star v0.4 kernel and related systems.

pub mod collector;
pub mod formatter;
pub mod types;

pub use collector::Collector;
pub use formatter::{JsonFormatter, TextFormatter};
pub use types::ReadinessReport;

#[cfg(test)]
mod tests;
