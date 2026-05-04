use thiserror::Error;

use crate::ethereum::client::EthereumRpcError;
use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error(transparent)]
    Rpc(#[from] EthereumRpcError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    Hex(#[from] hex::FromHexError),

    #[error("malformed log: {0}")]
    MalformedLog(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("unsupported event topic: 0x{0}")]
    UnsupportedTopic(String),

    #[error("rpc returned no result for {0}")]
    NoResult(&'static str),

    #[error("api key not configured: {0}")]
    KeyRequired(&'static str),

    #[error("range invalid: from_block {from} > to_block {to}")]
    InvalidRange { from: u64, to: u64 },

    #[error("decoded value overflows expected width: {0}")]
    Overflow(&'static str),
}
