use thiserror::Error;

/// Application-level errors for transaction validation
#[derive(Clone, Debug, Error, PartialEq)]
pub enum AppError {
    #[error("Transaction validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Transaction deserialization failed: {details}")]
    DeserializationFailed { details: String },

    #[error("Unsupported transaction type: {tx_type}")]
    UnsupportedTransactionType { tx_type: String },

    #[error("Transaction size too large: {size} bytes (max: {max_size} bytes)")]
    TransactionTooLarge { size: usize, max_size: usize },

    #[error("Invalid transaction format: {details}")]
    InvalidFormat { details: String },

    #[error("Internal application error: {details}")]
    Internal { details: String },
}

// Helper functions for creating common errors
impl AppError {
    pub fn validation_failed<S: Into<String>>(reason: S) -> Self {
        Self::ValidationFailed {
            reason: reason.into(),
        }
    }

    pub fn deserialization_failed<S: Into<String>>(details: S) -> Self {
        Self::DeserializationFailed {
            details: details.into(),
        }
    }

    pub fn invalid_format<S: Into<String>>(details: S) -> Self {
        Self::InvalidFormat {
            details: details.into(),
        }
    }

    pub fn transaction_too_large(size: usize, max_size: usize) -> Self {
        Self::TransactionTooLarge { size, max_size }
    }
}
