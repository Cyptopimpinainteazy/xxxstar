/// Transaction Signer — Multi-signature transaction approval engine
/// Sign, approve, and execute transactions with flexible approval workflows
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_core::{ed25519, sr25519};
use sp_io::crypto::{ed25519_verify, sr25519_verify};
#[allow(unused_imports)]
use sp_std::vec;
use sp_std::vec::Vec;

/// Domain separator for every message this module signs.
///
/// A signature is only meaningful against a stated message. Without a domain prefix the
/// same bytes could be replayed against any other structure that happens to hash the
/// same fields, so the prefix is part of the scheme rather than decoration.
pub const SIGNING_DOMAIN: &[u8] = b"x3-wallet/multisig/v1";

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

        // Verify before recording anything. This used to accept any blob that was
        // non-empty and at most 256 bytes, set `is_valid: true`, and count it toward
        // `required_signatures` — so `execute_transaction` could be reached with
        // signatures nobody had produced. A signature that does not verify must not
        // move the counter.
        if !Self::verify_signature(transaction, &signature)? {
            return Err("Signature does not verify for this transaction");
        }

        transaction.signature_count += 1;
        Ok(signature)
    }

    /// The exact bytes a signer commits to.
    ///
    /// Every field a signer is agreeing to is bound here, in a fixed order, behind the
    /// domain prefix. The mutable counters (`signature_count`, `is_executed`) are
    /// deliberately excluded: they change as further signatures arrive, so binding them
    /// would invalidate every earlier signature.
    pub fn signing_message(transaction: &SigningTransaction) -> Vec<u8> {
        let mut message = Vec::with_capacity(SIGNING_DOMAIN.len() + 32 * 3 + 16 + 8 + 4 + 8 * 3);
        message.extend_from_slice(SIGNING_DOMAIN);
        message.extend_from_slice(&transaction.id);
        message.extend_from_slice(&transaction.creator);
        message.extend_from_slice(&transaction.target);
        message.extend_from_slice(&transaction.value.to_le_bytes());
        message.extend_from_slice(&(transaction.data.len() as u32).to_le_bytes());
        message.extend_from_slice(&transaction.data);
        message.extend_from_slice(&transaction.nonce.to_le_bytes());
        message.extend_from_slice(&transaction.required_signatures.to_le_bytes());
        message.extend_from_slice(&transaction.created_block.to_le_bytes());
        message.extend_from_slice(&transaction.expiry_block.to_le_bytes());
        message
    }

    /// Verify a collected signature against the transaction it claims to authorise.
    ///
    /// Returns `Ok(true)` only when the signature is a real Ed25519 or Sr25519 signature
    /// by `signature.signer` over [`Self::signing_message`]. An unknown scheme, a wrong
    /// length, a signature for another transaction, and a transaction whose signed fields
    /// have changed all come back `Ok(false)` or an error — never `Ok(true)`.
    pub fn verify_signature(
        transaction: &SigningTransaction,
        signature: &TransactionSignature,
    ) -> Result<bool, &'static str> {
        if signature.signature_data.is_empty() {
            return Err("Empty signature");
        }
        if signature.signature_data.len() > 256 {
            return Err("Signature too large");
        }
        if !signature.is_valid {
            return Err("Signature marked invalid");
        }
        if signature.transaction_id != transaction.id {
            return Err("Signature is for a different transaction");
        }

        Ok(Self::signature_verifies(
            &Self::signing_message(transaction),
            &signature.signer,
            &signature.signature_data,
        ))
    }

    /// True when `signature` (64 bytes) verifies over `message` for `signer`.
    ///
    /// Both supported schemes use 32-byte public keys and 64-byte signatures, so the
    /// scheme is not recoverable from the bytes: the signature is accepted when either
    /// verifier accepts it. That is not a weakening — an attacker who cannot produce a
    /// valid Sr25519 *or* Ed25519 signature for the key still produces nothing usable.
    fn signature_verifies(message: &[u8], signer: &[u8; 32], signature: &[u8]) -> bool {
        let raw: [u8; 64] = match signature.try_into() {
            Ok(raw) => raw,
            Err(_) => return false,
        };

        let sr_signature = sr25519::Signature::from_raw(raw);
        if sr25519_verify(&sr_signature, message, &sr25519::Public::from_raw(*signer)) {
            return true;
        }

        let ed_signature = ed25519::Signature::from_raw(raw);
        ed25519_verify(&ed_signature, message, &ed25519::Public::from_raw(*signer))
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
    use sp_core::{ByteArray as _, Pair as _};

    /// A deterministic signer: a real Sr25519 key pair, not a placeholder byte string.
    fn signer(seed: u8) -> (sr25519::Pair, [u8; 32]) {
        let pair = sr25519::Pair::from_seed(&[seed; 32]);
        let public = pair.public().0;
        (pair, public)
    }

    fn signature_bytes(transaction: &SigningTransaction, pair: &sr25519::Pair) -> Vec<u8> {
        pair.sign(&TransactionSigner::signing_message(transaction))
            .to_raw()
            .to_vec()
    }

    /// Collect a real signature from a deterministic signer.
    ///
    /// Four tests in this module (threshold counting, execution, expiry, "signatures
    /// needed") used to reach the threshold by passing `vec![255]` as the signature
    /// data, which only worked while `add_signature` accepted any non-empty blob. They
    /// still test what they were written for; they now reach it the way a caller has to.
    fn authorise(transaction: &mut SigningTransaction, seed: u8) {
        let (pair, public) = signer(seed);
        let signature_data = signature_bytes(transaction, &pair);
        TransactionSigner::add_signature(transaction, public, signature_data, 0)
            .expect("a signature over the transaction itself verifies");
    }

    fn pending_transaction(required: u32) -> SigningTransaction {
        TransactionSigner::create_transaction(
            [1u8; 32],
            [2u8; 32],
            1000,
            vec![1, 2, 3],
            1,
            required,
            100,
            0,
        )
        .expect("a well-formed transaction")
    }

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
        let mut tx = pending_transaction(2);
        let (pair, public) = signer(3);
        let signature_data = signature_bytes(&tx, &pair);

        let result = TransactionSigner::add_signature(&mut tx, public, signature_data, 0);
        assert!(result.is_ok());
        assert_eq!(tx.signature_count, 1);
        assert!(result.unwrap().is_valid);
    }

    #[test]
    fn test_add_signature_refuses_a_well_formed_but_forged_signature() {
        let mut tx = pending_transaction(1);
        let (_, public) = signer(3);

        let result = TransactionSigner::add_signature(&mut tx, public, vec![0xAB; 64], 0);
        assert!(result.is_err());
        assert_eq!(
            tx.signature_count, 0,
            "a signature that does not verify must not count toward the threshold"
        );
    }

    #[test]
    fn test_add_signature_refuses_a_signature_for_another_transaction() {
        let mut tx = pending_transaction(1);
        let other =
            TransactionSigner::create_transaction([9u8; 32], [2u8; 32], 42, vec![], 7, 1, 100, 0)
                .expect("a well-formed transaction");
        let (pair, public) = signer(4);
        let signature_data = signature_bytes(&other, &pair);

        let result = TransactionSigner::add_signature(&mut tx, public, signature_data, 0);
        assert!(result.is_err());
        assert_eq!(tx.signature_count, 0);
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
        let tx = pending_transaction(1);
        let (pair, public) = signer(5);
        let sig = TransactionSignature {
            id: [1u8; 32],
            transaction_id: tx.id,
            signer: public,
            signature_data: signature_bytes(&tx, &pair),
            signed_block: 0,
            is_valid: true,
        };

        assert_eq!(TransactionSigner::verify_signature(&tx, &sig), Ok(true));
    }

    #[test]
    fn test_verify_signature_rejects_a_tampered_transaction() {
        let tx = pending_transaction(1);
        let (pair, public) = signer(6);
        let sig = TransactionSignature {
            id: [1u8; 32],
            transaction_id: tx.id,
            signer: public,
            signature_data: signature_bytes(&tx, &pair),
            signed_block: 0,
            is_valid: true,
        };

        let mut tampered = tx.clone();
        tampered.value = tx.value + 1;
        assert_eq!(
            TransactionSigner::verify_signature(&tampered, &sig),
            Ok(false)
        );

        let mut retargeted = tx.clone();
        retargeted.target = [7u8; 32];
        assert_eq!(
            TransactionSigner::verify_signature(&retargeted, &sig),
            Ok(false)
        );
    }

    #[test]
    fn test_verify_signature_rejects_a_signature_for_a_different_transaction() {
        let tx = pending_transaction(1);
        let (pair, public) = signer(7);
        let sig = TransactionSignature {
            id: [1u8; 32],
            transaction_id: [0xEE; 32],
            signer: public,
            signature_data: signature_bytes(&tx, &pair),
            signed_block: 0,
            is_valid: true,
        };

        assert!(TransactionSigner::verify_signature(&tx, &sig).is_err());
    }

    #[test]
    fn test_verify_signature_invalid() {
        let tx = pending_transaction(1);
        let (_, public) = signer(8);
        let sig = TransactionSignature {
            id: [1u8; 32],
            transaction_id: tx.id,
            signer: public,
            signature_data: vec![255; 64],
            signed_block: 0,
            is_valid: false,
        };

        let result = TransactionSigner::verify_signature(&tx, &sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_required_signatures() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 2, 100, 0)
                .unwrap();

        assert!(!TransactionSigner::has_required_signatures(&tx));

        authorise(&mut tx, 3);
        assert!(!TransactionSigner::has_required_signatures(&tx));

        authorise(&mut tx, 4);
        assert!(TransactionSigner::has_required_signatures(&tx));
    }

    #[test]
    fn test_execute_transaction() {
        let mut tx =
            TransactionSigner::create_transaction([1u8; 32], [2u8; 32], 1000, vec![], 1, 1, 100, 0)
                .unwrap();

        authorise(&mut tx, 3);

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

        authorise(&mut tx, 3);

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

        authorise(&mut tx, 3);
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
