//! # califax-crypto
//!
//! Post-quantum hybrid cryptography crate for the Califax VPN project.
//!
//! This crate provides:
//! - **pqc**: CRYSTALS-Kyber-1024 post-quantum key encapsulation
//! - **hybrid**: Hybrid X25519 + Kyber key exchange with HKDF-SHA256 combination
//! - **symmetric**: AES-256-GCM and ChaCha20-Poly1305 authenticated encryption
//! - **keystore**: Pluggable key storage with in-memory and HSM backends
//! - **error**: Unified error types for all cryptographic operations

pub mod error;
pub mod hybrid;
pub mod keystore;
pub mod pqc;
pub mod symmetric;

pub use error::CryptoError;

/// Convenience Result type for cryptographic operations.
pub type Result<T> = std::result::Result<T, CryptoError>;
