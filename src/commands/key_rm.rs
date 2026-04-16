use crate::error::{AppError, AppResult};
use crate::vault::store::{default_vault_path, load_or_init, save};
use rpassword::prompt_password;
use secrecy::SecretString;
use std::io::{self, Write};

pub fn execute(profile: &str, key: &str, yes: bool) -> AppResult<()> {
    let password = read_master_password()?;
    let path = default_vault_path()?;
    let mut vault = load_or_init(&path, &password)?;

    let selected = vault
        .profiles
        .get_mut(profile)
        .ok_or_else(|| AppError::ProfileNotFound(profile.to_string()))?;

    if !selected.vars.contains_key(key) {
        return Err(AppError::KeyNotFound {
            profile: profile.to_string(),
            key: key.to_string(),
        });
    }

    if !yes && !confirm(&format!("Delete key '{}' from profile '{}'?", key, profile))? {
        return Err(AppError::OperationCancelled);
    }

    selected.vars.remove(key);
    save(&path, &vault, &password)?;

    println!("Removed key '{}' from profile '{}'", key, profile);
    Ok(())
}

fn read_master_password() -> AppResult<SecretString> {
    match std::env::var("ENVKEY_MASTER_PASSWORD") {
        Ok(pw) => Ok(SecretString::new(pw)),
        Err(_) => Ok(SecretString::new(prompt_password("Master password: ")?)),
    }
}

fn confirm(prompt: &str) -> AppResult<bool> {
    eprint!("{} [y/N]: ", prompt);
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
