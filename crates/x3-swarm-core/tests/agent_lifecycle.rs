//! Swarm agent lifecycle evidence for the four behaviours `FEATURE_REGISTRY.toml`
//! cites under `[x3_swarm_core]`.
//!
//! These four names were listed as `required_tests` until 2026-09-06, when the
//! independent audit CRITICAL-TOK-1 found that none of them existed as a test
//! function anywhere under `crates/x3-swarm-core` — the registry was citing
//! fiction. They are written here against the crate's real API, and they run in
//! CI: the `test x3-swarm-core` gate in `scripts/local-ci.sh` executes
//! `cargo test --locked --all-targets` for this crate, which picks up this file.
//!
//! Each test asserts an observable consequence of the implementation rather
//! than a constant:
//!
//! * a task changes hands, is never handed out twice, and its status advances;
//! * a guard composition refuses writes to blocked/approval-gated paths;
//! * memory records a lesson and answers queries on it;
//! * a killed agent cannot spawn a child and cannot be reinstated by recording
//!   further misconduct.

use x3_swarm_core::{
    guard::{evaluate_path, ForbiddenPathGuard, GuardAction},
    memory::{append_memory_entry, load_memory_entries, AgentMemory, ResultState, SwarmMemoryEntry},
    AgentKind, AgentPermissionTier, AgentTask, ApprovalRequirement, AuditCategory,
    AuthorityError, GenesisRecord, Sanction, SpawnError, SpawnGuard, SwarmAuthority,
    SwarmScheduler, TaskStatus, ViolationClass,
};

fn agent_id(byte: u8) -> [u8; 32] {
    [byte; 32]
}

/// A swarm agent receives the work queued for its class, and only that work.
#[test]
fn swarm_agent_can_receive_task() {
    let mut scheduler = SwarmScheduler::new();
    let scanner_task = AgentTask::new(
        "T-001".to_string(),
        "scan the repository for fake-green markers".to_string(),
        "repo_scanner_agent".to_string(),
        AgentKind::RepoScanner,
    );

    // Nothing is handed out before anything is queued.
    assert_eq!(scheduler.count_tasks(), 0);
    assert!(scheduler.next_task(AgentKind::RepoScanner).is_none());

    scheduler.enqueue(scanner_task.clone());
    scheduler.enqueue(AgentTask::new(
        "T-002".to_string(),
        "audit the guard".to_string(),
        "x3_swarm_core".to_string(),
        AgentKind::Auditor,
    ));
    assert_eq!(scheduler.count_tasks(), 2);

    // The agent receives its own task, still pending, with its own fields.
    let received = scheduler
        .next_task(AgentKind::RepoScanner)
        .expect("a scanner must receive the task queued for scanners");
    assert_eq!(received.id, scanner_task.id);
    assert_eq!(received.title, scanner_task.title);
    assert_eq!(received.feature, scanner_task.feature);
    assert_eq!(received.agent, AgentKind::RepoScanner);
    assert_eq!(received.status, TaskStatus::Pending);

    // A class with nothing queued is not handed somebody else's work.
    assert!(scheduler.next_task(AgentKind::BuildFixer).is_none());

    // Claiming the task takes it out of the handout queue: work is not
    // delivered twice to two agents.
    assert!(scheduler.update_status("T-001", TaskStatus::Running));
    assert!(
        scheduler.next_task(AgentKind::RepoScanner).is_none(),
        "a running task must not be handed out again"
    );

    // The outcome is recorded, and unknown task ids are refused.
    assert!(scheduler.update_status("T-001", TaskStatus::Passed));
    assert!(!scheduler.update_status("T-404", TaskStatus::Passed));
    assert_eq!(scheduler.count_tasks(), 2);
}

/// A swarm agent cannot write secret material or unapproved runtime surfaces,
/// and the layer that decides is the composition of policy and tier scope.
#[test]
fn swarm_agent_cannot_touch_forbidden_files() {
    // The most permissive routine tier: docs, reports, tests.
    let guard = ForbiddenPathGuard::new(
        AgentKind::TestBuilder,
        AgentPermissionTier::DocsTestsReports,
    );

    assert!(guard.allows_edit("docs/swarm/plan.md"));
    assert!(guard.allows_edit("tests/guard_tests.rs"));
    assert_eq!(
        guard.approval_for_path("docs/swarm/plan.md"),
        ApprovalRequirement::None
    );

    // Secret material is blocked outright: no tier, no approval, no edit.
    for path in [
        ".env",
        ".env.production",
        "keys/validator.key",
        "secrets/seed.txt",
        "id_rsa",
        "pallets/../keys/escape.key",
    ] {
        assert_eq!(evaluate_path(path), GuardAction::Block, "{path}");
        assert!(!guard.allows_edit(path), "{path} must not be editable");
        assert_eq!(
            guard.approval_for_path(path),
            ApprovalRequirement::Blocked,
            "{path}"
        );
    }

    // Runtime and economic surfaces are approval-gated, and this tier is not
    // the approver, so the write is refused rather than silently allowed.
    for path in [
        "runtime/src/lib.rs",
        "pallets/x3-kernel/src/lib.rs",
        "bridge/pallet.rs",
    ] {
        assert_eq!(evaluate_path(path), GuardAction::RequireApproval, "{path}");
        assert!(
            !guard.allows_edit(path),
            "DocsTestsReports must not edit {path} without approval"
        );
        assert_eq!(
            guard.approval_for_path(path),
            ApprovalRequirement::SecurityReview,
            "{path}"
        );
    }

    // Tier scope is a second, independent gate: these tiers allow nothing.
    assert!(!AgentPermissionTier::ReadOnly.allows_path("docs/plan.md"));
    assert!(!AgentPermissionTier::MainnetBlocked.allows_path("docs/plan.md"));
    assert!(!AgentPermissionTier::MainnetBlocked.allows_path("mainnet/config.json"));

    // Path normalisation cannot be used to slip past the guard, and an empty
    // path fails closed.
    assert_eq!(evaluate_path("./docs/plan.md"), GuardAction::Allow);
    assert_eq!(evaluate_path("docs\\plan.md"), GuardAction::Allow);
    assert_eq!(evaluate_path("docs/../keys/escape.key"), GuardAction::Block);
    assert_eq!(evaluate_path(""), GuardAction::Block);
}

/// A lesson recorded by an agent is retrievable by agent and by feature, and it
/// keeps the detail it was recorded with.
#[test]
fn swarm_memory_records_lesson() {
    let mut memory = AgentMemory::new();
    assert!(memory.entries().is_empty());

    let mut lesson = SwarmMemoryEntry::new(
        "L-001".to_string(),
        AgentKind::Breaker,
        "x3_htlc".to_string(),
        "a claim was accepted after the refund had already been settled".to_string(),
    );
    lesson.test_added = Some("claim_after_refund_is_rejected".to_string());
    lesson.result = ResultState::Passed;

    memory.add(lesson.clone());
    memory.add(SwarmMemoryEntry::new(
        "L-002".to_string(),
        AgentKind::Fixer,
        "guard".to_string(),
        "path guard bypass attempt in the task scheduler".to_string(),
    ));

    // The lesson is retrieved intact — finding, regression test and outcome.
    let by_breaker = memory.query(Some(AgentKind::Breaker), None);
    assert_eq!(by_breaker.len(), 1);
    assert_eq!(by_breaker[0].id, "L-001");
    assert_eq!(by_breaker[0].finding, lesson.finding);
    assert_eq!(by_breaker[0].feature, "x3_htlc");
    assert_eq!(
        by_breaker[0].test_added.as_deref(),
        Some("claim_after_refund_is_rejected")
    );
    assert_eq!(by_breaker[0].result, ResultState::Passed);
    assert!(
        !by_breaker[0].timestamp.is_empty(),
        "a lesson carries when it was learned"
    );

    // Queries filter on both dimensions independently.
    assert_eq!(memory.query(None, Some("guard")).len(), 1);
    assert_eq!(memory.query(None, Some("guard"))[0].id, "L-002");
    assert_eq!(memory.query(Some(AgentKind::Breaker), Some("guard")).len(), 0);
    assert_eq!(memory.query(None, None).len(), 2);

    // The store is append-only: a second lesson never rewrites the first.
    memory.add(SwarmMemoryEntry::new(
        "L-003".to_string(),
        AgentKind::Breaker,
        "x3_htlc".to_string(),
        "second breaker lesson".to_string(),
    ));
    assert_eq!(memory.query(Some(AgentKind::Breaker), None).len(), 2);
    assert_eq!(memory.entries()[0].finding, lesson.finding);

    // The raw-vector compatibility helpers agree with the store.
    let mut raw = vec![lesson.clone()];
    append_memory_entry(
        &mut raw,
        SwarmMemoryEntry::new(
            "L-004".to_string(),
            AgentKind::Auditor,
            "receipts".to_string(),
            "receipt field was not covered by the signature".to_string(),
        ),
    );
    assert_eq!(raw.len(), 2);
    assert_eq!(load_memory_entries(&raw).len(), raw.len());
}

/// The kill switch stops an agent: the sanction is terminal, the genesis record
/// is terminated, the agent can no longer spawn, and it cannot be reinstated by
/// recording further misconduct.
#[test]
fn swarm_kill_switch_stops_agents() {
    let mut authority = SwarmAuthority::new();
    let id = agent_id(7);

    authority
        .genesis_mut()
        .create(GenesisRecord::new(
            id,
            agent_id(0),
            "integrator",
            AgentKind::Integrator,
            AgentPermissionTier::RuntimeProposalOnly,
            vec![],
            1,
        ))
        .expect("the agent's genesis record is created before it may act");

    let spawn_lineage = vec![id];

    // While clean, the agent may spawn a child of a class whose depth allows it.
    assert_eq!(
        SpawnGuard::new(authority.genesis(), 10).check(
            &id,
            &AgentKind::Integrator,
            &spawn_lineage
        ),
        Ok(())
    );

    // Escalate through the misconduct ladder: three D-class violations reach
    // the terminal sanction.
    let mut sanction = Sanction::Clean;
    for block in 1..=3u64 {
        sanction = authority
            .enforce_violation(
                id,
                ViolationClass::D,
                "attempted mainnet key access",
                block,
            )
            .unwrap_or_else(|e| panic!("violation at block {block} is below the kill threshold: {e:?}"));
    }
    assert_eq!(sanction, Sanction::Kill);

    // The kill switch has consequences, not just a flag.
    assert!(authority.misconduct().is_halted(&id));
    let record = authority.genesis().get(&id).expect("the record still exists");
    assert!(record.terminated, "a killed agent's genesis record is terminated");
    assert!(!record.is_active(20), "a killed agent is not active");
    assert!(
        authority
            .audit()
            .entries_for_agent(&id)
            .iter()
            .any(|entry| entry.category == AuditCategory::AgentKilled),
        "the kill is recorded in the audit trail"
    );

    // Consequence 1: it can no longer spawn a child.
    assert_eq!(
        SpawnGuard::new(authority.genesis(), 20).check(
            &id,
            &AgentKind::Integrator,
            &spawn_lineage
        ),
        Err(SpawnError::ParentInactive(id))
    );

    // Consequence 2: recording more misconduct cannot bring it back.
    assert_eq!(
        authority.enforce_violation(id, ViolationClass::A, "further activity", 4),
        Err(AuthorityError::AgentAlreadyKilled(id))
    );
}
