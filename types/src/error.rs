use thiserror::Error;
use utils::error::UtilsError;
#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Error encoding/decoding: {0}")]
    EncodingDecodingError(String),

    #[error("Error converting a hex to U64: {0}")]
    HexToU64Error(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Unsupported contract type: {0}")]
    UnsupportedContractType(String),

    #[error("Missing block hash")]
    MissingBlockHash,

    #[error("Missing transaction hash")]
    MissingTransactionHash,

    #[error("{0}")]
    TrieError(String),

    #[error("{0}")]
    UtilError(String),
}

pub type Result<T> = std::result::Result<T, TypeError>;

impl From<Box<bincode::ErrorKind>> for TypeError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        TypeError::EncodingDecodingError(error.to_string())
    }
}

impl From<UtilsError> for TypeError {
    fn from(error: UtilsError) -> Self {
        TypeError::UtilError(error.to_string())
    }
}
