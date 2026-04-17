use crate::daemon;
use crate::error::AppResult;
use rpassword::prompt_password;

pub fn execute() -> AppResult<()> {
    let password = match std::env::var("ENVKEY_AUTH_PASSWORD") {
        Ok(p) => p,
        Err(_) => prompt_password("Master password: ")?,
    };
    let token = daemon::auth_unlock(password)?;
    println!("export ENVKEY_SESSION='{}'", token);
    Ok(())
}
