//! Traffic obfuscation layers for deep-packet-inspection (DPI) evasion.
//!
//! Provides multiple obfuscation modes that can be layered on top of any
//! tunnel protocol to disguise VPN traffic as innocuous HTTPS, HTTP, or
//! randomised data.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::error::TunnelError;
use crate::Result;

// ---------------------------------------------------------------------------
// ObfuscationMode
// ---------------------------------------------------------------------------

/// Available traffic obfuscation strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObfuscationMode {
    /// No obfuscation -- pass data through unchanged.
    None,
    /// XOR every byte with a repeating key pattern.
    XorScramble,
    /// Prepend a TLS 1.3 Application Data record header so the payload
    /// looks like ordinary HTTPS traffic to a passive observer.
    TlsMimicry,
    /// Base64-encode the payload and wrap it inside an HTTP POST body so
    /// it appears to be a normal web request.
    HttpMasquerade,
    /// Califax Chameleon mode -- combines XOR scramble, random padding, and
    /// a TLS record header for maximum DPI evasion.
    Chameleon,
}

// ---------------------------------------------------------------------------
// ObfuscationLayer
// ---------------------------------------------------------------------------

/// A configured obfuscation layer that can wrap and unwrap tunnel payloads.
#[derive(Debug, Clone)]
pub struct ObfuscationLayer {
    /// The obfuscation strategy to apply.
    pub mode: ObfuscationMode,
    /// Optional key used by modes that require one (e.g. XorScramble,
    /// Chameleon).  Must be non-empty when the mode needs it.
    pub key: Option<Vec<u8>>,
}

impl ObfuscationLayer {
    /// Create a new obfuscation layer with the given mode and optional key.
    pub fn new(mode: ObfuscationMode, key: Option<Vec<u8>>) -> Self {
        Self { mode, key }
    }

    /// Create a no-op (passthrough) layer.
    pub fn none() -> Self {
        Self {
            mode: ObfuscationMode::None,
            key: None,
        }
    }

    // -- public API ---------------------------------------------------------

    /// Wrap (obfuscate) outgoing tunnel data.
    pub fn wrap(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.mode {
            ObfuscationMode::None => Ok(data.to_vec()),
            ObfuscationMode::XorScramble => self.xor_scramble(data),
            ObfuscationMode::TlsMimicry => Self::tls_mimicry_wrap(data),
            ObfuscationMode::HttpMasquerade => Self::http_masquerade_wrap(data),
            ObfuscationMode::Chameleon => self.chameleon_wrap(data),
        }
    }

    /// Unwrap (de-obfuscate) incoming tunnel data.
    pub fn unwrap(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.mode {
            ObfuscationMode::None => Ok(data.to_vec()),
            ObfuscationMode::XorScramble => self.xor_scramble(data), // XOR is its own inverse
            ObfuscationMode::TlsMimicry => Self::tls_mimicry_unwrap(data),
            ObfuscationMode::HttpMasquerade => Self::http_masquerade_unwrap(data),
            ObfuscationMode::Chameleon => self.chameleon_unwrap(data),
        }
    }

    // -- XOR scramble -------------------------------------------------------

    /// XOR every byte of `data` with the repeating key pattern.
    fn xor_scramble(&self, data: &[u8]) -> Result<Vec<u8>> {
        let key = self
            .key
            .as_ref()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| {
                TunnelError::ObfuscationFailed(
                    "XorScramble requires a non-empty key".into(),
                )
            })?;

        let out: Vec<u8> = data
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect();
        Ok(out)
    }

    // -- TLS 1.3 mimicry ----------------------------------------------------

    /// TLS 1.3 Application Data content type.
    const TLS_CONTENT_TYPE: u8 = 0x17;
    /// TLS 1.2 version bytes (used in the record layer even for TLS 1.3).
    const TLS_VERSION: [u8; 2] = [0x03, 0x03];
    /// Size of the prepended TLS record header.
    const TLS_HEADER_LEN: usize = 5;

    /// Prepend a TLS 1.3 record header: content_type(1) | version(2) | length(2).
    fn tls_mimicry_wrap(data: &[u8]) -> Result<Vec<u8>> {
        let len = data.len();
        if len > 0xFFFF {
            return Err(TunnelError::ObfuscationFailed(
                "payload too large for a single TLS record".into(),
            ));
        }
        let mut out = Vec::with_capacity(Self::TLS_HEADER_LEN + len);
        out.push(Self::TLS_CONTENT_TYPE);
        out.extend_from_slice(&Self::TLS_VERSION);
        out.push((len >> 8) as u8);
        out.push((len & 0xFF) as u8);
        out.extend_from_slice(data);
        Ok(out)
    }

    /// Strip the TLS record header and return the inner payload.
    fn tls_mimicry_unwrap(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < Self::TLS_HEADER_LEN {
            return Err(TunnelError::ObfuscationFailed(
                "data too short to contain a TLS record header".into(),
            ));
        }
        if data[0] != Self::TLS_CONTENT_TYPE
            || data[1] != Self::TLS_VERSION[0]
            || data[2] != Self::TLS_VERSION[1]
        {
            return Err(TunnelError::ObfuscationFailed(
                "invalid TLS record header".into(),
            ));
        }
        let payload_len = ((data[3] as usize) << 8) | (data[4] as usize);
        let payload = &data[Self::TLS_HEADER_LEN..];
        if payload.len() < payload_len {
            return Err(TunnelError::ObfuscationFailed(
                "TLS record length exceeds available data".into(),
            ));
        }
        Ok(payload[..payload_len].to_vec())
    }

    // -- HTTP masquerade ----------------------------------------------------

    /// HTTP POST request prefix used to wrap the payload.
    const HTTP_PREFIX: &'static str =
        "POST /api/v1/telemetry HTTP/1.1\r\n\
         Host: cdn.cloudflare-dns.com\r\n\
         Content-Type: application/octet-stream\r\n\
         Connection: keep-alive\r\n\
         Content-Length: ";

    /// Marker that separates HTTP headers from the body.
    const HTTP_BODY_SEP: &'static str = "\r\n\r\n";

    /// Base64-encode the payload and wrap it in an HTTP POST body.
    fn http_masquerade_wrap(data: &[u8]) -> Result<Vec<u8>> {
        use base64_encode as b64;
        let encoded = b64(data);
        let header = format!("{}{}{}", Self::HTTP_PREFIX, encoded.len(), Self::HTTP_BODY_SEP);
        let mut out = Vec::with_capacity(header.len() + encoded.len());
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(encoded.as_bytes());
        Ok(out)
    }

    /// Extract the base64-encoded body from an HTTP POST wrapper and decode.
    fn http_masquerade_unwrap(data: &[u8]) -> Result<Vec<u8>> {
        let text = std::str::from_utf8(data).map_err(|_| {
            TunnelError::ObfuscationFailed("HTTP masquerade data is not valid UTF-8".into())
        })?;

        let body_start = text.find("\r\n\r\n").ok_or_else(|| {
            TunnelError::ObfuscationFailed("cannot find HTTP body separator".into())
        })? + 4;

        let body = &text[body_start..];
        base64_decode(body)
    }

    // -- Chameleon ----------------------------------------------------------

    /// Chameleon wire format:
    /// ```text
    /// [TLS header (5)] [padding_len (1)] [random padding (N)] [XOR'd payload]
    /// ```
    fn chameleon_wrap(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Step 1: XOR the payload
        let xored = self.xor_scramble(data)?;

        // Step 2: Generate random padding (1..=32 bytes)
        let mut rng = rand::thread_rng();
        let pad_len: u8 = rng.gen_range(1..=32);
        let mut padding = vec![0u8; pad_len as usize];
        rng.fill(padding.as_mut_slice());

        // Step 3: Build inner frame = padding_len(1) || padding || xored_payload
        let inner_len = 1 + padding.len() + xored.len();
        let mut inner = Vec::with_capacity(inner_len);
        inner.push(pad_len);
        inner.extend_from_slice(&padding);
        inner.extend_from_slice(&xored);

        // Step 4: Wrap in TLS record header
        Self::tls_mimicry_wrap(&inner)
    }

    /// Unwrap a Chameleon frame.
    fn chameleon_unwrap(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Step 1: Strip TLS header
        let inner = Self::tls_mimicry_unwrap(data)?;

        if inner.is_empty() {
            return Err(TunnelError::ObfuscationFailed(
                "Chameleon frame is empty after TLS unwrap".into(),
            ));
        }

        // Step 2: Read padding length and skip padding
        let pad_len = inner[0] as usize;
        let payload_start = 1 + pad_len;
        if payload_start > inner.len() {
            return Err(TunnelError::ObfuscationFailed(
                "Chameleon padding length exceeds frame".into(),
            ));
        }
        let xored_payload = &inner[payload_start..];

        // Step 3: Reverse XOR
        self.xor_scramble(xored_payload)
    }
}

// ---------------------------------------------------------------------------
// Minimal base64 helpers (avoid pulling in a whole crate just for this)
// ---------------------------------------------------------------------------

/// Base64 alphabet (standard).
const B64_CHARS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode bytes to base64 string.
fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(B64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(B64_CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Decode a base64 string back to bytes.
fn base64_decode(encoded: &str) -> Result<Vec<u8>> {
    let encoded = encoded.trim_end_matches('=');
    let mut out = Vec::with_capacity(encoded.len() * 3 / 4);

    let val = |c: u8| -> Result<u32> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(TunnelError::ObfuscationFailed(format!(
                "invalid base64 character: {c}"
            ))),
        }
    };

    let bytes = encoded.as_bytes();
    let chunks = bytes.chunks(4);
    for chunk in chunks {
        let a = val(chunk[0])?;
        let b = if chunk.len() > 1 { val(chunk[1])? } else { 0 };
        let c = if chunk.len() > 2 { val(chunk[2])? } else { 0 };
        let d = if chunk.len() > 3 { val(chunk[3])? } else { 0 };

        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((triple >> 16) as u8);
        if chunk.len() > 2 {
            out.push((triple >> 8 & 0xFF) as u8);
        }
        if chunk.len() > 3 {
            out.push((triple & 0xFF) as u8);
        }
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> Vec<u8> {
        vec![0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89]
    }

    #[test]
    fn xor_roundtrip() {
        let layer = ObfuscationLayer::new(ObfuscationMode::XorScramble, Some(test_key()));
        let original = b"Hello, Califax VPN tunnel!";
        let wrapped = layer.wrap(original).unwrap();
        assert_ne!(&wrapped, original);
        let unwrapped = layer.unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, original);
    }

    #[test]
    fn tls_mimicry_roundtrip() {
        let layer = ObfuscationLayer::new(ObfuscationMode::TlsMimicry, None);
        let original = b"secret payload";
        let wrapped = layer.wrap(original).unwrap();
        // Should start with TLS record header
        assert_eq!(wrapped[0], 0x17);
        assert_eq!(wrapped[1], 0x03);
        assert_eq!(wrapped[2], 0x03);
        let unwrapped = layer.unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, original);
    }

    #[test]
    fn http_masquerade_roundtrip() {
        let layer = ObfuscationLayer::new(ObfuscationMode::HttpMasquerade, None);
        let original = b"covert data";
        let wrapped = layer.wrap(original).unwrap();
        let wrapped_str = String::from_utf8(wrapped.clone()).unwrap();
        assert!(wrapped_str.starts_with("POST /api/v1/telemetry HTTP/1.1"));
        let unwrapped = layer.unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, original);
    }

    #[test]
    fn chameleon_roundtrip() {
        let layer = ObfuscationLayer::new(ObfuscationMode::Chameleon, Some(test_key()));
        let original = b"chameleon test payload 1234567890";
        let wrapped = layer.wrap(original).unwrap();
        // Should be wrapped in TLS
        assert_eq!(wrapped[0], 0x17);
        let unwrapped = layer.unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, original);
    }

    #[test]
    fn none_passthrough() {
        let layer = ObfuscationLayer::none();
        let data = b"passthrough";
        let wrapped = layer.wrap(data).unwrap();
        assert_eq!(wrapped, data);
        let unwrapped = layer.unwrap(data).unwrap();
        assert_eq!(unwrapped, data);
    }

    #[test]
    fn xor_without_key_fails() {
        let layer = ObfuscationLayer::new(ObfuscationMode::XorScramble, None);
        assert!(layer.wrap(b"test").is_err());
    }
}
