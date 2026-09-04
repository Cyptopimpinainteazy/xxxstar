use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct IntentDraft {
    pub id: String,
    pub parties: Vec<String>,
    pub assets: Vec<AssetRequirement>,
    pub fee_caps: Vec<FeeCap>,
    pub expiry_timestamp_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AssetRequirement {
    pub chain: ChainType,
    pub chain_id: String,
    pub token_address: String,
    pub amount: String, // Stringified U256
    pub slippage_basis_points: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ChainType {
    EVM,
    SVM,
    BTC,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FeeCap {
    pub chain: String,
    pub max_fee: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attestation {
    pub intent_id: String,
    pub expiry: u64,
    pub risk_flags: Vec<String>,
    // The verifier signature for the intent preimage
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifiedIntent {
    pub intent: IntentDraft,
    pub attestation: Attestation,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignerCaps {
    pub chains: Vec<String>,
    pub max_tx_value: String,
    pub requires_hardware: bool,
}
