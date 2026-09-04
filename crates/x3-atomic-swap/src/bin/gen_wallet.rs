//! Generate a new Ethereum wallet for Sepolia testnet deployment.
//! Run with: cargo run -p x3-atomic-swap --features std --bin gen_wallet

use k256::ecdsa::SigningKey;
use rand::rngs::OsRng;
use sha3::Digest;

fn main() {
    println!("\n=============================================");
    println!("  New Sepolia Wallet Generated");
    println!("=============================================\n");

    // Generate random private key
    let mut rng = OsRng;
    let signing_key = SigningKey::random(&mut rng);

    // Get private key bytes
    let private_key_bytes = signing_key.to_bytes();
    let private_key_hex = hex::encode(private_key_bytes.as_slice());

    // Derive address
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_encoded_point(false);
    let public_key = public_key_bytes.as_bytes();
    let hash = sha3::Keccak256::digest(&public_key[1..]);
    let address_bytes = &hash[12..];
    let address = format!("0x{}", hex::encode(address_bytes));

    println!("  Private Key: 0x{}", private_key_hex);
    println!("  Address:     {}", address);
    println!("  Network:     Sepolia (chain_id: 11155111)");
    println!("\n---------------------------------------------");
    println!("  NEXT STEPS:");
    println!("  1. Fund this address with Sepolia ETH:");
    println!("     https://sepoliafaucet.com/");
    println!("  2. Export the key:");
    println!("     export DEPLOYER_PRIVATE_KEY={}", private_key_hex);
    println!("  3. Deploy AtlasHTLC:");
    println!("     cargo test -p x3-atomic-swap --features std \\");
    println!("       --test atlas_htlc_deploy_test -- \\");
    println!("       test_deploy_atlas_htlc --ignored");
    println!("=============================================\n");

    // Save to file
    let save_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("sepolia-deployer-wallet.txt");

    let contents = format!(
        "Sepolia Deployer Wallet\n\
         ======================\n\
         Address:     {}\n\
         Private Key: 0x{}\n\n\
         WARNING: Testnet key only. Never use on mainnet.\n\
         Fund at: https://sepoliafaucet.com/\n",
        address, private_key_hex
    );

    std::fs::write(&save_path, &contents).ok();
    println!("  Wallet saved to: {}", save_path.display());
}
