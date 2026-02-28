use crate::error::MeshError;
use crate::peer::{MeshPeer, PeerId, PeerRegistry};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitHop {
    pub hop_index: usize,
    pub peer_id: PeerId,
    pub endpoint: String,
    pub region: String,
    #[serde(skip)]
    pub session_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitStatus {
    Building,
    Active,
    Reshuffling,
    Destroyed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    pub circuit_id: String,
    pub hops: Vec<CircuitHop>,
    pub created_at_epoch: u64,
    #[serde(with = "duration_secs")]
    pub max_lifetime: Duration,
    pub status: CircuitStatus,
}

mod duration_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;
    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_secs().serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }
}

impl Circuit {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        now - self.created_at_epoch > self.max_lifetime.as_secs()
    }

    pub fn hop_count(&self) -> usize { self.hops.len() }
    pub fn entry_node(&self) -> Option<&CircuitHop> { self.hops.first() }
    pub fn exit_node(&self) -> Option<&CircuitHop> { self.hops.last() }
}

pub struct CircuitBuilder {
    min_hops: usize,
    max_hops: usize,
    reshuffle_interval: Duration,
}

impl CircuitBuilder {
    pub fn new(min_hops: usize, max_hops: usize, reshuffle_interval: Duration) -> Self {
        Self { min_hops: min_hops.max(3), max_hops: max_hops.min(7), reshuffle_interval }
    }

    pub fn default_builder() -> Self {
        Self::new(3, 5, Duration::from_secs(60))
    }

    pub fn build(&self, registry: &PeerRegistry, hop_count: usize, exclude: &[PeerId]) -> Result<Circuit, MeshError> {
        let hop_count = hop_count.clamp(self.min_hops, self.max_hops);
        let mut relays: Vec<MeshPeer> = registry.relay_peers().into_iter()
            .filter(|p| !exclude.contains(&p.peer_id)).collect();
        let mut exits: Vec<MeshPeer> = registry.exit_peers().into_iter()
            .filter(|p| !exclude.contains(&p.peer_id)).collect();

        let mut rng = rand::thread_rng();
        relays.shuffle(&mut rng);
        exits.shuffle(&mut rng);

        let total = relays.len() + exits.len();
        if total < hop_count {
            return Err(MeshError::InsufficientPeers { needed: hop_count, available: total });
        }

        let mut selected: Vec<MeshPeer> = Vec::with_capacity(hop_count);
        let relay_count = if exits.is_empty() { hop_count } else { hop_count - 1 };
        selected.extend(relays.into_iter().take(relay_count));
        if let Some(exit) = exits.into_iter().next() {
            selected.push(exit);
        }

        if selected.len() < hop_count {
            return Err(MeshError::InsufficientPeers { needed: hop_count, available: selected.len() });
        }

        let hops: Vec<CircuitHop> = selected.into_iter().take(hop_count).enumerate()
            .map(|(i, peer)| CircuitHop {
                hop_index: i, peer_id: peer.peer_id, endpoint: peer.endpoint,
                region: peer.region, session_key: None,
            }).collect();

        let circuit_id = hex::encode(rand::random::<[u8; 16]>());
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

        Ok(Circuit {
            circuit_id, hops, created_at_epoch: created_at,
            max_lifetime: self.reshuffle_interval, status: CircuitStatus::Active,
        })
    }

    pub fn reshuffle(&self, registry: &PeerRegistry, old: &Circuit) -> Result<Circuit, MeshError> {
        let exclude: Vec<PeerId> = old.hops.iter().map(|h| h.peer_id.clone()).collect();
        self.build(registry, old.hop_count(), &exclude)
    }
}
