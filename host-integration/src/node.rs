use fifo_mempool::{
    mempool::{spawn_mempool_actor, MempoolConfig, Msg},
    AppResult, CheckTxOutcome, MempoolApp, RawTx,
};
use libp2p_identity::Keypair;
use libp2p_network::spawn_mempool_network_actor;
use malachitebft_metrics::SharedRegistry;
use prometheus_client::registry::Registry;
use prost::bytes::Bytes;
use ractor::ActorRef;
use std::sync::Arc;

use crate::config::HostMempoolConfig;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestTx(pub u64); // Unique transaction by id

impl TestTx {
    pub fn serialize(&self) -> RawTx {
        RawTx(Bytes::from(self.0.to_le_bytes().to_vec()))
    }
    pub fn deserialize(bytes: &[u8]) -> Result<TestTx, anyhow::Error> {
        Ok(TestTx(u64::from_le_bytes(bytes.try_into().unwrap())))
    }
}

pub struct TestMempoolApp;
impl MempoolApp for TestMempoolApp {
    fn check_tx(&self, _tx: &RawTx) -> AppResult<CheckTxOutcome> {
        Ok(CheckTxOutcome)
    }
}

pub struct TestNode {
    pub id: usize,
    pub mempool_actor: ActorRef<Msg>,
}

impl TestNode {
    pub async fn new(id: usize, config: HostMempoolConfig) -> Self {
        let app = Arc::new(TestMempoolApp);

        let network_config = malachitebft_test_mempool::Config {
            listen_addr: config.p2p.listen_addr,
            persistent_peers: config.p2p.persistent_peers,
            idle_connection_timeout: config.idle_connection_timeout,
        };

        // Create metrics registry
        let metrics = SharedRegistry::new(Registry::with_prefix("test"), None);

        // Create network actor
        let keypair = Keypair::generate_ed25519();
        let network_actor = spawn_mempool_network_actor(&network_config, &keypair, &metrics).await;

        // Create mempool actor
        let mempool_config = MempoolConfig {
            max_txs_bytes: config.max_txs_bytes.as_u64(),
            max_txs_per_block: (config.max_txs_bytes.as_u64() / config.avg_tx_bytes.as_u64())
                as usize,
        };

        let mempool_actor = spawn_mempool_actor(
            network_actor.clone(),
            app,
            tracing::Span::current(),
            &mempool_config,
        )
        .await;

        Self { id, mempool_actor }
    }

    pub async fn add_tx(&mut self, tx: TestTx) {
        let raw_tx = tx.serialize();
        let tx_hash = raw_tx.hash();

        // Send add message to the mempool actor using RPC call
        let result = self
            .mempool_actor
            .call(|reply| Msg::Add { tx: raw_tx, reply }, None)
            .await;

        if result.is_ok() {
            println!(
                "Node {} added transaction {} with hash {}",
                self.id, tx.0, tx_hash
            );
        } else {
            println!(
                "Node {} failed to add transaction {}: {:?}",
                self.id, tx.0, result
            );
        }
    }

    pub async fn remove_tx(&mut self, tx: &TestTx) {
        // Send remove message to the mempool actor using cast (non-RPC)
        let raw_tx = tx.serialize();
        let tx_hash = raw_tx.hash();
        let result = self.mempool_actor.cast(Msg::Remove(vec![raw_tx]));
        if result.is_ok() {
            println!(
                "Node {} removed transaction {} with hash {}",
                self.id, tx.0, tx_hash
            );
        } else {
            println!(
                "Node {} failed to remove transaction {}: {:?}",
                self.id, tx.0, result
            );
        }
    }

    pub async fn get_transactions(&self) -> Vec<RawTx> {
        // Get transactions from the mempool actor using Take message
        let result = self
            .mempool_actor
            .call(|reply| Msg::Take { reply }, None)
            .await;
        match result {
            Ok(txs) => txs.unwrap_or(vec![]),
            Err(e) => {
                println!("Node {} failed to get transactions: {:?}", self.id, e);
                vec![]
            }
        }
    }
}
