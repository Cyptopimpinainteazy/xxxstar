use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatorId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attestation {
    pub validator: ValidatorId,
    pub statement_hash: [u8; 32],
    pub signature: Vec<u8>,
    pub weight: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    EmptySignature,
    DuplicateValidator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationSet {
    statement_hash: [u8; 32],
    attestations: HashMap<ValidatorId, Attestation>,
    total_weight: u64,
}

impl AttestationSet {
    pub fn new(statement_hash: [u8; 32]) -> Self {
        Self {
            statement_hash,
            attestations: HashMap::new(),
            total_weight: 0,
        }
    }

    pub fn add_attestation(&mut self, attestation: Attestation) -> Result<(), AttestationError> {
        if attestation.signature.is_empty() {
            return Err(AttestationError::EmptySignature);
        }

        if self.attestations.contains_key(&attestation.validator) {
            return Err(AttestationError::DuplicateValidator);
        }

        self.total_weight = self.total_weight.saturating_add(attestation.weight);
        self.attestations
            .insert(attestation.validator.clone(), attestation);
        Ok(())
    }

    pub fn total_weight(&self) -> u64 {
        self.total_weight
    }

    pub fn unique_validators(&self) -> usize {
        self.attestations.len()
    }

    pub fn has_quorum(&self, required_weight: u64) -> bool {
        self.total_weight >= required_weight
    }

    pub fn validators(&self) -> HashSet<ValidatorId> {
        self.attestations.keys().cloned().collect()
    }

    pub fn statement_hash(&self) -> [u8; 32] {
        self.statement_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_attestation(name: &str, weight: u64) -> Attestation {
        Attestation {
            validator: ValidatorId(name.to_string()),
            statement_hash: [7; 32],
            signature: vec![1, 2, 3],
            weight,
        }
    }

    #[test]
    fn rejects_duplicate_validator() {
        let mut set = AttestationSet::new([7; 32]);
        set.add_attestation(mk_attestation("alice", 30)).unwrap();
        let second = set.add_attestation(mk_attestation("alice", 20));
        assert!(matches!(second, Err(AttestationError::DuplicateValidator)));
    }

    #[test]
    fn computes_weight_and_quorum_correctly() {
        let mut set = AttestationSet::new([7; 32]);
        set.add_attestation(mk_attestation("alice", 40)).unwrap();
        set.add_attestation(mk_attestation("bob", 35)).unwrap();

        assert_eq!(set.total_weight(), 75);
        assert!(set.has_quorum(67));
        assert!(!set.has_quorum(80));
    }
}
