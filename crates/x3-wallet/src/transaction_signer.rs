/// Transaction Signer — Multi-signature transaction approval engine
/// Sign, approve, and execute transactions with flexible approval workflows
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
#[allow(unused_imports)]
use sp_std::vec;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SigningTransaction {
    pub id: [u8; 32],
    pub creator: [u8; 32],
    pub target: [u8; 32],
    pub value: u128,
    pub data: Vec<u8>,
    pub nonce: u64,
    pub signature_count: u32,
    pub required_signatures: u32,
    pub created_block: u64,
    pub expiry_block: u64,
    pub is_executed: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TransactionSignature {
    pub id: [u8; 32],
    pub transaction_id: [u8; 32],
    pub signer: [u8; 32],
    pub signature_data: Vec<u8>,
    pub signed_block: u64,
    pub is_valid: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SigningRequest {
    pub id: [u8; 32],
    pub transaction_id: [u8; 32],
    pub requested_signers: Vec<[u8; 32]>,
    pub received_signatures: Vec<[u8; 32]>,
    pub created_block: u64,
    pub deadline_block: u64,
}

pub struct TransactionSigner;

impl TransactionSigner {
    /// Create a transaction to be signed
    pub fn create_transaction(
        creator: [u8; 32],
        target: [u8; 32],
        value: u128,
        data: Vec<u8>,
        nonce: u64,
        required_sigs: u32,
        blocks_valid: u64,
        current_block: u64,
    ) -> Result<SigningTransaction, &'static str> {
        if required_sigs == 0 {
            return Err("At least 1 signature required");
        }
        if required_sigs > 50 {
            return Err("Too many signatures required");
        }

        let mut id = [0u8; 32];
        id[0..8].copy_from_slice(&nonce.to_le_bytes());
        id[8..16].copy_from_slice(&creator[0..8]);

        Ok(SigningTransaction {
            id,
            creator,
            target,
            value,
            data,
            nonce,
            signature_count: 0,
            required_signatures: required_sigs,
            created_block: current_block,
            expiry_block: current_block + blocks_valid,
            is_executed: false,
        })
    }

    /// Request signatures from a list of signers
    pub fn request_signatures(
        transaction: &SigningTransaction,
        signers: Vec<[u8; 32]>,
        current_block: u64,
    ) -> Result<SigningRequest, &'static str> {
        if signers.is_empty() {
            return Err("At least 1 signer required");
        }
        if signers.len() > 50 {
            return Err("Too many signers");
        }
        if current_block > transaction.expiry_block {
            return Err("Transaction expired");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&transaction.id[0..16]);
        id[16..32].copy_from_slice(&signers[0][0..16]);

        Ok(SigningRequest {
            id,
            transaction_id: transaction.id,
            requested_signers: signers,
            received_signatures: vec![],
            created_block: current_block,
            deadline_block: current_block + 100, // 100 blocks to sign
        })
    }

    /// Add a signature to transaction
    pub fn add_signature(
        transaction: &mut SigningTransaction,
        signer: [u8; 32],
        signature_data: Vec<u8>,
        current_block: u64,
    ) -> Result<TransactionSignature, &'static str> {
        if transaction.is_executed {
            return Err("Transaction already executed");
        }
        if current_block > transaction.expiry_block {
            return Err("Transaction expired");
        }
        if signature_data.is_empty() {
            return Err("Signature data empty");
        }
        if signature_data.len() > 256 {
            return Err("Signature too large");
        }

        let mut sig_id = [0u8; 32];
        sig_id[0..16].copy_from_slice(&transaction.id[0..16]);
        sig_id[16..24].copy_from_slice(&signer[0..8]);

        let signature = TransactionSignature {
            id: sig_id,
            transaction_id: transaction.id,
            signer,
            signature_data,
            signed_block: current_block,
            is_valid: true,
        };

        transaction.signature_count += 1;
        Ok(signature)
    }

    /// Verify signature is valid
    pub fn verify_signature(signature: &TransactionSignature) -> Result<bool, &'static str> {
        if signature.signature_data.is_empty() {
            return Err("Empty signature");
        }
        if !signature.is_valid {
            return Err("Signature marked invalid");
        }
        Err("Cryptographic signature verification not implemented")
    }

    /// Check if transaction has enough signatures
    pub fn has_required_signatures(transaction: &SigningTransaction) -> bool {
        transaction.signature_count >= transaction.required_signatures
    }

    /// Execute transaction (after signatures collected)
    pub fn execute_transaction(
        transaction: &mut SigningTransaction,
        current_block: u64,
    ) -> Result<(), &'static str> {
        if current_block > transaction.expiry_block {
            return Err("Transaction expired");
        }
        if !Self::has_required_signatures(transaction) {
            return Err("Not enough signatures");
        }

        transaction.is_executed = true;
        Ok(())
    }

    /// Check if signing request expired
    pub fn is_signing_request_expired(request: &SigningRequest, current_block: u64) -> bool {
        current_block > request.deadline_block
    }

    /// Mark signature as received for signing request
    pub fn mark_signature_received(
        request: &mut SigningRequest,
        signer: [u8; 32],
    ) -> Result<(), &'static str> {
        if !request.requested_signers.contains(&signer) {
            return Err("Signer not in requested list");
        }
        if request.received_signatures.contains(&signer) {
            return Err("Signature already received from this signer");
        }

        request.received_signatures.push(signer);
        Ok(())
    }

    /// Get signatures received count
    pub fn get_signatures_received(request: &SigningRequest) -> usize {
        request.received_signatures.len()
    }

    /// Check signer is authorized
    pub fn is_signer_authorized(request: &SigningRequest, signer: [u8; 32]) -> bool {
        request.requested_signers.contains(&signer)
    }

    /// Reject/cancel transaction
    pub fn cancel_transaction(transaction: &mut SigningTransaction) -> Result<(), &'static str> {
        if transaction.is_executed {
            return Err("Cannot cancel executed transaction");
        }
        transaction.signature_count = 0xFF; // mark as cancelled
        Ok(())
    }

    /// Get remaining signatures needed
    pub fn signatures_needed(transaction: &SigningTransaction) -> u32 {
        if transaction.required_signatures > transaction.signature_count {
            transaction.required_signatures - transaction.signature_count
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transaction() {
        let result = TransactionSigner::create_transaction(
            [1u8; 32],
            [2u8; 32],
            1000,
            vec![1, 2, 3],
            1,
            2,
            100,
            0,
        );
        assert!(result.is_ok());
        let tx = result.unwrap();
        assert_eq!(tx.required_signatures, 2);
        assert_eq!(tx.expiry_block, 100);
    }

    #[test]
    fn test_create_transaction_zero_sigs() {
        let result =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 0, 100, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_transaction_too_many_sigs() {
        let result = TransactionSigner::create_transaction(
            [1u8; 32],
            [2u8; 32],
            1000,
            vec![],
            1,
            51,
            100,
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_request_signatures() {
        let tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 2, 100, 0)
                .unwrap();

        let signers = vec![[3u8; 32], [4u8; 32]];
        let result = TransactionSigner::request_signatures(&tx, signers, 0);
        assert!(result.is_ok());
        let req = result.unwrap();
        assert_eq!(req.requested_signers.len(), 2);
    }

    #[test]
    fn test_add_signature() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 2, 100, 0)
                .unwrap();

        let result = TransactionSigner::add_signature(&mut tx, [3u8; 32], vec![255, 254], 0);
        assert!(result.is_ok());
        assert_eq!(tx.signature_count, 1);
    }

    #[test]
    fn test_add_signature_empty() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 1, 100, 0)
                .unwrap();

        let result = TransactionSigner::add_signature(&mut tx, [3u8; 32], vec![], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_signature_too_large() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 1, 100, 0)
                .unwrap();

        let large_sig = vec![0u8; 257];
        let result = TransactionSigner::add_signature(&mut tx, [3u8; 32], large_sig, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_signature() {
        let sig = TransactionSignature {
            id: [1u8; 32],
            transaction_id: [2u8; 32],
            signer: [3u8; 32],
            signature_data: vec![255, 254],
            signed_block: 0,
            is_valid: true,
        };

        let result = TransactionSigner::verify_signature(&sig);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_signature_invalid() {
        let sig = TransactionSignature {
            id: [1u8; 32],
            transaction_id: [2u8; 32],
            signer: [3u8; 32],
            signature_data: vec![255],
            signed_block: 0,
            is_valid: false,
        };

        let result = TransactionSigner::verify_signature(&sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_required_signatures() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 2, 100, 0)
                .unwrap();

        assert!(!TransactionSigner::has_required_signatures(&tx));

        TransactionSigner::add_signature(&mut tx, [3u8; 32], vec![255], 0).unwrap();
        assert!(!TransactionSigner::has_required_signatures(&tx));

        TransactionSigner::add_signature(&mut tx, [4u8; 32], vec![255], 0).unwrap();
        assert!(TransactionSigner::has_required_signatures(&tx));
    }

    #[test]
    fn test_execute_transaction() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 1, 100, 0)
                .unwrap();

        TransactionSigner::add_signature(&mut tx, [3u8; 32], vec![255], 0).unwrap();

        let result = TransactionSigner::execute_transaction(&mut tx, 50);
        assert!(result.is_ok());
        assert!(tx.is_executed);
    }

    #[test]
    fn test_execute_transaction_not_enough_sigs() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 2, 100, 0)
                .unwrap();

        let result = TransactionSigner::execute_transaction(&mut tx, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_transaction_expired() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 1, 100, 0)
                .unwrap();

        TransactionSigner::add_signature(&mut tx, [3u8; 32], vec![255], 0).unwrap();

        let result = TransactionSigner::execute_transaction(&mut tx, 101);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_signing_request_expired() {
        let request = SigningRequest {
            id: [1u8; 32],
            transaction_id: [2u8; 32],
            requested_signers: vec![[3u8; 32]],
            received_signatures: vec![],
            created_block: 0,
            deadline_block: 100,
        };

        assert!(!TransactionSigner::is_signing_request_expired(&request, 50));
        assert!(TransactionSigner::is_signing_request_expired(&request, 101));
    }

    #[test]
    fn test_mark_signature_received() {
        let mut request = SigningRequest {
            id: [1u8; 32],
            transaction_id: [2u8; 32],
            requested_signers: vec![[3u8; 32]],
            received_signatures: vec![],
            created_block: 0,
            deadline_block: 100,
        };

        let result = TransactionSigner::mark_signature_received(&mut request, [3u8; 32]);
        assert!(result.is_ok());
        assert_eq!(request.received_signatures.len(), 1);
    }

    #[test]
    fn test_mark_signature_received_duplicate() {
        let mut request = SigningRequest {
            id: [1u8; 32],
            transaction_id: [2u8; 32],
            requested_signers: vec![[3u8; 32]],
            received_signatures: vec![[3u8; 32]],
            created_block: 0,
            deadline_block: 100,
        };

        let result = TransactionSigner::mark_signature_received(&mut request, [3u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_transaction() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 1, 100, 0)
                .unwrap();

        let result = TransactionSigner::cancel_transaction(&mut tx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_signatures_needed() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 3, 100, 0)
                .unwrap();

        assert_eq!(TransactionSigner::signatures_needed(&tx), 3);

        TransactionSigner::add_signature(&mut tx, [3u8; 32], vec![255], 0).unwrap();
        assert_eq!(TransactionSigner::signatures_needed(&tx), 2);
    }

    #[test]
    fn test_is_signer_authorized() {
        let request = SigningRequest {
            id: [1u8; 32],
            transaction_id: [2u8; 32],
            requested_signers: vec![[3u8; 32], [4u8; 32]],
            received_signatures: vec![],
            created_block: 0,
            deadline_block: 100,
        };

        assert!(TransactionSigner::is_signer_authorized(&request, [3u8; 32]));
        assert!(!TransactionSigner::is_signer_authorized(
            &request, [99u8; 32]
        ));
    }
}
