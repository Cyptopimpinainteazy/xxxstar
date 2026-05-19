use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use solana_sdk::transport::TransportError;

#[tokio::test]
async fn deploy_and_increment_counter() -> Result<(), TransportError> {
    // Use a fixed program id for the test
    let program_id = Pubkey::new_unique();

    // Build ProgramTest with our local processor (from the svm_counter crate)
    let mut program_test = ProgramTest::new(
        "svm_counter",
        program_id,
        processor!(svm_counter::process_instruction),
    );

    // Pre-create counter account in the test bank
    let counter_key = Keypair::new();
    let lamports = 1_000_000_000u64;
    program_test.add_account(
        counter_key.pubkey(),
        solana_sdk::account::Account {
            lamports,
            data: vec![0u8; 8],
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start test banks client
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    // Invoke the program (increment)
    let instruction = solana_sdk::instruction::Instruction::new_with_bytes(
        program_id,
        &[],
        vec![AccountMeta::new(counter_key.pubkey(), false)],
    );

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    // Read account data and assert counter == 1
    let account = banks_client
        .get_account(counter_key.pubkey())
        .await
        .unwrap()
        .expect("account not found");

    assert_eq!(account.data.len(), 8);
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&account.data[0..8]);
    let counter = u64::from_le_bytes(arr);
    assert_eq!(counter, 1);

    Ok(())
}
