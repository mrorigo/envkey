use crate::error::{AppError, AppResult};
use crate::vault::store::{default_vault_path, load_or_init};
use rpassword::prompt_password;
use secrecy::SecretString;

pub fn execute(profile: &str) -> AppResult<()> {
    let password = read_master_password()?;
    let path = default_vault_path()?;
    let vault = load_or_init(&path, &password)?;

    let selected = vault
        .profiles
        .get(profile)
        .ok_or_else(|| AppError::ProfileNotFound(profile.to_string()))?;

    let mut vars: Vec<(&String, &String)> = selected.vars.iter().collect();
    vars.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (key, value) in vars {
        println!("export {}={}", key, shell_single_quote(value));
    }

    Ok(())
}

fn read_master_password() -> AppResult<SecretString> {
    match std::env::var("ENVKEY_MASTER_PASSWORD") {
        Ok(pw) => Ok(SecretString::new(pw)),
        Err(_) => Ok(SecretString::new(prompt_password("Master password: ")?)),
    }
}

fn shell_single_quote(input: &str) -> String {
    let escaped = input.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

#[cfg(test)]
mod tests {
    use super::shell_single_quote;

    #[test]
    fn escapes_single_quotes_for_shell_eval() {
        assert_eq!(shell_single_quote("abc"), "'abc'");
        assert_eq!(shell_single_quote("a'b"), "'a'\\''b'");
    }
}
