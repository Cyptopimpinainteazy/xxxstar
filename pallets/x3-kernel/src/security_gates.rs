use crate::pallet;
use frame_support::{pallet_prelude::TransactionSource, weights::Weight};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
    impl_tx_ext_default,
    traits::{DispatchInfoOf, Dispatchable, TransactionExtension, ValidateResult},
    transaction_validity::InvalidTransaction,
};
use sp_std::{fmt, marker::PhantomData};

/// Custom `InvalidTransaction` code emitted by the kernel security gates.
/// All gates are fail-closed and will be split into individual signed
/// extensions once the respective subsystems are wired in RC+1.
pub const KERNEL_SECURITY_GATE_REJECT_CODE: u8 = 200;

/// Composite signed extension that bundles three kernel-level security gates
/// that are currently fail-closed:
///
/// 1. **CapabilityEnvelopeCheck** — validates cross-VM messages have a
///    signed capability envelope. Wired in RC+1.
/// 2. **AtomicSettlementCheck** — enforces all cross-VM Comits are
///    atomically settled on both sides. Wired in RC+1.
/// 3. **FlashFinalityExtension** — commits tx ordering for relay trust
///    windows. Wired in RC+1.
///
/// All three gates are combined into a single signed extension to keep the
/// `SignedExtra` tuple within Rust's 12-element trait-implementation limit.
/// They will be split into `CapabilityEnvelopeCheck<T>`,
/// `AtomicSettlementCheck<T>`, and `FlashFinalityExtension<T>` once
/// individual subsystems exist.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct KernelSecurityGates<T: pallet::Config + Send + Sync + 'static>(PhantomData<T>);

impl<T: pallet::Config + Send + Sync + 'static> Default for KernelSecurityGates<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: pallet::Config + Send + Sync + 'static> fmt::Debug for KernelSecurityGates<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KernelSecurityGates(CapabilityEnvelope|AtomicSettlement|FlashFinality)"
        )
    }
}

impl<T: pallet::Config + Send + Sync + 'static> TransactionExtension<T::RuntimeCall>
    for KernelSecurityGates<T>
where
    T::RuntimeCall: Dispatchable,
{
    const IDENTIFIER: &'static str = "KernelSecurityGates";

    type Implicit = ();
    type Val = ();
    type Pre = ();

    fn weight(&self, _call: &T::RuntimeCall) -> Weight {
        Weight::zero()
    }

    fn validate(
        &self,
        _origin: <T::RuntimeCall as Dispatchable>::RuntimeOrigin,
        _call: &T::RuntimeCall,
        _info: &DispatchInfoOf<T::RuntimeCall>,
        _len: usize,
        _self_implicit: Self::Implicit,
        _inherited_implication: &impl Encode,
        _source: TransactionSource,
    ) -> ValidateResult<Self::Val, T::RuntimeCall> {
        Err(InvalidTransaction::Custom(KERNEL_SECURITY_GATE_REJECT_CODE).into())
    }

    impl_tx_ext_default!(T::RuntimeCall; prepare);
}

/// Type aliases for the individual gates — available for future use when
/// each subsystem is wired (RC+1).  Today all three are backed by the
/// same fail-closed `KernelSecurityGates` implementation.
pub use KernelSecurityGates as CapabilityEnvelopeCheck;
pub use KernelSecurityGates as AtomicSettlementCheck;
pub use KernelSecurityGates as FlashFinalityExtension;
