//! Intent detection and parsing for Voice-to-X3

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{VoiceError, VoiceResult};

/// Detected user intent from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Type of contract to generate
    pub contract_type: ContractType,
    /// Contract name
    pub name: String,
    /// Parameters extracted from description
    pub params: HashMap<String, ParamValue>,
    /// Raw user input
    pub raw_input: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

/// Types of contracts that can be generated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractType {
    /// ERC20-like token
    Token,
    /// NFT collection
    NFT,
    /// DEX / AMM
    DEX,
    /// Governance / DAO
    Governance,
    /// Vault / Staking
    Vault,
    /// Cross-chain bridge
    Bridge,
    /// Lending protocol
    Lending,
    /// Oracle
    Oracle,
    /// Multi-sig wallet
    MultiSig,
    /// Custom contract
    Custom,
}

/// Parameter value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamValue {
    String(String),
    Number(u128),
    Boolean(bool),
    Address(String),
    List(Vec<ParamValue>),
}

impl ParamValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ParamValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<u128> {
        match self {
            ParamValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParamValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Intent parser using keyword matching and patterns
pub struct IntentParser {
    patterns: Vec<IntentPattern>,
}

impl IntentParser {
    pub fn new() -> Self {
        Self {
            patterns: Self::default_patterns(),
        }
    }

    /// Parse natural language input into intent
    pub fn parse(&self, input: &str) -> VoiceResult<Intent> {
        let input_lower = input.to_lowercase();

        // Find best matching pattern
        let mut best_match: Option<(ContractType, f64)> = None;

        for pattern in &self.patterns {
            let score = pattern.score(&input_lower);
            if score > MATCH_THRESHOLD && best_match.is_none_or(|(_, best)| score > best) {
                best_match = Some((pattern.contract_type, score));
            }
        }

        let (contract_type, confidence) = best_match.ok_or_else(|| {
            VoiceError::IntentParseError("Could not determine contract type".to_string())
        })?;

        // Extract parameters
        let params = self.extract_params(&input_lower, contract_type);

        // Extract the name from the *original* text: `input_lower` has already
        // destroyed the casing, so "called MyToken" came back as "Mytoken" and
        // the generated contract no longer contained the name the user wrote.
        let name = self
            .extract_name(input)
            .unwrap_or_else(|| format!("My{:?}", contract_type));

        Ok(Intent {
            contract_type,
            name,
            params,
            raw_input: input.to_string(),
            confidence,
        })
    }

    /// Extract parameters from input based on contract type
    fn extract_params(
        &self,
        input: &str,
        contract_type: ContractType,
    ) -> HashMap<String, ParamValue> {
        let mut params = HashMap::new();

        match contract_type {
            ContractType::Token => {
                // Extract supply
                if let Some(supply) = self.extract_number(input, &["supply", "tokens", "total"]) {
                    params.insert("total_supply".to_string(), ParamValue::Number(supply));
                }
                // Extract decimals
                if let Some(decimals) = self.extract_number(input, &["decimals", "decimal"]) {
                    params.insert("decimals".to_string(), ParamValue::Number(decimals));
                } else {
                    params.insert("decimals".to_string(), ParamValue::Number(18));
                }
                // Extract symbol
                if let Some(symbol) = self.extract_symbol(input) {
                    params.insert("symbol".to_string(), ParamValue::String(symbol));
                }
                // Mintable
                let mintable = input.contains("mint") || input.contains("mintable");
                params.insert("mintable".to_string(), ParamValue::Boolean(mintable));
                // Burnable
                let burnable = input.contains("burn") || input.contains("burnable");
                params.insert("burnable".to_string(), ParamValue::Boolean(burnable));
            }
            ContractType::NFT => {
                // Extract max supply
                if let Some(supply) =
                    self.extract_number(input, &["collection", "nfts", "items", "max"])
                {
                    params.insert("max_supply".to_string(), ParamValue::Number(supply));
                }
                // Extract royalty
                if let Some(royalty) = self.extract_number(input, &["royalty", "royalties", "%"]) {
                    params.insert("royalty_percent".to_string(), ParamValue::Number(royalty));
                }
            }
            ContractType::DEX => {
                // Extract fee
                if let Some(fee) = self.extract_number(input, &["fee", "fees", "%"]) {
                    params.insert("fee_percent".to_string(), ParamValue::Number(fee));
                } else {
                    params.insert("fee_percent".to_string(), ParamValue::Number(30));
                    // 0.3%
                }
            }
            ContractType::Vault => {
                // Extract APY
                if let Some(apy) = self.extract_number(input, &["apy", "yield", "return", "%"]) {
                    params.insert("target_apy".to_string(), ParamValue::Number(apy));
                }
                // Extract lock period
                if let Some(lock) =
                    self.extract_number(input, &["lock", "locked", "days", "period"])
                {
                    params.insert("lock_days".to_string(), ParamValue::Number(lock));
                }
            }
            _ => {}
        }

        params
    }

    /// Extract a number following certain keywords
    fn extract_number(&self, input: &str, keywords: &[&str]) -> Option<u128> {
        for keyword in keywords {
            if let Some(pos) = input.find(keyword) {
                // Look for numbers after the keyword
                let after = &input[pos..];
                for word in after.split_whitespace() {
                    // Remove common suffixes
                    let clean = word
                        .replace(",", "")
                        .replace("k", "000")
                        .replace("m", "000000")
                        .replace("b", "000000000");

                    if let Ok(num) = clean.parse::<u128>() {
                        return Some(num);
                    }
                }
            }
        }
        None
    }

    /// Extract contract name from input
    fn extract_name(&self, input: &str) -> Option<String> {
        // Look for patterns like "called X", "named X", "X token".
        // Matched case-insensitively so it works on sentences that start with
        // "Called X"; the returned name keeps the casing from the input, and
        // `to_pascal_case` only fixes word boundaries.
        let patterns = ["called ", "named ", "name ", "Called ", "Named ", "Name "];

        for pattern in patterns {
            if let Some(pos) = input.find(pattern) {
                let after = &input[pos + pattern.len()..];
                if let Some(name) = after.split_whitespace().next() {
                    return Some(to_pascal_case(name));
                }
            }
        }

        None
    }

    /// Extract token symbol
    fn extract_symbol(&self, input: &str) -> Option<String> {
        // Look for patterns like "symbol X", "$X", "(X)"
        if let Some(pos) = input.find("symbol ") {
            let after = &input[pos + 7..];
            if let Some(symbol) = after.split_whitespace().next() {
                return Some(symbol.to_uppercase());
            }
        }

        // Look for $SYMBOL pattern
        for word in input.split_whitespace() {
            if word.starts_with('$') && word.len() > 1 {
                return Some(word[1..].to_uppercase());
            }
        }

        None
    }

    fn default_patterns() -> Vec<IntentPattern> {
        vec![
            IntentPattern {
                contract_type: ContractType::Token,
                keywords: vec!["token", "erc20", "coin", "currency", "fungible"],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::NFT,
                keywords: vec![
                    "nft",
                    "erc721",
                    "collectible",
                    "collection",
                    "artwork",
                    "non-fungible",
                ],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::DEX,
                keywords: vec![
                    "dex",
                    "exchange",
                    "swap",
                    "amm",
                    "liquidity pool",
                    "trading",
                ],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::Governance,
                keywords: vec!["governance", "dao", "voting", "proposal", "democracy"],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::Vault,
                keywords: vec!["vault", "staking", "stake", "yield", "farm", "deposit"],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::Bridge,
                keywords: vec!["bridge", "cross-chain", "crosschain", "transfer"],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::Lending,
                keywords: vec!["lending", "borrow", "loan", "collateral", "interest"],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::Oracle,
                keywords: vec!["oracle", "price feed", "data feed", "external data"],
                weight: 1.0,
            },
            IntentPattern {
                contract_type: ContractType::MultiSig,
                keywords: vec![
                    "multisig",
                    "multi-sig",
                    "multiple signatures",
                    "shared wallet",
                ],
                weight: 1.0,
            },
        ]
    }
}

impl Default for IntentParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern for matching contract types
struct IntentPattern {
    contract_type: ContractType,
    keywords: Vec<&'static str>,
    weight: f64,
}

impl IntentPattern {
    /// Score how well input matches this pattern
    ///
    /// The previous version divided the number of matched keywords by the
    /// pattern's *total* keyword list, so a pattern with many synonyms needed
    /// several of them in one sentence: `Token` lists five keywords, and
    /// "Create a token called MyToken" scored 1/5 = 0.2 against a 0.5
    /// threshold — no natural phrasing could ever name a contract type.
    /// Confidence now depends on how much was matched, not on how many synonyms
    /// the pattern happens to list.
    fn score(&self, input: &str) -> f64 {
        let matched: Vec<&str> = self
            .keywords
            .iter()
            .filter(|keyword| input.contains(*keyword))
            .copied()
            .collect();
        if matched.is_empty() {
            return 0.0;
        }

        // A long phrase ("liquidity pool", "governance") is stronger evidence
        // than a short word ("coin"), and every extra distinct keyword is
        // stronger still.
        let longest = matched
            .iter()
            .map(|keyword| keyword.chars().count())
            .max()
            .unwrap_or(0) as f64;
        let base = (longest / STRONG_KEYWORD_CHARS).min(1.0);
        let extra = EXTRA_KEYWORD_BONUS * matched.len().saturating_sub(1) as f64;
        (self.weight * (base + extra)).min(1.0)
    }
}

/// A keyword at least this long is strong evidence on its own ("collectible",
/// "governance", "fungible").
const STRONG_KEYWORD_CHARS: f64 = 8.0;
/// Each additional distinct keyword beyond the first adds this much confidence.
const EXTRA_KEYWORD_BONUS: f64 = 0.25;
/// Minimum confidence for a pattern to name the contract type.
const MATCH_THRESHOLD: f64 = 0.5;

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_intent() {
        let parser = IntentParser::new();
        let result = parser.parse("Create a token called MyToken with 1 million supply");

        assert!(result.is_ok());
        let intent = result.unwrap();
        assert_eq!(intent.contract_type, ContractType::Token);
        assert_eq!(intent.name, "MyToken");
    }

    #[test]
    fn test_parse_nft_intent() {
        let parser = IntentParser::new();
        let result = parser.parse("Make an NFT collection with 10000 items and 5% royalty");

        assert!(result.is_ok());
        let intent = result.unwrap();
        assert_eq!(intent.contract_type, ContractType::NFT);
    }
}
