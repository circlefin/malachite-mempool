use std::sync::Arc;

pub mod mempool;
pub mod types;

pub use libp2p_network::{
    GossipNetworkMsg, MempoolNetwork, MempoolNetworkActorRef, MempoolNetworkMsg,
};
pub use mempool::*;
pub use types::tx::{RawTx, TxHash};

pub type MempoolAppRef = Arc<dyn MempoolApp>;
pub type ActorResult<T> = Result<T, ractor::ActorProcessingErr>;

// Placeholder for AppResult
pub type AppResult<T> = Result<T, AppError>;

// Placeholder for AppError
#[derive(Clone, Debug)]
pub struct AppError;

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppError")
    }
}

impl std::error::Error for AppError {}

// Placeholder traits for Db, Vm, ProposalPreparer, Indexer
pub trait Db {
    type Error;
}
pub trait Vm: Clone {
    type Error;
}
pub trait ProposalPreparer {
    type Error;
}
pub trait Indexer {
    type Error;
}

pub trait CheckTxOutcomeTrait: Send + Sync + 'static {}

// Example struct implementing the trait (to be replaced with actual CheckTxOutcome definition)
#[derive(Debug, Clone, Copy)]
pub struct CheckTxOutcome;

impl CheckTxOutcomeTrait for CheckTxOutcome {}

pub trait MempoolApp: Send + Sync + 'static {
    fn check_tx(&self, tx: &RawTx) -> AppResult<CheckTxOutcome>;
}
