use crate::{permissions::Permissions, AgentPermissionTier};

/// Guard outcome for repository paths.
#[derive(Debug, PartialEq, Eq)]
pub enum GuardAction {
    Allow,
    RequireApproval,
    Block,
}

/// Forbidden path patterns (YOLO blocked).
pub const FORBIDDEN_PATHS: &[&str] = &[
    ".env",
    ".env.*",
    "*.key",
    "*.pem",
    "id_rsa",
    "keys/",
    "secrets/",
    "wallets/",
    "validator-keys/",
    "treasury-keys/",
    "mainnet/",
    "mainnet-deploy/",
    "chain-specs/mainnet",
    "genesis",
    "genesis.json",
    "mainnet-raw.json",
    "mainnet-plain.json",
    "btc-mainnet/",
    "external-bridge-mainnet/",
];

/// Approval-required paths.
pub const APPROVAL_PATHS: &[&str] = &[
    "runtime/",
    "pallets/",
    "bridge/",
    "gateway/",
    "btc/",
    "dex/",
    "supply-ledger/",
    "settlement/",
    "tokenomics/",
    "governance/",
    "chain-specs/",
];

/// Auto-allowed paths.
pub const ALLOWED_PATHS: &[&str] = &[
    "docs/",
    "reports/",
    "tests/",
    "apps/tauri-os/",
    "scripts/swarm/",
    "scripts/testnet/",
    "scripts/x3/",
    "crates/x3-swarm-core/",
    "crates/x3-readiness/",
    "services/*/health",
];

fn matches_pattern(path: &str, pattern: &str) -> bool {
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        if prefix.ends_with('/') && suffix.starts_with('/') {
            let Some(rest) = path.strip_prefix(prefix) else {
                return false;
            };
            let Some(segment) = rest.strip_suffix(suffix) else {
                return false;
            };
            return !segment.is_empty() && !segment.contains('/');
        }
        return path.starts_with(prefix) && path.ends_with(suffix);
    }
    if pattern.ends_with('/') {
        path.starts_with(pattern)
    } else {
        path == pattern || path.starts_with(&format!("{}/", pattern))
    }
}

pub fn evaluate_path(path: &str) -> GuardAction {
    let normalized = path
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase();
    if normalized.is_empty() || normalized.contains("../") {
        return GuardAction::Block;
    }
    if normalized
        .split('/')
        .any(|segment| segment == ".env" || segment.starts_with(".env."))
    {
        return GuardAction::Block;
    }
    if FORBIDDEN_PATHS
        .iter()
        .any(|pattern| matches_pattern(&normalized, pattern))
    {
        return GuardAction::Block;
    }
    if ALLOWED_PATHS
        .iter()
        .any(|pattern| matches_pattern(&normalized, pattern))
    {
        return GuardAction::Allow;
    }
    if APPROVAL_PATHS
        .iter()
        .any(|pattern| matches_pattern(&normalized, pattern))
    {
        return GuardAction::RequireApproval;
    }
    GuardAction::Block
}

/// Guard that enforces path permissions.
pub struct ForbiddenPathGuard {
    permissions: Permissions,
}

impl ForbiddenPathGuard {
    pub fn new(agent_kind: crate::AgentKind, tier: AgentPermissionTier) -> Self {
        Self {
            permissions: Permissions::new(agent_kind, tier),
        }
    }

    /// Check if path edit is allowed.
    pub fn allows_edit(&self, path: &str) -> bool {
        match evaluate_path(path) {
            GuardAction::Allow => true,
            GuardAction::RequireApproval => self.permissions.can_edit_path(path),
            GuardAction::Block => false,
        }
    }

    /// Get approval requirement for path.
    pub fn approval_for_path(&self, path: &str) -> crate::policy::ApprovalRequirement {
        match evaluate_path(path) {
            GuardAction::RequireApproval => crate::policy::ApprovalRequirement::SecurityReview,
            GuardAction::Block => crate::policy::ApprovalRequirement::Blocked,
            GuardAction::Allow => crate::policy::ApprovalRequirement::None,
        }
    }
}

// Test functions (output to reports for now)
pub fn run_guard_tests() -> Vec<String> {
    let guard = ForbiddenPathGuard::new(
        crate::AgentKind::TestBuilder,
        crate::AgentPermissionTier::DocsTestsReports,
    );
    let mut results = vec![];
    for (name, path, expected) in [
        (
            "guard_allows_docs_tests_reports",
            "docs/guide.md",
            GuardAction::Allow,
        ),
        (
            "guard_allows_tauri_ui",
            "apps/tauri-os/src/apps/SwarmCommand/SwarmCommand.tsx",
            GuardAction::Allow,
        ),
        ("guard_blocks_env_files", ".env.local", GuardAction::Block),
        (
            "guard_blocks_private_keys",
            "keys/private.pem",
            GuardAction::Block,
        ),
        (
            "guard_blocks_mainnet_scripts",
            "mainnet-deploy/deploy.sh",
            GuardAction::Block,
        ),
        (
            "guard_requires_approval_for_runtime",
            "runtime/src/lib.rs",
            GuardAction::RequireApproval,
        ),
        (
            "guard_requires_approval_for_pallets",
            "pallets/x3-example/src/lib.rs",
            GuardAction::RequireApproval,
        ),
        (
            "guard_requires_approval_for_bridge",
            "bridge/proposal.rs",
            GuardAction::RequireApproval,
        ),
        (
            "guard_requires_approval_for_btc_gateway",
            "btc/gateway.rs",
            GuardAction::RequireApproval,
        ),
    ] {
        let actual = evaluate_path(path);
        if actual == expected {
            results.push(format!("✅ {}", name));
        } else {
            results.push(format!(
                "❌ {} path={} expected={:?} actual={:?}",
                name, path, expected, actual
            ));
        }
    }

    if guard.allows_edit("docs/guide.md") {
        results.push("✅ guard_instance_allows_docs".to_string());
    } else {
        results.push("❌ guard_instance_allows_docs".to_string());
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_allows_docs_tests_reports() {
        assert_eq!(evaluate_path("docs/readme.md"), GuardAction::Allow);
        assert_eq!(
            evaluate_path("reports/swarm_scan_report.md"),
            GuardAction::Allow
        );
        assert_eq!(evaluate_path("tests/swarm/guard.rs"), GuardAction::Allow);
    }

    #[test]
    fn guard_allows_tauri_ui() {
        assert_eq!(
            evaluate_path("apps/tauri-os/src/apps/SwarmCommand/SwarmCommand.tsx"),
            GuardAction::Allow
        );
    }

    #[test]
    fn guard_blocks_env_files() {
        assert_eq!(evaluate_path(".env"), GuardAction::Block);
        assert_eq!(evaluate_path(".env.local"), GuardAction::Block);
    }

    #[test]
    fn guard_blocks_private_keys() {
        assert_eq!(evaluate_path("keys/private.pem"), GuardAction::Block);
        assert_eq!(evaluate_path("validator-keys/node.key"), GuardAction::Block);
        assert_eq!(
            evaluate_path("treasury-keys/secret.key"),
            GuardAction::Block
        );
    }

    #[test]
    fn guard_blocks_mainnet_scripts() {
        assert_eq!(
            evaluate_path("mainnet-deploy/deploy.sh"),
            GuardAction::Block
        );
        assert_eq!(
            evaluate_path("chain-specs/mainnet/config.json"),
            GuardAction::Block
        );
        assert_eq!(
            evaluate_path("btc-mainnet/checkpoint.txt"),
            GuardAction::Block
        );
    }

    #[test]
    fn guard_requires_approval_for_runtime() {
        assert_eq!(
            evaluate_path("runtime/src/lib.rs"),
            GuardAction::RequireApproval
        );
    }

    #[test]
    fn guard_requires_approval_for_pallets() {
        assert_eq!(
            evaluate_path("pallets/x3-example/src/lib.rs"),
            GuardAction::RequireApproval
        );
    }

    #[test]
    fn guard_requires_approval_for_bridge() {
        assert_eq!(
            evaluate_path("bridge/proposal.rs"),
            GuardAction::RequireApproval
        );
    }

    #[test]
    fn guard_requires_approval_for_btc_gateway() {
        assert_eq!(
            evaluate_path("btc/gateway.rs"),
            GuardAction::RequireApproval
        );
        assert_eq!(
            evaluate_path("gateway/protocol.rs"),
            GuardAction::RequireApproval
        );
    }
}
