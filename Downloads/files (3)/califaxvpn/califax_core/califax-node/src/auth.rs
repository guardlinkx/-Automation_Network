/// Axum middleware that enforces the `X-Node-Secret` header.
///
/// Mirrors the `require_secret` decorator from the Python `node_api.py`.
/// Any request whose header value does not match `NODE_API_SECRET` receives a
/// 401 Unauthorized response with `{"error": "Unauthorized"}`.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::AppState;

/// Middleware layer: reject requests that do not carry the correct secret.
pub async fn require_secret(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let header_value = request
        .headers()
        .get("X-Node-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if header_value != state.config.node_api_secret {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Unauthorized" })),
        )
            .into_response();
    }

    next.run(request).await
}
