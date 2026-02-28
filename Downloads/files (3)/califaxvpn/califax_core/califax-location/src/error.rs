use thiserror::Error;

#[derive(Error, Debug)]
pub enum LocationError {
    #[error("GPS spoofing failed: {0}")]
    SpoofFailed(String),
    #[error("Location firewall blocked access: {0}")]
    FirewallBlocked(String),
    #[error("Wi-Fi masking failed: {0}")]
    WifiMaskFailed(String),
    #[error("Invalid coordinates: lat={lat}, lon={lon}")]
    InvalidCoordinates { lat: f64, lon: f64 },
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}
