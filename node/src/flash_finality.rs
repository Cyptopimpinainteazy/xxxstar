use codec::{Decode, Encode};
use flash_finality::{FlashFinalityGadget, GossipMessage, FLASH_FINALITY_PROTOCOL_ID};
use futures::{future, prelude::*};
use log::{debug, info, warn};
use sc_client_api::BlockchainEvents;
use sc_network::service::traits::NotificationService;
use sc_network::PeerId;
use sc_network_gossip::{
    GossipEngine, MessageIntent, Network, Syncing, ValidationResult, Validator, ValidatorContext,
};
use sp_core::crypto::KeyTypeId;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_runtime::SaturatedConversion;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A validator for Flash Finality gossip messages.
pub struct FlashFinalityGossipValidator<Block: BlockT> {
    _phantom: std::marker::PhantomData<Block>,
}

impl<Block: BlockT> FlashFinalityGossipValidator<Block> {
    /// Create a new gossip validator for Flash Finality messages.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Block: BlockT> Validator<Block> for FlashFinalityGossipValidator<Block> {
    fn validate(
        &self,
        _context: &mut dyn ValidatorContext<Block>,
        _sender: &PeerId,
        data: &[u8],
    ) -> ValidationResult<Block::Hash> {
        if let Ok(_msg) = GossipMessage::decode(&mut &data[..]) {
            ValidationResult::ProcessAndKeep(Default::default())
        } else {
            ValidationResult::Discard
        }
    }

    fn message_expired<'a>(&'a self) -> Box<dyn FnMut(Block::Hash, &[u8]) -> bool + 'a> {
        Box::new(move |_hash, _data| false)
    }

    fn message_allowed<'a>(
        &'a self,
    ) -> Box<dyn FnMut(&PeerId, MessageIntent, &Block::Hash, &[u8]) -> bool + 'a> {
        Box::new(move |_who, _intent, _topic, _data| true)
    }
}

/// A bridge between the Flash Finality gadget and the network.
pub struct FlashFinalityBridge<Block: BlockT, Client> {
    gadget: Arc<FlashFinalityGadget>,
    client: Arc<Client>,
    gossip_engine: Arc<Mutex<GossipEngine<Block>>>,
    keystore: KeystorePtr,
}

impl<Block: BlockT, Client> FlashFinalityBridge<Block, Client>
where
    Client: BlockchainEvents<Block> + Send + Sync + 'static,
{
    /// Create a new Flash Finality network bridge.
    pub fn new<N, S>(
        gadget: Arc<FlashFinalityGadget>,
        client: Arc<Client>,
        network: N,
        sync_service: S,
        keystore: KeystorePtr,
        notification_service: Box<dyn NotificationService>,
    ) -> Self
    where
        N: Network<Block> + Send + Clone + 'static,
        S: Syncing<Block> + Send + Clone + 'static,
    {
        let validator = Arc::new(FlashFinalityGossipValidator::new());
        let gossip_engine = Arc::new(Mutex::new(GossipEngine::new(
            network,
            sync_service,
            notification_service,
            FLASH_FINALITY_PROTOCOL_ID,
            validator,
            None,
        )));

        Self {
            gadget,
            client,
            gossip_engine,
            keystore,
        }
    }

    /// Run the network bridge event loop.
    pub async fn run(self) {
        let mut import_notifications = self.client.import_notification_stream();
        let mut finality_notifications = self.client.finality_notification_stream();

        let mut incoming_messages = {
            let mut engine = self.gossip_engine.lock().await;
            let topic: Block::Hash = Default::default();
            engine.messages_for(topic)
        };

        info!(
            "⚡ [FlashFinality] Network bridge started — gossiping certificates on {} protocol",
            FLASH_FINALITY_PROTOCOL_ID
        );

        loop {
            let mut gossip_poll = future::poll_fn(|cx| {
                let mut engine = match self.gossip_engine.try_lock() {
                    Ok(e) => e,
                    Err(_) => return std::task::Poll::Pending,
                };
                engine.poll_unpin(cx)
            });

            tokio::select! {
                _ = &mut gossip_poll => {
                    warn!("[FlashFinality] Gossip engine terminated");
                    break;
                }

                Some(notification) = import_notifications.next() => {
                    let number: u64 = (*notification.header.number()).saturated_into();
                    let hash: [u8; 32] = notification.hash.as_ref().try_into().unwrap_or([0u8; 32]);

                    if let Some(proposal) = self.gadget.on_new_block(hash, number).await {
                        self.broadcast(GossipMessage::Proposal(proposal)).await;
                    }
                }

                Some(notification) = finality_notifications.next() => {
                    let number: u64 = (*notification.header.number()).saturated_into();
                    let hash: [u8; 32] = notification.hash.as_ref().try_into().unwrap_or([0u8; 32]);
                    self.gadget.update_grandpa_head(number, hash).await;
                    self.gadget.shadow_compare(number, hash).await;
                }

                Some(msg) = incoming_messages.next() => {
                    if let Ok(gossip_msg) = GossipMessage::decode(&mut &msg.message[..]) {
                        match gossip_msg {
                            GossipMessage::Proposal(p) => {
                                self.gadget.on_proposal(p.clone()).await;

                                // Sign a vote if we are a validator
                                let public_keys = self.keystore.sr25519_public_keys(KeyTypeId(*b"flsh"));
                                if let Some(pubkey) = public_keys.get(0) {
                                    let msg = p.message_hash();
                                    if let Ok(Some(sig)) = self.keystore.sr25519_sign(KeyTypeId(*b"flsh"), pubkey, &msg) {
                                        let vote = flash_finality::Vote {
                                            block_hash: p.block_hash,
                                            block_number: p.block_number,
                                            round: p.round,
                                            voter_id: pubkey.0,
                                            voter_sig: sig.0,
                                        };
                                        if let Some(cert) = self.gadget.on_vote(vote.clone()).await {
                                            self.broadcast(GossipMessage::Certificate(cert)).await;
                                        }
                                        self.broadcast(GossipMessage::Vote(vote)).await;
                                    }
                                }
                            }
                            GossipMessage::Vote(v) => {
                                if let Some(cert) = self.gadget.on_vote(v).await {
                                    self.broadcast(GossipMessage::Certificate(cert)).await;
                                }
                            }
                            GossipMessage::Certificate(c) => {
                                debug!("[FlashFinality] Received certificate for block #{}", c.block_number);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn broadcast(&self, msg: GossipMessage) {
        let topic: Block::Hash = Default::default();
        let mut engine = self.gossip_engine.lock().await;
        engine.gossip_message(topic, msg.encode(), true);
    }
}
