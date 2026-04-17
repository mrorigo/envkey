use crate::commands::session::require_session;
use crate::daemon;
use crate::error::AppResult;
use rpassword::prompt_password;

pub fn execute(profile: &str, key: &str, value: Option<&str>) -> AppResult<()> {
    let session = require_session()?;
    let secret_value = match value {
        Some(v) => v.to_owned(),
        None => prompt_password("Secret value: ")?,
    };

    daemon::vault_add(session, profile, key, &secret_value)?;
    println!("Updated key '{}' in profile '{}'", key, profile);
    Ok(())
}
