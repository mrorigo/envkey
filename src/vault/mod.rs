pub mod crypto;
pub mod store;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub schema_version: u32,
    pub profiles: HashMap<String, Profile>,
}

impl Default for Vault {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            profiles: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    pub vars: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Invalid vault password")]
    InvalidPassword,
    #[error("Vault appears corrupt or tampered")]
    CorruptVault,
    #[error("Unsupported vault schema version: {0}")]
    UnsupportedSchemaVersion(u32),
}
