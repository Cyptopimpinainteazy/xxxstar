use gpu_swarm::{bip39, wallet};

#[test]
fn test_ed25519_derivation_and_sign() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    // Use TREZOR passphrase to match BIP39 test vector above
    let seed = bip39::mnemonic_to_seed(mnemonic, "TREZOR");
    let seed64: [u8; 64] = seed;

    let kp1 = wallet::derive_ed25519_keypair(&seed64).expect("derive keypair");
    let kp2 = wallet::derive_ed25519_keypair(&seed64).expect("derive keypair again");

    // deterministic: public keys should match
    assert_eq!(wallet::public_key_hex(&kp1), wallet::public_key_hex(&kp2));

    // sign and verify
    let msg = b"hello world";
    let sig = wallet::sign_message(&kp1, msg);
    assert!(wallet::verify_signature(&kp1, msg, &sig));
}
