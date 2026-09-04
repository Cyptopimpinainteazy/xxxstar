pub mod audit;
pub mod client;
pub mod error;
pub mod hsm;
pub mod service;
/// Custody Service: Enterprise-grade vault operations with HSM, audit trail, and service boundaries
///
/// The custody service is a dedicated, isolated microservice responsible for:
/// - Managing vault operations (fund, sweep, reserve, release, transfer)
/// - Key lifecycle and HSM integration
/// - Operation tracking and audit trails
/// - Authorization and policy enforcement
/// - Cryptographic proofs for settlement
pub mod types;

pub use client::CustodyServiceClient;
pub use error::{CustodyError, Result};
pub use service::{CustodyService, CustodyServiceImpl};
pub use types::*;
