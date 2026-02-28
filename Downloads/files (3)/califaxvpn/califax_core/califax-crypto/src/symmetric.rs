//! Symmetric authenticated encryption using AES-256-GCM or ChaCha20-Poly1305.
//!
//! Both cipher suites provide 256-bit key security with 96-bit nonces and
//! support additional authenticated data (AAD).

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce as AesNonce,
};
use chacha20poly1305::{ChaCha20Poly1305, Nonce as ChaNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::{CryptoError, Result};

/// Supported symmetric cipher suites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CipherSuite {
    /// AES-256 in Galois/Counter Mode.
    Aes256Gcm,
    /// ChaCha20 stream cipher with Poly1305 MAC.
    ChaCha20Poly1305,
}

/// Encrypts `plaintext` with the specified cipher suite, key, and optional AAD.
///
/// Returns the randomly generated 12-byte nonce and the ciphertext (which
/// includes the authentication tag).
///
/// # Errors
///
/// Returns `CryptoError::InvalidKeyLength` if `key` is not 32 bytes.
/// Returns `CryptoError::EncryptionFailed` if the AEAD operation fails.
pub fn encrypt(
    cipher: CipherSuite,
    key: &[u8],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<([u8; 12], Vec<u8>)> {
    if key.len() != 32 {
        return Err(CryptoError::InvalidKeyLength {
            expected: 32,
            actual: key.len(),
        });
    }

    let nonce = generate_nonce();

    let ciphertext = match cipher {
        CipherSuite::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| {
                CryptoError::EncryptionFailed(format!("AES-256-GCM init: {e}"))
            })?;
            let aes_nonce = AesNonce::from_slice(&nonce);
            let payload = Payload { msg: plaintext, aad };
            cipher.encrypt(aes_nonce, payload).map_err(|e| {
                CryptoError::EncryptionFailed(format!("AES-256-GCM encrypt: {e}"))
            })?
        }
        CipherSuite::ChaCha20Poly1305 => {
            let cipher =
                ChaCha20Poly1305::new_from_slice(key).map_err(|e| {
                    CryptoError::EncryptionFailed(format!("ChaCha20-Poly1305 init: {e}"))
                })?;
            let cha_nonce = ChaNonce::from_slice(&nonce);
            let payload = Payload { msg: plaintext, aad };
            cipher.encrypt(cha_nonce, payload).map_err(|e| {
                CryptoError::EncryptionFailed(format!(
                    "ChaCha20-Poly1305 encrypt: {e}"
                ))
            })?
        }
    };

    Ok((nonce, ciphertext))
}

/// Decrypts `ciphertext` with the specified cipher suite, key, nonce, and AAD.
///
/// # Errors
///
/// Returns `CryptoError::InvalidKeyLength` if `key` is not 32 bytes.
/// Returns `CryptoError::DecryptionFailed` if the AEAD operation fails
/// (e.g., authentication tag mismatch or corrupted data).
pub fn decrypt(
    cipher: CipherSuite,
    key: &[u8],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(CryptoError::InvalidKeyLength {
            expected: 32,
            actual: key.len(),
        });
    }

    let plaintext = match cipher {
        CipherSuite::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| {
                CryptoError::DecryptionFailed(format!("AES-256-GCM init: {e}"))
            })?;
            let aes_nonce = AesNonce::from_slice(nonce);
            let payload = Payload {
                msg: ciphertext,
                aad,
            };
            cipher.decrypt(aes_nonce, payload).map_err(|e| {
                CryptoError::DecryptionFailed(format!("AES-256-GCM decrypt: {e}"))
            })?
        }
        CipherSuite::ChaCha20Poly1305 => {
            let cipher =
                ChaCha20Poly1305::new_from_slice(key).map_err(|e| {
                    CryptoError::DecryptionFailed(format!(
                        "ChaCha20-Poly1305 init: {e}"
                    ))
                })?;
            let cha_nonce = ChaNonce::from_slice(nonce);
            let payload = Payload {
                msg: ciphertext,
                aad,
            };
            cipher.decrypt(cha_nonce, payload).map_err(|e| {
                CryptoError::DecryptionFailed(format!(
                    "ChaCha20-Poly1305 decrypt: {e}"
                ))
            })?
        }
    };

    Ok(plaintext)
}

/// Generates a random 256-bit (32-byte) symmetric key.
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    key
}

/// Generates a random 96-bit (12-byte) nonce.
pub fn generate_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes256gcm_roundtrip() {
        let key = generate_key();
        let plaintext = b"Hello, Califax VPN!";
        let aad = b"session-001";

        let (nonce, ct) =
            encrypt(CipherSuite::Aes256Gcm, &key, plaintext, aad)
                .expect("encrypt failed");
        let pt = decrypt(CipherSuite::Aes256Gcm, &key, &nonce, &ct, aad)
            .expect("decrypt failed");

        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_chacha20poly1305_roundtrip() {
        let key = generate_key();
        let plaintext = b"Post-quantum secure tunnel data";
        let aad = b"tunnel-42";

        let (nonce, ct) =
            encrypt(CipherSuite::ChaCha20Poly1305, &key, plaintext, aad)
                .expect("encrypt failed");
        let pt =
            decrypt(CipherSuite::ChaCha20Poly1305, &key, &nonce, &ct, aad)
                .expect("decrypt failed");

        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = [0u8; 16];
        let result = encrypt(CipherSuite::Aes256Gcm, &short_key, b"data", b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext() {
        let key = generate_key();
        let (nonce, mut ct) =
            encrypt(CipherSuite::Aes256Gcm, &key, b"secret", b"")
                .expect("encrypt failed");
        // Flip a bit in the ciphertext
        ct[0] ^= 0xFF;
        let result = decrypt(CipherSuite::Aes256Gcm, &key, &nonce, &ct, b"");
        assert!(result.is_err());
    }
}
