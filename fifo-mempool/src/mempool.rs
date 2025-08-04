use {
    crate::{types::tx::TxHash, ActorResult, MempoolApp, RawTx},
    ractor::{Actor, ActorRef, RpcReplyPort},
    std::{
        cmp::min,
        collections::{HashMap, HashSet, VecDeque},
        sync::Arc,
    },
    thiserror::Error,
    tracing::{debug, error, info, warn, Span},
};

// Placeholder types for external dependencies
pub type NetworkEvent = libp2p_network::Event;
pub type GossipNetworkMsg = libp2p_network::NetworkMsg;
pub type MempoolNetworkActorRef = ActorRef<libp2p_network::Msg>;
pub type MempoolNetworkMsg = libp2p_network::Msg;
pub type MempoolTransactionBatch = libp2p_network::types::MempoolTransactionBatch;
pub type MempoolAppRef = Arc<dyn MempoolApp>;

// Placeholder for MempoolConfig
#[derive(Clone, Debug)]
pub struct MempoolConfig {
    pub max_txs_bytes: u64,
    pub max_txs_per_block: usize,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_txs_bytes: 4 * 1024 * 1024, // 4MB
            max_txs_per_block: 100,         // 100KB
        }
    }
}

pub type MempoolMsg = Msg;
pub type MempoolActorRef = ActorRef<Msg>;

#[derive(Default, Clone)]
pub struct State {
    pub txs: VecDeque<RawTx>,
    pub tx_hashes: HashMap<TxHash, usize>,
}

impl State {
    pub fn exists(&self, tx: &TxHash) -> bool {
        self.tx_hashes.contains_key(tx)
    }
}

pub enum Msg {
    NetworkEvent(Arc<NetworkEvent>),
    Add {
        tx: RawTx,
        reply: RpcReplyPort<Result<Box<dyn crate::CheckTxOutcome>, MempoolError>>,
    },
    // TODO: figure out how to properly handle messages outside consensus
    CheckTxResult {
        tx: RawTx,
        result: Result<Box<dyn crate::CheckTxOutcome>, Box<dyn std::error::Error + Send + Sync>>,
        reply: Option<RpcReplyPort<Result<Box<dyn crate::CheckTxOutcome>, MempoolError>>>,
    },
    Take {
        reply: RpcReplyPort<Vec<RawTx>>,
    },
    Remove(Vec<RawTx>),
}

impl From<Arc<NetworkEvent>> for Msg {
    fn from(event: Arc<NetworkEvent>) -> Self {
        Self::NetworkEvent(event)
    }
}

#[allow(dead_code)]
pub struct Mempool {
    mempool_network: MempoolNetworkActorRef,
    app: MempoolAppRef,
    span: Span,
    config: MempoolConfig,
    /// The maximum average number of transactions that can be taken from the mempool.
    max_txs_per_block: usize,
}

impl Mempool {
    pub async fn spawn(
        mempool_network: MempoolNetworkActorRef,
        app: MempoolAppRef,
        span: Span,
        config: MempoolConfig,
    ) -> Result<MempoolActorRef, ractor::SpawnErr> {
        let node = Self {
            mempool_network,
            app,
            span,
            max_txs_per_block: config.max_txs_per_block,
            config,
        };

        let (actor_ref, _) = Actor::spawn(None, node, ()).await?;
        Ok(actor_ref)
    }

    fn handle_msg(&self, msg: Msg, state: &mut State) -> ActorResult<()> {
        match msg {
            Msg::NetworkEvent(event) => self.handle_network_event(&event, state)?,
            Msg::Add { tx, reply } => self.add_tx(tx, Some(reply), state)?,
            Msg::CheckTxResult { tx, result, reply } => {
                self.handle_check_tx_result(tx, result, reply, state)?
            }
            Msg::Take { reply } => self.take(state, reply)?,
            Msg::Remove(tx_hashes) => self.remove(tx_hashes, state)?,
        }

        Ok(())
    }

    fn handle_network_event(&self, event: &NetworkEvent, state: &mut State) -> ActorResult<()> {
        // Handle network events from the gossip network
        debug!("Received network event: {:?}", event);

        match event {
            NetworkEvent::Message(.., network_msg) => {
                self.handle_network_msg(network_msg, state)?;
            }
            e => info!("Network event: {:?}", e),
        }

        Ok(())
    }

    #[allow(dead_code)]
    #[tracing::instrument("handle_network_msg", skip_all)]
    fn handle_network_msg(&self, msg: &GossipNetworkMsg, state: &mut State) -> ActorResult<()> {
        match msg {
            GossipNetworkMsg::TransactionBatch(batch) => {
                let tx = RawTx(prost::bytes::Bytes::from(
                    batch.transaction_batch.value.clone(),
                ));
                self.add_tx(tx, None, state)
            }
        }
    }

    #[tracing::instrument("add_tx", skip_all)]
    fn add_tx(
        &self,
        tx: RawTx,
        reply: Option<RpcReplyPort<Result<Box<dyn crate::CheckTxOutcome>, MempoolError>>>,
        state: &mut State,
    ) -> ActorResult<()> {
        let tx_hash = tx.hash();

        if state.exists(&tx_hash) {
            warn!("tx already exists in mempool");
            if let Some(reply) = reply {
                reply.send(Err(MempoolError::TxAlreadyExists(tx_hash)))?;
            }
            return Ok(());
        }

        // Instead of calling check_tx directly, simulate sending a message to the app
        // In a real implementation, this would be an async message to the app actor
        let check_tx_outcome = self.app.check_tx(&tx);

        // Simulate the message passing by immediately handling the result
        // In a real implementation, this would be handled by a separate CheckTxResult message
        self.handle_check_tx_result(tx, check_tx_outcome, reply, state)?;

        Ok(())
    }

    #[tracing::instrument("handle_check_tx_result", skip_all)]
    fn handle_check_tx_result(
        &self,
        tx: RawTx,
        result: Result<Box<dyn crate::CheckTxOutcome>, Box<dyn std::error::Error + Send + Sync>>,
        reply: Option<RpcReplyPort<Result<Box<dyn crate::CheckTxOutcome>, MempoolError>>>,
        state: &mut State,
    ) -> ActorResult<()> {
        match result {
            Ok(check_tx_outcome) => {
                if check_tx_outcome.is_valid() {
                    debug!("check_tx successful, tx is valid, adding tx to mempool");
                    let tx_hash = tx.hash();
                    state.tx_hashes.insert(tx_hash, state.txs.len());
                    state.txs.push_back(tx.clone());
                    self.gossip_tx(tx)?;
                } else {
                    warn!(reason = ?check_tx_outcome, "check_tx successful, tx is invalid, not adding to mempool");
                }
                if let Some(reply) = reply {
                    reply.send(Ok(check_tx_outcome))?;
                }
            }

            Err(app_error) => {
                error!(reason = ?app_error, "check_tx failed!");
                if let Some(reply) = reply {
                    reply.send(Err(MempoolError::App(app_error.to_string())))?;
                }
            }
        }

        Ok(())
    }

    fn take(&self, state: &mut State, reply: RpcReplyPort<Vec<RawTx>>) -> ActorResult<()> {
        let mut txs = Vec::with_capacity(min(self.max_txs_per_block, state.txs.len()));

        let mut max_tx_bytes = self.config.max_txs_bytes as usize;

        for tx in state.txs.iter() {
            // we are not removing the tx from the mempool, here because prepare proposal could not
            // include some txs. Txs will be removed during decided.

            max_tx_bytes = max_tx_bytes.saturating_sub(tx.len());

            if max_tx_bytes == 0 {
                break;
            }

            txs.push(tx.clone());
        }

        reply.send(txs)?;

        Ok(())
    }

    #[tracing::instrument("remove", skip_all)]
    fn remove(&self, txs: Vec<RawTx>, state: &mut State) -> ActorResult<()> {
        let mut ignore = HashSet::new();
        for tx in txs {
            if let Some(index) = state.tx_hashes.remove(&tx.hash()) {
                ignore.insert(index);
            }
        }

        debug!("removed {} txs from mempool", ignore.len());

        let mut new_txs = VecDeque::with_capacity(state.txs.len() - ignore.len());
        let mut new_hashes = HashMap::with_capacity(state.txs.len() - ignore.len());

        let mut counter = 0;
        for (index, tx) in state.txs.iter().enumerate() {
            if !ignore.contains(&index) {
                new_hashes.insert(tx.hash(), counter);
                new_txs.push_back(tx.clone());
                counter += 1;
            }
        }

        state.txs = new_txs;
        state.tx_hashes = new_hashes;

        Ok(())
    }

    #[tracing::instrument("gossip_tx", skip_all)]
    fn gossip_tx(&self, tx: RawTx) -> ActorResult<()> {
        // Create a transaction batch for gossiping
        // In a real implementation, you might want to batch multiple transactions
        let batch = MempoolTransactionBatch::new(prost_types::Any {
            type_url: "mempool.transaction".to_string(),
            value: tx.to_vec(),
        });

        // Send the batch to the network actor for broadcasting
        // Note: This is a cast since we don't need a reply for gossip
        if let Err(e) = self
            .mempool_network
            .cast(MempoolNetworkMsg::Broadcast(batch))
        {
            error!("Failed to gossip transaction: {:?}", e);
        } else {
            debug!("Successfully gossiped transaction");
        }

        Ok(())
    }
}

impl Actor for Mempool {
    type Arguments = ();
    type Msg = Msg;
    type State = State;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> ActorResult<Self::State> {
        self.mempool_network.link(myself.get_cell());

        self.mempool_network
            .cast(MempoolNetworkMsg::Subscribe(Box::new(myself.clone())))?;

        Ok(State::default())
    }

    #[tracing::instrument("mempool", parent = &self.span, skip_all)]
    async fn handle(
        &self,
        _myself: MempoolActorRef,
        msg: MempoolMsg,
        state: &mut State,
    ) -> ActorResult<()> {
        if let Err(e) = self.handle_msg(msg, state) {
            error!("Error handling message: {:?}", e);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Error)]
pub enum MempoolError {
    #[error("Application error: {0}")]
    App(String),

    #[error("tx already exists in mempool: {0}")]
    TxAlreadyExists(TxHash),
}

pub async fn spawn_mempool_actor(
    mempool_network: MempoolNetworkActorRef,
    app: Arc<dyn MempoolApp>,
    span: Span,
    config: &MempoolConfig,
) -> MempoolActorRef {
    Mempool::spawn(mempool_network, app, span, config.clone())
        .await
        .unwrap()
}
