use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("DID not found: {0}")]
    DidNotFound(String),
    #[error("DID already registered: {0}")]
    DidAlreadyRegistered(String),
    #[error("ZK proof verification failed: {0}")]
    ZkVerificationFailed(String),
    #[error("ZK proof generation failed: {0}")]
    ZkGenerationFailed(String),
    #[error("Invalid proof format: {0}")]
    InvalidProofFormat(String),
    #[error("Canary is dead — service may be compromised")]
    CanaryDead,
    #[error("Access denied: {0}")]
    AccessDenied(String),
    #[error("Blockchain unavailable: {0}")]
    BlockchainUnavailable(String),
    #[error("Crypto error: {0}")]
    Crypto(#[from] califax_crypto::CryptoError),
}
