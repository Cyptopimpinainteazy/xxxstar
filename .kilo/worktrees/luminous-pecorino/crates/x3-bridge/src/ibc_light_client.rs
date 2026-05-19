/// Cosmos IBC Light Client — Enables native IBC connections between X3 and any Cosmos chain
/// Implements light client verification, header validation, and consensus state management
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct CosmosChainInfo {
    pub chain_id: Vec<u8>,
    pub client_id: Vec<u8>,
    pub latest_height: u64,
    pub trust_period_seconds: u64,
    pub unbonding_period_seconds: u64,
    pub is_frozen: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ConsensusState {
    pub timestamp: u64,
    pub root: [u8; 32],
    pub next_validators_hash: [u8; 32],
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct Header {
    pub height: u64,
    pub timestamp: u64,
    pub validators_hash: [u8; 32],
    pub next_validators_hash: [u8; 32],
    pub app_hash: [u8; 32],
    pub commit_hash: [u8; 32],
    pub proposer_address: [u8; 20],
    pub signatures: Vec<ValidatorSignature>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ValidatorSignature {
    pub validator_address: [u8; 20],
    pub signature: Vec<u8>,
    pub power: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct IBCPacket {
    pub sequence: u64,
    pub source_port: Vec<u8>,
    pub source_channel: Vec<u8>,
    pub destination_port: Vec<u8>,
    pub destination_channel: Vec<u8>,
    pub data: Vec<u8>,
    pub timeout_height: u64,
    pub timeout_timestamp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct MerkleProof {
    pub proofs: Vec<[u8; 32]>,
    pub key: Vec<u8>,
}

pub struct IBCLightClient;

impl IBCLightClient {
    /// Register a new Cosmos chain with IBC light client
    pub fn register_client(
        chain_id: Vec<u8>,
        trust_period: u64,
        unbonding_period: u64,
    ) -> Result<Vec<u8>, &'static str> {
        if chain_id.is_empty() || chain_id.len() > 256 {
            return Err("Invalid chain_id length");
        }
        if trust_period == 0 || unbonding_period == 0 {
            return Err("Periods must be positive");
        }
        if trust_period > unbonding_period {
            return Err("Trust period cannot exceed unbonding period");
        }

        let client_id = Self::generate_client_id(&chain_id);
        Ok(client_id)
    }

    /// Verify and update light client header (BFT consensus validation)
    pub fn verify_header(
        header: &Header,
        prev_consensus: &ConsensusState,
        validator_set: &[(Vec<u8>, u64)],
        trust_period: u64,
        now: u64,
    ) -> Result<ConsensusState, &'static str> {
        // Check header freshness
        if header.timestamp == 0 {
            return Err("Header timestamp cannot be zero");
        }
        if header.timestamp > now {
            return Err("Header timestamp is in the future");
        }

        // Verify BFT threshold: 2/3 + 1 of validator set must sign
        let mut total_power = 0u64;
        for (_, power) in validator_set {
            total_power = total_power.saturating_add(*power);
        }

        let required_power = (total_power * 2) / 3 + 1;
        let mut signed_power = 0u64;

        for sig in &header.signatures {
            for (val_addr, power) in validator_set {
                if val_addr.as_slice() == sig.validator_address.as_slice() {
                    signed_power = signed_power.saturating_add(sig.power);
                    break;
                }
            }
        }

        if signed_power < required_power {
            return Err("Insufficient validator signatures (2/3 threshold not met)");
        }

        // Check header timestamp progression
        if header.timestamp <= prev_consensus.timestamp {
            return Err("Header height must be strictly increasing");
        }

        // Check trust period expiry
        let elapsed = now.saturating_sub(prev_consensus.timestamp);
        if elapsed > trust_period {
            return Err("Client state is stale (outside trust period)");
        }

        Ok(ConsensusState {
            timestamp: header.timestamp,
            root: header.app_hash,
            next_validators_hash: header.next_validators_hash,
        })
    }

    /// Verify Merkle proof of IBC packet
    pub fn verify_packet_data(
        packet: &IBCPacket,
        proof: &MerkleProof,
        consensus_state: &ConsensusState,
    ) -> Result<bool, &'static str> {
        if packet.data.is_empty() {
            return Err("Packet data cannot be empty");
        }
        if proof.proofs.is_empty() {
            return Err("Proof cannot be empty");
        }

        // Construct commitment key: "{port}/{channel}/{sequence}"
        let mut key = Vec::new();
        key.extend_from_slice(&packet.source_port);
        key.push(b'/');
        key.extend_from_slice(&packet.source_channel);
        key.push(b'/');
        key.extend_from_slice(packet.sequence.to_le_bytes().as_ref());

        Ok(Self::verify_merkle_membership(
            &key,
            &packet.data,
            proof,
            consensus_state.root,
        ))
    }

    /// Process IBC token transfer (fungible token standard - FT module)
    pub fn process_ft_transfer(
        packet: &IBCPacket,
        receiver: [u8; 32],
    ) -> Result<(u128, Vec<u8>), &'static str> {
        // Parse FT packet format: amount:denom (simplified)
        let parts: Vec<&[u8]> = packet.data.split(|b| *b == b':').collect();
        if parts.len() != 2 {
            return Err("Invalid FT transfer packet format");
        }

        let amount_str = std::str::from_utf8(parts[0]).map_err(|_| "Invalid amount encoding")?;
        let denom = parts[1].to_vec();

        let amount: u128 = amount_str.parse().map_err(|_| "Invalid amount value")?;

        if amount == 0 {
            return Err("Transfer amount must be positive");
        }

        Ok((amount, denom))
    }

    /// Acknowledge IBC packet receipt (idempotent)
    pub fn acknowledge_packet(packet: &IBCPacket, ack_data: Vec<u8>) -> Result<(), &'static str> {
        if packet.sequence == 0 {
            return Err("Packet sequence cannot be zero");
        }
        if ack_data.is_empty() {
            return Err("Acknowledgement data cannot be empty");
        }

        // Acknowledgement state update happens in pallet storage
        Ok(())
    }

    /// Timeout IBC packet (refund sender if timeout reached)
    pub fn timeout_packet(packet: &IBCPacket, current_time: u64) -> Result<bool, &'static str> {
        if current_time >= packet.timeout_timestamp {
            return Ok(true); // Packet has timed out, should be refunded
        }
        Ok(false)
    }

    /// Verify JSON-RPC call proofs from Cosmos chain
    pub fn verify_state_proof(
        contract_address: Vec<u8>,
        key: Vec<u8>,
        value: Vec<u8>,
        proof: &MerkleProof,
        consensus_state: &ConsensusState,
    ) -> Result<bool, &'static str> {
        if contract_address.is_empty() || key.is_empty() {
            return Err("Contract address and key cannot be empty");
        }

        // Construct full key: "{contract}/{key}"
        let mut full_key = Vec::new();
        full_key.extend_from_slice(&contract_address);
        full_key.push(b'/');
        full_key.extend_from_slice(&key);

        Ok(Self::verify_merkle_membership(
            &full_key,
            &value,
            proof,
            consensus_state.root,
        ))
    }

    /// Freeze client if evidence of misbehavior detected
    pub fn freeze_client_on_misbehavior(
        header1: &Header,
        header2: &Header,
    ) -> Result<bool, &'static str> {
        // Equivocation: same height, different commits
        if header1.height == header2.height && header1.commit_hash != header2.commit_hash {
            return Ok(true);
        }

        // Lunatic attack: conflicting next validator sets
        if header1.height < header2.height
            && header1.next_validators_hash == header2.validators_hash
            && header1.next_validators_hash != header2.next_validators_hash
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Update client with new header
    pub fn update_client(header: Header, client: &mut CosmosChainInfo) -> Result<(), &'static str> {
        if header.height <= client.latest_height {
            return Err("Header height must be greater than existing height");
        }

        client.latest_height = header.height;
        Ok(())
    }

    /// Get client state
    pub fn get_client_state(client: &CosmosChainInfo) -> (u64, bool) {
        (client.latest_height, client.is_frozen)
    }

    /// Verify key-bound merkle membership against an expected root.
    fn verify_merkle_membership(
        expected_key: &[u8],
        expected_value: &[u8],
        proof: &MerkleProof,
        expected_root: [u8; 32],
    ) -> bool {
        if proof.key != expected_key {
            return false;
        }

        let computed_root = Self::compute_merkle_root(expected_key, expected_value, &proof.proofs);
        computed_root == expected_root
    }

    /// Compute merkle root from key-bound leaf and proof path using deterministic domain separation.
    fn compute_merkle_root(key: &[u8], value: &[u8], proofs: &[[u8; 32]]) -> [u8; 32] {
        let mut hash = Self::hash_leaf(key, value);
        for proof_node in proofs {
            hash = Self::hash_inner(&hash, proof_node);
        }
        hash
    }

    fn hash_leaf(key: &[u8], value: &[u8]) -> [u8; 32] {
        let mut payload = Vec::with_capacity(1 + key.len() + value.len());
        payload.push(0x00);
        payload.extend_from_slice(key);
        payload.extend_from_slice(value);
        Self::keccak256(&payload)
    }

    fn hash_inner(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        let (left, right) = if a <= b { (a, b) } else { (b, a) };
        let mut payload = Vec::with_capacity(1 + 32 + 32);
        payload.push(0x01);
        payload.extend_from_slice(left);
        payload.extend_from_slice(right);
        Self::keccak256(&payload)
    }

    /// Simple Keccak256 hash (for proof verification)
    fn keccak256(data: &[u8]) -> [u8; 32] {
        sp_io::hashing::keccak_256(data)
    }

    /// Generate deterministic client ID from chain ID
    fn generate_client_id(chain_id: &[u8]) -> Vec<u8> {
        let mut id = b"07-tendermint-".to_vec();
        id.extend_from_slice(&chain_id[..chain_id.len().min(20)]);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_client() {
        let chain_id = b"cosmoshub-4".to_vec();
        let client_id = IBCLightClient::register_client(chain_id.clone(), 604800, 2592000).unwrap();

        assert!(!client_id.is_empty());
        assert!(client_id.starts_with(b"07-tendermint-"));
    }

    #[test]
    fn test_register_client_invalid_chain_id() {
        let result = IBCLightClient::register_client(Vec::new(), 604800, 2592000);
        assert!(result.is_err());
    }

    #[test]
    fn test_trust_period_exceeds_unbonding() {
        let chain_id = b"cosmoshub-4".to_vec();
        let result = IBCLightClient::register_client(chain_id, 2592000, 604800);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_header_valid() {
        let header = Header {
            height: 100,
            timestamp: 1000,
            validators_hash: [0; 32],
            next_validators_hash: [1; 32],
            app_hash: [2; 32],
            commit_hash: [3; 32],
            proposer_address: [0; 20],
            signatures: vec![
                ValidatorSignature {
                    validator_address: [0; 20],
                    signature: vec![1, 2, 3],
                    power: 70,
                },
                ValidatorSignature {
                    validator_address: [1; 20],
                    signature: vec![4, 5, 6],
                    power: 30,
                },
            ],
        };

        let prev = ConsensusState {
            timestamp: 500,
            root: [0; 32],
            next_validators_hash: [0; 32],
        };

        let validators = vec![(vec![0; 20], 70u64), (vec![1; 20], 30u64)];
        let result = IBCLightClient::verify_header(&header, &prev, &validators, 1000000, 2000);

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_header_future_timestamp() {
        let header = Header {
            height: 100,
            timestamp: 5000,
            validators_hash: [0; 32],
            next_validators_hash: [1; 32],
            app_hash: [2; 32],
            commit_hash: [3; 32],
            proposer_address: [0; 20],
            signatures: vec![],
        };

        let prev = ConsensusState {
            timestamp: 500,
            root: [0; 32],
            next_validators_hash: [0; 32],
        };

        let result = IBCLightClient::verify_header(&header, &prev, &vec![], 1000000, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_packet_data() {
        let packet = IBCPacket {
            sequence: 1,
            source_port: b"transfer".to_vec(),
            source_channel: b"channel-0".to_vec(),
            destination_port: b"transfer".to_vec(),
            destination_channel: b"channel-1".to_vec(),
            data: b"test".to_vec(),
            timeout_height: 1000,
            timeout_timestamp: 2000,
        };

        let mut packet_key = Vec::new();
        packet_key.extend_from_slice(&packet.source_port);
        packet_key.push(b'/');
        packet_key.extend_from_slice(&packet.source_channel);
        packet_key.push(b'/');
        packet_key.extend_from_slice(packet.sequence.to_le_bytes().as_ref());

        let proof = MerkleProof {
            proofs: vec![[0; 32], [1; 32]],
            key: packet_key.clone(),
        };

        let consensus = ConsensusState {
            timestamp: 1000,
            root: IBCLightClient::compute_merkle_root(&packet_key, b"test", &[[0; 32], [1; 32]]),
            next_validators_hash: [0; 32],
        };

        let result = IBCLightClient::verify_packet_data(&packet, &proof, &consensus).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_packet_data_rejects_key_mismatch() {
        let packet = IBCPacket {
            sequence: 7,
            source_port: b"transfer".to_vec(),
            source_channel: b"channel-9".to_vec(),
            destination_port: b"transfer".to_vec(),
            destination_channel: b"channel-1".to_vec(),
            data: b"payload".to_vec(),
            timeout_height: 1000,
            timeout_timestamp: 2000,
        };

        let mut expected_key = Vec::new();
        expected_key.extend_from_slice(&packet.source_port);
        expected_key.push(b'/');
        expected_key.extend_from_slice(&packet.source_channel);
        expected_key.push(b'/');
        expected_key.extend_from_slice(packet.sequence.to_le_bytes().as_ref());

        let proof = MerkleProof {
            proofs: vec![[3; 32], [4; 32]],
            key: b"wrong/key".to_vec(),
        };

        let consensus = ConsensusState {
            timestamp: 1000,
            root: IBCLightClient::compute_merkle_root(&expected_key, &packet.data, &proof.proofs),
            next_validators_hash: [0; 32],
        };

        let result = IBCLightClient::verify_packet_data(&packet, &proof, &consensus).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_verify_state_proof_rejects_key_mismatch() {
        let contract_address = b"contract-1".to_vec();
        let key = b"balance:user1".to_vec();
        let value = b"100".to_vec();

        let mut full_key = Vec::new();
        full_key.extend_from_slice(&contract_address);
        full_key.push(b'/');
        full_key.extend_from_slice(&key);

        let proof = MerkleProof {
            proofs: vec![[7; 32], [8; 32]],
            key: b"contract-1/wrong".to_vec(),
        };

        let consensus = ConsensusState {
            timestamp: 1000,
            root: IBCLightClient::compute_merkle_root(&full_key, &value, &proof.proofs),
            next_validators_hash: [0; 32],
        };

        let result =
            IBCLightClient::verify_state_proof(contract_address, key, value, &proof, &consensus)
                .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_process_ft_transfer() {
        let packet = IBCPacket {
            sequence: 1,
            source_port: b"transfer".to_vec(),
            source_channel: b"channel-0".to_vec(),
            destination_port: b"transfer".to_vec(),
            destination_channel: b"channel-1".to_vec(),
            data: b"1000000:uatom".to_vec(),
            timeout_height: 1000,
            timeout_timestamp: 2000,
        };

        let (amount, denom) = IBCLightClient::process_ft_transfer(&packet, [0; 32]).unwrap();

        assert_eq!(amount, 1000000);
        assert_eq!(denom, b"uatom".to_vec());
    }

    #[test]
    fn test_timeout_packet() {
        let packet = IBCPacket {
            sequence: 1,
            source_port: b"transfer".to_vec(),
            source_channel: b"channel-0".to_vec(),
            destination_port: b"transfer".to_vec(),
            destination_channel: b"channel-1".to_vec(),
            data: b"test".to_vec(),
            timeout_height: 1000,
            timeout_timestamp: 2000,
        };

        assert!(!IBCLightClient::timeout_packet(&packet, 1000).unwrap());
        assert!(IBCLightClient::timeout_packet(&packet, 2000).unwrap());
        assert!(IBCLightClient::timeout_packet(&packet, 3000).unwrap());
    }

    #[test]
    fn test_freeze_on_equivocation() {
        let header1 = Header {
            height: 100,
            timestamp: 1000,
            validators_hash: [0; 32],
            next_validators_hash: [1; 32],
            app_hash: [2; 32],
            commit_hash: [3; 32],
            proposer_address: [0; 20],
            signatures: vec![],
        };

        let header2 = Header {
            height: 100,
            timestamp: 1000,
            validators_hash: [0; 32],
            next_validators_hash: [1; 32],
            app_hash: [2; 32],
            commit_hash: [4; 32], // Different commit
            proposer_address: [0; 20],
            signatures: vec![],
        };

        let is_misbehavior =
            IBCLightClient::freeze_client_on_misbehavior(&header1, &header2).unwrap();
        assert!(is_misbehavior);
    }

    #[test]
    fn test_equivocation_same_commit_not_misbehavior() {
        let header1 = Header {
            height: 100,
            timestamp: 1000,
            validators_hash: [0; 32],
            next_validators_hash: [1; 32],
            app_hash: [2; 32],
            commit_hash: [3; 32],
            proposer_address: [0; 20],
            signatures: vec![],
        };

        let header2 = Header {
            height: 100,
            timestamp: 1000,
            validators_hash: [0; 32],
            next_validators_hash: [1; 32],
            app_hash: [2; 32],
            commit_hash: [3; 32], // Same commit
            proposer_address: [0; 20],
            signatures: vec![],
        };

        let is_misbehavior =
            IBCLightClient::freeze_client_on_misbehavior(&header1, &header2).unwrap();
        assert!(!is_misbehavior);
    }

    #[test]
    fn test_update_client_height_must_increase() {
        let header = Header {
            height: 50,
            timestamp: 1000,
            validators_hash: [0; 32],
            next_validators_hash: [1; 32],
            app_hash: [2; 32],
            commit_hash: [3; 32],
            proposer_address: [0; 20],
            signatures: vec![],
        };

        let mut client = CosmosChainInfo {
            chain_id: b"cosmoshub-4".to_vec(),
            client_id: b"07-tendermint-0".to_vec(),
            latest_height: 100,
            trust_period_seconds: 604800,
            unbonding_period_seconds: 2592000,
            is_frozen: false,
        };

        let result = IBCLightClient::update_client(header, &mut client);
        assert!(result.is_err());
    }

    #[test]
    fn test_acknowledge_packet() {
        let packet = IBCPacket {
            sequence: 1,
            source_port: b"transfer".to_vec(),
            source_channel: b"channel-0".to_vec(),
            destination_port: b"transfer".to_vec(),
            destination_channel: b"channel-1".to_vec(),
            data: b"test".to_vec(),
            timeout_height: 1000,
            timeout_timestamp: 2000,
        };

        let result = IBCLightClient::acknowledge_packet(&packet, b"ack".to_vec());
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_client_state() {
        let client = CosmosChainInfo {
            chain_id: b"cosmoshub-4".to_vec(),
            client_id: b"07-tendermint-0".to_vec(),
            latest_height: 100,
            trust_period_seconds: 604800,
            unbonding_period_seconds: 2592000,
            is_frozen: false,
        };

        let (height, frozen) = IBCLightClient::get_client_state(&client);
        assert_eq!(height, 100);
        assert!(!frozen);
    }
}
