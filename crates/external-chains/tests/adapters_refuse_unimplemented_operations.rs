//! Every shipped adapter must refuse the bridge operations it cannot perform.
//!
//! These adapters used to answer with invented values that a caller could not
//! tell from the real thing:
//!
//! * `send_message` returned `message.hash()` for a message never broadcast,
//! * `initiate_transfer` returned `transfer.id` for a transfer never sent,
//! * `check_transfer_status` returned `Completed` for any transfer id,
//! * `verify_message_proof` returned `Ok(true)` for any non-empty byte string,
//! * `finalize_transfer` returned the id it was given,
//! * `receive_messages` returned an empty list no matter what the chain said.
//!
//! A relayer cannot distinguish those answers from real ones, so each is a
//! path to releasing funds for a message that never arrived. This test pins the
//! refusal in place for every adapter in the production registry.

use sp_core::{H160, H256, U256};
use x3_external_chains::adapter::{ChainMessage, CrossChainTransfer, TransferStatus};
use x3_external_chains::{create_all_adapters, ChainType, ExternalChainError};

fn message() -> ChainMessage {
    ChainMessage::new(
        8453,
        1,
        H160::from_low_u64_be(0xAA),
        H160::from_low_u64_be(0xBB),
        vec![1, 2, 3],
        U256::from(1u64),
    )
}

fn transfer() -> CrossChainTransfer {
    CrossChainTransfer {
        id: H256::from_low_u64_be(9),
        source_chain: 8453,
        dest_chain: 1,
        source_token: H160::zero(),
        dest_token: H160::zero(),
        sender: H160::from_low_u64_be(0xAA),
        recipient: H160::from_low_u64_be(0xBB),
        amount: U256::from(1_000u64),
        fee: U256::zero(),
        status: TransferStatus::Pending,
        source_tx: None,
        dest_tx: None,
    }
}

#[tokio::test]
async fn no_adapter_reports_a_send_it_did_not_perform() {
    for adapter in create_all_adapters() {
        let chain = adapter.chain_type();
        let result = adapter.send_message(message()).await;
        assert!(
            matches!(result, Err(ExternalChainError::AdapterUnimplemented(_))),
            "{:?} must refuse send_message, got {:?}",
            chain,
            result
        );
    }
}

#[tokio::test]
async fn no_adapter_reports_an_initiation_it_did_not_perform() {
    for adapter in create_all_adapters() {
        let chain = adapter.chain_type();
        let result = adapter.initiate_transfer(transfer()).await;
        assert!(
            matches!(result, Err(ExternalChainError::AdapterUnimplemented(_))),
            "{:?} must refuse initiate_transfer, got {:?}",
            chain,
            result
        );
    }
}

#[tokio::test]
async fn no_adapter_calls_a_transfer_completed() {
    for adapter in create_all_adapters() {
        let chain = adapter.chain_type();
        let result = adapter
            .check_transfer_status(H256::from_low_u64_be(9))
            .await;
        assert!(
            matches!(result, Err(ExternalChainError::AdapterUnimplemented(_))),
            "{:?} must refuse check_transfer_status, got {:?}",
            chain,
            result
        );
    }
}

#[tokio::test]
async fn no_adapter_accepts_a_proof_it_cannot_verify() {
    for adapter in create_all_adapters() {
        let chain = adapter.chain_type();
        let result = adapter.verify_message_proof(&message(), &[0u8; 65]).await;
        assert!(
            matches!(result, Err(ExternalChainError::VerificationUnavailable)),
            "{:?} must refuse to verify a proof it cannot check, got {:?}",
            chain,
            result
        );
    }
}

#[tokio::test]
async fn no_adapter_reports_finalization_it_did_not_perform() {
    for adapter in create_all_adapters() {
        let chain = adapter.chain_type();
        let result = adapter
            .finalize_transfer(H256::from_low_u64_be(9), vec![0u8; 65])
            .await;
        assert!(
            matches!(result, Err(ExternalChainError::AdapterUnimplemented(_))),
            "{:?} must refuse finalize_transfer, got {:?}",
            chain,
            result
        );
    }
}

#[tokio::test]
async fn no_adapter_answers_the_message_queue_it_cannot_decode() {
    for adapter in create_all_adapters() {
        let chain = adapter.chain_type();
        // Base and Arbitrum decode `receive_messages` now. Their default configs
        // point at real mainnet endpoints, so calling them here would make this
        // test depend on the network; what they do when a chain *answers* is
        // covered by `receive_messages_decodes_logs.rs`, which drives a stub and
        // asserts the query's emitter address, event topic and block range — and
        // that a malformed log is an error rather than a short queue.
        if matches!(chain, ChainType::Base | ChainType::Arbitrum) {
            continue;
        }
        let result = adapter.receive_messages().await;
        assert!(
            matches!(result, Err(ExternalChainError::AdapterUnimplemented(_))),
            "{:?} must refuse receive_messages rather than returning an empty queue, got {:?}",
            chain,
            result
        );
    }
}
