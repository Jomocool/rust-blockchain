use thiserror::Error;

#[derive(Error, Debug)]
pub enum Web3Error {
    #[error("Error creating a new HTTP JSON-RPC client: {0}")]
    ClientError(String),

    #[error("Error serializing or deserializing JSON data: {0}")]
    JsonParseError(String),

    #[error("Error sending a HTTP JSON-RPC call: {0}")]
    RpcRequestError(String),

    #[error("Error receiving a HTTP JSON-RPC response: {0}")]
    RpcResponseError(String),

    #[error("Error signing transaction: {0}")]
    TransactionSigningError(String),
}

pub type Result<T> = std::result::Result<T, Web3Error>;

impl From<serde_json::Error> for Web3Error {
    fn from(error: serde_json::Error) -> Self {
        Web3Error::JsonParseError(error.to_string())
    }
}
