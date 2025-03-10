use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Error invoking function {0}")]
    CallFunctionError(String),

    #[error("Error executing {0}")]
    ExecutionError(String),

    #[error("Error exporting function {0}")]
    ExportFunctionError(String),

    #[error("Invalid parameter type {0}")]
    InvalidParamType(String),

    #[error("Wasmtime error {0}")]
    WasmtimeError(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

impl From<anyhow::Error> for RuntimeError {
    fn from(error: anyhow::Error) -> Self {
        RuntimeError::WasmtimeError(error.to_string())
    }
}