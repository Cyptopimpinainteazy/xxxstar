// X3 Bridge Message Decode Fuzz Target
// Intended for use with cargo-fuzz via libfuzzer-sys
//
// #![no_main]
// use libfuzzer_sys::fuzz_target;
// fuzz_target!(|data: &[u8]| {
//     // Decode attempts should not panic
//     if let Ok(msg) = x3_bridge::BridgeMessage::decode(&mut &data[..]) {
//         // Verify invariants on decoded message
//         assert!(msg.nonce != 0 || msg.origin != ChainId::default());
//     }
// });