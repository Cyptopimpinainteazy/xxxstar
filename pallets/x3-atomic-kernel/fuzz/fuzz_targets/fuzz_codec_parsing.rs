//! Fuzz target: SCALE codec round-trip for all bundle types
//!
//! Verifies that encode→decode is identity and that arbitrary byte sequences
//! never cause panics. This catches: malformed length prefixes, OOM via
//! oversized BoundedVec, and codec implementation bugs.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::{Decode, Encode};
use pallet_x3_atomic_kernel::proof::{BundleLeg, DeclaredAccess, PoaeProof, VmType};

fuzz_target!(|data: &[u8]| {
    // Round-trip: decode then re-encode must yield the same bytes
    macro_rules! round_trip {
        ($T:ty) => {
            if let Ok(val) = <$T>::decode(&mut &*data) {
                let re_encoded = val.encode();
                let re_decoded = <$T>::decode(&mut &*re_encoded.as_slice());
                match re_decoded {
                    Ok(v2) => assert_eq!(
                        val, v2,
                        "CODEC BUG: encode→decode round-trip not identity for {}",
                        stringify!($T)
                    ),
                    Err(e) => panic!(
                        "CODEC BUG: re-encode produced un-decodable bytes for {}: {}",
                        stringify!($T),
                        e
                    ),
                }
            }
        };
    }

    round_trip!(PoaeProof);
    round_trip!(BundleLeg);
    round_trip!(DeclaredAccess);
    round_trip!(VmType);
});
