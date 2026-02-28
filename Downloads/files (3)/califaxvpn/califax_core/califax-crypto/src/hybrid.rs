//! Hybrid key exchange combining X25519 (classical) and Kyber-1024 (post-quantum).
//!
//! The two shared secrets are combined using HKDF-SHA256 to produce a single
//! 32-byte key that is secure against both classical and quantum adversaries.

use hkdf::Hkdf;
use rand::rngs::OsRng;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::pqc::{self, KyberKeypair};
use crate::{CryptoError, Result};

/// HKDF info string that domain-separates the Califax hybrid KDF output.
const HKDF_INFO: &[u8] = b"califax-vpn-hybrid-kex-v1";

/// A hybrid key pair containing both X25519 and Kyber-1024 key material.
pub struct HybridKeypair {
    /// X25519 static secret key.
    pub x25519_secret: StaticSecret,
    /// X25519 public key derived from the secret.
    pub x25519_public: X25519PublicKey,
    /// Kyber-1024 key pair.
    pub kyber_keypair: KyberKeypair,
}

/// A 32-byte shared secret produced by the hybrid key exchange.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct HybridSharedSecret {
    /// The combined shared secret.
    pub combined: [u8; 32],
}

/// Generates a new hybrid key pair (X25519 + Kyber-1024).
///
/// # Errors
///
/// Returns `CryptoError::KeyGenerationFailed` if Kyber key generation fails.
pub fn generate_hybrid_keypair() -> Result<HybridKeypair> {
    let x25519_secret = StaticSecret::random_from_rng(OsRng);
    let x25519_public = X25519PublicKey::from(&x25519_secret);
    let kyber_keypair = pqc::generate_kyber_keypair()?;

    Ok(HybridKeypair {
        x25519_secret,
        x25519_public,
        kyber_keypair,
    })
}

/// Performs the initiator side of the hybrid key exchange.
///
/// Given the peer's X25519 public key and Kyber public key, this function:
/// 1. Generates an ephemeral X25519 secret and computes the DH shared secret.
/// 2. Encapsulates a Kyber shared secret against the peer's Kyber public key.
/// 3. Combines both shared secrets via HKDF-SHA256.
///
/// Returns the combined shared secret, the ephemeral X25519 public key, and
/// the Kyber ciphertext. The caller must send the X25519 public key and Kyber
/// ciphertext to the peer.
///
/// # Errors
///
/// Returns an error if Kyber encapsulation or HKDF derivation fails.
pub fn hybrid_encapsulate(
    peer_x25519_public: &[u8; 32],
    peer_kyber_public: &[u8],
) -> Result<(HybridSharedSecret, [u8; 32], Vec<u8>)> {
    // X25519 ephemeral DH
    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);
    let peer_pk = X25519PublicKey::from(*peer_x25519_public);
    let x25519_shared = ephemeral_secret.diffie_hellman(&peer_pk);

    // Kyber-1024 KEM encapsulation
    let (kyber_shared, kyber_ct) = pqc::kyber_encapsulate(peer_kyber_public)?;

    // Combine via HKDF-SHA256
    let combined = combine_secrets(x25519_shared.as_bytes(), &kyber_shared)?;

    Ok((combined, ephemeral_public.to_bytes(), kyber_ct))
}

/// Performs the responder side of the hybrid key exchange.
///
/// Given the local hybrid key pair, the peer's ephemeral X25519 public key,
/// and the Kyber ciphertext, this function:
/// 1. Computes the X25519 DH shared secret.
/// 2. Decapsulates the Kyber shared secret.
/// 3. Combines both shared secrets via HKDF-SHA256.
///
/// # Errors
///
/// Returns an error if Kyber decapsulation or HKDF derivation fails.
pub fn hybrid_decapsulate(
    keypair: &HybridKeypair,
    peer_x25519_public: &[u8; 32],
    kyber_ciphertext: &[u8],
) -> Result<HybridSharedSecret> {
    // X25519 DH
    let peer_pk = X25519PublicKey::from(*peer_x25519_public);
    let x25519_shared = keypair.x25519_secret.diffie_hellman(&peer_pk);

    // Kyber-1024 KEM decapsulation
    let kyber_shared =
        pqc::kyber_decapsulate(&keypair.kyber_keypair.secret_key, kyber_ciphertext)?;

    // Combine via HKDF-SHA256
    combine_secrets(x25519_shared.as_bytes(), &kyber_shared)
}

/// Combines two shared secrets using HKDF-SHA256.
///
/// The input keying material is the concatenation of both secrets.
fn combine_secrets(
    x25519_secret: &[u8],
    kyber_secret: &[u8],
) -> Result<HybridSharedSecret> {
    let mut ikm = Vec::with_capacity(x25519_secret.len() + kyber_secret.len());
    ikm.extend_from_slice(x25519_secret);
    ikm.extend_from_slice(kyber_secret);

    let hk = Hkdf::<Sha256>::new(None, &ikm);
    let mut okm = [0u8; 32];
    hk.expand(HKDF_INFO, &mut okm).map_err(|e| {
        CryptoError::KeyGenerationFailed(format!("HKDF expansion failed: {e}"))
    })?;

    // Zeroize intermediate keying material
    ikm.zeroize();

    Ok(HybridSharedSecret { combined: okm })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_roundtrip() {
        let responder = generate_hybrid_keypair().expect("responder keypair failed");

        let (initiator_secret, eph_pub, kyber_ct) = hybrid_encapsulate(
            responder.x25519_public.as_bytes(),
            &responder.kyber_keypair.public_key,
        )
        .expect("encapsulate failed");

        let responder_secret =
            hybrid_decapsulate(&responder, &eph_pub, &kyber_ct)
                .expect("decapsulate failed");

        assert_eq!(
            initiator_secret.combined, responder_secret.combined,
            "hybrid shared secrets must match"
        );
    }
}
