//! PyO3 bindings for califax-crypto.
//!
//! Exposes Rust post-quantum and symmetric cryptography primitives to Python
//! through the `califax_core.crypto` submodule.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use califax_crypto::CryptoError;

// ---------------------------------------------------------------------------
// Helper: convert CryptoError into a Python ValueError
// ---------------------------------------------------------------------------

fn crypto_err(e: CryptoError) -> PyErr {
    PyValueError::new_err(format!("{e}"))
}

fn hex_decode(hex: &str) -> PyResult<Vec<u8>> {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(hex.get(i..i + 2).ok_or_else(|| {
                PyValueError::new_err("hex string has odd length")
            })?, 16)
            .map_err(|e| PyValueError::new_err(format!("invalid hex: {e}")))
        })
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Kyber (post-quantum KEM)
// ---------------------------------------------------------------------------

#[pyfunction]
fn generate_kyber_keypair(py: Python<'_>) -> PyResult<Py<PyDict>> {
    let kp = califax_crypto::pqc::generate_kyber_keypair().map_err(crypto_err)?;
    let dict = PyDict::new(py);
    dict.set_item("public_key", hex_encode(&kp.public_key))?;
    dict.set_item("secret_key", hex_encode(&kp.secret_key))?;
    Ok(dict.into())
}

#[pyfunction]
fn kyber_encapsulate(py: Python<'_>, public_key_hex: &str) -> PyResult<Py<PyDict>> {
    let pk = hex_decode(public_key_hex)?;
    let (shared_secret, ciphertext) =
        califax_crypto::pqc::kyber_encapsulate(&pk).map_err(crypto_err)?;
    let dict = PyDict::new(py);
    dict.set_item("shared_secret", hex_encode(&shared_secret))?;
    dict.set_item("ciphertext", hex_encode(&ciphertext))?;
    Ok(dict.into())
}

#[pyfunction]
fn kyber_decapsulate(secret_key_hex: &str, ciphertext_hex: &str) -> PyResult<String> {
    let sk = hex_decode(secret_key_hex)?;
    let ct = hex_decode(ciphertext_hex)?;
    let ss = califax_crypto::pqc::kyber_decapsulate(&sk, &ct).map_err(crypto_err)?;
    Ok(hex_encode(&ss))
}

// ---------------------------------------------------------------------------
// Hybrid key exchange (X25519 + Kyber)
// ---------------------------------------------------------------------------

#[pyfunction]
fn generate_hybrid_keypair(py: Python<'_>) -> PyResult<Py<PyDict>> {
    let kp = califax_crypto::hybrid::generate_hybrid_keypair().map_err(crypto_err)?;

    let handle = serde_json::json!({
        "x25519_secret": hex_encode(&kp.x25519_secret.to_bytes()),
        "kyber_secret": hex_encode(&kp.kyber_keypair.secret_key),
    });

    let dict = PyDict::new(py);
    dict.set_item("x25519_public", hex_encode(kp.x25519_public.as_bytes()))?;
    dict.set_item("kyber_public", hex_encode(&kp.kyber_keypair.public_key))?;
    dict.set_item("secret_key_handle", handle.to_string())?;
    Ok(dict.into())
}

#[pyfunction]
fn hybrid_encapsulate(
    py: Python<'_>,
    peer_x25519_public_hex: &str,
    peer_kyber_public_hex: &str,
) -> PyResult<Py<PyDict>> {
    let peer_x25519 = hex_decode(peer_x25519_public_hex)?;
    let peer_kyber = hex_decode(peer_kyber_public_hex)?;

    let peer_x25519_arr: [u8; 32] = peer_x25519.try_into().map_err(|_| {
        PyValueError::new_err("peer X25519 public key must be exactly 32 bytes")
    })?;

    let (shared_secret, eph_pub, kyber_ct) =
        califax_crypto::hybrid::hybrid_encapsulate(&peer_x25519_arr, &peer_kyber)
            .map_err(crypto_err)?;

    let dict = PyDict::new(py);
    dict.set_item("shared_secret", hex_encode(&shared_secret.combined))?;
    dict.set_item("x25519_public", hex_encode(&eph_pub))?;
    dict.set_item("kyber_ciphertext", hex_encode(&kyber_ct))?;
    Ok(dict.into())
}

// ---------------------------------------------------------------------------
// Symmetric encryption: AES-256-GCM
// ---------------------------------------------------------------------------

#[pyfunction]
fn encrypt_aes256gcm<'py>(
    py: Python<'py>,
    key_hex: &str,
    plaintext: &[u8],
    aad: &[u8],
) -> PyResult<Py<PyDict>> {
    let key = hex_decode(key_hex)?;
    let (nonce, ct) = califax_crypto::symmetric::encrypt(
        califax_crypto::symmetric::CipherSuite::Aes256Gcm,
        &key, plaintext, aad,
    ).map_err(crypto_err)?;
    let dict = PyDict::new(py);
    dict.set_item("nonce", hex_encode(&nonce))?;
    dict.set_item("ciphertext", pyo3::types::PyBytes::new(py, &ct))?;
    Ok(dict.into())
}

#[pyfunction]
fn decrypt_aes256gcm<'py>(
    py: Python<'py>,
    key_hex: &str,
    nonce_hex: &str,
    ciphertext: &[u8],
    aad: &[u8],
) -> PyResult<Py<pyo3::types::PyBytes>> {
    let key = hex_decode(key_hex)?;
    let nonce_vec = hex_decode(nonce_hex)?;
    let nonce: [u8; 12] = nonce_vec.try_into().map_err(|_| {
        PyValueError::new_err("nonce must be exactly 12 bytes")
    })?;
    let plaintext = califax_crypto::symmetric::decrypt(
        califax_crypto::symmetric::CipherSuite::Aes256Gcm,
        &key, &nonce, ciphertext, aad,
    ).map_err(crypto_err)?;
    Ok(pyo3::types::PyBytes::new(py, &plaintext).into())
}

// ---------------------------------------------------------------------------
// Symmetric encryption: ChaCha20-Poly1305
// ---------------------------------------------------------------------------

#[pyfunction]
fn encrypt_chacha20<'py>(
    py: Python<'py>,
    key_hex: &str,
    plaintext: &[u8],
    aad: &[u8],
) -> PyResult<Py<PyDict>> {
    let key = hex_decode(key_hex)?;
    let (nonce, ct) = califax_crypto::symmetric::encrypt(
        califax_crypto::symmetric::CipherSuite::ChaCha20Poly1305,
        &key, plaintext, aad,
    ).map_err(crypto_err)?;
    let dict = PyDict::new(py);
    dict.set_item("nonce", hex_encode(&nonce))?;
    dict.set_item("ciphertext", pyo3::types::PyBytes::new(py, &ct))?;
    Ok(dict.into())
}

#[pyfunction]
fn decrypt_chacha20<'py>(
    py: Python<'py>,
    key_hex: &str,
    nonce_hex: &str,
    ciphertext: &[u8],
    aad: &[u8],
) -> PyResult<Py<pyo3::types::PyBytes>> {
    let key = hex_decode(key_hex)?;
    let nonce_vec = hex_decode(nonce_hex)?;
    let nonce: [u8; 12] = nonce_vec.try_into().map_err(|_| {
        PyValueError::new_err("nonce must be exactly 12 bytes")
    })?;
    let plaintext = califax_crypto::symmetric::decrypt(
        califax_crypto::symmetric::CipherSuite::ChaCha20Poly1305,
        &key, &nonce, ciphertext, aad,
    ).map_err(crypto_err)?;
    Ok(pyo3::types::PyBytes::new(py, &plaintext).into())
}

// ---------------------------------------------------------------------------
// Symmetric key generation
// ---------------------------------------------------------------------------

#[pyfunction]
fn generate_symmetric_key() -> PyResult<String> {
    let key = califax_crypto::symmetric::generate_key();
    Ok(hex_encode(&key))
}

// ---------------------------------------------------------------------------
// PyO3 module registration
// ---------------------------------------------------------------------------

#[pymodule]
fn califax_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let crypto = PyModule::new(m.py(), "crypto")?;

    crypto.add_function(wrap_pyfunction!(generate_kyber_keypair, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(kyber_encapsulate, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(kyber_decapsulate, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(generate_hybrid_keypair, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(hybrid_encapsulate, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(encrypt_aes256gcm, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(decrypt_aes256gcm, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(encrypt_chacha20, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(decrypt_chacha20, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(generate_symmetric_key, &crypto)?)?;

    m.add_submodule(&crypto)?;

    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("califax_core.crypto", &crypto)?;

    Ok(())
}
