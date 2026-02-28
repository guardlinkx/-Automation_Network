//! # califax-tunnel
//!
//! Multi-protocol VPN tunnel management crate for the Califax VPN project.
//!
//! This crate provides:
//! - **protocol**: VPN protocol definitions, tunnel configuration, and provider traits
//!   (WireGuard, IKEv2, Shadowsocks, and more)
//! - **failover**: Ordered protocol failover chains with automatic retry logic
//! - **smart_switch**: AI-driven protocol selection based on network conditions
//! - **obfuscation**: Traffic obfuscation layers for DPI evasion (XOR, TLS mimicry,
//!   HTTP masquerade, Chameleon)
//! - **double_tunnel**: Nested VPN-in-VPN for maximum privacy
//! - **error**: Unified error types for tunnel operations

pub mod protocol;
pub mod failover;
pub mod smart_switch;
pub mod obfuscation;
pub mod double_tunnel;
pub mod error;

pub use error::TunnelError;

/// Convenience Result type for tunnel operations.
pub type Result<T> = std::result::Result<T, TunnelError>;
