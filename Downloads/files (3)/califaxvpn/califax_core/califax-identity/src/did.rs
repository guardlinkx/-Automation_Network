use crate::error::IdentityError;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: String,
    pub wallet_address: String,
    pub public_key: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_active: bool,
    pub authentication: Vec<String>,
    pub service_endpoints: Vec<ServiceEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub id: String,
    pub service_type: String,
    pub endpoint: String,
}

pub struct DidManager {
    documents: RwLock<HashMap<String, DidDocument>>,
}

impl DidManager {
    pub fn new() -> Self {
        Self { documents: RwLock::new(HashMap::new()) }
    }

    pub fn create_did(&self, wallet_address: &str, public_key: &str) -> Result<DidDocument, IdentityError> {
        let did = format!("did:califax:{}", &wallet_address.to_lowercase()[..wallet_address.len().min(10)]);

        let mut docs = self.documents.write().unwrap();
        if docs.contains_key(&did) {
            return Err(IdentityError::DidAlreadyRegistered(did));
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

        let doc = DidDocument {
            id: did.clone(),
            wallet_address: wallet_address.to_string(),
            public_key: public_key.to_string(),
            created_at: now,
            updated_at: now,
            is_active: true,
            authentication: vec![format!("{}#key-1", did)],
            service_endpoints: vec![ServiceEndpoint {
                id: format!("{}#vpn", did),
                service_type: "CalifaxVPN".to_string(),
                endpoint: "https://califaxvpn.guardlinkx.com/api/vpn".to_string(),
            }],
        };

        docs.insert(did, doc.clone());
        Ok(doc)
    }

    pub fn resolve(&self, did: &str) -> Result<DidDocument, IdentityError> {
        let docs = self.documents.read().unwrap();
        docs.get(did).cloned().ok_or_else(|| IdentityError::DidNotFound(did.to_string()))
    }

    pub fn deactivate(&self, did: &str) -> Result<(), IdentityError> {
        let mut docs = self.documents.write().unwrap();
        let doc = docs.get_mut(did).ok_or_else(|| IdentityError::DidNotFound(did.to_string()))?;
        doc.is_active = false;
        doc.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.documents.read().unwrap().len()
    }
}

impl Default for DidManager {
    fn default() -> Self { Self::new() }
}
