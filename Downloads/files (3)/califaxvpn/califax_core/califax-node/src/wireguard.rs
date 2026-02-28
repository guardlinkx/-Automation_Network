/// WireGuard CLI wrapper — Rust equivalent of the Python `wg_manager.py`.
///
/// Manages peers on a WireGuard interface (`wg0`) by shelling out to the `wg`
/// and `wg-quick` commands and persisting the peer-to-IP mapping in a JSON
/// file.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum WgError {
    #[error("No available IPs in subnet")]
    SubnetExhausted,
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type WgResult<T> = Result<T, WgError>;

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

/// Returned on successful peer creation — matches the Python dict.
#[derive(Debug, Serialize, Deserialize)]
pub struct PeerResult {
    pub client_ip: String,
    pub server_pubkey: String,
    pub endpoint: String,
}

/// Returned from `get_status()`.
#[derive(Debug, Serialize, Deserialize)]
pub struct WgStatus {
    pub interface: String,
    pub connected_peers: usize,
    pub allocated_peers: usize,
}

// ---------------------------------------------------------------------------
// WgManager
// ---------------------------------------------------------------------------

pub struct WgManager {
    /// WireGuard interface name (e.g. "wg0").
    interface: String,
    /// Subnet in CIDR — e.g. "10.100.0.0/24".
    subnet: Ipv4Addr,
    prefix_len: u8,
    /// Server's own address inside the tunnel (e.g. "10.100.0.1").
    server_ip: Ipv4Addr,
    /// Path to the JSON file that persists pubkey -> allocated IP mappings.
    peers_file: PathBuf,
}

impl WgManager {
    /// Create a manager with the same defaults as the Python code.
    pub fn new() -> Self {
        Self {
            interface: "wg0".to_string(),
            subnet: Ipv4Addr::new(10, 100, 0, 0),
            prefix_len: 24,
            server_ip: Ipv4Addr::new(10, 100, 0, 1),
            peers_file: PathBuf::from("/etc/wireguard/peers.json"),
        }
    }

    // -- Peer persistence ---------------------------------------------------

    /// Load the pubkey -> IP map from disk. Returns empty map if file missing.
    fn load_peers(&self) -> WgResult<HashMap<String, String>> {
        if !self.peers_file.exists() {
            return Ok(HashMap::new());
        }
        let data = std::fs::read_to_string(&self.peers_file)?;
        let map: HashMap<String, String> = serde_json::from_str(&data)?;
        Ok(map)
    }

    /// Persist the pubkey -> IP map to disk.
    fn save_peers(&self, peers: &HashMap<String, String>) -> WgResult<()> {
        let json = serde_json::to_string_pretty(peers)?;
        std::fs::write(&self.peers_file, json)?;
        Ok(())
    }

    // -- IP allocation ------------------------------------------------------

    /// Iterate through all host addresses in the subnet, skip the server IP
    /// and any already-allocated IPs, and return the first available one.
    fn next_available_ip(&self) -> WgResult<Ipv4Addr> {
        let peers = self.load_peers()?;
        let used: std::collections::HashSet<String> = peers
            .values()
            .cloned()
            .chain(std::iter::once(self.server_ip.to_string()))
            .collect();

        let base = u32::from(self.subnet);
        let host_bits = 32 - self.prefix_len;
        let total_hosts = (1u32 << host_bits) - 2; // exclude network & broadcast

        for i in 1..=total_hosts {
            let candidate = Ipv4Addr::from(base + i);
            if !used.contains(&candidate.to_string()) {
                return Ok(candidate);
            }
        }

        Err(WgError::SubnetExhausted)
    }

    // -- WireGuard operations -----------------------------------------------

    /// Add a peer. Allocates an IP, runs `wg set`, persists, saves config.
    pub fn add_peer(&self, client_pubkey: &str) -> WgResult<PeerResult> {
        let client_ip = self.next_available_ip()?;

        // wg set wg0 peer <pubkey> allowed-ips <ip>/32
        run_cmd(&[
            "wg",
            "set",
            &self.interface,
            "peer",
            client_pubkey,
            "allowed-ips",
            &format!("{}/32", client_ip),
        ])?;

        // Persist mapping
        let mut peers = self.load_peers()?;
        peers.insert(client_pubkey.to_string(), client_ip.to_string());
        self.save_peers(&peers)?;

        // Sync to conf for reboot persistence
        run_cmd(&["wg-quick", "save", &self.interface])?;

        let server_pubkey = self.get_server_pubkey()?;
        let public_ip = self.get_public_ip();

        Ok(PeerResult {
            client_ip: format!("{}/32", client_ip),
            server_pubkey,
            endpoint: format!("{}:51820", public_ip),
        })
    }

    /// Remove a peer by public key.
    pub fn remove_peer(&self, client_pubkey: &str) -> WgResult<()> {
        run_cmd(&[
            "wg",
            "set",
            &self.interface,
            "peer",
            client_pubkey,
            "remove",
        ])?;

        let mut peers = self.load_peers()?;
        peers.remove(client_pubkey);
        self.save_peers(&peers)?;

        run_cmd(&["wg-quick", "save", &self.interface])?;
        Ok(())
    }

    /// Query `wg show` for connected peer count and return overall status.
    pub fn get_status(&self) -> WgStatus {
        let connected_peers = match run_cmd(&["wg", "show", &self.interface, "dump"]) {
            Ok(output) => {
                let lines: Vec<&str> = output.trim().split('\n').collect();
                // First line is interface info; remaining lines are peers.
                if lines.len() > 1 { lines.len() - 1 } else { 0 }
            }
            Err(_) => 0,
        };

        let allocated_peers = self.load_peers().map(|p| p.len()).unwrap_or(0);

        WgStatus {
            interface: self.interface.clone(),
            connected_peers,
            allocated_peers,
        }
    }

    /// Derive the server public key from the private key file.
    pub fn get_server_pubkey(&self) -> WgResult<String> {
        let privkey_path = Path::new("/etc/wireguard/server_private.key");
        let privkey = std::fs::read_to_string(privkey_path)
            .map_err(|e| WgError::Io(e))?;

        let output = Command::new("wg")
            .arg("pubkey")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(privkey.trim().as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|e| WgError::CommandFailed(format!("wg pubkey: {}", e)))?;

        if !output.status.success() {
            return Err(WgError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Fetch the node's public IP via AWS EC2 IMDSv2 metadata.
    /// Falls back to "0.0.0.0" on any failure, and caches the result on disk.
    pub fn get_public_ip(&self) -> String {
        let cache_path = Path::new("/tmp/.califax_public_ip");

        // Return cached value if present.
        if let Ok(cached) = std::fs::read_to_string(cache_path) {
            let ip = cached.trim().to_string();
            if !ip.is_empty() {
                return ip;
            }
        }

        // IMDSv2: acquire a session token, then query the metadata endpoint.
        let token = Command::new("curl")
            .args([
                "-s",
                "-X",
                "PUT",
                "http://169.254.169.254/latest/api/token",
                "-H",
                "X-aws-ec2-metadata-token-ttl-seconds: 300",
            ])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            });

        let ip = token
            .and_then(|t| {
                Command::new("curl")
                    .args([
                        "-s",
                        "http://169.254.169.254/latest/meta-data/public-ipv4",
                        "-H",
                        &format!("X-aws-ec2-metadata-token: {}", t),
                    ])
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() {
                            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                        } else {
                            None
                        }
                    })
            })
            .unwrap_or_else(|| "0.0.0.0".to_string());

        // Best-effort cache write.
        let _ = std::fs::write(cache_path, &ip);

        ip
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run an external command, returning its stdout on success.
fn run_cmd(args: &[&str]) -> WgResult<String> {
    let output = Command::new(args[0])
        .args(&args[1..])
        .output()
        .map_err(|e| WgError::CommandFailed(format!("{}: {}", args[0], e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WgError::CommandFailed(format!(
            "{} exited with {}: {}",
            args.join(" "),
            output.status,
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
