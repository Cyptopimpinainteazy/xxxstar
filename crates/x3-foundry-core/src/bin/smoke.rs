use x3_foundry_core::{oracle::default_pyth_oracle, revenue_bridge::default_fee_config};

fn main() {
    let oracle = default_pyth_oracle();
    let fees = default_fee_config();
    println!(
        "x3-foundry smoke: {fees:?} with {} registered feeds",
        oracle.price_feeds.len()
    );
}
