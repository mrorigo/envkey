use crate::vault::crypto::{NONCE_LEN, SALT_LEN, decrypt, encrypt, random_salt};
use crate::vault::{CURRENT_SCHEMA_VERSION, Vault, VaultError};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

const DIR_NAME: &str = ".envkey";
const FILE_NAME: &str = "vault.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedVaultFile {
    schema_version: u32,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
    ciphertext: Vec<u8>,
}

pub fn default_vault_path() -> Result<PathBuf, VaultError> {
    let home = std::env::var("HOME").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "HOME environment variable not set",
        )
    })?;
    Ok(Path::new(&home).join(DIR_NAME).join(FILE_NAME))
}

pub fn load_or_init(path: &Path, password: &SecretString) -> Result<Vault, VaultError> {
    if !path.exists() {
        return Ok(Vault::default());
    }

    let data = fs::read(path)?;
    let encrypted: EncryptedVaultFile = serde_json::from_slice(&data)?;

    if encrypted.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(VaultError::UnsupportedSchemaVersion(
            encrypted.schema_version,
        ));
    }

    let mut plaintext = decrypt(
        &encrypted.ciphertext,
        password,
        &encrypted.salt,
        &encrypted.nonce,
    )?;

    let vault: Vault = serde_json::from_slice(&plaintext)?;
    plaintext.zeroize();

    if vault.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(VaultError::UnsupportedSchemaVersion(vault.schema_version));
    }

    Ok(vault)
}

pub fn save(path: &Path, vault: &Vault, password: &SecretString) -> Result<(), VaultError> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid vault path")
    })?;

    if !parent.exists() {
        fs::create_dir_all(parent)?;
        set_dir_permissions(parent)?;
    }

    let mut plaintext = serde_json::to_vec(vault)?;
    let salt = random_salt();
    let (ciphertext, nonce) = encrypt(&plaintext, password, &salt)?;

    let payload = EncryptedVaultFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        salt,
        nonce,
        ciphertext,
    };

    let payload_bytes = serde_json::to_vec(&payload)?;

    let tmp_path = path.with_extension("db.tmp");
    let mut tmp_file = File::create(&tmp_path)?;
    tmp_file.write_all(&payload_bytes)?;
    tmp_file.sync_all()?;
    fs::rename(&tmp_path, path)?;

    plaintext.zeroize();
    Ok(())
}

fn set_dir_permissions(path: &Path) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn pw() -> SecretString {
        SecretString::new("test-password".to_string())
    }

    #[test]
    fn roundtrip_save_and_load() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.db");
        let mut vault = Vault::default();
        vault
            .profiles
            .entry("dev".to_string())
            .or_default()
            .vars
            .insert("API_KEY".to_string(), "abc123".to_string());

        save(&path, &vault, &pw()).expect("save");
        let loaded = load_or_init(&path, &pw()).expect("load");

        assert_eq!(loaded.profiles["dev"].vars["API_KEY"], "abc123");
    }

    #[test]
    fn wrong_password_fails() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.db");

        save(&path, &Vault::default(), &pw()).expect("save");
        let err =
            load_or_init(&path, &SecretString::new("wrong".to_string())).expect_err("must fail");

        assert!(matches!(err, VaultError::InvalidPassword));
    }

    #[test]
    fn tamper_detected() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.db");
        save(&path, &Vault::default(), &pw()).expect("save");

        let mut bytes = fs::read(&path).expect("read");
        let idx = bytes.len() / 2;
        bytes[idx] ^= 0x01;
        fs::write(&path, bytes).expect("write");

        let err = load_or_init(&path, &pw()).expect_err("must fail");
        assert!(matches!(
            err,
            VaultError::InvalidPassword | VaultError::Serialize(_) | VaultError::CorruptVault
        ));
    }
}
