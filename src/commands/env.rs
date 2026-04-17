use crate::commands::session::require_session;
use crate::daemon;
use crate::error::AppResult;

pub fn execute(profile: &str) -> AppResult<()> {
    let session = require_session()?;
    let lines = daemon::vault_env(session, profile)?;
    for line in lines {
        println!("{line}");
    }
    Ok(())
}
