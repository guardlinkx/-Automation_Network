/// Route handlers — Rust equivalent of the Flask routes in `node_api.py`.
///
/// Every handler receives `AppState` via Axum's `State` extractor and
/// delegates WireGuard operations to `WgManager`.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreatePeerRequest {
    pub client_pubkey: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeletePeerRequest {
    pub client_pubkey: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub region: String,
    pub uptime_seconds: u64,
    pub connected_peers: usize,
    pub allocated_peers: usize,
    pub cpu_percent: f64,
    pub memory_percent: f64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /health
///
/// Returns node health metrics including WireGuard peer counts, uptime, and
/// basic resource usage. Mirrors the Python `/health` endpoint.
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let wg_status = state.wg_manager.get_status();
    let uptime = state.start_time.elapsed().as_secs();

    // CPU and memory are best-effort; on failure we report 0.0.
    let (cpu_percent, memory_percent) = read_system_metrics();

    let resp = HealthResponse {
        status: "healthy".to_string(),
        region: state.config.vpn_region.clone(),
        uptime_seconds: uptime,
        connected_peers: wg_status.connected_peers,
        allocated_peers: wg_status.allocated_peers,
        cpu_percent,
        memory_percent,
    };

    (StatusCode::OK, Json(resp))
}

/// POST /peers
///
/// Expects JSON body `{ "client_pubkey": "<base64>" }`.
/// Allocates a tunnel IP, configures WireGuard, and returns the connection
/// details. Matches the Python 201 / 400 / 503 / 500 responses exactly.
pub async fn create_peer(
    State(state): State<AppState>,
    Json(payload): Json<CreatePeerRequest>,
) -> impl IntoResponse {
    let client_pubkey = match payload.client_pubkey {
        Some(ref k) if !k.is_empty() => k.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "client_pubkey required" })),
            )
                .into_response();
        }
    };

    match state.wg_manager.add_peer(&client_pubkey) {
        Ok(result) => (
            StatusCode::CREATED,
            Json(json!({
                "client_ip": result.client_ip,
                "server_pubkey": result.server_pubkey,
                "endpoint": result.endpoint,
            })),
        )
            .into_response(),
        Err(crate::wireguard::WgError::SubnetExhausted) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "No available IPs in subnet" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to add peer: {}", e) })),
        )
            .into_response(),
    }
}

/// DELETE /peers
///
/// Expects JSON body `{ "client_pubkey": "<base64>" }`.
/// Removes the peer from WireGuard and the persistent mapping.
pub async fn delete_peer(
    State(state): State<AppState>,
    Json(payload): Json<DeletePeerRequest>,
) -> impl IntoResponse {
    let client_pubkey = match payload.client_pubkey {
        Some(ref k) if !k.is_empty() => k.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "client_pubkey required" })),
            )
                .into_response();
        }
    };

    match state.wg_manager.remove_peer(&client_pubkey) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "removed": client_pubkey })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to remove peer: {}", e) })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// System metrics helper
// ---------------------------------------------------------------------------

/// Read CPU and memory utilisation from `/proc` (Linux only).
/// Returns `(cpu_percent, memory_percent)`. Falls back to `(0.0, 0.0)`.
fn read_system_metrics() -> (f64, f64) {
    let cpu = read_cpu_percent().unwrap_or(0.0);
    let mem = read_memory_percent().unwrap_or(0.0);
    (cpu, mem)
}

/// Parse `/proc/stat` twice (with a short sleep) to compute a rough CPU%.
fn read_cpu_percent() -> Option<f64> {
    fn parse_cpu_line() -> Option<(u64, u64)> {
        let stat = std::fs::read_to_string("/proc/stat").ok()?;
        let line = stat.lines().next()?;
        let parts: Vec<u64> = line
            .split_whitespace()
            .skip(1) // skip "cpu"
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() < 4 {
            return None;
        }
        let idle = parts[3];
        let total: u64 = parts.iter().sum();
        Some((idle, total))
    }

    let (idle1, total1) = parse_cpu_line()?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    let (idle2, total2) = parse_cpu_line()?;

    let idle_delta = idle2.saturating_sub(idle1) as f64;
    let total_delta = total2.saturating_sub(total1) as f64;
    if total_delta == 0.0 {
        return Some(0.0);
    }
    Some(((total_delta - idle_delta) / total_delta) * 100.0)
}

/// Parse `/proc/meminfo` to compute memory usage percentage.
fn read_memory_percent() -> Option<f64> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total: Option<u64> = None;
    let mut available: Option<u64> = None;
    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            total = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
        } else if line.starts_with("MemAvailable:") {
            available = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
        }
        if total.is_some() && available.is_some() {
            break;
        }
    }
    let t = total? as f64;
    let a = available? as f64;
    if t == 0.0 {
        return Some(0.0);
    }
    Some(((t - a) / t) * 100.0)
}
