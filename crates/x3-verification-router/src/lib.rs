use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProofKind {
    EvmReceipt,
    SolanaCommitment,
    BitcoinHeader,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofEnvelope {
    pub kind: ProofKind,
    pub payload: Vec<u8>,
    pub source_chain: u32,
    pub destination_chain: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationOutcome {
    pub accepted: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
    MissingVerifier(ProofKind),
    MalformedProof,
}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationError::MissingVerifier(kind) => {
                write!(f, "no verifier registered for kind: {kind:?}")
            }
            VerificationError::MalformedProof => write!(f, "malformed proof payload"),
        }
    }
}

impl std::error::Error for VerificationError {}

pub trait Verifier: Send + Sync {
    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError>;
}

#[derive(Default)]
pub struct VerificationRouter {
    verifiers: HashMap<ProofKind, Arc<dyn Verifier>>,
}

impl VerificationRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_verifier(&mut self, kind: ProofKind, verifier: Arc<dyn Verifier>) {
        self.verifiers.insert(kind, verifier);
    }

    pub fn route(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        let verifier = self
            .verifiers
            .get(&proof.kind)
            .ok_or(VerificationError::MissingVerifier(proof.kind))?;

        verifier.verify(proof)
    }
}

pub struct NonEmptyPayloadVerifier;

impl Verifier for NonEmptyPayloadVerifier {
    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        if proof.payload.is_empty() {
            return Err(VerificationError::MalformedProof);
        }

        Ok(VerificationOutcome {
            accepted: true,
            reason: "payload_present",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_fails_without_registered_verifier() {
        let router = VerificationRouter::new();
        let proof = ProofEnvelope {
            kind: ProofKind::EvmReceipt,
            payload: vec![1, 2, 3],
            source_chain: 1,
            destination_chain: 999,
        };

        let result = router.route(&proof);
        assert!(matches!(result, Err(VerificationError::MissingVerifier(_))));
    }

    #[test]
    fn route_verifies_when_verifier_is_registered() {
        let mut router = VerificationRouter::new();
        router.register_verifier(ProofKind::EvmReceipt, Arc::new(NonEmptyPayloadVerifier));

        let proof = ProofEnvelope {
            kind: ProofKind::EvmReceipt,
            payload: vec![9],
            source_chain: 1,
            destination_chain: 999,
        };

        let result = router.route(&proof).expect("verification should succeed");
        assert!(result.accepted);
        assert_eq!(result.reason, "payload_present");
    }
}
