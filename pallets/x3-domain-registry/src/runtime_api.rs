//! Runtime API definitions for the X3 Domain Registry pallet.
//!
//! These APIs allow off-chain consumers (node RPC, DNS server) to query the
//! canonical on-chain `.x3` zone data without submitting transactions.

use codec::{Codec, Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Minimal record types needed to serve `.x3` endpoints.
///
/// Values are aligned to IANA DNS RR TYPE codes for easy interoperability:
/// - A = 1
/// - CNAME = 5
/// - TXT = 16
/// - AAAA = 28
#[derive(Clone, Copy, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum X3RecordType {
    A,
    Cname,
    Txt,
    Aaaa,
}

impl X3RecordType {
    pub fn to_iana_code(self) -> u16 {
        match self {
            X3RecordType::A => 1,
            X3RecordType::Cname => 5,
            X3RecordType::Txt => 16,
            X3RecordType::Aaaa => 28,
        }
    }
}

/// A single DNS record entry for runtime API responses.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct X3DnsRecordResponse {
    /// IANA RR type code (e.g. 1, 5, 16, 28).
    pub rr_type: u16,
    /// TTL in seconds.
    pub ttl: u32,
    /// Record data.
    ///
    /// Encoding:
    /// - A: 4 raw bytes
    /// - AAAA: 16 raw bytes
    /// - CNAME/TXT: UTF-8 bytes
    pub data: Vec<u8>,
}

/// Domain snapshot for runtime API responses.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct X3DomainResponse<AccountId> {
    pub domain: Vec<u8>,
    pub owner: AccountId,
    pub records: Vec<X3DnsRecordResponse>,
}

sp_api::decl_runtime_apis! {
    /// Runtime API for querying on-chain `.x3` domain records.
    pub trait X3DomainRegistryApi<AccountId>
    where
        AccountId: Codec,
    {
        /// Get all records for a domain.
        fn get_records(domain: Vec<u8>) -> Vec<X3DnsRecordResponse>;

        /// Get full domain snapshot (owner + records).
        fn get_domain(domain: Vec<u8>) -> Option<X3DomainResponse<AccountId>>;

        /// List all registered domains.
        fn list_domains() -> Vec<Vec<u8>>;
    }
}
