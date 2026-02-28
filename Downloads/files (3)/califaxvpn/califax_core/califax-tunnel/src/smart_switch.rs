//! AI-driven protocol selection based on real-time network conditions.
//!
//! The `SmartSwitch` engine evaluates metrics such as latency, packet loss,
//! bandwidth, and censorship indicators to choose the optimal VPN protocol.
//! When an AI routing engine (`califax_ai::routing::RouteOptimizer`) is
//! available it delegates to that; otherwise it falls back to a deterministic
//! heuristic.

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::protocol::Protocol;

// ---------------------------------------------------------------------------
// NetworkConditions
// ---------------------------------------------------------------------------

/// A snapshot of the current network environment used by the smart switch
/// engine to select the best protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConditions {
    /// Round-trip latency in milliseconds.
    pub latency_ms: f64,
    /// Observed packet loss as a percentage (0.0 -- 100.0).
    pub packet_loss_percent: f64,
    /// Estimated available bandwidth in Mbps.
    pub bandwidth_mbps: f64,
    /// Whether the network appears to be behind a national firewall or
    /// censorship system.
    pub is_censored_network: bool,
    /// Whether the device is connected to a public / untrusted Wi-Fi
    /// network.
    pub is_public_wifi: bool,
    /// Whether deep-packet-inspection signatures have been detected on
    /// the current path.
    pub dpi_detected: bool,
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self {
            latency_ms: 50.0,
            packet_loss_percent: 0.0,
            bandwidth_mbps: 100.0,
            is_censored_network: false,
            is_public_wifi: false,
            dpi_detected: false,
        }
    }
}

// ---------------------------------------------------------------------------
// SmartSwitch
// ---------------------------------------------------------------------------

/// AI-driven (or heuristic-based) protocol selector.
///
/// When a `califax_ai::routing::RouteOptimizer` is available the engine uses
/// machine-learned models for selection; otherwise it applies a fixed decision
/// tree.
pub struct SmartSwitch {
    /// Optional AI routing engine.  When `None`, purely heuristic selection
    /// is used.
    _ai_engine: Option<AiEngineHandle>,
}

/// Opaque handle wrapping the optional AI engine.
///
/// The concrete `califax_ai` crate may not be compiled yet, so we store the
/// engine as a type-erased trait object behind a feature-gated wrapper.
/// For now this is a placeholder that will be replaced once `califax-ai` is
/// available.
pub struct AiEngineHandle {
    _inner: Box<dyn AiRouteRecommender + Send + Sync>,
}

/// Trait that an AI routing engine must implement so SmartSwitch can call it.
pub trait AiRouteRecommender: Send + Sync {
    /// Given current network conditions, return the recommended protocol.
    fn recommend(&self, conditions: &NetworkConditions) -> Protocol;
}

impl SmartSwitch {
    /// Create a new `SmartSwitch`.
    ///
    /// Pass `None` to use heuristic-only mode.
    pub fn new(ai_engine: Option<AiEngineHandle>) -> Self {
        Self {
            _ai_engine: ai_engine,
        }
    }

    /// Create a heuristic-only `SmartSwitch` (no AI).
    pub fn heuristic() -> Self {
        Self::new(None)
    }

    /// Recommend the best protocol for the given network conditions.
    pub fn recommend_protocol(&self, conditions: &NetworkConditions) -> Protocol {
        // If an AI engine is available, delegate to it.
        if let Some(ref engine) = self._ai_engine {
            let recommendation = engine._inner.recommend(conditions);
            info!(
                protocol = %recommendation,
                "SmartSwitch: AI engine recommendation"
            );
            return recommendation;
        }

        // Otherwise, apply heuristic decision tree.
        let protocol = Self::heuristic_select(conditions);
        info!(
            protocol = %protocol,
            latency_ms = conditions.latency_ms,
            packet_loss = conditions.packet_loss_percent,
            bandwidth = conditions.bandwidth_mbps,
            censored = conditions.is_censored_network,
            public_wifi = conditions.is_public_wifi,
            dpi = conditions.dpi_detected,
            "SmartSwitch: heuristic recommendation"
        );
        protocol
    }

    /// Deterministic heuristic selection logic.
    fn heuristic_select(c: &NetworkConditions) -> Protocol {
        // Priority 1: DPI or censorship detected -- use Chameleon (strongest
        // obfuscation).
        if c.dpi_detected || c.is_censored_network {
            return Protocol::Chameleon;
        }

        // Priority 2: Public / untrusted Wi-Fi -- use ObfuscatedWireGuard to
        // hide VPN signatures from local network operators.
        if c.is_public_wifi {
            return Protocol::ObfuscatedWireGuard;
        }

        // Priority 3: High packet loss -- WireGuard handles lossy links well
        // because it is UDP-based, but if loss is extreme consider
        // Shadowsocks over TCP which can retransmit.
        if c.packet_loss_percent > 15.0 {
            return Protocol::Shadowsocks;
        }

        // Priority 4: Very low latency requirement or high bandwidth -- plain
        // WireGuard is the fastest.
        if c.latency_ms < 100.0 && c.bandwidth_mbps > 10.0 {
            return Protocol::WireGuard;
        }

        // Priority 5: High latency or low bandwidth -- ObfuscatedWireGuard
        // still performs well while adding a layer of privacy.
        if c.latency_ms >= 100.0 {
            return Protocol::ObfuscatedWireGuard;
        }

        // Default: standard WireGuard.
        Protocol::WireGuard
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn switch() -> SmartSwitch {
        SmartSwitch::heuristic()
    }

    #[test]
    fn censored_network_selects_chameleon() {
        let c = NetworkConditions {
            is_censored_network: true,
            ..Default::default()
        };
        assert_eq!(switch().recommend_protocol(&c), Protocol::Chameleon);
    }

    #[test]
    fn dpi_detected_selects_chameleon() {
        let c = NetworkConditions {
            dpi_detected: true,
            ..Default::default()
        };
        assert_eq!(switch().recommend_protocol(&c), Protocol::Chameleon);
    }

    #[test]
    fn public_wifi_selects_obfuscated_wg() {
        let c = NetworkConditions {
            is_public_wifi: true,
            ..Default::default()
        };
        assert_eq!(
            switch().recommend_protocol(&c),
            Protocol::ObfuscatedWireGuard
        );
    }

    #[test]
    fn high_packet_loss_selects_shadowsocks() {
        let c = NetworkConditions {
            packet_loss_percent: 20.0,
            ..Default::default()
        };
        assert_eq!(switch().recommend_protocol(&c), Protocol::Shadowsocks);
    }

    #[test]
    fn good_conditions_select_wireguard() {
        let c = NetworkConditions::default();
        assert_eq!(switch().recommend_protocol(&c), Protocol::WireGuard);
    }

    #[test]
    fn high_latency_selects_obfuscated_wg() {
        let c = NetworkConditions {
            latency_ms: 200.0,
            bandwidth_mbps: 50.0,
            ..Default::default()
        };
        assert_eq!(
            switch().recommend_protocol(&c),
            Protocol::ObfuscatedWireGuard
        );
    }
}
