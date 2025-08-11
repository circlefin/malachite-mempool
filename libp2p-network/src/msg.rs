use prost::bytes::Bytes;
use prost::{Message, Name};
use prost_types::Any;

use malachitebft_proto::{Error as ProtoError, Protobuf};

use crate::types::MempoolTransactionBatch;
use crate::Channel;

#[derive(Clone, Debug, PartialEq)]
pub enum NetworkMsg {
    TransactionBatch(MempoolTransactionBatch),
}

impl NetworkMsg {
    pub fn channel(&self) -> Channel {
        Channel::Mempool
    }

    pub fn from_network_bytes(bytes: &[u8]) -> Result<Self, ProtoError> {
        Protobuf::from_bytes(bytes).map(NetworkMsg::TransactionBatch)
    }

    pub fn to_network_bytes(&self) -> Result<Bytes, ProtoError> {
        match self {
            NetworkMsg::TransactionBatch(batch) => batch.to_bytes(),
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            NetworkMsg::TransactionBatch(batch) => batch.transaction_batch.encoded_len(),
        }
    }
}

impl Protobuf for NetworkMsg {
    type Proto = Any;

    fn from_proto(proto: Self::Proto) -> Result<Self, ProtoError> {
        if proto.type_url == crate::proto::MempoolTransactionBatch::type_url() {
            Ok(NetworkMsg::TransactionBatch(MempoolTransactionBatch {
                transaction_batch: proto,
            }))
        } else {
            Err(ProtoError::Other(format!(
                "Unknown type URL: {}",
                proto.type_url
            )))
        }
    }

    fn to_proto(&self) -> Result<Self::Proto, ProtoError> {
        match self {
            NetworkMsg::TransactionBatch(batch) => Ok(batch.transaction_batch.clone()),
        }
    }
}
