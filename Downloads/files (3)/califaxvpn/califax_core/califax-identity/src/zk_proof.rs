use crate::error::IdentityError;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof {
    pub proof_type: ProofType,
    pub proof_data: Vec<u8>,
    pub public_inputs: Vec<Vec<u8>>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofType {
    Groth16,
    Plonk,
    Stub,
}

pub struct ZkVerifier {
    proof_type: ProofType,
}

impl ZkVerifier {
    pub fn new(proof_type: ProofType) -> Self {
        Self { proof_type }
    }

    /// Generate a stub ZK proof for testing (real implementation would use bellman/arkworks)
    pub fn generate_proof(&self, secret: &[u8], public_key: &[u8]) -> Result<ZkProof, IdentityError> {
        // Stub: hash(secret || public_key) as proof
        let mut hasher = Sha256::new();
        hasher.update(secret);
        hasher.update(public_key);
        let proof_data = hasher.finalize().to_vec();

        let public_hash = Sha256::digest(public_key).to_vec();

        Ok(ZkProof {
            proof_type: self.proof_type,
            proof_data,
            public_inputs: vec![public_hash],
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        })
    }

    /// Verify a ZK proof against a public key
    pub fn verify(&self, proof: &ZkProof, public_key: &[u8]) -> Result<bool, IdentityError> {
        if proof.proof_data.is_empty() {
            return Err(IdentityError::InvalidProofFormat("Empty proof data".into()));
        }

        match proof.proof_type {
            ProofType::Groth16 | ProofType::Plonk => {
                // Stub verification: check that public_inputs contains hash of public_key
                let expected_hash = Sha256::digest(public_key).to_vec();
                let valid = proof.public_inputs.iter().any(|input| *input == expected_hash);
                Ok(valid)
            }
            ProofType::Stub => {
                // Always accept non-empty stub proofs
                Ok(proof.proof_data.len() >= 32)
            }
        }
    }

    /// Verify a hex-encoded proof string (for Flask API)
    pub fn verify_hex_proof(&self, proof_hex: &str, public_key_hex: &str) -> Result<bool, IdentityError> {
        let proof_data = hex::decode(proof_hex)
            .map_err(|e| IdentityError::InvalidProofFormat(e.to_string()))?;
        let public_key = hex::decode(public_key_hex)
            .map_err(|e| IdentityError::InvalidProofFormat(e.to_string()))?;

        if proof_data.len() < 32 {
            return Err(IdentityError::InvalidProofFormat("Proof too short".into()));
        }

        let public_hash = Sha256::digest(&public_key).to_vec();
        let proof = ZkProof {
            proof_type: self.proof_type,
            proof_data,
            public_inputs: vec![public_hash.clone()],
            created_at: 0,
        };
        self.verify(&proof, &public_key)
    }
}
