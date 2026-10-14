//! Fault schedule.
//!
//! Faults are chosen from the same seeded stream as the workload and stored as
//! a sorted plan, so "when does the node crash" is part of what `--seed` pins
//! down.

use crate::network::NodeId;
use crate::params::SimParams;
use crate::rng::SimRng;
use crate::Scenario;

/// One injected fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultKind {
    /// Cut all links between two groups of nodes.
    Partition {
        group_a: Vec<NodeId>,
        group_b: Vec<NodeId>,
    },
    /// Restore all links.
    Heal,
    /// The coordinator process dies and restarts from persistence.
    CrashAndRestart { duration_ms: u64 },
    /// Degrade every link.
    SlowLink {
        latency_ms: u64,
        extra_drop_percent: u32,
    },
    /// The most recent persisted write for one session disappears.
    StaleWrite { session: usize },
}

#[derive(Debug, Clone)]
pub struct FaultEvent {
    pub at_ms: u64,
    pub kind: FaultKind,
    pub fired: bool,
}

/// A time-ordered fault plan.
#[derive(Debug, Clone)]
pub struct FaultPlan {
    events: Vec<FaultEvent>,
}

impl FaultPlan {
    /// Build the plan for a scenario.
    ///
    /// `horizon_ms` is the expected length of the run; faults land inside it so
    /// that every scenario actually experiences what it advertises.
    pub fn generate(
        scenario: Scenario,
        rng: &mut SimRng,
        clients: usize,
        sessions: usize,
        start_ms: u64,
        horizon_ms: u64,
    ) -> Self {
        Self::generate_with(
            scenario,
            &SimParams::for_scenario(scenario),
            rng,
            clients,
            sessions,
            start_ms,
            horizon_ms,
        )
    }

    /// Build the plan at an arbitrary point in the parameter space.
    ///
    /// `SimParams::for_scenario` reproduces the original fixed plans exactly,
    /// so this is a strict generalization: the number of faults, their
    /// placement and their intensity all come from `params`.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_with(
        scenario: Scenario,
        params: &SimParams,
        rng: &mut SimRng,
        clients: usize,
        sessions: usize,
        start_ms: u64,
        horizon_ms: u64,
    ) -> Self {
        let mut events: Vec<FaultEvent> = Vec::new();
        let mut push = |at_ms: u64, kind: FaultKind| {
            events.push(FaultEvent {
                at_ms,
                kind,
                fired: false,
            });
        };

        let window = |frac_percent: u64| start_ms + (horizon_ms * frac_percent) / 100;
        // Spread N events across the run instead of stacking them at the front:
        // a fault that always lands in the first tenth only ever tests the
        // opening state.
        let spread = |count: usize, index: usize| -> u64 {
            if count == 0 {
                0
            } else {
                (index as u64 * 100) / (count as u64 + 1)
            }
        };

        // Every fault class is scheduled from `params`, whatever the scenario
        // shape. An earlier version only honoured crash and stale-write counts
        // for `CrashRecovery`, so a campaign that added crashes to a
        // `ClaimRefundRace` base sampled the variable, produced no crash, and
        // reported a clean sweep over a fault it never injected. The ladders
        // below reproduce each scenario's original placement exactly at its
        // default point; the scenario shape now only supplies those defaults.
        // `HappyPath` stays the positive control and schedules nothing.
        if scenario == Scenario::HappyPath {
            return Self { events };
        }

        // ── partitions ────────────────────────────────────────────────────
        let partition_ladder = [12u64, 30, 52, 74, 88];
        for (index, position) in (0..params.partition_events).enumerate() {
            // A single partition on the race scenario keeps its original 40%
            // placement; everything else follows the ladder, then spreads.
            let frac = if scenario == Scenario::ClaimRefundRace && params.partition_events == 1 {
                40
            } else {
                partition_ladder
                    .get(index)
                    .copied()
                    .unwrap_or_else(|| spread(params.partition_events, index))
            };
            let cut = window(frac);
            push(
                cut,
                FaultKind::Partition {
                    group_a: vec![0],
                    group_b: (1..=clients).collect(),
                },
            );
            let heal_after = if scenario == Scenario::ClaimRefundRace {
                horizon_ms / 10
            } else {
                horizon_ms / 20 + (position as u64) * 7
            };
            push(cut + heal_after, FaultKind::Heal);
        }

        // ── degraded links ────────────────────────────────────────────────
        if params.slow_link_events >= 1 {
            push(
                window(20),
                FaultKind::SlowLink {
                    latency_ms: 2_500,
                    extra_drop_percent: 20,
                },
            );
        }
        if params.slow_link_events >= 2 {
            push(
                window(65),
                FaultKind::SlowLink {
                    latency_ms: 25,
                    extra_drop_percent: 0,
                },
            );
        }

        // ── crashes ───────────────────────────────────────────────────────
        let crash_ladder = [25u64, 60, 80];
        for index in 0..params.crash_events {
            let frac = crash_ladder
                .get(index)
                .copied()
                .unwrap_or_else(|| spread(params.crash_events, index));
            // The first crash is the configured one; later crashes are half as
            // long, matching the original plan.
            let duration = if index == 0 {
                params.crash_duration_ms
            } else {
                (params.crash_duration_ms / 2).max(1)
            };
            push(
                window(frac),
                FaultKind::CrashAndRestart {
                    duration_ms: duration,
                },
            );
        }

        // ── lost persisted writes ─────────────────────────────────────────
        for index in 0..params.stale_write_events {
            if sessions == 0 {
                break;
            }
            let frac = if params.stale_write_events == 1 {
                45
            } else {
                spread(params.stale_write_events, index)
            };
            push(
                window(frac),
                FaultKind::StaleWrite {
                    session: rng.below(sessions),
                },
            );
        }

        events.sort_by_key(|e| e.at_ms);
        Self { events }
    }

    /// Take every event due at or before `now_ms`, marking them fired.
    pub fn due(&mut self, now_ms: u64) -> Vec<FaultKind> {
        let mut out = Vec::new();
        for event in self.events.iter_mut() {
            if !event.fired && event.at_ms <= now_ms {
                event.fired = true;
                out.push(event.kind.clone());
            }
        }
        out
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// The scheduled events, in the order they will fire.
    ///
    /// Exposed so a fixture can pin the plan itself rather than only its
    /// consequences: a refactor that quietly moves a fault earlier changes what
    /// every seeded run means.
    pub fn events(&self) -> &[FaultEvent] {
        &self.events
    }
}
