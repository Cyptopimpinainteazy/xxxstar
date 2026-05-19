//! Parametric inflation schedule with on-chain governance
//!
//! Defines a deflating inflation curve from year 1 (8%) → terminal (1.5%).
//! Governance can adjust parameters without runtime upgrade.
//!
//! Supports common schedules: linear decay, stepwise, exponential, or custom.

use sp_runtime::Permill;

/// Inflation curve type
#[derive(Clone, Debug)]
pub enum InflationCurve {
    /// Linear decay: start_rate → end_rate over N years
    Linear {
        start_rate: Permill,
        end_rate: Permill,
        years_to_terminal: u32,
    },
    /// Stepwise: fixed rate for X years, then drop to next
    Stepwise(Vec<(u32, Permill)>), // (year_until, rate)
    /// Exponential: rate * decay_factor^year
    Exponential {
        initial_rate: Permill,
        decay_factor: f64,
    },
    /// Custom parametric function (governance can update coefficients)
    Custom {
        coefficients: Vec<f64>,
    },
}

impl InflationCurve {
    /// Default X3 curve: 8% → 1.5% linear over 4 years
    pub fn x3_default() -> Self {
        Self::Linear {
            start_rate: Permill::from_percent(8),
            end_rate: Permill::from_percent(1),
            years_to_terminal: 4,
        }
    }

    /// Get inflation rate for a given year
    pub fn rate_at_year(&self, year: u32) -> Permill {
        match self {
            Self::Linear {
                start_rate,
                end_rate,
                years_to_terminal,
            } => {
                if year >= *years_to_terminal {
                    *end_rate
                } else {
                    let start_parts = start_rate.deconstruct() as f64;
                    let end_parts = end_rate.deconstruct() as f64;
                    let decay_per_year = (start_parts - end_parts) / (*years_to_terminal as f64);
                    let rate_parts = start_parts - (decay_per_year * year as f64);
                    Permill::from_parts(rate_parts as u32)
                }
            }

            Self::Stepwise(steps) => {
                steps
                    .iter()
                    .find(|(y, _)| year < *y)
                    .map(|(_, rate)| *rate)
                    .unwrap_or(steps.last().map(|(_, r)| *r).unwrap_or_default())
            }

            Self::Exponential {
                initial_rate,
                decay_factor,
            } => {
                let decayed = initial_rate.deconstruct() as f64 * decay_factor.powi(year as i32);
                Permill::from_parts(decayed as u32)
            }

            Self::Custom { coefficients } => {
                // Polynomial: c0 + c1*year + c2*year^2 + ...
                let mut rate = coefficients.get(0).cloned().unwrap_or(0.0);
                for (i, coef) in coefficients.iter().enumerate().skip(1) {
                    rate += coef * (year as f64).powi(i as i32);
                }
                Permill::from_parts((rate as u32).min(1_000_000))
            }
        }
    }

    /// Block-level inflation (annual rate / blocks per year)
    pub fn block_inflation(&self, year: u32, blocks_per_year: u32) -> Permill {
        let annual = self.rate_at_year(year);
        let per_block = (annual.deconstruct() as u64) / (blocks_per_year as u64);
        Permill::from_parts(per_block as u32)
    }
}

/// Inflation schedule state machine
#[derive(Clone, Debug)]
pub struct InflationSchedule {
    /// Current curve
    pub curve: InflationCurve,
    /// Current year (0 = genesis)
    pub current_year: u32,
    /// Total tokens minted so far
    pub total_minted: u128,
    /// Mint history by year: (year, amount_minted)
    pub year_history: Vec<(u32, u128)>,
    /// Governor who can update curve
    pub governor: String,
}

impl InflationSchedule {
    /// Create schedule with default X3 curve
    pub fn new(governor: String) -> Self {
        Self {
            curve: InflationCurve::x3_default(),
            current_year: 0,
            total_minted: 0,
            year_history: Vec::new(),
            governor,
        }
    }

    /// Calculate mint for this block
    pub fn mint_this_block(
        &self,
        total_supply: u128,
        blocks_per_year: u32,
    ) -> u128 {
        let block_rate = self.curve.block_inflation(self.current_year, blocks_per_year);
        (total_supply as u64).saturating_mul(block_rate.deconstruct() as u64) / 1_000_000
            as u128
    }

    /// Advance year and record mint
    pub fn advance_year(&mut self, amount_minted_this_year: u128) {
        self.year_history.push((self.current_year, amount_minted_this_year));
        self.total_minted = self.total_minted.saturating_add(amount_minted_this_year);
        self.current_year = self.current_year.saturating_add(1);
    }

    /// Governance: update inflation curve
    pub fn update_curve(&mut self, new_curve: InflationCurve, caller: &str) -> bool {
        if caller != self.governor {
            return false;
        }

        self.curve = new_curve;
        true
    }

    /// Get projected supply at end of year N
    pub fn projected_supply_at_year(&self, year: u32, initial_supply: u128) -> u128 {
        let mut supply = initial_supply;
        for y in 0..year {
            let rate = self.curve.rate_at_year(y);
            let yearly_mint = (supply as u64).saturating_mul(rate.deconstruct() as u64) / 1_000_000;
            supply = supply.saturating_add(yearly_mint as u128);
        }
        supply
    }

    /// Terminal rate verification (ensures it converges)
    pub fn terminal_rate(&self) -> Permill {
        // Evaluate at year 100 (should be stable by then)
        self.curve.rate_at_year(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_decay_progression() {
        let curve = InflationCurve::x3_default();

        let year0 = curve.rate_at_year(0);
        let year2 = curve.rate_at_year(2);
        let year4 = curve.rate_at_year(4);

        assert_eq!(year0, Permill::from_percent(8));
        assert!(year2 < year0);
        assert_eq!(year4, Permill::from_percent(1));
    }

    #[test]
    fn test_stepwise_inflation() {
        let curve = InflationCurve::Stepwise(vec![
            (2, Permill::from_percent(5)),
            (4, Permill::from_percent(3)),
            (100, Permill::from_percent(1)),
        ]);

        assert_eq!(curve.rate_at_year(0), Permill::from_percent(5));
        assert_eq!(curve.rate_at_year(2), Permill::from_percent(3));
        assert_eq!(curve.rate_at_year(5), Permill::from_percent(1));
    }

    #[test]
    fn test_block_inflation_calculation() {
        let curve = InflationCurve::x3_default();
        let block_rate = curve.block_inflation(0, 6_000_000); // 6M blocks/year

        // Year 0: 8% annual ÷ 6M blocks ≈ 1.33 parts per million per block
        assert!(block_rate.deconstruct() > 0);
        assert!(block_rate.deconstruct() < Permill::from_percent(1).deconstruct());
    }

    #[test]
    fn test_schedule_state_machine() {
        let mut schedule = InflationSchedule::new("governance".to_string());

        assert_eq!(schedule.current_year, 0);

        schedule.advance_year(800_000);
        assert_eq!(schedule.current_year, 1);
        assert_eq!(schedule.total_minted, 800_000);

        schedule.advance_year(750_000);
        assert_eq!(schedule.total_minted, 1_550_000);
    }

    #[test]
    fn test_curve_update_requires_governance() {
        let mut schedule = InflationSchedule::new("gov".to_string());
        let new_curve = InflationCurve::Linear {
            start_rate: Permill::from_percent(5),
            end_rate: Permill::from_percent(0),
            years_to_terminal: 5,
        };

        assert!(!schedule.update_curve(new_curve.clone(), "attacker"));
        assert!(schedule.update_curve(new_curve, "gov"));
    }

    #[test]
    fn test_projected_supply() {
        let schedule = InflationSchedule::new("gov".to_string());
        let initial = 1_000_000_000u128;

        let year5_supply = schedule.projected_supply_at_year(5, initial);
        assert!(year5_supply > initial); // Inflation happened
    }

    #[test]
    fn test_terminal_convergence() {
        let schedule = InflationSchedule::new("gov".to_string());
        let terminal = schedule.terminal_rate();

        // X3 default terminal should be low (≤1.5%)
        assert!(terminal <= Permill::from_percent(2));
    }

    #[test]
    fn test_exponential_decay() {
        let curve = InflationCurve::Exponential {
            initial_rate: Permill::from_percent(8),
            decay_factor: 0.8,
        };

        let year0 = curve.rate_at_year(0);
        let year1 = curve.rate_at_year(1);

        assert!(year1 < year0); // Decays each year
    }

    #[test]
    fn test_mint_calculation() {
        let schedule = InflationSchedule::new("gov".to_string());
        let total_supply = 1_000_000_000u128;

        let mint = schedule.mint_this_block(total_supply, 6_000_000);
        assert!(mint > 0); // Some inflation
    }
}
