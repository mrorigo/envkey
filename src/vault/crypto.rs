use crate::vault::VaultError;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use rand::rngs::OsRng;
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

fn argon2_instance() -> Argon2<'static> {
    let params = Params::new(19_456, 2, 1, Some(KEY_LEN)).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

fn random_nonce() -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

pub fn derive_key(
    password: &SecretString,
    salt: &[u8; SALT_LEN],
) -> Result<[u8; KEY_LEN], VaultError> {
    let mut key = [0u8; KEY_LEN];
    argon2_instance()
        .hash_password_into(password.expose_secret().as_bytes(), salt, &mut key)
        .map_err(|_| VaultError::CorruptVault)?;
    Ok(key)
}

pub fn encrypt(
    plaintext: &[u8],
    password: &SecretString,
    salt: &[u8; SALT_LEN],
) -> Result<(Vec<u8>, [u8; NONCE_LEN]), VaultError> {
    let mut key_bytes = derive_key(password, salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes = random_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| VaultError::CorruptVault)?;
    key_bytes.zeroize();
    Ok((ciphertext, nonce_bytes))
}

pub fn decrypt(
    ciphertext: &[u8],
    password: &SecretString,
    salt: &[u8; SALT_LEN],
    nonce_bytes: &[u8; NONCE_LEN],
) -> Result<Vec<u8>, VaultError> {
    let mut key_bytes = derive_key(password, salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| VaultError::InvalidPassword)?;
    key_bytes.zeroize();
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pw() -> SecretString {
        SecretString::new("pw".to_string())
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let salt = random_salt();
        let data = b"hello";
        let (ciphertext, nonce) = encrypt(data, &pw(), &salt).expect("encrypt");
        let plaintext = decrypt(&ciphertext, &pw(), &salt, &nonce).expect("decrypt");
        assert_eq!(plaintext, data);
    }

    #[test]
    fn wrong_password_fails() {
        let salt = random_salt();
        let data = b"secret";
        let (ciphertext, nonce) = encrypt(data, &pw(), &salt).expect("encrypt");
        let err = decrypt(
            &ciphertext,
            &SecretString::new("wrong".to_string()),
            &salt,
            &nonce,
        )
        .expect_err("must fail");
        assert!(matches!(err, VaultError::InvalidPassword));
    }
}
