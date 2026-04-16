use crate::vault;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Vault error: {0}")]
    Vault(#[from] vault::VaultError),
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),
    #[error("No command provided to run")]
    MissingCommand,
}

pub type AppResult<T> = Result<T, AppError>;
