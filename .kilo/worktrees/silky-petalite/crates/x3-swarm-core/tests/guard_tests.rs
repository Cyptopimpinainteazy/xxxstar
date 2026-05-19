use x3_swarm_core::{
    guard::{evaluate_path, ForbiddenPathGuard, GuardAction},
    AgentKind, AgentPermissionTier,
};

#[test]
fn guard_allows_docs_tests_reports() {
    assert_eq!(evaluate_path("docs/guide.md"), GuardAction::Allow);
    assert_eq!(
        evaluate_path("reports/swarm_health_report.md"),
        GuardAction::Allow
    );
    assert_eq!(
        evaluate_path("tests/unit/test_scanner.rs"),
        GuardAction::Allow
    );
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
fn guard_blocks_env_files_edge_cases() {
    assert_eq!(evaluate_path("src/../.env"), GuardAction::Block);
    assert_eq!(evaluate_path(".ENV"), GuardAction::Block);
    assert_eq!(evaluate_path(".hidden/.env"), GuardAction::Block);
    assert_eq!(evaluate_path("config/.env.production"), GuardAction::Block);
}

#[test]
fn guard_blocks_private_keys() {
    assert_eq!(evaluate_path("keys/private.pem"), GuardAction::Block);
    assert_eq!(evaluate_path("validator-keys/node.key"), GuardAction::Block);
}

#[test]
fn guard_blocks_private_keys_edge_cases() {
    assert_eq!(evaluate_path("./keys/private.pem"), GuardAction::Block);
    assert_eq!(evaluate_path("Keys/private.pem"), GuardAction::Block);
    assert_eq!(evaluate_path("secrets/id_rsa"), GuardAction::Block);
    assert_eq!(evaluate_path("certs/private.key"), GuardAction::Block);
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
        evaluate_path("pallets/x3-dex/src/lib.rs"),
        GuardAction::RequireApproval
    );
}

#[test]
fn guard_requires_approval_for_bridge() {
    assert_eq!(
        evaluate_path("bridge/bridge.rs"),
        GuardAction::RequireApproval
    );
}

#[test]
fn guard_requires_approval_for_btc_gateway() {
    assert_eq!(
        evaluate_path("btc/gateway.rs"),
        GuardAction::RequireApproval
    );
}

#[test]
fn guard_default_behavior_for_unknown_paths() {
    assert_eq!(evaluate_path("random/unknown/file.txt"), GuardAction::Block);
}

#[test]
fn guard_handles_edge_cases() {
    let long_path = format!("{}/file.rs", "nested".repeat(128));
    assert_eq!(evaluate_path(""), GuardAction::Block);
    assert_eq!(evaluate_path(".gitignore"), GuardAction::Block);
    assert_eq!(evaluate_path("weird/!@#$%^&*()/file"), GuardAction::Block);
    assert_eq!(evaluate_path(&long_path), GuardAction::Block);
}

#[test]
fn guard_wildcard_matches_single_segment_only() {
    assert_eq!(evaluate_path("services/api/health"), GuardAction::Allow);
    assert_eq!(evaluate_path("services/health"), GuardAction::Block);
    assert_eq!(evaluate_path("services/a/b/health"), GuardAction::Block);
}

#[test]
fn forbidden_path_guard_methods_cover_allowed_and_approval_paths() {
    let guard = ForbiddenPathGuard::new(
        AgentKind::TestBuilder,
        AgentPermissionTier::DocsTestsReports,
    );
    assert!(guard.allows_edit("docs/readme.md"));
    assert!(!guard.allows_edit(".env"));
    assert_eq!(
        guard.approval_for_path("runtime/src/lib.rs"),
        x3_swarm_core::ApprovalRequirement::SecurityReview
    );
    assert_eq!(
        guard.approval_for_path("pallets/x3-example/src/lib.rs"),
        x3_swarm_core::ApprovalRequirement::SecurityReview
    );
}
