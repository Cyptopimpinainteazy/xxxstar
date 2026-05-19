//! ABI generator: produces JSON ABI descriptors from X3 function declarations.
//!
//! The ABI describes the external interface of a compiled X3 contract — function
//! names, parameter types, return types, and visibility. Used by SDKs and tools
//! to encode/decode calls without access to source code.

use serde::{Deserialize, Serialize};

/// ABI type descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AbiType {
    I64,
    U64,
    Bool,
    Str,
    Bytes,
    Unit,
    Named(String),
}

/// ABI parameter descriptor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbiParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: AbiType,
}

/// ABI function descriptor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbiFn {
    pub name: String,
    pub is_public: bool,
    pub params: Vec<AbiParam>,
    pub returns: AbiType,
    /// Function selector: first 4 bytes of keccak256(signature), hex-encoded.
    pub selector: String,
}

/// Complete ABI for a compiled contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractAbi {
    pub contract_name: String,
    pub version: String,
    pub functions: Vec<AbiFn>,
}

impl ContractAbi {
    /// Serialize the ABI to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize an ABI from a JSON string.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

/// Build the ABI type from an X3 parser TypeExpr string name.
pub fn abi_type_from_str(s: &str) -> AbiType {
    match s {
        "i64" | "I64" => AbiType::I64,
        "u64" | "U64" => AbiType::U64,
        "bool" | "Bool" => AbiType::Bool,
        "str" | "Str" => AbiType::Str,
        "()" | "Unit" | "unit" => AbiType::Unit,
        other => AbiType::Named(other.to_string()),
    }
}

/// Compute a 4-byte function selector from a function signature string.
/// Format: "name(type1,type2,...)"
pub fn compute_selector(signature: &str) -> String {
    // Simple deterministic hash: djb2 truncated to 4 bytes
    let hash = djb2_hash(signature.as_bytes());
    format!("{:08x}", hash & 0xFFFF_FFFF)
}

fn djb2_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 5381;
    for &b in data {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// Build a function signature string from name and parameter types.
pub fn build_signature(name: &str, params: &[AbiParam]) -> String {
    let types: Vec<_> = params.iter().map(|p| format!("{:?}", p.ty)).collect();
    format!("{}({})", name, types.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_abi() -> ContractAbi {
        ContractAbi {
            contract_name: "MyToken".into(),
            version: "1.0.0".into(),
            functions: vec![
                AbiFn {
                    name: "transfer".into(),
                    is_public: true,
                    params: vec![
                        AbiParam { name: "to".into(), ty: AbiType::Str },
                        AbiParam { name: "amount".into(), ty: AbiType::U64 },
                    ],
                    returns: AbiType::Bool,
                    selector: compute_selector("transfer(str,u64)"),
                },
                AbiFn {
                    name: "balance".into(),
                    is_public: true,
                    params: vec![AbiParam { name: "account".into(), ty: AbiType::Str }],
                    returns: AbiType::U64,
                    selector: compute_selector("balance(str)"),
                },
            ],
        }
    }

    #[test]
    fn test_serialize_and_deserialize() {
        let abi = sample_abi();
        let json = abi.to_json().unwrap();
        let recovered = ContractAbi::from_json(&json).unwrap();
        assert_eq!(recovered.contract_name, "MyToken");
        assert_eq!(recovered.functions.len(), 2);
    }

    #[test]
    fn test_abi_type_from_str() {
        assert_eq!(abi_type_from_str("i64"), AbiType::I64);
        assert_eq!(abi_type_from_str("bool"), AbiType::Bool);
        assert_eq!(abi_type_from_str("Unit"), AbiType::Unit);
        assert_eq!(abi_type_from_str("MyStruct"), AbiType::Named("MyStruct".into()));
    }

    #[test]
    fn test_selector_is_deterministic() {
        let s1 = compute_selector("transfer(str,u64)");
        let s2 = compute_selector("transfer(str,u64)");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_different_signatures_produce_different_selectors() {
        let s1 = compute_selector("transfer(str,u64)");
        let s2 = compute_selector("balance(str)");
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_build_signature() {
        let params = vec![
            AbiParam { name: "a".into(), ty: AbiType::I64 },
            AbiParam { name: "b".into(), ty: AbiType::Bool },
        ];
        let sig = build_signature("foo", &params);
        assert!(sig.starts_with("foo("));
        assert!(sig.contains("I64"));
    }

    #[test]
    fn test_json_round_trip_preserves_selector() {
        let abi = sample_abi();
        let json = abi.to_json().unwrap();
        let recovered = ContractAbi::from_json(&json).unwrap();
        assert_eq!(
            recovered.functions[0].selector,
            compute_selector("transfer(str,u64)")
        );
    }

    #[test]
    fn test_empty_contract_abi() {
        let abi = ContractAbi {
            contract_name: "Empty".into(),
            version: "0.1.0".into(),
            functions: vec![],
        };
        let json = abi.to_json().unwrap();
        let recovered = ContractAbi::from_json(&json).unwrap();
        assert!(recovered.functions.is_empty());
    }
}
