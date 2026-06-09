// SPDX-License-Identifier: Apache-2.0
//
// Runtime API declarations for the x3-cross-vm-router pallet.
//
// This module exposes `submit_x3_lang_program` as a pallet extrinsic
// (call_index 8).  The actual implementation lives in the main `impl<T: Config> Pallet<T>`
// block in lib.rs — see the `submit_x3_lang_program` extrinsic there.
//
// This file also contains the sp_api `decl_runtime_apis!` macro for
// runtime-to-host-bridge APIs used by offchain workers and sidecars.

use super::*;
use sp_api::decl_runtime_apis;

decl_runtime_apis! {
    /// Runtime API exposed to the host for cross-VM router introspection.
    pub trait X3CrossVmRouterApi {
        /// Check whether external bridges are currently enabled.
        fn external_bridges_enabled() -> bool;

        /// Check whether the bridge audit gate has been passed.
        fn external_bridge_audit_gate() -> bool;

        /// Get a stored bridge root for a specific chain.
        fn get_bridge_root(chain_id: u32) -> Option<(sp_core::H256, u32, u32)>;
    }
}
