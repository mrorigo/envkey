use crate::error::{AppError, AppResult};

pub fn require_session() -> AppResult<String> {
    std::env::var("ENVKEY_SESSION").map_err(|_| AppError::SessionMissing)
}
