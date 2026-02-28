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
    // Accept both upper and lower-case hex, strip optional leading "0x"
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

/// Generate a Kyber-1024 key pair.
///
/// Returns a dict with keys ``public_key`` and ``secret_key``, both hex-encoded.
#[pyfunction]
fn generate_kyber_keypair(py: Python<'_>) -> PyResult<Py<PyDict>> {
    let kp = califax_crypto::pqc::generate_kyber_keypair().map_err(crypto_err)?;
    let dict = PyDict::new(py);
    dict.set_item("public_key", hex_encode(&kp.public_key))?;
    dict.set_item("secret_key", hex_encode(&kp.secret_key))?;
    Ok(dict.into())
}

/// Encapsulate a shared secret using a peer's Kyber-1024 public key (hex).
///
/// Returns a dict with ``shared_secret`` (hex) and ``ciphertext`` (hex).
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

/// Decapsulate a shared secret given the local secret key (hex) and ciphertext (hex).
///
/// Returns the shared secret as a hex string.
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

/// Generate a hybrid X25519 + Kyber-1024 key pair.
///
/// Returns a dict with ``x25519_public`` (hex), ``kyber_public`` (hex), and
/// ``secret_key_handle`` (str -- opaque JSON blob holding both secret keys).
#[pyfunction]
fn generate_hybrid_keypair(py: Python<'_>) -> PyResult<Py<PyDict>> {
    let kp = califax_crypto::hybrid::generate_hybrid_keypair().map_err(crypto_err)?;

    // Encode the secret key material as an opaque JSON handle so the caller
    // can pass it back for decapsulation without interpreting it.
    let handle = serde_json::json!({
        "x25519_secret": hex_encode(&kp.x25519_secret),
        "kyber_secret": hex_encode(&kp.kyber_secret),
    });

    let dict = PyDict::new(py);
    dict.set_item("x25519_public", hex_encode(&kp.x25519_public))?;
    dict.set_item("kyber_public", hex_encode(&kp.kyber_public))?;
    dict.set_item("secret_key_handle", handle.to_string())?;
    Ok(dict.into())
}

/// Perform hybrid encapsulation toward a peer.
///
/// Arguments:
///   - ``peer_x25519_public_hex``: the peer's X25519 public key (hex)
///   - ``peer_kyber_public_hex``:  the peer's Kyber-1024 public key (hex)
///
/// Returns a dict with:
///   - ``shared_secret`` (hex) -- the combined HKDF-derived shared secret
///   - ``x25519_public`` (hex) -- ephemeral X25519 public key to send to peer
///   - ``kyber_ciphertext`` (hex) -- Kyber ciphertext to send to peer
#[pyfunction]
fn hybrid_encapsulate(
    py: Python<'_>,
    peer_x25519_public_hex: &str,
    peer_kyber_public_hex: &str,
) -> PyResult<Py<PyDict>> {
    let peer_x25519 = hex_decode(peer_x25519_public_hex)?;
    let peer_kyber = hex_decode(peer_kyber_public_hex)?;

    let result = califax_crypto::hybrid::hybrid_encapsulate(&peer_x25519, &peer_kyber)
        .map_err(crypto_err)?;

    let dict = PyDict::new(py);
    dict.set_item("shared_secret", hex_encode(&result.shared_secret))?;
    dict.set_item("x25519_public", hex_encode(&result.x25519_public))?;
    dict.set_item("kyber_ciphertext", hex_encode(&result.kyber_ciphertext))?;
    Ok(dict.into())
}

// ---------------------------------------------------------------------------
// Symmetric encryption: AES-256-GCM
// ---------------------------------------------------------------------------

/// Encrypt with AES-256-GCM.
///
/// Arguments:
///   - ``key_hex``: 256-bit key as hex
///   - ``plaintext``: plaintext bytes
///   - ``aad``: additional authenticated data (bytes)
///
/// Returns a dict with ``nonce`` (hex) and ``ciphertext`` (bytes).
#[pyfunction]
fn encrypt_aes256gcm<'py>(
    py: Python<'py>,
    key_hex: &str,
    plaintext: &[u8],
    aad: &[u8],
) -> PyResult<Py<PyDict>> {
    let key = hex_decode(key_hex)?;
    let result = califax_crypto::symmetric::encrypt_aes256gcm(&key, plaintext, aad)
        .map_err(crypto_err)?;
    let dict = PyDict::new(py);
    dict.set_item("nonce", hex_encode(&result.nonce))?;
    dict.set_item("ciphertext", pyo3::types::PyBytes::new(py, &result.ciphertext))?;
    Ok(dict.into())
}

/// Decrypt with AES-256-GCM.
///
/// Arguments:
///   - ``key_hex``: 256-bit key as hex
///   - ``nonce_hex``: nonce/IV as hex
///   - ``ciphertext``: ciphertext bytes (including auth tag)
///   - ``aad``: additional authenticated data (bytes)
///
/// Returns the decrypted plaintext bytes.
#[pyfunction]
fn decrypt_aes256gcm<'py>(
    py: Python<'py>,
    key_hex: &str,
    nonce_hex: &str,
    ciphertext: &[u8],
    aad: &[u8],
) -> PyResult<Py<pyo3::types::PyBytes>> {
    let key = hex_decode(key_hex)?;
    let nonce = hex_decode(nonce_hex)?;
    let plaintext = califax_crypto::symmetric::decrypt_aes256gcm(&key, &nonce, ciphertext, aad)
        .map_err(crypto_err)?;
    Ok(pyo3::types::PyBytes::new(py, &plaintext).into())
}

// ---------------------------------------------------------------------------
// Symmetric encryption: ChaCha20-Poly1305
// ---------------------------------------------------------------------------

/// Encrypt with ChaCha20-Poly1305.
///
/// Arguments:
///   - ``key_hex``: 256-bit key as hex
///   - ``plaintext``: plaintext bytes
///   - ``aad``: additional authenticated data (bytes)
///
/// Returns a dict with ``nonce`` (hex) and ``ciphertext`` (bytes).
#[pyfunction]
fn encrypt_chacha20<'py>(
    py: Python<'py>,
    key_hex: &str,
    plaintext: &[u8],
    aad: &[u8],
) -> PyResult<Py<PyDict>> {
    let key = hex_decode(key_hex)?;
    let result = califax_crypto::symmetric::encrypt_chacha20(&key, plaintext, aad)
        .map_err(crypto_err)?;
    let dict = PyDict::new(py);
    dict.set_item("nonce", hex_encode(&result.nonce))?;
    dict.set_item("ciphertext", pyo3::types::PyBytes::new(py, &result.ciphertext))?;
    Ok(dict.into())
}

/// Decrypt with ChaCha20-Poly1305.
///
/// Arguments:
///   - ``key_hex``: 256-bit key as hex
///   - ``nonce_hex``: nonce/IV as hex
///   - ``ciphertext``: ciphertext bytes (including auth tag)
///   - ``aad``: additional authenticated data (bytes)
///
/// Returns the decrypted plaintext bytes.
#[pyfunction]
fn decrypt_chacha20<'py>(
    py: Python<'py>,
    key_hex: &str,
    nonce_hex: &str,
    ciphertext: &[u8],
    aad: &[u8],
) -> PyResult<Py<pyo3::types::PyBytes>> {
    let key = hex_decode(key_hex)?;
    let nonce = hex_decode(nonce_hex)?;
    let plaintext = califax_crypto::symmetric::decrypt_chacha20(&key, &nonce, ciphertext, aad)
        .map_err(crypto_err)?;
    Ok(pyo3::types::PyBytes::new(py, &plaintext).into())
}

// ---------------------------------------------------------------------------
// Symmetric key generation
// ---------------------------------------------------------------------------

/// Generate a random 256-bit symmetric key.
///
/// Returns the key as a 64-character hex string.
#[pyfunction]
fn generate_symmetric_key() -> PyResult<String> {
    let key = califax_crypto::symmetric::generate_symmetric_key().map_err(crypto_err)?;
    Ok(hex_encode(&key))
}

// ---------------------------------------------------------------------------
// PyO3 module registration
// ---------------------------------------------------------------------------

/// The top-level `califax_core` Python module.
///
/// Exposes a `crypto` submodule containing all cryptographic functions.
#[pymodule]
fn califax_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let crypto = PyModule::new(m.py(), "crypto")?;

    // Kyber (PQC KEM)
    crypto.add_function(wrap_pyfunction!(generate_kyber_keypair, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(kyber_encapsulate, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(kyber_decapsulate, &crypto)?)?;

    // Hybrid key exchange
    crypto.add_function(wrap_pyfunction!(generate_hybrid_keypair, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(hybrid_encapsulate, &crypto)?)?;

    // AES-256-GCM
    crypto.add_function(wrap_pyfunction!(encrypt_aes256gcm, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(decrypt_aes256gcm, &crypto)?)?;

    // ChaCha20-Poly1305
    crypto.add_function(wrap_pyfunction!(encrypt_chacha20, &crypto)?)?;
    crypto.add_function(wrap_pyfunction!(decrypt_chacha20, &crypto)?)?;

    // Key generation
    crypto.add_function(wrap_pyfunction!(generate_symmetric_key, &crypto)?)?;

    m.add_submodule(&crypto)?;

    // Allow `from califax_core.crypto import ...` to work properly.
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("califax_core.crypto", &crypto)?;

    Ok(())
}
