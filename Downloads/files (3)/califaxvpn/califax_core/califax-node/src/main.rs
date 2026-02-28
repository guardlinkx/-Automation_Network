/// Califax VPN Node API — Rust/Axum replacement for the Python `node_api.py`.
///
/// Runs on each VPN server (default port 8443) and exposes:
///   GET    /health  — node health metrics
///   POST   /peers   — add a WireGuard peer
///   DELETE /peers   — remove a WireGuard peer
///
/// All routes are authenticated via the `X-Node-Secret` header.

mod auth;
mod config;
mod handlers;
mod wireguard;

use std::sync::Arc;
use std::time::Instant;

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, EnvFilter};

use config::NodeConfig;
use wireguard::WgManager;

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// State shared across all handlers and middleware via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<NodeConfig>,
    pub wg_manager: Arc<WgManager>,
    pub start_time: Instant,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Initialise structured logging (RUST_LOG env var controls verbosity).
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = NodeConfig::from_env();
    let listen_addr = config.listen_addr.clone();

    tracing::info!(
        region = %config.vpn_region,
        listen = %listen_addr,
        "Starting califax-node API server"
    );

    let state = AppState {
        config: Arc::new(config),
        wg_manager: Arc::new(WgManager::new()),
        start_time: Instant::now(),
    };

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/peers", post(handlers::create_peer))
        .route("/peers", delete(handlers::delete_peer))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_secret,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("Failed to bind to listen address");

    tracing::info!("Listening on {}", listen_addr);

    axum::serve(listener, app)
        .await
        .expect("Server exited with error");
}
