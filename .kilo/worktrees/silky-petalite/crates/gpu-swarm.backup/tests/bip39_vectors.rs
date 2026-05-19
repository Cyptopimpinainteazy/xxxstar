use gpu_swarm::bip39;

#[test]
fn test_bip39_vector_1() {
    // Test vector 1 from Trezor vectors.json (English)
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    // seed derived in official vectors (hex)
    let expected = "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04";
    // many vectors use passphrase "TREZOR" in tests; try that first
    let seed = bip39::mnemonic_to_seed(mnemonic, "TREZOR");
    assert_eq!(hex::encode(seed), expected);
}

#[test]
fn test_bip39_vector_2() {
    // Another known vector (7f... entropy -> "legal winner thank year wave sausage worth useful legal winner thank yellow")
    let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank yellow";
    let expected = "2e8905819b8723fe2c1d161860e5ee1830318dbf49a83bd451cfb8440c28bd6fa457fe1296106559a3c80937a1c1069be3a3a5bd381ee6260e8d9739fce1f607";
    let seed = bip39::mnemonic_to_seed(mnemonic, "TREZOR");
    assert_eq!(hex::encode(seed), expected);
}
