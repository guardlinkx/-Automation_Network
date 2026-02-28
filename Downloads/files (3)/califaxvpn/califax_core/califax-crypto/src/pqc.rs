//! Post-quantum cryptography module using CRYSTALS-Kyber-1024.
//!
//! Provides key generation, encapsulation, and decapsulation for
//! the Kyber-1024 key encapsulation mechanism (KEM), which is
//! believed to be resistant to attacks by quantum computers.

use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};
use zeroize::Zeroize;

use crate::{CryptoError, Result};

/// A Kyber-1024 key pair consisting of a public key and a secret key.
#[derive(Clone)]
pub struct KyberKeypair {
    /// The public key, safe to share with peers.
    pub public_key: Vec<u8>,
    /// The secret key, must be kept private.
    pub secret_key: Vec<u8>,
}

impl Drop for KyberKeypair {
    fn drop(&mut self) {
        self.secret_key.zeroize();
    }
}

/// Generates a new Kyber-1024 key pair.
///
/// # Errors
///
/// Returns `CryptoError::KeyGenerationFailed` if the underlying library
/// fails to produce a valid key pair.
pub fn generate_kyber_keypair() -> Result<KyberKeypair> {
    let (pk, sk) = kyber1024::keypair();
    let public_key = pk.as_bytes().to_vec();
    let secret_key = sk.as_bytes().to_vec();

    if public_key.is_empty() || secret_key.is_empty() {
        return Err(CryptoError::KeyGenerationFailed(
            "Kyber-1024 keypair generation produced empty keys".into(),
        ));
    }

    Ok(KyberKeypair {
        public_key,
        secret_key,
    })
}

/// Encapsulates a shared secret using the peer's Kyber-1024 public key.
///
/// Returns the shared secret and the ciphertext that must be sent to the peer.
///
/// # Errors
///
/// Returns `CryptoError::EncapsulationFailed` if the public key is malformed
/// or encapsulation otherwise fails.
pub fn kyber_encapsulate(public_key: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let pk = kyber1024::PublicKey::from_bytes(public_key).map_err(|e| {
        CryptoError::EncapsulationFailed(format!("invalid public key: {e}"))
    })?;

    let (ss, ct) = kyber1024::encapsulate(&pk);

    Ok((ss.as_bytes().to_vec(), ct.as_bytes().to_vec()))
}

/// Decapsulates a shared secret using the local secret key and the received ciphertext.
///
/// # Errors
///
/// Returns `CryptoError::DecapsulationFailed` if the secret key or ciphertext
/// is malformed, or decapsulation otherwise fails.
pub fn kyber_decapsulate(secret_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let sk = kyber1024::SecretKey::from_bytes(secret_key).map_err(|e| {
        CryptoError::DecapsulationFailed(format!("invalid secret key: {e}"))
    })?;

    let ct = kyber1024::Ciphertext::from_bytes(ciphertext).map_err(|e| {
        CryptoError::DecapsulationFailed(format!("invalid ciphertext: {e}"))
    })?;

    let ss = kyber1024::decapsulate(&ct, &sk);

    Ok(ss.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyber_roundtrip() {
        let keypair = generate_kyber_keypair().expect("keypair generation failed");
        let (ss_enc, ct) =
            kyber_encapsulate(&keypair.public_key).expect("encapsulation failed");
        let ss_dec =
            kyber_decapsulate(&keypair.secret_key, &ct).expect("decapsulation failed");
        assert_eq!(ss_enc, ss_dec, "shared secrets must match");
    }
}
