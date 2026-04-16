use crate::error::{AppError, AppResult};
use crate::vault::store::{default_vault_path, load_or_init};
use rpassword::prompt_password;
use secrecy::SecretString;
use std::process::Command;

pub fn execute(profile: &str, args: &[String]) -> AppResult<i32> {
    if args.is_empty() {
        return Err(AppError::MissingCommand);
    }

    let password = read_master_password()?;
    let path = default_vault_path()?;
    let vault = load_or_init(&path, &password)?;

    let selected = vault
        .profiles
        .get(profile)
        .ok_or_else(|| AppError::ProfileNotFound(profile.to_string()))?;

    let status = Command::new(&args[0])
        .args(&args[1..])
        .envs(&selected.vars)
        .status()?;

    Ok(status.code().unwrap_or(1))
}

fn read_master_password() -> AppResult<SecretString> {
    match std::env::var("ENVKEY_MASTER_PASSWORD") {
        Ok(pw) => Ok(SecretString::new(pw)),
        Err(_) => Ok(SecretString::new(prompt_password("Master password: ")?)),
    }
}
