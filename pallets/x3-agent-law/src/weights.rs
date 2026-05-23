use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_MILLIS, Weight};

pub trait WeightInfo {
    fn register_policy() -> Weight;
    fn slash_agent() -> Weight;
    fn remove_blacklist() -> Weight;
}

pub struct SubstrateWeight;

impl WeightInfo for SubstrateWeight {
    /// Weight for registering a policy (16 policies max)
    /// Includes StorageMap inserts + event emission
    fn register_policy() -> Weight {
        Weight::from_parts(45_000 * WEIGHT_REF_TIME_PER_MILLIS, 8000)
    }

    /// Weight for slashing an agent
    /// Includes reputation update + violation tracking + auto-enforcement check
    fn slash_agent() -> Weight {
        Weight::from_parts(28_000 * WEIGHT_REF_TIME_PER_MILLIS, 5000)
    }

    /// Weight for removing blacklist
    /// Includes StorageMap deletion
    fn remove_blacklist() -> Weight {
        Weight::from_parts(18_000 * WEIGHT_REF_TIME_PER_MILLIS, 3000)
    }
}

impl WeightInfo for () {
    fn register_policy() -> Weight {
        Weight::from_parts(45_000, 8000)
    }

    fn slash_agent() -> Weight {
        Weight::from_parts(28_000, 5000)
    }

    fn remove_blacklist() -> Weight {
        Weight::from_parts(18_000, 3000)
    }
}
