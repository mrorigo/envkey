use crate::error::AppResult;
use crate::vault::store::{default_vault_path, load_or_init};
use rpassword::prompt_password;
use secrecy::SecretString;

pub fn execute() -> AppResult<()> {
    let password = read_master_password()?;
    let path = default_vault_path()?;
    let vault = load_or_init(&path, &password)?;

    let mut profiles: Vec<&String> = vault.profiles.keys().collect();
    profiles.sort();

    for profile in profiles {
        println!("{profile}");
    }

    Ok(())
}

fn read_master_password() -> AppResult<SecretString> {
    match std::env::var("ENVKEY_MASTER_PASSWORD") {
        Ok(pw) => Ok(SecretString::new(pw)),
        Err(_) => Ok(SecretString::new(prompt_password("Master password: ")?)),
    }
}
