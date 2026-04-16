use crate::error::AppResult;
use crate::vault::store::{default_vault_path, load_or_init, save};
use rpassword::prompt_password;
use secrecy::SecretString;

pub fn execute(profile: &str, key: &str, value: Option<&str>) -> AppResult<()> {
    let password = read_master_password()?;
    let secret_value = match value {
        Some(v) => v.to_owned(),
        None => prompt_password("Secret value: ")?,
    };

    let path = default_vault_path()?;
    let mut vault = load_or_init(&path, &password)?;

    vault
        .profiles
        .entry(profile.to_string())
        .or_default()
        .vars
        .insert(key.to_string(), secret_value);

    save(&path, &vault, &password)?;

    println!("Updated key '{}' in profile '{}'", key, profile);
    Ok(())
}

fn read_master_password() -> AppResult<SecretString> {
    match std::env::var("ENVKEY_MASTER_PASSWORD") {
        Ok(pw) => Ok(SecretString::new(pw)),
        Err(_) => Ok(SecretString::new(prompt_password("Master password: ")?)),
    }
}
