use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub type PeerId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshPeer {
    pub peer_id: PeerId,
    pub public_key: Vec<u8>,
    pub endpoint: String,
    pub region: String,
    pub is_relay: bool,
    pub is_exit: bool,
    pub bandwidth_mbps: u32,
    pub latency_ms: Option<u32>,
    #[serde(skip)]
    pub last_seen: Option<Instant>,
}

impl MeshPeer {
    pub fn new(peer_id: PeerId, public_key: Vec<u8>, endpoint: String, region: String) -> Self {
        Self {
            peer_id, public_key, endpoint, region,
            is_relay: true, is_exit: false, bandwidth_mbps: 100,
            latency_ms: None, last_seen: Some(Instant::now()),
        }
    }

    pub fn is_alive(&self, timeout: Duration) -> bool {
        self.last_seen.map(|t| t.elapsed() < timeout).unwrap_or(false)
    }
}

pub struct PeerRegistry {
    peers: Arc<RwLock<HashMap<PeerId, MeshPeer>>>,
    timeout: Duration,
}

impl PeerRegistry {
    pub fn new(timeout: Duration) -> Self {
        Self { peers: Arc::new(RwLock::new(HashMap::new())), timeout }
    }

    pub fn register(&self, peer: MeshPeer) {
        self.peers.write().unwrap().insert(peer.peer_id.clone(), peer);
    }

    pub fn deregister(&self, peer_id: &str) -> Option<MeshPeer> {
        self.peers.write().unwrap().remove(peer_id)
    }

    pub fn get(&self, peer_id: &str) -> Option<MeshPeer> {
        self.peers.read().unwrap().get(peer_id).cloned()
    }

    pub fn alive_peers(&self) -> Vec<MeshPeer> {
        self.peers.read().unwrap().values()
            .filter(|p| p.is_alive(self.timeout)).cloned().collect()
    }

    pub fn relay_peers(&self) -> Vec<MeshPeer> {
        self.alive_peers().into_iter().filter(|p| p.is_relay).collect()
    }

    pub fn exit_peers(&self) -> Vec<MeshPeer> {
        self.alive_peers().into_iter().filter(|p| p.is_exit).collect()
    }

    pub fn update_seen(&self, peer_id: &str) {
        if let Some(peer) = self.peers.write().unwrap().get_mut(peer_id) {
            peer.last_seen = Some(Instant::now());
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.read().unwrap().len()
    }
}
