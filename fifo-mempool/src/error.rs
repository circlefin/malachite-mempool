use thiserror::Error;

use crate::TxHash;

/// Core mempool errors
#[derive(Clone, Debug, Error, PartialEq)]
pub enum MempoolError {
    #[error("Mempool is full (current size: {current_size}, max: {max_size})")]
    MempoolFull {
        current_size: usize,
        max_size: usize,
    },

    #[error("Invalid transaction data: {reason}")]
    InvalidTransaction { reason: String },

    #[error("Transaction already exists: {tx_hash}")]
    TxAlreadyExists { tx_hash: TxHash },
}

// Helper functions for creating common errors
impl MempoolError {
    pub fn mempool_full(current_size: usize, max_size: usize) -> Self {
        Self::MempoolFull {
            current_size,
            max_size,
        }
    }

    pub fn invalid_transaction<S: Into<String>>(reason: S) -> Self {
        Self::InvalidTransaction {
            reason: reason.into(),
        }
    }

    pub fn tx_already_exists(tx_hash: TxHash) -> Self {
        Self::TxAlreadyExists { tx_hash }
    }
}
