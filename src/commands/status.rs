use crate::daemon;
use crate::error::AppResult;

pub fn execute() -> AppResult<()> {
    let session = std::env::var("ENVKEY_SESSION").ok();
    let (daemon_running, session_valid, locked) = daemon::auth_status(session)?;

    println!("daemon_running={daemon_running}");
    println!("session_valid={session_valid}");
    println!("session_locked={locked}");
    Ok(())
}
