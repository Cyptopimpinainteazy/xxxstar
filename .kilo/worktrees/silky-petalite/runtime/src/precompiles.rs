use core::marker::PhantomData;
use pallet_evm::{
    IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult, PrecompileSet,
};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use sp_core::H160;

// ─── Precompile Gas Cost Constants ────────────────────────────────────────────
//
// RFC t5-3: These are *conservative initial estimates* derived from Ethereum Yellow Paper
// appendix E (EIPs 152, 196, 197) and empirical measurements on testnet hardware.
//
// TODO(benchmarking): Replace with values produced by `frame-benchmarking` pallet once
// the `pallet-evm-precompile-*` benchmarks are wired into the runtime's `WeightInfo`.
// Governance council vote required before changing these values on mainnet.
// See: docs/rfc/RFC-t5-3-precompile-gas-addresses.md

/// ECRecover — validates signature, fixed base cost (EIP-2)
pub const GAS_ECRECOVER: u64 = 3_000;

/// SHA-256 — 60 base + 12/word (here we charge base; per-word charged by execute())
pub const GAS_SHA256_BASE: u64 = 60;

/// RIPEMD-160 — 600 base + 120/word (same — base only at is_precompile)
pub const GAS_RIPEMD160_BASE: u64 = 600;

/// Identity — 15 base + 3/word (negligible — base only)
pub const GAS_IDENTITY_BASE: u64 = 15;

/// MODEXP (EIP-198) — highly variable; 200 minimum
pub const GAS_MODEXP_MIN: u64 = 200;

/// SHA3-FIPS256 — ~same cost as SHA-256
pub const GAS_SHA3FIPS256_BASE: u64 = 60;

/// ECRecoverPublicKey — similar to ECRecover
pub const GAS_ECRECOVER_PUBKEY: u64 = 3_000;

pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn used_addresses() -> [H160; 7] {
        [
            hash(1),
            hash(2),
            hash(3),
            hash(4),
            hash(5),
            hash(1024),
            hash(1025),
        ]
    }

    /// Returns the conservative extra_cost estimate for a given precompile address.
    ///
    /// Callers: `is_precompile()` — used by the EVM to pre-charge an access cost.
    /// TODO(benchmarking): Swap for measured values. See RFC t5-3.
    pub fn extra_cost_for(address: H160) -> u64 {
        match address {
            a if a == hash(1) => GAS_ECRECOVER,
            a if a == hash(2) => GAS_SHA256_BASE,
            a if a == hash(3) => GAS_RIPEMD160_BASE,
            a if a == hash(4) => GAS_IDENTITY_BASE,
            a if a == hash(5) => GAS_MODEXP_MIN,
            a if a == hash(1024) => GAS_SHA3FIPS256_BASE,
            a if a == hash(1025) => GAS_ECRECOVER_PUBKEY,
            _ => 0,
        }
    }
}

impl<R> Default for FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R> PrecompileSet for FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            // Ethereum precompiles
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            // X3 specific extras
            a if a == hash(1024) => Some(Sha3FIPS256::<R, ()>::execute(handle)),
            a if a == hash(1025) => Some(ECRecoverPublicKey::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        let is_precompile = Self::used_addresses().contains(&address);
        IsPrecompileResult::Answer {
            is_precompile,
            // Charge a conservative access cost so the EVM budgets correctly.
            // Zero for unknown addresses (not a precompile → normal call pricing applies).
            extra_cost: if is_precompile { Self::extra_cost_for(address) } else { 0 },
        }
    }
}

fn hash(id: u64) -> H160 {
    H160::from_low_u64_be(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Address registry tests ────────────────────────────────────────────────

    #[test]
    fn known_addresses_are_precompiles() {
        // All 7 registered addresses must be recognised by used_addresses()
        let registered = FrontierPrecompiles::<()>::used_addresses();
        for &id in &[1u64, 2, 3, 4, 5, 1024, 1025] {
            let addr = H160::from_low_u64_be(id);
            assert!(
                registered.contains(&addr),
                "Address {} must be in used_addresses()", id,
            );
        }
    }

    #[test]
    fn unknown_addresses_are_not_precompiles() {
        let strangers = [0u64, 6, 100, 1023, 1026, 65535, u64::MAX];
        for id in strangers {
            let addr = H160::from_low_u64_be(id);
            assert!(
                !FrontierPrecompiles::<()>::used_addresses().contains(&addr),
                "Address {} must NOT be a precompile", id,
            );
        }
    }

    // ─── Gas constant sanity checks ───────────────────────────────────────────

    #[test]
    fn gas_constants_are_non_zero() {
        assert!(GAS_ECRECOVER > 0);
        assert!(GAS_SHA256_BASE > 0);
        assert!(GAS_RIPEMD160_BASE > 0);
        assert!(GAS_IDENTITY_BASE > 0);
        assert!(GAS_MODEXP_MIN > 0);
        assert!(GAS_SHA3FIPS256_BASE > 0);
        assert!(GAS_ECRECOVER_PUBKEY > 0);
    }

    #[test]
    fn gas_constants_within_reasonable_eip_ranges() {
        // ECRecover: EIP-2 specifies 3000
        assert_eq!(GAS_ECRECOVER, 3_000);
        // ECRecoverPublicKey should match ECRecover
        assert_eq!(GAS_ECRECOVER_PUBKEY, GAS_ECRECOVER);
        // RIPEMD160 is more expensive than SHA-256 per EYP
        assert!(GAS_RIPEMD160_BASE > GAS_SHA256_BASE);
        // Identity should be cheapest
        assert!(GAS_IDENTITY_BASE < GAS_SHA256_BASE);
    }

    #[test]
    fn extra_cost_for_returns_zero_for_unknown() {
        let unknown = H160::from_low_u64_be(9999);
        assert_eq!(FrontierPrecompiles::<()>::extra_cost_for(unknown), 0);
    }

    #[test]
    fn extra_cost_for_known_addresses_non_zero() {
        for &id in &[1u64, 2, 3, 4, 5, 1024, 1025] {
            let addr = H160::from_low_u64_be(id);
            assert!(
                FrontierPrecompiles::<()>::extra_cost_for(addr) > 0,
                "Address {} should have non-zero extra_cost", id,
            );
        }
    }

    #[test]
    fn unknown_address_not_in_used_addresses() {
        // Verify the address-lookup table doesn't include random non-precompile addresses.
        // (Full execute() coverage requires a PrecompileHandle mock — tested in integration.)
        let unknown = H160::from_low_u64_be(42);
        assert!(!FrontierPrecompiles::<()>::used_addresses().contains(&unknown));
    }

    /// Every address in `used_addresses()` must map to a non-zero `extra_cost_for()` value,
    /// and every address NOT in `used_addresses()` must map to zero.
    /// This guards against adding an address to one table but forgetting the other.
    #[test]
    fn used_addresses_and_extra_cost_for_are_consistent() {
        let registered = FrontierPrecompiles::<()>::used_addresses();

        // All registered addresses → non-zero extra_cost
        for addr in &registered {
            let cost = FrontierPrecompiles::<()>::extra_cost_for(*addr);
            assert!(
                cost > 0,
                "used_addresses() contains {:?} but extra_cost_for returns 0 — \
                 add a GAS_* constant for this precompile",
                addr,
            );
        }

        // Spot-check: addresses just outside the known set → zero extra_cost
        for &id in &[0u64, 6, 1023, 1026, u16::MAX as u64] {
            let addr = H160::from_low_u64_be(id);
            if !registered.contains(&addr) {
                assert_eq!(
                    FrontierPrecompiles::<()>::extra_cost_for(addr),
                    0,
                    "Non-precompile address {} should have extra_cost 0",
                    id,
                );
            }
        }
    }

    #[test]
    fn used_addresses_has_exactly_7_entries() {
        assert_eq!(FrontierPrecompiles::<()>::used_addresses().len(), 7);
    }

    #[test]
    fn used_addresses_are_unique() {
        let addrs = FrontierPrecompiles::<()>::used_addresses();
        let mut seen = std::collections::HashSet::new();
        for addr in &addrs {
            assert!(seen.insert(addr), "Duplicate precompile address: {:?}", addr);
        }
    }
}

