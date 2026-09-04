use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Trait providing the set of authorized Ed25519 public keys for human review
/// and security council verification.
pub trait ReviewerRegistry {
    /// Returns the set of authorized human reviewer verifying keys.
    fn human_reviewer_keys(&self) -> &[VerifyingKey];
    /// Returns the set of security council verifying keys.
    fn security_council_keys(&self) -> &[VerifyingKey];
    /// Returns the quorum threshold (number of council signatures required).
    /// Default: ≥2/3 of council size.
    fn security_council_threshold(&self) -> usize {
        let total = self.security_council_keys().len();
        if total == 0 {
            return 0;
        }
        // ceil(2/3 * total): 2-of-3, 3-of-4, 4-of-5, etc.
        (total * 2 + 2) / 3
    }
}

/// Trait for querying on-chain governance state.
pub trait GovernanceChecker {
    /// Check whether the given proposal is in `Approved` or `Executed` state
    /// and references `action_hash`.  Returns `true` if the proposal authorizes
    /// this action.
    fn is_proposal_authorized(
        &self,
        proposal_id: &[u8; 32],
        action_hash: &[u8; 32],
    ) -> bool;
}

/// Dummy implementations for contexts where real verification isn't wired.
/// These ALWAYS fail authorization — they never return `true` so no unsafe
/// fallthrough is possible.
pub struct NoopRegistry;
impl ReviewerRegistry for NoopRegistry {
    fn human_reviewer_keys(&self) -> &[VerifyingKey] {
        &[]
    }
    fn security_council_keys(&self) -> &[VerifyingKey] {
        &[]
    }
}

pub struct NoopGovernance;
impl GovernanceChecker for NoopGovernance {
    fn is_proposal_authorized(
        &self,
        _proposal_id: &[u8; 32],
        _action_hash: &[u8; 32],
    ) -> bool {
        false
    }
}

/// Approval levels required for task execution or file changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalRequirement {
    None,
    HumanReview,
    SecurityReview,
    GovernanceReview,
    Blocked,
}

impl ApprovalRequirement {
    /// Check if approval is satisfied given an optional request context and
    /// verifier references.
    ///
    /// - `None`: always satisfied.
    /// - `HumanReview`: requires a valid Ed25519 signature in
    ///   `human_approval_token` from one of the registered human reviewer keys
    ///   covering `action_hash`.
    /// - `SecurityReview`: requires ≥2/3 quorum of valid Ed25519 signatures
    ///   from the security council keys covering `action_hash`.
    /// - `GovernanceReview`: queries the governance checker for proposal state.
    /// - `Blocked`: never satisfied.
    ///
    /// When `reviewer_registry` or `governance_checker` are `None`, the
    /// corresponding approval types that require them will return `false`.
    pub fn is_satisfied(
        &self,
        request_ctx: Option<&ApprovalContext>,
        reviewer_registry: Option<&dyn ReviewerRegistry>,
        governance_checker: Option<&dyn GovernanceChecker>,
    ) -> bool {
        match self {
            ApprovalRequirement::None => true,
            ApprovalRequirement::Blocked => false,
            ApprovalRequirement::HumanReview => {
                let ctx = match request_ctx {
                    Some(c) => c,
                    None => return false,
                };
                let action_hash = match ctx.action_hash {
                    Some(h) => h,
                    None => return false,
                };
                let token_bytes = match ctx.human_approval_token.as_ref() {
                    Some(b) => b,
                    None => return false,
                };
                let registry = match reviewer_registry {
                    Some(r) => r,
                    None => return false,
                };

                // Token format: [32-byte pubkey || 64-byte signature]
                if token_bytes.len() < 96 {
                    return false;
                }
                let pubkey_bytes: [u8; 32] =
                    match token_bytes[..32].try_into() {
                        Ok(b) => b,
                        Err(_) => return false,
                    };
                let sig_bytes: [u8; 64] =
                    match token_bytes[32..96].try_into() {
                        Ok(b) => b,
                        Err(_) => return false,
                    };
                let vk = VerifyingKey::from_bytes(&pubkey_bytes).ok();
                let signature = Signature::from_bytes(&sig_bytes);

                match (vk, signature) {
                    (Some(key), sig) => {
                        // Require the key to be in the authorized set
                        if !registry
                            .human_reviewer_keys()
                            .iter()
                            .any(|k| k == &key)
                        {
                            return false;
                        }
                        key.verify(&action_hash[..], &sig).is_ok()
                    }
                    _ => false,
                }
            }
            ApprovalRequirement::SecurityReview => {
                let ctx = match request_ctx {
                    Some(c) => c,
                    None => return false,
                };
                let action_hash = match ctx.action_hash {
                    Some(h) => h,
                    None => return false,
                };
                let sig_bytes_vec = match ctx.security_quorum_sig.as_ref() {
                    Some(b) => b,
                    None => return false,
                };
                let registry = match reviewer_registry {
                    Some(r) => r,
                    None => return false,
                };
                let threshold = registry.security_council_threshold();
                if threshold == 0 {
                    return false;
                }

                // Quorum sig format: repeating [32-byte pubkey || 64-byte sig]
                // entries.
                let entry_len = 96;
                if sig_bytes_vec.len() < entry_len * threshold {
                    return false;
                }
                let chunks = sig_bytes_vec.chunks_exact(entry_len);
                if !chunks.remainder().is_empty() {
                    return false; // malformed — trailing bytes
                }

                let council_keys = registry.security_council_keys();
                let mut valid_count = 0usize;
                let mut seen_keys: Vec<VerifyingKey> = Vec::new();

                for chunk in sig_bytes_vec.chunks_exact(entry_len) {
                    let pubkey_bytes: [u8; 32] = match chunk[..32].try_into()
                    {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    let sig_arr: [u8; 64] = match chunk[32..96].try_into() {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    let vk = match VerifyingKey::from_bytes(&pubkey_bytes) {
                        Ok(k) => k,
                        Err(_) => continue,
                    };
                    let sig = Signature::from_bytes(&sig_arr);

                    // Skip duplicate signers — only count each council
                    // member once to prevent replay attacks.
                    if seen_keys.iter().any(|k| k == &vk) {
                        continue;
                    }

                    // Only count if the key is in the council and sig verifies
                    if council_keys.iter().any(|k| k == &vk)
                        && vk.verify(&action_hash[..], &sig).is_ok()
                    {
                        seen_keys.push(vk);
                        valid_count += 1;
                        if valid_count >= threshold {
                            return true;
                        }
                    }
                }
                false
            }
            ApprovalRequirement::GovernanceReview => {
                let ctx = match request_ctx {
                    Some(c) => c,
                    None => return false,
                };
                let action_hash = match ctx.action_hash {
                    Some(h) => h,
                    None => return false,
                };
                let proposal_id = match ctx.governance_proposal_id.as_ref() {
                    Some(id) => id,
                    None => return false,
                };
                let checker = match governance_checker {
                    Some(c) => c,
                    None => return false,
                };
                checker.is_proposal_authorized(proposal_id, &action_hash)
            }
        }
    }

    /// Legacy compatibility: without context or verifiers, only None and
    /// Blocked are deterministically resolvable.
    pub fn is_satisfied_legacy(&self) -> bool {
        self.is_satisfied(None, None, None)
    }
}

/// Context required for approval verification.
///
/// SECURITY: All approval types require `action_hash` to be present.
/// Without it, tokens/signatures/proposal IDs can be replayed across
/// different actions.  The `action_hash` binds the approval evidence
/// to the specific action being authorized.
#[derive(Clone, Debug, Default)]
pub struct ApprovalContext {
    /// Hash of the action being approved.  Must be present for any
    /// approval type other than `None` or `Blocked`; without it
    /// signatures and tokens are not bound to a specific action.
    pub action_hash: Option<[u8; 32]>,
    /// Signed approval token from a human reviewer.
    /// Format: [32-byte ed25519 pubkey || 64-byte ed25519 signature]
    pub human_approval_token: Option<Vec<u8>>,
    /// Multi-signatures from the security council quorum.
    /// Format: zero or more [32-byte pubkey || 64-byte sig] entries.
    pub security_quorum_sig: Option<Vec<u8>>,
    /// Governance proposal ID that authorizes this action.
    pub governance_proposal_id: Option<[u8; 32]>,
}

/// Agent policy structure for swarm control.
///
/// `forbidden_paths` take precedence over `auto_edit_allowed`; policies
/// with overlapping entries should be rejected by `validate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub kind: crate::agent::AgentKind,
    pub permission_tier: crate::agent::AgentPermissionTier,
    pub auto_edit_allowed: Vec<String>,
    pub approval_required: Vec<String>,
    pub forbidden_paths: Vec<String>,
}

impl AgentPolicy {
    pub fn allows_path(&self, path: &str) -> bool {
        if self
            .forbidden_paths
            .iter()
            .any(|prefix| path == prefix || path.starts_with(prefix))
        {
            return false;
        }
        self.auto_edit_allowed
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }

    pub fn validate(&self) -> Result<(), String> {
        for allowed in &self.auto_edit_allowed {
            if self.forbidden_paths.iter().any(|forbidden| {
                allowed == forbidden
                    || allowed.starts_with(forbidden)
                    || forbidden.starts_with(allowed)
            }) {
                return Err(format!(
                    "policy for {:?} has overlapping allowed and forbidden path: {}",
                    self.kind, allowed
                ));
            }
        }
        Ok(())
    }
}

const COMMON_FORBIDDEN_PATHS: &[&str] = &[".env", "private_keys", "validator_keys"];

pub fn default_agent_policies() -> Vec<AgentPolicy> {
    let common_forbidden: Vec<String> = COMMON_FORBIDDEN_PATHS
        .iter()
        .map(|path| (*path).into())
        .collect();
    vec![
        AgentPolicy {
            kind: crate::agent::AgentKind::RepoScanner,
            permission_tier: crate::agent::AgentPermissionTier::ReadOnly,
            auto_edit_allowed: vec![],
            approval_required: vec![],
            forbidden_paths: common_forbidden.clone(),
        },
        AgentPolicy {
            kind: crate::agent::AgentKind::TestBuilder,
            permission_tier: crate::agent::AgentPermissionTier::DocsTestsReports,
            auto_edit_allowed: vec!["tests/".into(), "docs/".into(), "reports/".into()],
            approval_required: vec!["runtime/".into(), "pallets/".into()],
            forbidden_paths: common_forbidden,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    /// Generates a valid signed token (pubkey || signature) over action_hash.
    fn make_human_token(
        signing_key: &SigningKey,
        action_hash: &[u8; 32],
    ) -> Vec<u8> {
        let vk: VerifyingKey = signing_key.verifying_key();
        let sig: Signature = signing_key.sign(&action_hash[..]);
        let mut token = Vec::with_capacity(96);
        token.extend_from_slice(vk.as_bytes());
        token.extend_from_slice(&sig.to_bytes());
        token
    }

    /// Generates quorum sig bytes: each entry is [pubkey || sig].
    fn make_quorum_sig(
        signers: &[&SigningKey],
        action_hash: &[u8; 32],
    ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(signers.len() * 96);
        for sk in signers {
            let vk: VerifyingKey = sk.verifying_key();
            let sig: Signature = sk.sign(&action_hash[..]);
            buf.extend_from_slice(vk.as_bytes());
            buf.extend_from_slice(&sig.to_bytes());
        }
        buf
    }

    struct TestRegistry {
        human_keys: Vec<VerifyingKey>,
        council_keys: Vec<VerifyingKey>,
    }

    impl ReviewerRegistry for TestRegistry {
        fn human_reviewer_keys(&self) -> &[VerifyingKey] {
            &self.human_keys
        }
        fn security_council_keys(&self) -> &[VerifyingKey] {
            &self.council_keys
        }
    }

    struct TestGovernance {
        authorized: Vec<([u8; 32], [u8; 32])>,
    }
    impl GovernanceChecker for TestGovernance {
        fn is_proposal_authorized(
            &self,
            proposal_id: &[u8; 32],
            action_hash: &[u8; 32],
        ) -> bool {
            self.authorized
                .iter()
                .any(|(pid, ah)| pid == proposal_id && ah == action_hash)
        }
    }

    #[test]
    fn none_is_always_satisfied() {
        assert!(ApprovalRequirement::None.is_satisfied(None, None, None));
    }

    #[test]
    fn blocked_is_never_satisfied() {
        assert!(!ApprovalRequirement::Blocked.is_satisfied(None, None, None));
    }

    #[test]
    fn human_review_verifies_real_signature() {
        let mut rng = OsRng;
        let sk = SigningKey::generate(&mut rng);
        let vk: VerifyingKey = sk.verifying_key();
        let action_hash: [u8; 32] = [0xAA; 32];

        let token = make_human_token(&sk, &action_hash);
        let registry = TestRegistry {
            human_keys: vec![vk],
            council_keys: vec![],
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            human_approval_token: Some(token),
            ..Default::default()
        };

        assert!(ApprovalRequirement::HumanReview.is_satisfied(
            Some(&ctx),
            Some(&registry),
            None,
        ));
    }

    #[test]
    fn human_review_rejects_wrong_signer() {
        let mut rng = OsRng;
        let sk1 = SigningKey::generate(&mut rng);
        let sk2 = SigningKey::generate(&mut rng);
        let vk1: VerifyingKey = sk1.verifying_key();
        let action_hash: [u8; 32] = [0xAA; 32];

        // Token signed by sk2, but registry only has vk1
        let token = make_human_token(&sk2, &action_hash);
        let registry = TestRegistry {
            human_keys: vec![vk1],
            council_keys: vec![],
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            human_approval_token: Some(token),
            ..Default::default()
        };

        assert!(!ApprovalRequirement::HumanReview.is_satisfied(
            Some(&ctx),
            Some(&registry),
            None,
        ));
    }

    #[test]
    fn human_review_rejects_bad_signature() {
        let mut rng = OsRng;
        let sk = SigningKey::generate(&mut rng);
        let vk: VerifyingKey = sk.verifying_key();
        let action_hash: [u8; 32] = [0xAA; 32];

        // Valid token for action_hash, but we corrupt the signature
        let mut token = make_human_token(&sk, &action_hash);
        // Flip a byte in the signature portion (bytes 40..96)
        token[40] ^= 0x01;

        let registry = TestRegistry {
            human_keys: vec![vk],
            council_keys: vec![],
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            human_approval_token: Some(token),
            ..Default::default()
        };

        assert!(!ApprovalRequirement::HumanReview.is_satisfied(
            Some(&ctx),
            Some(&registry),
            None,
        ));
    }

    #[test]
    fn human_review_requires_token_and_action_hash() {
        // No context → fails
        assert!(!ApprovalRequirement::HumanReview.is_satisfied(None, None, None));
        // Missing action_hash → fails even with token
        let ctx_no_hash = ApprovalContext {
            human_approval_token: Some(vec![0u8; 96]),
            ..Default::default()
        };
        assert!(!ApprovalRequirement::HumanReview.is_satisfied(
            Some(&ctx_no_hash),
            None,
            None,
        ));
    }

    #[test]
    fn security_review_verifies_quorum() {
        let mut rng = OsRng;
        let sk1 = SigningKey::generate(&mut rng);
        let sk2 = SigningKey::generate(&mut rng);
        let sk3 = SigningKey::generate(&mut rng);
        let vk1: VerifyingKey = sk1.verifying_key();
        let vk2: VerifyingKey = sk2.verifying_key();
        let vk3: VerifyingKey = sk3.verifying_key();
        let action_hash: [u8; 32] = [0xBB; 32];

        let council_keys = vec![vk1, vk2, vk3];
        // 3 members → threshold = 2 (≥2/3)
        let quorum_sig = make_quorum_sig(&[&sk1, &sk2], &action_hash);

        let registry = TestRegistry {
            human_keys: vec![],
            council_keys,
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            security_quorum_sig: Some(quorum_sig),
            ..Default::default()
        };

        assert!(ApprovalRequirement::SecurityReview.is_satisfied(
            Some(&ctx),
            Some(&registry),
            None,
        ));
    }

    #[test]
    fn security_review_rejects_insufficient_quorum() {
        let mut rng = OsRng;
        let sk1 = SigningKey::generate(&mut rng);
        let sk2 = SigningKey::generate(&mut rng);
        let sk3 = SigningKey::generate(&mut rng);
        let vk1: VerifyingKey = sk1.verifying_key();
        let vk2: VerifyingKey = sk2.verifying_key();
        let vk3: VerifyingKey = sk3.verifying_key();
        let action_hash: [u8; 32] = [0xCC; 32];

        let council_keys = vec![vk1, vk2, vk3];
        // Only 1 of 3 signs → threshold (2) not met
        let quorum_sig = make_quorum_sig(&[&sk1], &action_hash);

        let registry = TestRegistry {
            human_keys: vec![],
            council_keys,
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            security_quorum_sig: Some(quorum_sig),
            ..Default::default()
        };

        assert!(!ApprovalRequirement::SecurityReview.is_satisfied(
            Some(&ctx),
            Some(&registry),
            None,
        ));
    }

    #[test]
    fn governance_review_verifies_proposal_state() {
        let action_hash: [u8; 32] = [0xDD; 32];
        let proposal_id: [u8; 32] = [0xEE; 32];

        let governance = TestGovernance {
            authorized: vec![(proposal_id, action_hash)],
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            governance_proposal_id: Some(proposal_id),
            ..Default::default()
        };

        assert!(ApprovalRequirement::GovernanceReview.is_satisfied(
            Some(&ctx),
            None,
            Some(&governance),
        ));
    }

    #[test]
    fn governance_review_rejects_unauthorized_proposal() {
        let action_hash: [u8; 32] = [0x11; 32];
        let proposal_id: [u8; 32] = [0x22; 32];

        let governance = TestGovernance {
            authorized: vec![], // nothing authorized
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            governance_proposal_id: Some(proposal_id),
            ..Default::default()
        };

        assert!(!ApprovalRequirement::GovernanceReview.is_satisfied(
            Some(&ctx),
            None,
            Some(&governance),
        ));
    }

    #[test]
    fn noop_registry_always_fails() {
        let action_hash: [u8; 32] = [0xAA; 32];
        let token = vec![0u8; 96]; // dummy token

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            human_approval_token: Some(token),
            ..Default::default()
        };

        assert!(!ApprovalRequirement::HumanReview.is_satisfied(
            Some(&ctx),
            Some(&NoopRegistry),
            None,
        ));
    }

    #[test]
    fn noop_governance_always_fails() {
        let ctx = ApprovalContext {
            action_hash: Some([0xFF; 32]),
            governance_proposal_id: Some([0xAA; 32]),
            ..Default::default()
        };

        assert!(!ApprovalRequirement::GovernanceReview.is_satisfied(
            Some(&ctx),
            None,
            Some(&NoopGovernance),
        ));
    }

    #[test]
    fn legacy_compat() {
        assert!(ApprovalRequirement::None.is_satisfied_legacy());
        assert!(!ApprovalRequirement::Blocked.is_satisfied_legacy());
        assert!(!ApprovalRequirement::HumanReview.is_satisfied_legacy());
        assert!(!ApprovalRequirement::SecurityReview.is_satisfied_legacy());
        assert!(!ApprovalRequirement::GovernanceReview.is_satisfied_legacy());
    }

    #[test]
    fn security_review_rejects_duplicate_signer_replay() {
        let mut rng = OsRng;
        let sk1 = SigningKey::generate(&mut rng);
        let sk2 = SigningKey::generate(&mut rng);
        let vk1: VerifyingKey = sk1.verifying_key();
        let vk2: VerifyingKey = sk2.verifying_key();
        let action_hash: [u8; 32] = [0x11; 32];

        // 2-member council => threshold = 2 (≥2/3 of 2)
        let council_keys = vec![vk1, vk2];

        // Repeat sk1's signature twice — enough entries to meet threshold
        // but only one distinct council member signed.
        let entry1 = make_quorum_sig(&[&sk1], &action_hash);
        let entry2 = make_quorum_sig(&[&sk1], &action_hash);
        let mut quorum_sig = entry1;
        quorum_sig.extend_from_slice(&entry2);

        let registry = TestRegistry {
            human_keys: vec![],
            council_keys,
        };

        let ctx = ApprovalContext {
            action_hash: Some(action_hash),
            security_quorum_sig: Some(quorum_sig),
            ..Default::default()
        };

        // Must fail: same signer replayed cannot satisfy distinct-member quorum.
        assert!(
            !ApprovalRequirement::SecurityReview.is_satisfied(
                Some(&ctx),
                Some(&registry),
                None,
            ),
            "duplicate signer replay must not satisfy quorum"
        );
    }
}
