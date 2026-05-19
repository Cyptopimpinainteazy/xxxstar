use lazy_static::lazy_static;
use prometheus::{Counter, Opts, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref TRADES_EXECUTED: Counter = Counter::with_opts(Opts::new(
        "trades_executed",
        "Total number of trades executed"
    ))
    .unwrap();
    pub static ref ARB_OPPORTUNITIES: Counter = Counter::with_opts(Opts::new(
        "arb_opportunities",
        "Total arbitrage opportunities detected"
    ))
    .unwrap();
    pub static ref ATOMIC_SWAPS_STARTED: Counter = Counter::with_opts(Opts::new(
        "atomic_swaps_started",
        "Total atomic swaps initiated"
    ))
    .unwrap();
    pub static ref ATOMIC_SWAPS_SUCCESS: Counter = Counter::with_opts(Opts::new(
        "atomic_swaps_success",
        "Total atomic swaps successfully committed"
    ))
    .unwrap();
    pub static ref ATOMIC_SWAPS_FAILED: Counter = Counter::with_opts(Opts::new(
        "atomic_swaps_failed",
        "Total atomic swaps rolled back"
    ))
    .unwrap();
}

pub fn init() {
    REGISTRY
        .register(Box::new(TRADES_EXECUTED.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(ARB_OPPORTUNITIES.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(ATOMIC_SWAPS_STARTED.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(ATOMIC_SWAPS_SUCCESS.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(ATOMIC_SWAPS_FAILED.clone()))
        .unwrap();
}
