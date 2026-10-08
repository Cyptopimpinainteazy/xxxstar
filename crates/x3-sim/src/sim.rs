//! The simulation loop.
//!
//! A run has four moving parts, all deterministic:
//!
//! * a [`VirtualClock`] that only moves forward when the loop moves it;
//! * a [`SimRng`] seeded from `--seed`;
//! * a [`VirtualNetwork`] that decides what actually arrives;
//! * a [`FaultPlan`] that says when a node dies or a link is cut.
//!
//! Everything the simulation *asserts* about is produced by the real
//! `SwapCoordinator`. The simulator never re-implements a transition; it only
//! chooses the order in which the real API is called and observes the result.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use x3_cross_vm_coordinator::{
    CoordinatorConfig, HtlcCreateParams, HtlcId, HtlcRecord, HtlcSecret, HtlcStatus,
    InMemoryPersistence, SessionPersistence, SwapCoordinator, SwapPhase, SwapSession, VmTarget,
};

use crate::clock::VirtualClock;
use crate::faults::{FaultKind, FaultPlan};
use crate::invariants::{check_sessions, Violation};
use crate::network::{NetworkStats, VirtualNetwork};
use crate::rng::SimRng;

/// Fixed start instant: 2023-11-14T22:13:20Z. A constant, not a clock reading.
pub const START_UNIX_MS: u64 = 1_700_000_000_000;

const FAST_TIMELOCK_SECS: u64 = 3_600;
const SLOW_TIMELOCK_SECS: u64 = 7_200;

/// Which adversarial schedule to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scenario {
    /// Every session walks the happy path once, in order.
    HappyPath,
    /// Claims and refunds are scheduled against each other, with a partition.
    ClaimRefundRace,
    /// Repeated partitions, slow links and message loss.
    PartitionStorm,
    /// Node crashes, restart-from-persistence and a lost write.
    CrashRecovery,
}

impl Scenario {
    pub fn as_str(self) -> &'static str {
        match self {
            Scenario::HappyPath => "happy-path",
            Scenario::ClaimRefundRace => "claim-refund-race",
            Scenario::PartitionStorm => "partition-storm",
            Scenario::CrashRecovery => "crash-recovery",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "happy-path" | "happy" => Some(Scenario::HappyPath),
            "claim-refund-race" | "race" => Some(Scenario::ClaimRefundRace),
            "partition-storm" | "partition" => Some(Scenario::PartitionStorm),
            "crash-recovery" | "crash" => Some(Scenario::CrashRecovery),
            _ => None,
        }
    }

    pub fn all() -> [Scenario; 4] {
        [
            Scenario::HappyPath,
            Scenario::ClaimRefundRace,
            Scenario::PartitionStorm,
            Scenario::CrashRecovery,
        ]
    }
}

/// Knobs for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub seed: u64,
    pub scenario: Scenario,
    pub sessions: usize,
    pub steps: usize,
    pub nodes: usize,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            scenario: Scenario::HappyPath,
            sessions: 8,
            steps: 120,
            nodes: 4,
        }
    }
}

impl SimConfig {
    /// The CLI arguments that re-enter this exact config.
    ///
    /// Every dimension the binary reads is present, so a replay cannot
    /// silently fall back to a default and run a different schedule.
    pub fn replay_args(&self) -> Vec<String> {
        vec![
            "--seed".to_string(),
            self.seed.to_string(),
            "--scenario".to_string(),
            self.scenario.as_str().to_string(),
            "--sessions".to_string(),
            self.sessions.to_string(),
            "--steps".to_string(),
            self.steps.to_string(),
            "--nodes".to_string(),
            self.nodes.to_string(),
        ]
    }

    /// The `cargo run` line that reproduces this config.
    ///
    /// `crates/x3-sim` is a standalone workspace root, so the manifest path
    /// form works from the repository root, where packets are produced and
    /// replay is most useful.
    pub fn replay_command(&self) -> String {
        format!(
            "cargo run --manifest-path crates/x3-sim/Cargo.toml -- {}",
            self.replay_args().join(" ")
        )
    }
}

/// One client intent, as sent to the coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimOp {
    LockFast { session: usize },
    LockSlow { session: usize },
    BeginFlash { session: usize },
    Settle { session: usize },
    ClaimFast { session: usize },
    ClaimSlow { session: usize },
    Abort { session: usize },
    Refund { session: usize },
}

impl SimOp {
    pub fn name(self) -> &'static str {
        match self {
            SimOp::LockFast { .. } => "lock_fast",
            SimOp::LockSlow { .. } => "lock_slow",
            SimOp::BeginFlash { .. } => "begin_flash",
            SimOp::Settle { .. } => "settle",
            SimOp::ClaimFast { .. } => "claim_fast",
            SimOp::ClaimSlow { .. } => "claim_slow",
            SimOp::Abort { .. } => "abort",
            SimOp::Refund { .. } => "refund",
        }
    }

    pub fn session(self) -> usize {
        match self {
            SimOp::LockFast { session }
            | SimOp::LockSlow { session }
            | SimOp::BeginFlash { session }
            | SimOp::Settle { session }
            | SimOp::ClaimFast { session }
            | SimOp::ClaimSlow { session }
            | SimOp::Abort { session }
            | SimOp::Refund { session } => session,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct OpEntry {
    op: SimOp,
    /// When the client prepared the message. The HTLC record is built from this
    /// instant, so a duplicate delivery carries byte-identical evidence and the
    /// coordinator's idempotency journal can collapse it.
    created_secs: u64,
}

/// Everything one run produced.
#[derive(Debug, Clone, Serialize)]
pub struct SimOutcome {
    pub seed: u64,
    pub scenario: String,
    /// Effective sessions in the ledger for this run (`config.sessions`,
    /// clamped to at least one).
    pub sessions: usize,
    pub steps: usize,
    /// Effective nodes in the simulated topology (`config.nodes`, clamped to
    /// at least two).
    pub nodes: usize,
    pub accepted: u64,
    pub rejected: u64,
    pub restarts: u64,
    pub stale_writes: u64,
    pub partitions: u64,
    pub sessions_completed: usize,
    pub sessions_refunded: usize,
    pub network: NetworkStats,
    pub trace_digest: String,
    pub state_digest: String,
    /// Scheduler step whose execution introduced the first violation. `None`
    /// when the violation appeared during the drain, after the last step.
    pub first_bad_step: Option<u64>,
    /// `"step N"` or `"drain"`, for humans.
    pub first_bad_step_label: String,
    /// The operation applied just before the first violation was observed.
    pub first_bad_op: Option<String>,
    /// Faults that had fired by the time the first violation appeared.
    pub active_faults: Vec<String>,
    /// The violating session's state immediately before the step that
    /// introduced the first violation, when one step can be attributed.
    pub state_before: Option<serde_json::Value>,
    /// The same session's state after that step.
    pub state_after: Option<serde_json::Value>,
    pub violations: Vec<Violation>,
    #[serde(skip)]
    pub trace: Vec<String>,
}

/// The last operation that touched one session, kept so the first violation
/// can be attributed to the step that produced it rather than to the schedule
/// as a whole. Keyed by session: several deliveries on different sessions can
/// happen between two invariant checks, and the first violation must be
/// attributed to *its own* session, not to whichever ran last.
#[derive(Debug, Clone)]
struct LastApplied {
    op: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
}

impl SimOutcome {
    pub fn is_pass(&self) -> bool {
        self.violations.is_empty()
    }

    /// The `cargo run` line that reproduces this exact run.
    ///
    /// Delegates to `SimConfig::replay_command` so the packet, the minimizer,
    /// and the CLI help can never drift apart.
    pub fn replay_command(&self) -> String {
        SimConfig {
            seed: self.seed,
            scenario: Scenario::parse(&self.scenario).unwrap_or(Scenario::HappyPath),
            sessions: self.sessions,
            steps: self.steps,
            nodes: self.nodes,
        }
        .replay_command()
    }

    pub fn to_evidence_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Build a deterministic HTLC record for a session leg.
fn htlc_record(session: usize, secret: &[u8; 32], fast: bool, now_secs: u64) -> HtlcRecord {
    let hash_lock = HtlcSecret(*secret).hash();
    let (vm, timelock) = if fast {
        (VmTarget::X3Vm, now_secs + FAST_TIMELOCK_SECS)
    } else {
        (VmTarget::Svm, now_secs + SLOW_TIMELOCK_SECS)
    };
    HtlcRecord {
        id: HtlcId(vec![if fast { 1 } else { 2 }, session as u8]),
        params: HtlcCreateParams {
            vm,
            recipient: vec![0x11; 32],
            hash_lock,
            timelock,
            asset: vec![0xAA; 32],
            amount: 1_000 + session as u128,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [if fast { 1 } else { 2 }; 32],
    }
}

fn coordinator(persistence: &Arc<InMemoryPersistence>) -> SwapCoordinator<InMemoryPersistence> {
    // `with_persistence`, not `new`: the latter deliberately panics outside
    // `cfg(test)` because non-durable persistence loses funds on a crash.
    SwapCoordinator::with_persistence(CoordinatorConfig::default(), Arc::clone(persistence))
}

/// Drive the real coordinator and report what happened.
pub fn run(config: &SimConfig) -> SimOutcome {
    let mut rng = SimRng::from_seed(config.seed);
    let mut clock = VirtualClock::new(START_UNIX_MS);
    let persistence = Arc::new(InMemoryPersistence::new());
    let sessions = config.sessions.max(1);
    let clients = config.nodes.saturating_sub(1).max(1);
    let nodes = config.nodes.max(2);

    // ── Seed the ledger with deterministic sessions ───────────────────────
    // `setup_swap` draws its secret from `OsRng`, which is correct in
    // production and fatal for replay. The simulator therefore constructs the
    // sessions from seeded bytes and hands them to the coordinator through
    // persistence, which is the same path a restart takes.
    let mut session_ids: Vec<String> = Vec::with_capacity(sessions);
    let mut secrets: Vec<[u8; 32]> = Vec::with_capacity(sessions);
    for index in 0..sessions {
        let secret_bytes = rng.bytes32();
        let secret = HtlcSecret(secret_bytes);
        let now = clock.now_secs();
        let id = format!(
            "x3sim-{index:04}-{}",
            hex::encode(&blake3::hash(&secret_bytes).as_bytes()[..8])
        );
        let session = SwapSession {
            session_id: id.clone(),
            hash_lock: secret.hash(),
            htlc_fast: None,
            htlc_slow: None,
            flash_legs: Vec::new(),
            leg_outcomes: Vec::new(),
            phase: SwapPhase::Setup,
            timelock_fast: now + FAST_TIMELOCK_SECS,
            timelock_slow: now + SLOW_TIMELOCK_SECS,
            created_at: now,
            updated_at: now,
            operation_journal: Vec::new(),
            requires_merkle_verification: false,
        };
        persistence.save(&session);
        session_ids.push(id);
        secrets.push(secret_bytes);
    }

    let mut coord = coordinator(&persistence);
    // The happy path is a positive control: it asks whether the lifecycle
    // completes when nothing goes wrong, so its network is ordered and lossless.
    // The adversarial scenarios keep jitter and loss, because a reordered
    // message is exactly one of the failures they exist to schedule.
    let mut net = match config.scenario {
        Scenario::HappyPath => VirtualNetwork::new(nodes, 25, 0, 0),
        _ => VirtualNetwork::new(nodes, 25, 40, 2),
    };
    let horizon_ms = (config.steps as u64).saturating_mul(500).max(1_000);
    let mut plan = FaultPlan::generate(
        config.scenario,
        &mut rng,
        clients,
        sessions,
        START_UNIX_MS,
        horizon_ms,
    );

    let mut trace: Vec<String> = Vec::new();
    let mut ops: Vec<OpEntry> = Vec::new();
    let mut cursors = vec![0usize; sessions];
    let mut shadow: HashMap<String, SwapSession> = HashMap::new();
    let mut violations: Vec<Violation> = Vec::new();
    let mut accepted = 0u64;
    let mut rejected = 0u64;
    let mut restarts = 0u64;
    let mut stale_writes = 0u64;
    let mut partitions = 0u64;
    let mut node_down_until_ms = 0u64;
    let mut last_op_index: Option<usize> = None;
    let mut last_applied: HashMap<String, LastApplied> = HashMap::new();
    let mut first_bad_step: Option<u64> = None;
    let mut first_bad_step_label: Option<String> = None;
    let mut first_bad_op: Option<String> = None;
    // Faults *in effect right now*: a partition that was healed is no longer
    // active, and a slow link stays installed across a heal (the network's
    // `heal_all` restores connectivity, not latency or drop settings).
    let mut live_partitions: Vec<String> = Vec::new();
    let mut live_slow_link: Option<String> = None;
    let mut live_node_down: Option<(u64, String)> = None;
    let mut active_faults: Vec<String> = Vec::new();
    let mut state_before: Option<serde_json::Value> = None;
    let mut state_after: Option<serde_json::Value> = None;

    macro_rules! record_violations {
        ($trace:expr, $human:expr, $index:expr) => {{
            let mut sessions_now: Vec<SwapSession> = persistence.load_all().into_values().collect();
            sessions_now.sort_by(|a, b| a.session_id.cmp(&b.session_id));
            let mut found = check_sessions(&sessions_now);
            if !found.is_empty() {
                found.retain(|candidate| !violations.contains(candidate));
                for item in &found {
                    trace.push(format!(
                        "{} INVARIANT {} session={} {}",
                        $trace, item.code, item.session_id, item.detail
                    ));
                }
                if violations.is_empty() && !found.is_empty() {
                    // The first violation is the one worth attributing; later
                    // ones are often the same defect seen through other
                    // sessions.
                    first_bad_step = if $index == u64::MAX {
                        None
                    } else {
                        Some($index)
                    };
                    first_bad_step_label = Some($human.to_string());
                    let blamed = found[0].session_id.clone();
                    if let Some(last) = last_applied.get(&blamed) {
                        first_bad_op = Some(last.op.clone());
                        state_before = last.before.clone();
                        state_after = last.after.clone();
                    } else {
                        state_before = None;
                        state_after = None;
                    }
                    // Report the faults that are *in effect*, not every
                    // fault that ever fired: a healed partition must not
                    // mislead the investigator about the network state.
                    let mut live = live_partitions.clone();
                    if let Some(label) = &live_slow_link {
                        live.push(label.clone());
                    }
                    if let Some((until, label)) = &live_node_down {
                        if clock.now_ms() < *until {
                            live.push(label.clone());
                        }
                    }
                    active_faults = live;
                }
                violations.extend(found);
            }
        }};
    }

    for step in 0..config.steps {
        // Move time, then let whatever is due happen.
        clock.advance(250 + rng.next_u64() % 500);
        let now_ms = clock.now_ms();
        let now_secs = clock.now_secs();

        for fault in plan.due(now_ms) {
            match fault {
                FaultKind::Partition { group_a, group_b } => {
                    net.partition(&group_a, &group_b);
                    partitions += 1;
                    let label = format!("{step:04} partition {group_a:?}|{group_b:?}");
                    live_partitions.clear();
                    live_partitions.push(label.clone());
                    trace.push(format!("{step:04} FAULT partition {group_a:?}|{group_b:?}"));
                }
                FaultKind::Heal => {
                    net.heal_all();
                    live_partitions.clear();
                    // `heal_all` restores connectivity only; latency and drop
                    // settings stay installed, so a slow link is still active.
                    trace.push(format!("{step:04} FAULT heal"));
                }
                FaultKind::SlowLink {
                    latency_ms,
                    extra_drop_percent,
                } => {
                    net.set_latency(latency_ms, latency_ms / 4);
                    net.set_drop_percent(extra_drop_percent);
                    live_slow_link = Some(format!(
                        "{step:04} slow-link latency={latency_ms}ms drop={extra_drop_percent}%"
                    ));
                    trace.push(format!(
                        "{step:04} FAULT slow-link latency={latency_ms}ms drop={extra_drop_percent}%"
                    ));
                }
                FaultKind::CrashAndRestart { duration_ms } => {
                    let dropped = net.drop_in_flight_to(0);
                    coord = coordinator(&persistence);
                    restarts += 1;
                    node_down_until_ms = now_ms + duration_ms;
                    live_node_down = Some((
                        node_down_until_ms,
                        format!(
                            "{step:04} crash-restart down_for={duration_ms}ms dropped={dropped}"
                        ),
                    ));
                    trace.push(format!(
                        "{step:04} FAULT crash-restart down_for={duration_ms}ms dropped={dropped}"
                    ));
                }
                FaultKind::StaleWrite { session } => {
                    if let Some(id) = session_ids.get(session) {
                        if let Some(previous) = shadow.get(id).cloned() {
                            // This session's state changed outside an
                            // operation, so the last operation no longer
                            // describes how it reached the current state.
                            last_applied.remove(id);
                            persistence.save(&previous);
                            coord = coordinator(&persistence);
                            stale_writes += 1;
                            trace.push(format!("{step:04} FAULT stale-write session={id}"));
                        }
                    }
                }
            }
        }

        // ── Choose and send the next intent ───────────────────────────────
        if let Some(op) = next_op(&mut rng, config.scenario, step, sessions, &mut cursors) {
            ops.push(OpEntry {
                op,
                created_secs: now_secs,
            });
            let index = ops.len() - 1;
            let from = 1 + rng.below(clients);
            net.send(from, 0, now_ms, index, &mut rng);

            // Duplicate delivery: the same prepared message arrives twice.
            let duplicate_percent = match config.scenario {
                Scenario::HappyPath => 0,
                Scenario::ClaimRefundRace => 15,
                Scenario::PartitionStorm => 25,
                Scenario::CrashRecovery => 20,
            };
            if rng.chance(duplicate_percent) {
                let index = last_op_index.unwrap_or(index);
                net.send(from, 0, now_ms, index, &mut rng);
                trace.push(format!("{step:04} RETRANSMIT op_index={index}"));
            }
            last_op_index = Some(index);
        }

        // ── Deliver whatever arrived ──────────────────────────────────────
        for envelope in net.deliver_ready(now_ms) {
            let entry = match ops.get(envelope.op_index) {
                Some(entry) => *entry,
                None => continue,
            };
            if now_ms < node_down_until_ms {
                trace.push(format!(
                    "{step:04} t={now_secs} node={} op={} s={} DROPPED(node down)",
                    envelope.from,
                    entry.op.name(),
                    entry.op.session()
                ));
                continue;
            }

            let id = session_ids[entry.op.session()].clone();
            match persistence.load(&id) {
                Some(snapshot) => {
                    shadow.insert(id.clone(), snapshot);
                }
                // A stale-write fault has nothing to restore for a session that is gone;
                // the trace records it so the digest still tells the two runs apart.
                None => {
                    shadow.remove(&id);
                    trace.push(format!("{step:04} s={id} MISSING from persistence"));
                }
            }
            let before = serde_json::to_value(persistence.load(&id)).ok();

            match apply_op(&mut coord, entry, &session_ids, &secrets, now_secs) {
                Ok(()) => {
                    accepted += 1;
                    last_applied.insert(
                        id.clone(),
                        LastApplied {
                            op: entry.op.name().to_string(),
                            before,
                            after: serde_json::to_value(persistence.load(&id)).ok(),
                        },
                    );
                    trace.push(format!(
                        "{step:04} t={now_secs} node={} op={} s={} ok",
                        envelope.from,
                        entry.op.name(),
                        entry.op.session()
                    ));
                }
                Err(error) => {
                    rejected += 1;
                    let after = serde_json::to_value(persistence.load(&id)).ok();
                    // A rejected call normally leaves the session untouched.
                    // Do not let it overwrite the last operation that really
                    // changed state, or a violation would be blamed on a call
                    // that was refused.
                    if before != after || !last_applied.contains_key(&id) {
                        last_applied.insert(
                            id.clone(),
                            LastApplied {
                                op: entry.op.name().to_string(),
                                before,
                                after,
                            },
                        );
                    }
                    trace.push(format!(
                        "{step:04} t={now_secs} node={} op={} s={} rejected: {error}",
                        envelope.from,
                        entry.op.name(),
                        entry.op.session()
                    ));
                }
            }

            // Judge after every delivered operation, not once per step. The
            // envelope that introduced a violation owns the before/after
            // snapshots; a later envelope in the same step must not rewrite
            // the story the packet tells.
            record_violations!(format!("{step:04}"), format!("step {step}"), step as u64);
        }

        record_violations!(format!("{step:04}"), format!("step {step}"), step as u64);
    }

    // ── Drain: let late messages land before judging the run ──────────────
    for _ in 0..40 {
        clock.advance(250);
        let now_ms = clock.now_ms();
        let now_secs = clock.now_secs();
        for envelope in net.deliver_ready(now_ms) {
            let entry = match ops.get(envelope.op_index) {
                Some(entry) => *entry,
                None => continue,
            };
            match apply_op(&mut coord, entry, &session_ids, &secrets, now_secs) {
                Ok(()) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }
    }

    record_violations!("drain", "drain", u64::MAX);

    // ── Judge ─────────────────────────────────────────────────────────────
    let mut final_sessions: Vec<SwapSession> = persistence.load_all().into_values().collect();
    final_sessions.sort_by(|a, b| a.session_id.cmp(&b.session_id));

    let sessions_completed = final_sessions
        .iter()
        .filter(|s| s.phase == SwapPhase::Complete)
        .count();
    let sessions_refunded = final_sessions
        .iter()
        .filter(|s| s.phase == SwapPhase::Refunded)
        .count();

    let mut state_hasher = blake3::Hasher::new();
    for session in &final_sessions {
        let encoded = serde_json::to_string(session).unwrap_or_default();
        state_hasher.update(encoded.as_bytes());
        state_hasher.update(b"\n");
    }

    let mut trace_hasher = blake3::Hasher::new();
    for line in &trace {
        trace_hasher.update(line.as_bytes());
        trace_hasher.update(b"\n");
    }

    SimOutcome {
        seed: config.seed,
        scenario: config.scenario.as_str().to_string(),
        // The effective sizes, not the requested ones: `run` clamps them
        // above, and a packet must replay the run that actually happened.
        sessions,
        steps: config.steps,
        nodes,
        accepted,
        rejected,
        restarts,
        stale_writes,
        partitions,
        sessions_completed,
        sessions_refunded,
        network: net.stats(),
        trace_digest: trace_hasher.finalize().to_hex().to_string(),
        state_digest: state_hasher.finalize().to_hex().to_string(),
        first_bad_step,
        first_bad_step_label: first_bad_step_label.unwrap_or_else(|| "none".to_string()),
        first_bad_op,
        active_faults,
        state_before,
        state_after,
        violations,
        trace,
    }
}

/// Pick the next intent.
///
/// The happy path walks each session's lifecycle in order so the run is a
/// meaningful positive control. The adversarial scenarios draw from a weighted
/// mix that deliberately schedules claims against refunds.
fn next_op(
    rng: &mut SimRng,
    scenario: Scenario,
    step: usize,
    sessions: usize,
    cursors: &mut [usize],
) -> Option<SimOp> {
    if scenario == Scenario::HappyPath {
        let sequence = [
            SimOp::LockFast { session: 0 },
            SimOp::LockSlow { session: 0 },
            SimOp::BeginFlash { session: 0 },
            SimOp::Settle { session: 0 },
            SimOp::ClaimFast { session: 0 },
            SimOp::ClaimSlow { session: 0 },
        ];
        for offset in 0..sessions {
            let index = (step + offset) % sessions;
            let cursor = cursors[index];
            if cursor >= sequence.len() {
                continue;
            }
            cursors[index] += 1;
            return Some(match sequence[cursor] {
                SimOp::LockFast { .. } => SimOp::LockFast { session: index },
                SimOp::LockSlow { .. } => SimOp::LockSlow { session: index },
                SimOp::BeginFlash { .. } => SimOp::BeginFlash { session: index },
                SimOp::Settle { .. } => SimOp::Settle { session: index },
                SimOp::ClaimFast { .. } => SimOp::ClaimFast { session: index },
                SimOp::ClaimSlow { .. } => SimOp::ClaimSlow { session: index },
                other => other,
            });
        }
        return None;
    }

    let session = rng.below(sessions);
    let roll = rng.below(100);
    let op = if roll < 25 {
        SimOp::LockFast { session }
    } else if roll < 45 {
        SimOp::LockSlow { session }
    } else if roll < 57 {
        SimOp::BeginFlash { session }
    } else if roll < 69 {
        SimOp::Settle { session }
    } else if roll < 81 {
        SimOp::ClaimFast { session }
    } else if roll < 89 {
        SimOp::ClaimSlow { session }
    } else if roll < 97 {
        SimOp::Abort { session }
    } else {
        SimOp::Refund { session }
    };
    Some(op)
}

/// Call the real coordinator. Success and refusal are both real outcomes.
fn apply_op(
    coord: &mut SwapCoordinator<InMemoryPersistence>,
    entry: OpEntry,
    session_ids: &[String],
    secrets: &[[u8; 32]],
    now_secs: u64,
) -> Result<(), String> {
    let session = entry.op.session();
    let id = session_ids
        .get(session)
        .ok_or_else(|| format!("session index {session} out of range"))?
        .clone();
    let secret = secrets[session];

    let result = match entry.op {
        SimOp::LockFast { .. } => coord
            .record_htlc_fast(
                &id,
                htlc_record(session, &secret, true, entry.created_secs),
                now_secs,
            )
            .map(|_| ()),
        SimOp::LockSlow { .. } => coord
            .record_htlc_slow(
                &id,
                htlc_record(session, &secret, false, entry.created_secs),
                now_secs,
            )
            .map(|_| ()),
        SimOp::BeginFlash { .. } => coord.begin_flash_execution(&id, now_secs),
        SimOp::Settle { .. } => coord.begin_settlement(&id, now_secs).map(|_| ()),
        SimOp::ClaimFast { .. } => coord
            .record_fast_claim(&id, HtlcSecret(secret), now_secs)
            .map(|_| ()),
        // The slow-chain claim carries no preimage: the secret was already
        // revealed by `record_fast_claim`, and this call records the slow leg
        // being claimed with it.
        SimOp::ClaimSlow { .. } => coord.record_slow_claim(&id, now_secs),
        SimOp::Abort { .. } => coord.abort(&id, "simulated abort", now_secs),
        SimOp::Refund { .. } => coord.record_refunds(&id, now_secs),
    };

    result.map_err(|error| error.to_string())
}
