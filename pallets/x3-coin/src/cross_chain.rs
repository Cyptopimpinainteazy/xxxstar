//! Consolidated cross-chain module for X3 Coin.
//!
//! This module delegates relayer and event pipeline operations to the
//! active storage-backed runtime helpers implemented in `lib.rs`.

use super::*;
use frame_support::dispatch::DispatchResult;
use sp_std::marker::PhantomData;

pub mod relayer {
    use super::*;

    pub type RelayerConfig<AccountId, Balance> = RelayerRuntimeConfig<AccountId, Balance>;

    #[derive(
        Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo,
    )]
    pub struct RelayerPath<Balance> {
        pub source_chain: u32,
        pub target_chain: u32,
        pub fee_bps: u32,
        pub max_fee: Balance,
    }

    pub struct RelayerRegistry<T: Config>(PhantomData<T>);

    impl<T: Config> RelayerRegistry<T> {
        pub fn register_relayer(config: RelayerConfig<T::AccountId, T::Balance>) -> DispatchResult {
            Pallet::<T>::register_relayer_config(
                config.relayer,
                config.enabled_chains,
                config.min_confirmations,
                config.max_gas_price,
            )?;
            Ok(())
        }

        pub fn get_relayer_config(
            relayer: &T::AccountId,
        ) -> Option<RelayerConfig<T::AccountId, T::Balance>> {
            Pallet::<T>::get_relayer_config_entry(relayer)
        }

        pub fn get_available_paths(
            source_chain: u32,
            target_chain: u32,
            operation_type: u8,
        ) -> Vec<RelayerPath<T::Balance>> {
            Pallet::<T>::get_available_relayer_paths(source_chain, target_chain, operation_type)
                .into_iter()
                .map(|(_relayer, fee_bps, max_fee)| RelayerPath {
                    source_chain,
                    target_chain,
                    fee_bps,
                    max_fee,
                })
                .collect()
        }
    }
}

pub mod events {
    use super::*;

    pub type CrossChainEvent = CrossChainRuntimeEvent;

    pub struct EventHandler<T: Config>(PhantomData<T>);

    impl<T: Config> EventHandler<T> {
        pub fn process_event(event: CrossChainEvent) -> DispatchResult {
            Pallet::<T>::process_cross_chain_event(
                event.operation_id,
                event.chain_id,
                event.event_type,
                event.timestamp,
                event.data,
            )?;
            Ok(())
        }

        pub fn get_event_history(chain_id: u32, limit: u32) -> Vec<CrossChainEvent> {
            Pallet::<T>::get_cross_chain_event_history(chain_id, limit)
        }
    }
}
