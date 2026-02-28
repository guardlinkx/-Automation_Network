//! VPN protocol definitions, tunnel configuration, and provider implementations.
//!
//! Supports WireGuard (via subprocess), IKEv2, Shadowsocks, and experimental
//! obfuscated / censorship-resistant protocols.

use std::net::SocketAddr;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

use crate::error::TunnelError;
use crate::Result;

// ---------------------------------------------------------------------------
// Protocol enum
// ---------------------------------------------------------------------------

/// Supported VPN tunnel protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// Standard WireGuard (UDP, kernel or userspace).
    WireGuard,
    /// IKEv2/IPsec.
    IKEv2,
    /// WireGuard with an obfuscation layer applied on top.
    ObfuscatedWireGuard,
    /// Shadowsocks proxy tunnel.
    Shadowsocks,
    /// Tor bridge transport.
    TorBridge,
    /// Califax Chameleon protocol -- combines XOR scramble, random padding,
    /// and TLS header wrapping for deep-packet-inspection evasion.
    Chameleon,
}

impl Protocol {
    /// Returns the default listening port for this protocol.
    pub fn default_port(&self) -> u16 {
        match self {
            Protocol::WireGuard => 51820,
            Protocol::IKEv2 => 500,
            Protocol::ObfuscatedWireGuard => 443,
            Protocol::Shadowsocks => 8388,
            Protocol::TorBridge => 9001,
            Protocol::Chameleon => 443,
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Protocol::WireGuard => "WireGuard",
            Protocol::IKEv2 => "IKEv2/IPsec",
            Protocol::ObfuscatedWireGuard => "Obfuscated WireGuard",
            Protocol::Shadowsocks => "Shadowsocks",
            Protocol::TorBridge => "Tor Bridge",
            Protocol::Chameleon => "Chameleon",
        }
    }

    /// Whether this protocol supports post-quantum cryptography key exchange.
    pub fn supports_pqc(&self) -> bool {
        matches!(
            self,
            Protocol::WireGuard | Protocol::ObfuscatedWireGuard | Protocol::Chameleon
        )
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

// ---------------------------------------------------------------------------
// Tunnel configuration
// ---------------------------------------------------------------------------

/// Full configuration needed to establish a VPN tunnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// Which protocol to use.
    pub protocol: Protocol,
    /// Remote VPN server endpoint (ip:port).
    pub server_endpoint: SocketAddr,
    /// Client private key (base64-encoded).
    pub client_private_key: String,
    /// Server public key (base64-encoded).
    pub server_public_key: String,
    /// IP address assigned to this client inside the tunnel.
    pub assigned_ip: String,
    /// DNS servers to use while the tunnel is active.
    pub dns: Vec<String>,
    /// Maximum transmission unit for the tunnel interface.
    pub mtu: u16,
    /// Keepalive interval in seconds (0 = disabled).
    pub keepalive: u16,
    /// Enable post-quantum cryptographic key exchange.
    pub pqc_enabled: bool,
    /// Enable traffic obfuscation on top of the tunnel.
    pub obfuscation_enabled: bool,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            protocol: Protocol::WireGuard,
            server_endpoint: "0.0.0.0:51820".parse().unwrap(),
            client_private_key: String::new(),
            server_public_key: String::new(),
            assigned_ip: "10.0.0.2".into(),
            dns: vec!["1.1.1.1".into(), "1.0.0.1".into()],
            mtu: 1420,
            keepalive: 25,
            pqc_enabled: false,
            obfuscation_enabled: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Tunnel state & status
// ---------------------------------------------------------------------------

/// Current state of a tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

impl std::fmt::Display for TunnelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            TunnelState::Disconnected => "Disconnected",
            TunnelState::Connecting => "Connecting",
            TunnelState::Connected => "Connected",
            TunnelState::Reconnecting => "Reconnecting",
            TunnelState::Failed => "Failed",
        };
        f.write_str(label)
    }
}

/// Runtime status snapshot of a tunnel.
#[derive(Debug, Clone)]
pub struct TunnelStatus {
    /// Protocol in use.
    pub protocol: Protocol,
    /// Current state.
    pub state: TunnelState,
    /// When the tunnel entered the `Connected` state (if ever).
    pub connected_since: Option<Instant>,
    /// Total bytes transmitted through the tunnel.
    pub bytes_tx: u64,
    /// Total bytes received through the tunnel.
    pub bytes_rx: u64,
    /// Server we are currently connected to (display string).
    pub current_server: Option<String>,
}

impl TunnelStatus {
    /// Create a fresh status in the `Disconnected` state.
    pub fn disconnected(protocol: Protocol) -> Self {
        Self {
            protocol,
            state: TunnelState::Disconnected,
            connected_since: None,
            bytes_tx: 0,
            bytes_rx: 0,
            current_server: None,
        }
    }
}

// ---------------------------------------------------------------------------
// TunnelProvider trait
// ---------------------------------------------------------------------------

/// Trait that every protocol-specific tunnel implementation must satisfy.
///
/// Implementations are expected to be `Send + Sync` so they can be shared
/// across async tasks.
#[async_trait]
pub trait TunnelProvider: Send + Sync {
    /// Establish the tunnel using the given configuration.
    async fn connect(&mut self, config: &TunnelConfig) -> Result<()>;

    /// Tear down the tunnel gracefully.
    async fn disconnect(&mut self) -> Result<()>;

    /// Return a snapshot of the current tunnel status.
    fn status(&self) -> TunnelStatus;

    /// Convenience: returns `true` when the tunnel state is `Connected`.
    fn is_connected(&self) -> bool {
        self.status().state == TunnelState::Connected
    }
}

// ---------------------------------------------------------------------------
// WireGuardProvider
// ---------------------------------------------------------------------------

/// WireGuard tunnel provider -- manages WireGuard via `wg` and `wg-quick`
/// subprocess commands.
pub struct WireGuardProvider {
    status: TunnelStatus,
}

impl WireGuardProvider {
    pub fn new() -> Self {
        Self {
            status: TunnelStatus::disconnected(Protocol::WireGuard),
        }
    }

    /// Write a WireGuard configuration file and bring the interface up via
    /// `wg-quick up`.
    async fn bring_up(&self, config: &TunnelConfig) -> std::result::Result<(), String> {
        let conf = format!(
            "[Interface]\n\
             PrivateKey = {}\n\
             Address = {}\n\
             DNS = {}\n\
             MTU = {}\n\
             \n\
             [Peer]\n\
             PublicKey = {}\n\
             Endpoint = {}\n\
             AllowedIPs = 0.0.0.0/0, ::/0\n\
             PersistentKeepalive = {}\n",
            config.client_private_key,
            config.assigned_ip,
            config.dns.join(", "),
            config.mtu,
            config.server_public_key,
            config.server_endpoint,
            config.keepalive,
        );

        // Write temp config
        let conf_path = std::env::temp_dir().join("califax_wg0.conf");
        tokio::fs::write(&conf_path, &conf)
            .await
            .map_err(|e| format!("failed to write wg config: {e}"))?;

        // Bring interface up
        let output = tokio::process::Command::new("wg-quick")
            .args(["up", conf_path.to_str().unwrap_or("califax_wg0.conf")])
            .output()
            .await
            .map_err(|e| format!("failed to run wg-quick: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("wg-quick up failed: {stderr}"));
        }

        Ok(())
    }

    /// Tear down the WireGuard interface via `wg-quick down`.
    async fn bring_down(&self) -> std::result::Result<(), String> {
        let conf_path = std::env::temp_dir().join("califax_wg0.conf");
        let output = tokio::process::Command::new("wg-quick")
            .args(["down", conf_path.to_str().unwrap_or("califax_wg0.conf")])
            .output()
            .await
            .map_err(|e| format!("failed to run wg-quick: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("wg-quick down failed: {stderr}"));
        }

        // Clean up config file
        let _ = tokio::fs::remove_file(&conf_path).await;

        Ok(())
    }
}

#[async_trait]
impl TunnelProvider for WireGuardProvider {
    async fn connect(&mut self, config: &TunnelConfig) -> Result<()> {
        if self.status.state == TunnelState::Connected {
            return Err(TunnelError::TunnelAlreadyActive);
        }

        info!(
            protocol = %Protocol::WireGuard,
            server = %config.server_endpoint,
            "initiating WireGuard tunnel"
        );

        self.status.state = TunnelState::Connecting;
        self.status.current_server = Some(config.server_endpoint.to_string());

        match self.bring_up(config).await {
            Ok(()) => {
                self.status.state = TunnelState::Connected;
                self.status.connected_since = Some(Instant::now());
                info!("WireGuard tunnel established");
                Ok(())
            }
            Err(e) => {
                self.status.state = TunnelState::Failed;
                error!(error = %e, "WireGuard tunnel failed");
                Err(TunnelError::ConnectionFailed(e))
            }
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        if self.status.state == TunnelState::Disconnected {
            return Err(TunnelError::TunnelNotActive);
        }

        info!("tearing down WireGuard tunnel");

        match self.bring_down().await {
            Ok(()) => {
                self.status = TunnelStatus::disconnected(Protocol::WireGuard);
                info!("WireGuard tunnel disconnected");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "WireGuard teardown encountered an error");
                // Force state to disconnected even on error
                self.status = TunnelStatus::disconnected(Protocol::WireGuard);
                Ok(())
            }
        }
    }

    fn status(&self) -> TunnelStatus {
        self.status.clone()
    }
}

// ---------------------------------------------------------------------------
// IKEv2Provider (stub)
// ---------------------------------------------------------------------------

/// IKEv2/IPsec tunnel provider -- placeholder implementation.
///
/// Returns `ProtocolUnsupported` for all connection attempts until a proper
/// strongSwan / libreswan integration is implemented.
pub struct IKEv2Provider {
    status: TunnelStatus,
}

impl IKEv2Provider {
    pub fn new() -> Self {
        Self {
            status: TunnelStatus::disconnected(Protocol::IKEv2),
        }
    }
}

#[async_trait]
impl TunnelProvider for IKEv2Provider {
    async fn connect(&mut self, _config: &TunnelConfig) -> Result<()> {
        warn!("IKEv2 provider is not yet implemented");
        self.status.state = TunnelState::Failed;
        Err(TunnelError::ProtocolUnsupported(
            "IKEv2/IPsec support is not yet implemented".into(),
        ))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.status = TunnelStatus::disconnected(Protocol::IKEv2);
        Ok(())
    }

    fn status(&self) -> TunnelStatus {
        self.status.clone()
    }
}

// ---------------------------------------------------------------------------
// ShadowsocksProvider (stub)
// ---------------------------------------------------------------------------

/// Shadowsocks tunnel provider -- placeholder implementation.
///
/// Returns `ProtocolUnsupported` for all connection attempts until a proper
/// Shadowsocks client integration is completed.
pub struct ShadowsocksProvider {
    status: TunnelStatus,
}

impl ShadowsocksProvider {
    pub fn new() -> Self {
        Self {
            status: TunnelStatus::disconnected(Protocol::Shadowsocks),
        }
    }
}

#[async_trait]
impl TunnelProvider for ShadowsocksProvider {
    async fn connect(&mut self, _config: &TunnelConfig) -> Result<()> {
        warn!("Shadowsocks provider is not yet implemented");
        self.status.state = TunnelState::Failed;
        Err(TunnelError::ProtocolUnsupported(
            "Shadowsocks support is not yet implemented".into(),
        ))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.status = TunnelStatus::disconnected(Protocol::Shadowsocks);
        Ok(())
    }

    fn status(&self) -> TunnelStatus {
        self.status.clone()
    }
}
