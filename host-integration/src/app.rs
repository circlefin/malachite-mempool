use fifo_mempool::{AppResult, CheckTxOutcome, MempoolApp, RawTx, TxHash};
use prost::bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestTx(pub u64); // Unique transaction by id

impl TestTx {
    pub fn serialize(&self) -> RawTx {
        RawTx(Bytes::from(self.0.to_le_bytes().to_vec()))
    }
    pub fn deserialize(bytes: &[u8]) -> Result<TestTx, anyhow::Error> {
        Ok(TestTx(u64::from_le_bytes(bytes.try_into().unwrap())))
    }
    pub fn hash(&self) -> TxHash {
        TxHash(Bytes::from(self.0.to_le_bytes().to_vec()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestCheckTxOutcome {
    Success(TxHash),
    Error(TxHash, String),
}

impl CheckTxOutcome for TestCheckTxOutcome {
    fn is_valid(&self) -> bool {
        matches!(self, TestCheckTxOutcome::Success(_))
    }
    fn hash(&self) -> TxHash {
        match self {
            TestCheckTxOutcome::Success(hash) => hash.clone(),
            TestCheckTxOutcome::Error(hash, _) => hash.clone(),
        }
    }
}
pub struct TestMempoolApp;
impl MempoolApp for TestMempoolApp {
    fn check_tx(&self, tx: &RawTx) -> AppResult<Box<dyn CheckTxOutcome>> {
        // Deserialize the transaction
        match TestTx::deserialize(&tx.0) {
            Ok(test_tx) => {
                // Return error for transaction with value 9999
                if test_tx.0 == 9999 {
                    Ok(Box::new(TestCheckTxOutcome::Error(
                        tx.hash(),
                        "Transaction 9999 is not allowed".to_string(),
                    )))
                } else {
                    Ok(Box::new(TestCheckTxOutcome::Success(tx.hash())))
                }
            }
            Err(e) => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))),
        }
    }
}
