use crate::commands::session::require_session;
use crate::daemon;
use crate::error::AppResult;

pub fn execute() -> AppResult<()> {
    let session = require_session()?;
    daemon::auth_logout(session)?;
    println!("Session logged out");
    Ok(())
}
