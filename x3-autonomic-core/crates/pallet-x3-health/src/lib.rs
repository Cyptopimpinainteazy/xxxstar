//! Pallet X3 Health
//! 
//! Substrate pallet for health monitoring on X3 chain

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use x3_autonomic_types::{HealthStatus, HealthMetricDefinition};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Configuration for this pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Storage for registered health metrics
    #[pallet::storage]
    #[pallet::getter(fn health_metrics)]
    pub type HealthMetrics<T> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        HealthMetricDefinition,
        OptionQuery,
    >;

    /// Current overall health status
    #[pallet::storage]
    #[pallet::getter(fn overall_health)]
    pub type OverallHealth<T> = StorageValue<_, HealthStatus, ValueQuery>;

    /// Last health check timestamp
    #[pallet::storage]
    #[pallet::getter(fn last_check)]
    pub type LastHealthCheck<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T> {
        /// Health metric registered
        MetricRegistered { id: Vec<u8> },
        /// Health status changed
        HealthChanged { old: HealthStatus, new: HealthStatus },
        /// Health check completed
        HealthCheckCompleted { status: HealthStatus },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Metric already exists
        MetricExists,
        /// Metric not found
        MetricNotFound,
        /// Not authorized
        NotAuthorized,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new health metric
        #[pallet::call_index(0)]
        pub fn register_metric(
            origin: OriginFor<T>,
            metric: HealthMetricDefinition,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            ensure!(
                !HealthMetrics::<T>::contains_key(&metric.id),
                Error::<T>::MetricExists
            );
            HealthMetrics::<T>::insert(&metric.id, &metric);
            Self::deposit_event(Event::MetricRegistered { id: metric.id });
            Ok(())
        }

        /// Update overall health status
        #[pallet::call_index(1)]
        pub fn update_health(
            origin: OriginFor<T>,
            status: HealthStatus,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let old = OverallHealth::<T>::get();
            OverallHealth::<T>::set(status.clone());
            LastHealthCheck::<T>::set(<frame_system::Pallet<T>>::block_number().saturated_into());
            Self::deposit_event(Event::HealthChanged { old, new: status });
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Perform a health check across all metrics
    pub fn perform_health_check() -> HealthStatus {
        // Simplified - real implementation would aggregate all metrics
        HealthStatus::Healthy
    }

    /// Get health status for a specific metric
    pub fn get_metric_status(id: &[u8]) -> Option<HealthStatus> {
        HealthMetrics::<T>::get(id).map(|m| m.status)
    }
}