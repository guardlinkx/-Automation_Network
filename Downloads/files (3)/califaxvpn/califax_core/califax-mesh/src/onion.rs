use crate::circuit::CircuitHop;
use crate::error::MeshError;
use califax_crypto::symmetric::{self, CipherSuite};

pub struct OnionLayer;

impl OnionLayer {
    pub fn wrap(data: &[u8], hops: &[CircuitHop]) -> Result<Vec<u8>, MeshError> {
        let mut payload = data.to_vec();
        for hop in hops.iter().rev() {
            let key = hop.session_key.as_ref().ok_or_else(|| {
                MeshError::OnionEncryptionFailed(format!("No session key for hop {}", hop.hop_index))
            })?;
            let key_arr: [u8; 32] = key[..32].try_into()
                .map_err(|_| MeshError::OnionEncryptionFailed("Invalid key length".into()))?;
            let aad = format!("califax-onion-hop-{}", hop.hop_index);
            let (nonce, ct) = symmetric::encrypt(CipherSuite::ChaCha20Poly1305, &key_arr, &payload, aad.as_bytes())
                .map_err(|e| MeshError::OnionEncryptionFailed(e.to_string()))?;
            payload = Vec::with_capacity(nonce.len() + ct.len());
            payload.extend_from_slice(&nonce);
            payload.extend_from_slice(&ct);
        }
        Ok(payload)
    }

    pub fn unwrap_layer(data: &[u8], hop: &CircuitHop) -> Result<Vec<u8>, MeshError> {
        if data.len() < 12 {
            return Err(MeshError::OnionEncryptionFailed("Data too short".into()));
        }
        let key = hop.session_key.as_ref().ok_or_else(|| {
            MeshError::OnionEncryptionFailed(format!("No session key for hop {}", hop.hop_index))
        })?;
        let key_arr: [u8; 32] = key[..32].try_into()
            .map_err(|_| MeshError::OnionEncryptionFailed("Invalid key length".into()))?;
        let nonce: [u8; 12] = data[..12].try_into()
            .map_err(|_| MeshError::OnionEncryptionFailed("Invalid nonce".into()))?;
        let aad = format!("califax-onion-hop-{}", hop.hop_index);
        symmetric::decrypt(CipherSuite::ChaCha20Poly1305, &key_arr, &nonce, &data[12..], aad.as_bytes())
            .map_err(|e| MeshError::OnionEncryptionFailed(e.to_string()))
    }
}
