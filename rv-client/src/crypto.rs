//! Cryptographic primitives for Rusty Vault.
//!
//! All encryption and decryption happens client-side. The server stores and
//! serves only ciphertext — it never has access to the plaintext or the key.

// Phase 4 will wire these into the push/pull commands.
#![allow(dead_code)]

use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, AeadCore, KeyInit},
    Aes256Gcm, Key,
};
use anyhow::{anyhow, Result};
use argon2::{Algorithm, Argon2, Params, Version};

const NONCE_LEN: usize = 12;

/// Derives a 256-bit AES key from a passphrase and salt using Argon2id.
///
/// Uses 64 MB of memory and 2 iterations — chosen to be slow enough to resist
/// brute-force while remaining usable on low-end hardware like a Raspberry Pi.
///
/// # Parameters
/// - `passphrase`: The user-supplied master passphrase.
/// - `salt`: A random, persistent salt stored in the client config (≥8 bytes).
///
/// # Panics
/// Panics if the Argon2 parameters are invalid, which cannot happen with the
/// hardcoded values used here.
pub fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    // m=65536 KiB (64 MB), t=2 iterations, p=1 lane, output=32 bytes.
    let params = Params::new(65536, 2, 1, Some(32)).expect("valid argon2 params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .expect("argon2 key derivation failed");
    key
}

/// Encrypts `plaintext` with AES-256-GCM and returns `nonce || ciphertext`.
///
/// A fresh random 96-bit nonce is generated for each call, so two encryptions
/// of identical plaintext will produce different outputs.
///
/// The returned payload format is:
/// ```text
/// [ nonce (12 bytes) ][ ciphertext + 16-byte GCM auth tag ]
/// ```
///
/// # Parameters
/// - `plaintext`: Raw bytes to encrypt.
/// - `key`: A 256-bit key, typically produced by [`derive_key`].
pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    // generate_nonce uses the cipher's built-in RNG adapter (rand feature)
    // and returns a GenericArray<u8, U12> — the correct nonce type for AES-GCM.
    let nonce = Aes256Gcm::generate_nonce(&mut rand::thread_rng());

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .expect("AES-256-GCM encryption failed");

    let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(nonce.as_slice());
    payload.extend_from_slice(&ciphertext);
    payload
}

/// Decrypts a payload produced by [`encrypt`].
///
/// The payload must begin with the 12-byte nonce followed by the ciphertext.
/// If the authentication tag does not verify, an error is returned — this means
/// either the wrong passphrase was used or the data has been tampered with.
///
/// # Parameters
/// - `payload`: The `nonce || ciphertext` bytes as received from the server.
/// - `key`: The 256-bit key to decrypt with (must match the key used to encrypt).
///
/// # Errors
/// Returns an error if the payload is shorter than 12 bytes or if
/// authentication/decryption fails (wrong key, corrupted data).
pub fn decrypt(payload: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if payload.len() < NONCE_LEN {
        return Err(anyhow!(
            "payload is too short to contain a nonce ({} bytes, need at least {})",
            payload.len(),
            NONCE_LEN
        ));
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    // GenericArray::from_slice borrows the slice; Rust infers the array length
    // (U12) from the cipher.decrypt() call below.
    let nonce = GenericArray::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("decryption failed: wrong passphrase or corrupted data"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{Algorithm, Argon2, Params, Version};

    /// Derives a test key using reduced Argon2 parameters for speed.
    fn test_key() -> [u8; 32] {
        let params = Params::new(8192, 1, 1, Some(32)).expect("valid test params");
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(b"test-passphrase", b"test-salt-16byte", &mut key)
            .expect("test key derivation failed");
        key
    }

    #[test]
    fn roundtrip_short_string() {
        let key = test_key();
        let plaintext = b"hello, vault!";
        let payload = encrypt(plaintext, &key);
        let decrypted = decrypt(&payload, &key).expect("decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn roundtrip_env_file() {
        let key = test_key();
        let plaintext = b"DATABASE_URL=postgres://user:pass@localhost/mydb\n\
                          SECRET_KEY=supersecretkey_do_not_share\n\
                          API_TOKEN=abc123xyz789\n\
                          DEBUG=false\n\
                          PORT=8080\n";
        let payload = encrypt(plaintext, &key);
        let decrypted = decrypt(&payload, &key).expect("decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_is_rejected() {
        let key = test_key();
        let mut wrong_key = key;
        wrong_key[0] ^= 0xff;

        let payload = encrypt(b"sensitive data", &key);
        assert!(
            decrypt(&payload, &wrong_key).is_err(),
            "decryption with a wrong key must fail authentication"
        );
    }

    #[test]
    fn payload_too_short_is_rejected() {
        let key = test_key();
        let too_short = [0u8; 5];
        assert!(decrypt(&too_short, &key).is_err());
    }

    #[test]
    fn each_encryption_produces_unique_output() {
        let key = test_key();
        let plaintext = b"same plaintext";
        let payload1 = encrypt(plaintext, &key);
        let payload2 = encrypt(plaintext, &key);
        assert_ne!(
            payload1, payload2,
            "random nonces must produce different ciphertexts for identical inputs"
        );
    }
}
