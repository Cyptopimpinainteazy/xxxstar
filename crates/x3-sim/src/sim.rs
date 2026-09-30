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

use serde::Serialize;
use x3_cross_vm_coordinator::{
    CoordinatorConfig, HtlcCreateParams, HtlcId, HtlcRecord, HtlcSecret, HtlcStatus,
    InMemoryPersistence, SessionPersistence, SwapCoordinator, SwapPhase, SwapSession, VmTarget,
};

use crate::clock::VirtualClock;
use crate::faults::{FaultKind, FaultPlan};
use crate::invariants::{check_sessions, Violation};
use crate::network::{NetworkStats, VirtualNetwork};
use crate::params::SimParams;
use crate::rng::SimRng;

/// Fixed start instant: 2023-11-14T22:13:20Z. A constant, not a clock reading.
pub const START_UNIX_MS: u64 = 1_700_000_000_000;

const FAST_TIMELOCK_SECS: u64 = 3_600;
const SLOW_TIMELOCK_SECS: u64 = 7_200;

/// Which adversarial schedule to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
    pub steps: usize,
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
    pub violations: Vec<Violation>,
    #[serde(skip)]
    pub trace: Vec<String>,
}

impl SimOutcome {
    pub fn is_pass(&self) -> bool {
        self.violations.is_empty()
    }

    /// The `cargo run` line that reproduces this exact run.
    pub fn replay_command(&self) -> String {
        format!(
            "cargo run -p x3-sim -- --seed {} --scenario {}",
            self.seed, self.scenario
        )
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

fn coordinator(
    persistence: &Arc<InMemoryPersistence>,
) -> SwapCoordinator<InMemoryPersistence> {
    // `with_persistence`, not `new`: the latter deliberately panics outside
    // `cfg(test)` because non-durable persistence loses funds on a crash.
    SwapCoordinator::with_persistence(CoordinatorConfig::default(), Arc::clone(persistence))
}

/// Drive the real coordinator and report what happened.
pub fn run(config: &SimConfig) -> SimOutcome {
    run_with(config, &SimParams::for_scenario(config.scenario))
}

/// Drive the real coordinator at an explicitly chosen point in the parameter
/// space.
///
/// `run` is this function at the scenario's default point, so every existing
/// fixture keeps exercising exactly what it did before. A search supplies its
/// own [`SimParams`]; the semantics being tested still come entirely from the
/// coordinator crate.
pub fn run_with(config: &SimConfig, params: &SimParams) -> SimOutcome {
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
    let mut net = VirtualNetwork::new(
        nodes,
        params.latency_ms,
        params.jitter_ms,
        params.drop_percent,
    );
    let horizon_ms = (config.steps as u64).saturating_mul(500).max(1_000);
    let mut plan = FaultPlan::generate_with(
        config.scenario,
        params,
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

    macro_rules! record_violations {
        ($step:expr) => {{
            let mut sessions_now: Vec<SwapSession> = persistence.load_all().into_values().collect();
            sessions_now.sort_by(|a, b| a.session_id.cmp(&b.session_id));
            let mut found = check_sessions(&sessions_now);
            if !found.is_empty() {
                found.retain(|candidate| !violations.contains(candidate));
                for item in &found {
                    trace.push(format!(
                        "{} INVARIANT {} session={} {}",
                        $step, item.code, item.session_id, item.detail
                    ));
                }
                violations.extend(found);
            }
        }};
    }

    for step in 0..config.steps {
        // Move time, then let whatever is due happen.
        clock.advance(
            params.clock_step_ms
                + if params.clock_jitter_ms > 0 {
                    rng.next_u64() % params.clock_jitter_ms
                } else {
                    0
                },
        );
        let now_ms = clock.now_ms();
        let now_secs = clock.now_secs();

        for fault in plan.due(now_ms) {
            match fault {
                FaultKind::Partition { group_a, group_b } => {
                    net.partition(&group_a, &group_b);
                    partitions += 1;
                    trace.push(format!("{step:04} FAULT partition {group_a:?}|{group_b:?}"));
                }
                FaultKind::Heal => {
                    net.heal_all();
                    trace.push(format!("{step:04} FAULT heal"));
                }
                FaultKind::SlowLink {
                    latency_ms,
                    extra_drop_percent,
                } => {
                    net.set_latency(latency_ms, latency_ms / 4);
                    net.set_drop_percent(extra_drop_percent);
                    trace.push(format!(
                        "{step:04} FAULT slow-link latency={latency_ms}ms drop={extra_drop_percent}%"
                    ));
                }
                FaultKind::CrashAndRestart { duration_ms } => {
                    let dropped = net.drop_in_flight_to(0);
                    coord = coordinator(&persistence);
                    restarts += 1;
                    node_down_until_ms = now_ms + duration_ms;
                    trace.push(format!(
                        "{step:04} FAULT crash-restart down_for={duration_ms}ms dropped={dropped}"
                    ));
                }
                FaultKind::StaleWrite { session } => {
                    if let Some(id) = session_ids.get(session) {
                        if let Some(previous) = shadow.get(id).cloned() {
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
            if rng.chance(params.duplicate_percent) {
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
            shadow.insert(id.clone(), persistence.load(&id).unwrap_or_else(|| {
                panic!("session {id} vanished from persistence")
            }));

            match apply_op(&mut coord, entry, &session_ids, &secrets, now_secs) {
                Ok(()) => {
                    accepted += 1;
                    trace.push(format!(
                        "{step:04} t={now_secs} node={} op={} s={} ok",
                        envelope.from,
                        entry.op.name(),
                        entry.op.session()
                    ));
                }
                Err(error) => {
                    rejected += 1;
                    trace.push(format!(
                        "{step:04} t={now_secs} node={} op={} s={} rejected: {error}",
                        envelope.from,
                        entry.op.name(),
                        entry.op.session()
                    ));
                }
            }
        }

        record_violations!(step);
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

    record_violations!("drain");

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
        steps: config.steps,
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
