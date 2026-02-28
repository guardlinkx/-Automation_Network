//! Ordered protocol failover chains with automatic retry logic.
//!
//! When a tunnel connection fails, the `FailoverManager` walks through an
//! ordered list of protocols, trying each one in turn until a connection
//! succeeds or all options are exhausted.

use std::collections::HashMap;
use std::time::Duration;

use tracing::{info, warn, error};

use crate::error::TunnelError;
use crate::protocol::{Protocol, TunnelConfig, TunnelProvider};
use crate::Result;

// ---------------------------------------------------------------------------
// FailoverChain
// ---------------------------------------------------------------------------

/// An ordered sequence of protocols to attempt when establishing a tunnel.
#[derive(Debug, Clone)]
pub struct FailoverChain {
    /// Protocols in priority order (first = most preferred).
    protocols: Vec<Protocol>,
    /// Index of the protocol currently being tried.
    current_index: usize,
}

impl FailoverChain {
    /// Create a new failover chain with the given protocol order.
    ///
    /// # Panics
    ///
    /// Panics if `protocols` is empty.
    pub fn new(protocols: Vec<Protocol>) -> Self {
        assert!(!protocols.is_empty(), "failover chain must contain at least one protocol");
        Self {
            protocols,
            current_index: 0,
        }
    }

    /// Return the protocol that should be tried next (the current head of the
    /// chain).
    pub fn current(&self) -> Protocol {
        self.protocols[self.current_index]
    }

    /// Advance to the next protocol in the chain.  Returns `Some(protocol)` if
    /// there is another protocol to try, or `None` if the chain is exhausted.
    pub fn next(&mut self) -> Option<Protocol> {
        if self.current_index + 1 < self.protocols.len() {
            self.current_index += 1;
            Some(self.protocols[self.current_index])
        } else {
            None
        }
    }

    /// Reset the chain back to the first (most preferred) protocol.
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Total number of protocols in the chain.
    pub fn len(&self) -> usize {
        self.protocols.len()
    }

    /// Whether the chain is empty (always false after construction).
    pub fn is_empty(&self) -> bool {
        self.protocols.is_empty()
    }

    /// Return a slice of all protocols in order.
    pub fn protocols(&self) -> &[Protocol] {
        &self.protocols
    }
}

impl Default for FailoverChain {
    /// Default failover order:
    /// WireGuard -> ObfuscatedWireGuard -> Shadowsocks -> IKEv2 -> TorBridge -> Chameleon
    fn default() -> Self {
        Self::new(vec![
            Protocol::WireGuard,
            Protocol::ObfuscatedWireGuard,
            Protocol::Shadowsocks,
            Protocol::IKEv2,
            Protocol::TorBridge,
            Protocol::Chameleon,
        ])
    }
}

// ---------------------------------------------------------------------------
// FailoverManager
// ---------------------------------------------------------------------------

/// Manages automatic protocol failover with per-protocol retry limits.
pub struct FailoverManager {
    /// The ordered chain of protocols to try.
    chain: FailoverChain,
    /// Maximum number of connection attempts per protocol before moving to
    /// the next one.
    pub max_retries_per_protocol: u32,
    /// Delay between retry attempts for the same protocol.
    pub retry_delay: Duration,
}

impl FailoverManager {
    /// Create a new failover manager with the given chain.
    pub fn new(chain: FailoverChain) -> Self {
        Self {
            chain,
            max_retries_per_protocol: 3,
            retry_delay: Duration::from_secs(2),
        }
    }

    /// Create a failover manager with the default chain and settings.
    pub fn with_defaults() -> Self {
        Self::new(FailoverChain::default())
    }

    /// Immutable access to the underlying chain.
    pub fn chain(&self) -> &FailoverChain {
        &self.chain
    }

    /// Try each protocol in the failover chain until one connects
    /// successfully.
    ///
    /// `providers` maps each `Protocol` to a boxed `TunnelProvider`
    /// implementation.  Protocols that do not have a provider entry are
    /// skipped.
    ///
    /// The `config` will have its `protocol` field overwritten with the
    /// protocol being attempted on each iteration.
    ///
    /// Returns the `Protocol` that succeeded.
    pub async fn connect_with_failover(
        &mut self,
        providers: &mut HashMap<Protocol, Box<dyn TunnelProvider>>,
        config: &mut TunnelConfig,
    ) -> Result<Protocol> {
        self.chain.reset();

        loop {
            let protocol = self.chain.current();

            if let Some(provider) = providers.get_mut(&protocol) {
                config.protocol = protocol;

                info!(
                    protocol = %protocol,
                    "failover: attempting connection"
                );

                let mut connected = false;
                for attempt in 1..=self.max_retries_per_protocol {
                    info!(
                        protocol = %protocol,
                        attempt,
                        max = self.max_retries_per_protocol,
                        "connection attempt"
                    );

                    match provider.connect(config).await {
                        Ok(()) => {
                            info!(
                                protocol = %protocol,
                                "failover: connection established"
                            );
                            connected = true;
                            break;
                        }
                        Err(e) => {
                            warn!(
                                protocol = %protocol,
                                attempt,
                                error = %e,
                                "connection attempt failed"
                            );
                            if attempt < self.max_retries_per_protocol {
                                tokio::time::sleep(self.retry_delay).await;
                            }
                        }
                    }
                }

                if connected {
                    return Ok(protocol);
                }
            } else {
                warn!(
                    protocol = %protocol,
                    "failover: no provider registered, skipping"
                );
            }

            // Move to next protocol in chain
            match self.chain.next() {
                Some(next_proto) => {
                    info!(
                        from = %protocol,
                        to = %next_proto,
                        "failover: switching to next protocol"
                    );
                }
                None => {
                    error!("failover: all protocols exhausted");
                    return Err(TunnelError::AllProtocolsFailed);
                }
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
    fn chain_traversal() {
        let mut chain = FailoverChain::new(vec![
            Protocol::WireGuard,
            Protocol::Shadowsocks,
            Protocol::IKEv2,
        ]);

        assert_eq!(chain.current(), Protocol::WireGuard);
        assert_eq!(chain.next(), Some(Protocol::Shadowsocks));
        assert_eq!(chain.current(), Protocol::Shadowsocks);
        assert_eq!(chain.next(), Some(Protocol::IKEv2));
        assert_eq!(chain.current(), Protocol::IKEv2);
        assert_eq!(chain.next(), None);

        chain.reset();
        assert_eq!(chain.current(), Protocol::WireGuard);
    }

    #[test]
    fn default_chain_order() {
        let chain = FailoverChain::default();
        let protos = chain.protocols();
        assert_eq!(protos[0], Protocol::WireGuard);
        assert_eq!(protos[1], Protocol::ObfuscatedWireGuard);
        assert_eq!(protos[2], Protocol::Shadowsocks);
        assert_eq!(protos[3], Protocol::IKEv2);
        assert_eq!(protos[4], Protocol::TorBridge);
        assert_eq!(protos[5], Protocol::Chameleon);
        assert_eq!(protos.len(), 6);
    }

    #[test]
    #[should_panic(expected = "failover chain must contain at least one protocol")]
    fn empty_chain_panics() {
        FailoverChain::new(vec![]);
    }
}
