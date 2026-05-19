#![allow(dead_code)] // intentional scoring scaffold; tracked in readiness backlog

// Scoring module for proof score calculation
// Formula: 15% compile + 15% unit + 20% integration + 20% invariants + 15% adversarial + 5% benchmarks + 5% wiring + 5% drift

pub mod formula;

/// Scoring context with all measurements
#[derive(Debug, Clone)]
pub struct ScoringContext {
    pub compile_score: f64,
    pub unit_tests_score: f64,
    pub integration_score: f64,
    pub invariant_score: f64,
    pub adversarial_score: f64,
    pub benchmark_score: f64,
    pub wiring_score: f64,
    pub drift_score: f64,
}

impl Default for ScoringContext {
    fn default() -> Self {
        Self {
            compile_score: 1.0,
            unit_tests_score: 1.0,
            integration_score: 1.0,
            invariant_score: 1.0,
            adversarial_score: 1.0,
            benchmark_score: 1.0,
            wiring_score: 1.0,
            drift_score: 1.0,
        }
    }
}

impl ScoringContext {
    /// Calculate final proof score using the 8-component formula
    pub fn calculate_score(&self) -> f64 {
        0.15 * self.compile_score
            + 0.15 * self.unit_tests_score
            + 0.20 * self.integration_score
            + 0.20 * self.invariant_score
            + 0.15 * self.adversarial_score
            + 0.05 * self.benchmark_score
            + 0.05 * self.wiring_score
            + 0.05 * self.drift_score
    }

    /// Normalize all scores to 0-1 range
    pub fn normalize(&mut self) {
        self.compile_score = self.compile_score.clamp(0.0, 1.0);
        self.unit_tests_score = self.unit_tests_score.clamp(0.0, 1.0);
        self.integration_score = self.integration_score.clamp(0.0, 1.0);
        self.invariant_score = self.invariant_score.clamp(0.0, 1.0);
        self.adversarial_score = self.adversarial_score.clamp(0.0, 1.0);
        self.benchmark_score = self.benchmark_score.clamp(0.0, 1.0);
        self.wiring_score = self.wiring_score.clamp(0.0, 1.0);
        self.drift_score = self.drift_score.clamp(0.0, 1.0);
    }
}

/// Scoring grade (A-F scale with + variants)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScoreGrade {
    APlus,  // 0.98-1.00
    A,      // 0.93-0.97
    AMinus, // 0.88-0.92
    BPlus,  // 0.83-0.87
    B,      // 0.78-0.82
    BMinus, // 0.73-0.77
    CPlus,  // 0.68-0.72
    C,      // 0.63-0.67
    D,      // 0.50-0.62
    F,      // 0.00-0.49
}

impl ScoreGrade {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.98 => ScoreGrade::APlus,
            s if s >= 0.93 => ScoreGrade::A,
            s if s >= 0.88 => ScoreGrade::AMinus,
            s if s >= 0.83 => ScoreGrade::BPlus,
            s if s >= 0.78 => ScoreGrade::B,
            s if s >= 0.73 => ScoreGrade::BMinus,
            s if s >= 0.68 => ScoreGrade::CPlus,
            s if s >= 0.63 => ScoreGrade::C,
            s if s >= 0.50 => ScoreGrade::D,
            _ => ScoreGrade::F,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ScoreGrade::APlus => "A+",
            ScoreGrade::A => "A",
            ScoreGrade::AMinus => "A-",
            ScoreGrade::BPlus => "B+",
            ScoreGrade::B => "B",
            ScoreGrade::BMinus => "B-",
            ScoreGrade::CPlus => "C+",
            ScoreGrade::C => "C",
            ScoreGrade::D => "D",
            ScoreGrade::F => "F",
        }
    }

    pub fn is_passing(&self) -> bool {
        *self >= ScoreGrade::C
    }
}
