/// Node configuration loaded from environment variables.
///
/// Mirrors the env-var contract from the Python `node_api.py`:
///   - NODE_API_SECRET  (default: "change-me-in-production")
///   - VPN_REGION       (default: "us-east-1")
///   - LISTEN_ADDR      (default: "0.0.0.0:8443")

#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Shared secret that callers must send in the `X-Node-Secret` header.
    pub node_api_secret: String,
    /// AWS-style region tag reported in /health responses.
    pub vpn_region: String,
    /// Socket address the HTTP server binds to.
    pub listen_addr: String,
}

impl NodeConfig {
    /// Build a `NodeConfig` from the current process environment.
    pub fn from_env() -> Self {
        Self {
            node_api_secret: std::env::var("NODE_API_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            vpn_region: std::env::var("VPN_REGION")
                .unwrap_or_else(|_| "us-east-1".to_string()),
            listen_addr: std::env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8443".to_string()),
        }
    }
}
