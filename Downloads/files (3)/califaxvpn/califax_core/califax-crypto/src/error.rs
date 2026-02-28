//! Unified error types for all cryptographic operations in Califax.

use thiserror::Error;

/// Errors that can occur during cryptographic operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Failed to generate a cryptographic key pair.
    #[error("key generation failed: {0}")]
    KeyGenerationFailed(String),

    /// Failed to encapsulate a shared secret using a public key.
    #[error("encapsulation failed: {0}")]
    EncapsulationFailed(String),

    /// Failed to decapsulate a shared secret using a secret key and ciphertext.
    #[error("decapsulation failed: {0}")]
    DecapsulationFailed(String),

    /// Failed to encrypt plaintext.
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    /// Failed to decrypt ciphertext.
    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    /// The provided key material has an invalid length.
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    /// The hardware security module is not available.
    #[error("HSM unavailable: {0}")]
    HsmUnavailable(String),

    /// The requested key was not found in the key store.
    #[error("key not found: {0}")]
    KeyNotFound(String),
}
