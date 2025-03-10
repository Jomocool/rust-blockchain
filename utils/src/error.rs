use thiserror::Error;

#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("Could not create message: {0}")]
    CreateMessage(String),

    #[error("Error recovering key: {0}")]
    RecoverError(String),

    #[error("Error verifying signature: {0}")]
    VerifyError(String),
}

pub type Result<T> = std::result::Result<T, UtilsError>;
