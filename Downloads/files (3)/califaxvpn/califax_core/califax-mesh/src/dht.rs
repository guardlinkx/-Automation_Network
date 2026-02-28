use crate::peer::MeshPeer;
use sha2::{Sha256, Digest};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct DhtEntry {
    pub peer_id: String,
    pub node_hash: Vec<u8>,
    pub endpoint: String,
    pub region: String,
}

pub struct KademliaDht {
    local_id: Vec<u8>,
    k_bucket_size: usize,
    buckets: Arc<RwLock<BTreeMap<usize, Vec<DhtEntry>>>>,
}

impl KademliaDht {
    pub fn new(local_peer_id: &str, k_bucket_size: usize) -> Self {
        Self {
            local_id: Self::hash_id(local_peer_id),
            k_bucket_size,
            buckets: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    fn hash_id(id: &str) -> Vec<u8> {
        Sha256::digest(id.as_bytes()).to_vec()
    }

    fn xor_distance(a: &[u8], b: &[u8]) -> Vec<u8> {
        a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
    }

    fn bucket_index(distance: &[u8]) -> usize {
        for (i, byte) in distance.iter().enumerate() {
            if *byte != 0 { return i * 8 + byte.leading_zeros() as usize; }
        }
        distance.len() * 8
    }

    pub fn insert(&self, peer: &MeshPeer) {
        let hash = Self::hash_id(&peer.peer_id);
        let dist = Self::xor_distance(&self.local_id, &hash);
        let idx = Self::bucket_index(&dist);
        let entry = DhtEntry {
            peer_id: peer.peer_id.clone(), node_hash: hash,
            endpoint: peer.endpoint.clone(), region: peer.region.clone(),
        };
        let mut buckets = self.buckets.write().unwrap();
        let bucket = buckets.entry(idx).or_default();
        bucket.retain(|e| e.peer_id != peer.peer_id);
        if bucket.len() < self.k_bucket_size { bucket.push(entry); }
    }

    pub fn find_closest(&self, target_id: &str, count: usize) -> Vec<DhtEntry> {
        let target_hash = Self::hash_id(target_id);
        let buckets = self.buckets.read().unwrap();
        let mut all: Vec<(Vec<u8>, DhtEntry)> = buckets.values().flatten()
            .map(|e| (Self::xor_distance(&target_hash, &e.node_hash), e.clone())).collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all.into_iter().take(count).map(|(_, e)| e).collect()
    }

    pub fn remove(&self, peer_id: &str) {
        let mut buckets = self.buckets.write().unwrap();
        for bucket in buckets.values_mut() { bucket.retain(|e| e.peer_id != peer_id); }
    }

    pub fn entry_count(&self) -> usize {
        self.buckets.read().unwrap().values().map(|b| b.len()).sum()
    }
}
