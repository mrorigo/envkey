use crate::daemon;
use crate::error::AppResult;
use rpassword::prompt_password;

pub fn require_session() -> AppResult<String> {
    if let Ok(session) = std::env::var("ENVKEY_SESSION") {
        return Ok(session);
    }

    let password = match std::env::var("ENVKEY_AUTH_PASSWORD") {
        Ok(p) => p,
        Err(_) => prompt_password("Master password: ")?,
    };

    let token = daemon::auth_unlock(password)?;
    eprintln!("No ENVKEY_SESSION found. Authenticated for this command.");
    eprintln!("Tip: export it for reuse: export ENVKEY_SESSION='{token}'");
    Ok(token)
}
