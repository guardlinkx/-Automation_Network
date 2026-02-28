//! Nested VPN-in-VPN (double tunnel) for maximum privacy.
//!
//! A `DoubleTunnel` establishes two layered tunnels:
//!
//! 1. An **outer** tunnel connects to a first-hop VPN server.
//! 2. An **inner** tunnel runs *inside* the outer one, connecting to a
//!    second-hop server.
//!
//! All traffic traverses both hops, so neither server alone can correlate
//! the client's real IP with the destination.

use std::time::Instant;

use tracing::{info, warn, error};

use crate::error::TunnelError;
use crate::protocol::{
    Protocol, TunnelConfig, TunnelProvider, TunnelState, TunnelStatus,
    IKEv2Provider, ShadowsocksProvider, WireGuardProvider,
};
use crate::Result;

// ---------------------------------------------------------------------------
// DoubleTunnel
// ---------------------------------------------------------------------------

/// A nested VPN-in-VPN tunnel manager.
///
/// The outer tunnel is established first; once it is connected the inner
/// tunnel is brought up inside it.  Disconnection tears them down in
/// reverse order (inner first, then outer).
pub struct DoubleTunnel {
    /// Protocol used for the outer (first-hop) tunnel.
    outer_protocol: Protocol,
    /// Protocol used for the inner (second-hop) tunnel.
    inner_protocol: Protocol,
    /// Provider instance for the outer tunnel.
    outer_provider: Box<dyn TunnelProvider>,
    /// Provider instance for the inner tunnel.
    inner_provider: Box<dyn TunnelProvider>,
}

impl DoubleTunnel {
    /// Create a new double tunnel with the specified outer and inner
    /// protocols.
    ///
    /// Providers are instantiated automatically based on the protocol
    /// variants.  Protocols that only have stub providers will fail at
    /// connect time with `ProtocolUnsupported`.
    pub fn new(outer_protocol: Protocol, inner_protocol: Protocol) -> Self {
        let outer_provider = Self::make_provider(outer_protocol);
        let inner_provider = Self::make_provider(inner_protocol);

        Self {
            outer_protocol,
            inner_protocol,
            outer_provider,
            inner_provider,
        }
    }

    /// Establish the double tunnel.
    ///
    /// `outer_config` is used for the first-hop connection and
    /// `inner_config` for the second-hop.  The inner tunnel's traffic is
    /// routed through the already-established outer tunnel.
    pub async fn connect(
        &mut self,
        outer_config: &TunnelConfig,
        inner_config: &TunnelConfig,
    ) -> Result<()> {
        // Validate that neither tunnel is already active.
        if self.outer_provider.is_connected() || self.inner_provider.is_connected() {
            return Err(TunnelError::TunnelAlreadyActive);
        }

        // --- Outer tunnel ---------------------------------------------------
        info!(
            outer = %self.outer_protocol,
            inner = %self.inner_protocol,
            "DoubleTunnel: establishing outer tunnel"
        );

        self.outer_provider.connect(outer_config).await.map_err(|e| {
            error!(error = %e, "DoubleTunnel: outer tunnel failed");
            TunnelError::ConnectionFailed(format!(
                "outer tunnel ({}) failed: {e}",
                self.outer_protocol
            ))
        })?;

        info!("DoubleTunnel: outer tunnel connected");

        // --- Inner tunnel ---------------------------------------------------
        info!("DoubleTunnel: establishing inner tunnel inside outer");

        if let Err(e) = self.inner_provider.connect(inner_config).await {
            // If the inner fails, tear down the outer so we don't leave a
            // half-open double tunnel.
            warn!(
                error = %e,
                "DoubleTunnel: inner tunnel failed, tearing down outer"
            );
            let _ = self.outer_provider.disconnect().await;
            return Err(TunnelError::ConnectionFailed(format!(
                "inner tunnel ({}) failed: {e}",
                self.inner_protocol
            )));
        }

        info!("DoubleTunnel: both tunnels established");
        Ok(())
    }

    /// Disconnect both tunnels (inner first, then outer).
    pub async fn disconnect(&mut self) -> Result<()> {
        let mut errors: Vec<String> = Vec::new();

        // Tear down inner first.
        if self.inner_provider.is_connected() {
            info!("DoubleTunnel: disconnecting inner tunnel");
            if let Err(e) = self.inner_provider.disconnect().await {
                warn!(error = %e, "DoubleTunnel: inner disconnect error");
                errors.push(format!("inner: {e}"));
            }
        }

        // Then outer.
        if self.outer_provider.is_connected() {
            info!("DoubleTunnel: disconnecting outer tunnel");
            if let Err(e) = self.outer_provider.disconnect().await {
                warn!(error = %e, "DoubleTunnel: outer disconnect error");
                errors.push(format!("outer: {e}"));
            }
        }

        if errors.is_empty() {
            info!("DoubleTunnel: fully disconnected");
            Ok(())
        } else {
            Err(TunnelError::ConnectionFailed(format!(
                "errors during double tunnel disconnect: {}",
                errors.join("; ")
            )))
        }
    }

    /// Return status snapshots for both tunnels as `(outer, inner)`.
    pub fn status(&self) -> (TunnelStatus, TunnelStatus) {
        (
            self.outer_provider.status(),
            self.inner_provider.status(),
        )
    }

    /// Whether both tunnels are connected.
    pub fn is_fully_connected(&self) -> bool {
        self.outer_provider.is_connected() && self.inner_provider.is_connected()
    }

    /// The outer (first-hop) protocol.
    pub fn outer_protocol(&self) -> Protocol {
        self.outer_protocol
    }

    /// The inner (second-hop) protocol.
    pub fn inner_protocol(&self) -> Protocol {
        self.inner_protocol
    }

    // -- helpers ------------------------------------------------------------

    /// Instantiate a `TunnelProvider` for the given protocol.
    fn make_provider(protocol: Protocol) -> Box<dyn TunnelProvider> {
        match protocol {
            Protocol::WireGuard | Protocol::ObfuscatedWireGuard | Protocol::Chameleon => {
                Box::new(WireGuardProvider::new())
            }
            Protocol::IKEv2 => Box::new(IKEv2Provider::new()),
            Protocol::Shadowsocks | Protocol::TorBridge => {
                Box::new(ShadowsocksProvider::new())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_tunnel_creation() {
        let dt = DoubleTunnel::new(Protocol::WireGuard, Protocol::Shadowsocks);
        assert_eq!(dt.outer_protocol(), Protocol::WireGuard);
        assert_eq!(dt.inner_protocol(), Protocol::Shadowsocks);
        assert!(!dt.is_fully_connected());
    }

    #[test]
    fn status_starts_disconnected() {
        let dt = DoubleTunnel::new(Protocol::WireGuard, Protocol::IKEv2);
        let (outer, inner) = dt.status();
        assert_eq!(outer.state, TunnelState::Disconnected);
        assert_eq!(inner.state, TunnelState::Disconnected);
    }
}
