use crate::vault;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Vault error: {0}")]
    Vault(#[from] vault::VaultError),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("No command provided to run")]
    MissingCommand,
    #[error("Operation cancelled")]
    OperationCancelled,
    #[error("envkey daemon unavailable. Run: ek auth")]
    DaemonUnavailable,
    #[error("Daemon error [{code}]: {message}")]
    DaemonResponse { code: String, message: String },
    #[error("Protocol error: {0}")]
    Protocol(String),
}

pub type AppResult<T> = Result<T, AppError>;
