use thiserror::Error;

#[derive(Error, Debug)]
pub enum MeshError {
    #[error("Not enough peers for circuit: need {needed}, have {available}")]
    InsufficientPeers { needed: usize, available: usize },
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    #[error("Circuit construction failed: {0}")]
    CircuitBuildFailed(String),
    #[error("DHT lookup failed: {0}")]
    DhtLookupFailed(String),
    #[error("Gossip propagation failed: {0}")]
    GossipFailed(String),
    #[error("Onion encryption failed: {0}")]
    OnionEncryptionFailed(String),
    #[error("Circuit expired")]
    CircuitExpired,
    #[error("Node unreachable: {0}")]
    NodeUnreachable(String),
    #[error("Crypto error: {0}")]
    Crypto(#[from] califax_crypto::CryptoError),
}
