//! Unified error types for tunnel operations.

use thiserror::Error;

/// Errors that can occur during VPN tunnel operations.
#[derive(Debug, Error)]
pub enum TunnelError {
    /// Failed to establish a connection to the VPN server.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// The requested VPN protocol is not supported on this platform.
    #[error("protocol unsupported: {0}")]
    ProtocolUnsupported(String),

    /// The cryptographic handshake with the server failed.
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),

    /// Traffic obfuscation processing failed.
    #[error("obfuscation failed: {0}")]
    ObfuscationFailed(String),

    /// Every protocol in the failover chain was attempted and none succeeded.
    #[error("all protocols in failover chain failed")]
    AllProtocolsFailed,

    /// A tunnel is already active; disconnect first before connecting again.
    #[error("tunnel is already active")]
    TunnelAlreadyActive,

    /// No tunnel is currently active to perform the requested operation.
    #[error("tunnel is not active")]
    TunnelNotActive,

    /// Invalid or missing tunnel configuration.
    #[error("configuration error: {0}")]
    ConfigError(String),
}
