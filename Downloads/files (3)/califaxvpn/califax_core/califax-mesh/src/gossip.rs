use crate::peer::PeerId;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    PeerAnnounce { peer_id: PeerId, endpoint: String, region: String, timestamp: u64 },
    PeerLeave { peer_id: PeerId, timestamp: u64 },
    CircuitState { circuit_id: String, status: String, timestamp: u64 },
    HealthReport { peer_id: PeerId, cpu_percent: f32, memory_percent: f32, connected_peers: u32, timestamp: u64 },
}

impl GossipMessage {
    pub fn message_id(&self) -> String {
        let serialized = serde_json::to_string(self).unwrap_or_default();
        let hash = Sha256::digest(serialized.as_bytes());
        hex::encode(&hash[..16])
    }

    pub fn timestamp(&self) -> u64 {
        match self {
            Self::PeerAnnounce { timestamp, .. } | Self::PeerLeave { timestamp, .. }
            | Self::CircuitState { timestamp, .. } | Self::HealthReport { timestamp, .. } => *timestamp,
        }
    }
}

pub struct GossipProtocol {
    local_peer_id: PeerId,
    seen_messages: Arc<RwLock<HashSet<String>>>,
    subscribers: Arc<RwLock<HashMap<String, Vec<PeerId>>>>,
    history: Arc<RwLock<Vec<GossipMessage>>>,
    max_history: usize,
}

impl GossipProtocol {
    pub fn new(local_peer_id: PeerId, max_history: usize) -> Self {
        Self {
            local_peer_id,
            seen_messages: Arc::new(RwLock::new(HashSet::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }

    pub fn publish(&self, message: GossipMessage) -> Vec<PeerId> {
        let msg_id = message.message_id();
        {
            let mut seen = self.seen_messages.write().unwrap();
            if seen.contains(&msg_id) { return vec![]; }
            seen.insert(msg_id);
        }
        {
            let mut history = self.history.write().unwrap();
            history.push(message);
            if history.len() > self.max_history {
                let drain = history.len() - self.max_history;
                history.drain(0..drain);
            }
        }
        let subs = self.subscribers.read().unwrap();
        subs.get("mesh-state").cloned().unwrap_or_default()
            .into_iter().filter(|p| *p != self.local_peer_id).collect()
    }

    pub fn subscribe(&self, peer_id: PeerId) {
        let mut subs = self.subscribers.write().unwrap();
        let topic = subs.entry("mesh-state".to_string()).or_default();
        if !topic.contains(&peer_id) { topic.push(peer_id); }
    }

    pub fn unsubscribe(&self, peer_id: &str) {
        let mut subs = self.subscribers.write().unwrap();
        for v in subs.values_mut() { v.retain(|p| p != peer_id); }
    }

    pub fn get_history(&self, since: u64) -> Vec<GossipMessage> {
        self.history.read().unwrap().iter().filter(|m| m.timestamp() > since).cloned().collect()
    }

    pub fn cleanup(&self) {
        let mut seen = self.seen_messages.write().unwrap();
        if seen.len() > 10000 { seen.clear(); }
    }
}
